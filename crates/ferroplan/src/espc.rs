//! ESPC — Extended Saddle-Point Condition penalty-resolution outer loop.
//!
//! SGPlan's defining contribution on top of Metric-FF (Wah & Chen; Hsu, Wah,
//! Huang & Chen, IPC-2006): coordinate cross-partition / global constraints by a
//! **finite penalty-update loop** that re-solves under fixed penalties, raises the
//! penalty on still-violated constraints, and converges to an *extended saddle
//! point* (a constrained local minimum) — keeping the best plan as an anytime
//! incumbent. See docs/sgplan6-spec.md §4 and ferroplan/docs/espc-preferences-spec.md.
//!
//! Here the "global constraint" is the openstacks make/start coordination: a
//! once-only conditional achievement (`make-product`) that fires while its enabling
//! orders are still `waiting` permanently forfeits the delivery preference. The
//! delete-relaxed RPG is blind to this (it can re-add the deliverable), so the
//! penalty is read on the CONCRETE state (see [`crate::search::SatGuidance::deadline`]).
//! The loop adapts a **per-trigger** penalty `λ[M]` — raising it only on the products
//! whose deliveries were actually missed — so it auto-tunes per instance, which a
//! single fixed penalty (the Phase-0 `FF_DEADLINE_WEIGHT` lever) cannot. Penalty
//! multipliers are SEPARATE from preference weights: weights compute the reported
//! metric, λ only reorders the search (never changing legality or the metric).
//!
//! Anytime + monotone-ascent: λ never decreases, but the returned plan is the best
//! seen across all iterations, so an overshooting λ can never regress the result.

use std::time::{Duration, Instant};

use crate::hash::{FxHashMap, FxHashSet};
use crate::packed::PackedTask;
use crate::partition::{merge_at, merge_with_neighbor, Subgoal};
use crate::pddl3::{apply_tail, plan_cost, PhaseTail};
use crate::search::{solve_subgoal_bounded, solve_subgoal_guided, SatGuidance, SearchCfg};

pub struct EspcResult {
    pub ops: Vec<usize>,
    pub cost: f64,
    pub iterations: usize,
}

/// The partitioned coupling ("increment 2", docs/espc-preferences-spec.md): the
/// real (non-`P3*`) goal split into interaction components — shared guidance
/// variables excluded from edge formation, priced by the λ schedule instead —
/// plus the exact phase tail that closes the `P3END`/collect/forgo bookkeeping
/// after composition. Built by the PDDL3 gate (`pddl3::metric_optimize`); `None`
/// or fewer than 2 components (or `FF_ESPC_MONO=1`) runs the monolithic loop.
pub struct EspcPartition {
    pub comps: Vec<Subgoal>,
    pub tail: PhaseTail,
    /// Real-goal fact → the preference deliverable facts structurally tied to it
    /// (the deliverable's conditional-achievement CONDITION shares a mutex
    /// variable with the goal fact — openstacks: `delivered(o,p)` fires on
    /// `started(o)`, same variable as goal `shipped(o)`). Each stage first tries
    /// its goal PLUS its own deliverables — the per-stage quality pressure that
    /// replaces the monolithic B&B's cost bound (stage plans are cost-flat, so a
    /// bound can't do it) — and falls back to the bare goal when infeasible.
    pub assoc: FxHashMap<u32, Vec<u32>>,
}

/// Why a partitioned composition attempt didn't produce a plan.
enum ComposeErr {
    /// Stage `i` failed or broke already-done sibling `j` — merge and retry.
    Conflict(usize, Option<usize>),
    /// Wall-clock budget exhausted between stages — stop the outer loop.
    Budget,
    /// The composed plan failed the full-goal safety check (never expected;
    /// falls back to the monolithic pass for this iteration).
    Invalid,
}

fn env_i64(key: &str, default: i64) -> i64 {
    std::env::var(key)
        .ok()
        .and_then(|s| s.parse::<i64>().ok())
        .unwrap_or(default)
}

/// One sequential composition pass over the current components (the resolve.rs
/// Phase-B pattern on the evolving state, λ-guided): solve each subgoal in fixed
/// order under sibling protection (forbid ops deleting a done sibling's goal
/// fact; retry unprotected before declaring conflict), then close with the phase
/// tail and check the FULL goal. Stages carry no cost bound — during the
/// planning phase `total-cost` is flat (forgo/pref-variant actions only), so
/// bounds can't prune here; quality pressure comes from `sat`'s global
/// λ-weighted penalty in the heap key, which pushes each stage to coordinate
/// with orders it shares triggers with.
#[allow(clippy::too_many_arguments)]
fn compose_once(
    task: &PackedTask,
    sat: &SatGuidance,
    part: &EspcPartition,
    threads: usize,
    cfg: SearchCfg,
    deadline: Instant,
) -> Result<Vec<usize>, ComposeErr> {
    let (comps, tail) = (&part.comps, &part.tail);
    let mut state = task.initial();
    let mut plan: Vec<usize> = Vec::new();
    let mut done = vec![false; comps.len()];
    for i in 0..comps.len() {
        // Budget guard BETWEEN stages (stage 0 always runs; a single in-flight
        // stage search is not preemptible, same convention as the monolithic
        // tightening loop).
        if i > 0 && Instant::now() >= deadline {
            return Err(ComposeErr::Budget);
        }
        if task.goal_met_with(&state, &comps[i].pos, &comps[i].num) {
            done[i] = true;
            continue; // already achieved (e.g. by a sibling's stage plan)
        }
        // Protect already-achieved siblings: forbid ops deleting one of their
        // goal facts; if infeasible under protection, relax and let the breaker
        // check below trigger the semantic merge.
        let protected: FxHashSet<u32> = (0..i)
            .filter(|&j| done[j])
            .flat_map(|j| comps[j].pos.iter().copied())
            .collect();
        let forbidden: Vec<bool> = if protected.is_empty() {
            Vec::new()
        } else {
            (0..task.n_ops)
                .map(|oi| task.del.slice(oi).iter().any(|f| protected.contains(f)))
                .collect()
        };
        // Quality pressure: first try the goal ENRICHED with this component's own
        // preference deliverables — skipping ones already true and ones already
        // LOCKED OUT (their once-only trigger fired without them, so no search
        // can earn them; asking for one would force the stage to exhaust before
        // falling back). Kept under sibling protection only, so an enriched
        // failure degrades to the bare goal instead of registering a spurious
        // conflict merge.
        let locked = |d: u32| {
            sat.deadline
                .iter()
                .any(|&(m, dd, _)| dd == d && crate::bitset::test(&state.bits, m as usize))
        };
        let mut extra: Vec<u32> = comps[i]
            .pos
            .iter()
            .flat_map(|f| part.assoc.get(f).map(|v| v.as_slice()).unwrap_or(&[]))
            .copied()
            .filter(|&d| {
                !crate::bitset::test(&state.bits, d as usize)
                    && !comps[i].pos.contains(&d)
                    && !locked(d)
            })
            .collect();
        extra.sort_unstable();
        extra.dedup();
        let enriched = (!extra.is_empty()).then(|| {
            let mut g = comps[i].pos.clone();
            g.extend(extra);
            g
        });
        let solved = enriched
            .and_then(|g| {
                solve_subgoal_guided(
                    task,
                    &state,
                    &g,
                    &comps[i].num,
                    &forbidden,
                    threads,
                    cfg,
                    Some(sat),
                )
            })
            .or_else(|| {
                solve_subgoal_guided(
                    task,
                    &state,
                    &comps[i].pos,
                    &comps[i].num,
                    &forbidden,
                    threads,
                    cfg,
                    Some(sat),
                )
            })
            .or_else(|| {
                if forbidden.is_empty() {
                    None
                } else {
                    solve_subgoal_guided(
                        task,
                        &state,
                        &comps[i].pos,
                        &comps[i].num,
                        &[],
                        threads,
                        cfg,
                        Some(sat),
                    )
                }
            });
        let Some(ops) = solved else {
            return Err(ComposeErr::Conflict(i, None));
        };
        // Apply, but reject if it broke an already-achieved sibling.
        let mut ns = state.clone();
        for &oi in &ops {
            ns = task.apply(oi, &ns);
        }
        if let Some(j) =
            (0..i).find(|&j| done[j] && !task.goal_met_with(&ns, &comps[j].pos, &comps[j].num))
        {
            return Err(ComposeErr::Conflict(i, Some(j)));
        }
        state = ns;
        plan.extend(ops);
        done[i] = true;
    }
    let tail_ops = apply_tail(task, &mut state, tail).ok_or(ComposeErr::Invalid)?;
    plan.extend(tail_ops);
    // Full-goal safety check: an invalid composition must never become the
    // incumbent (it would corrupt the reported==verified metric contract).
    if !task.goal_met_with(&state, &task.goal_pos, &task.goal_num) {
        return Err(ComposeErr::Invalid);
    }
    Ok(plan)
}

/// The partitioned inner solve: compose under the current penalties, coarsening
/// on conflict (semantic `merge_at` for a known breaker pair, positional
/// neighbor otherwise) and retrying within the same outer iteration. Merges
/// persist in `part` across iterations (dynamic grain-size control, as in
/// `resolve::solve`); the retry count is bounded by the component count. Errors:
/// `Budget` stops the outer loop, `Invalid`/1-component-conflict falls back to
/// one monolithic pass for the iteration.
fn solve_partitioned(
    task: &PackedTask,
    cost_fluent: usize,
    sat: &SatGuidance,
    part: &mut EspcPartition,
    threads: usize,
    cfg: SearchCfg,
    deadline: Instant,
) -> Result<(Vec<usize>, f64), ComposeErr> {
    loop {
        match compose_once(task, sat, part, threads, cfg, deadline) {
            Ok(plan) => {
                let cost = plan_cost(task, &plan, cost_fluent);
                return Ok((plan, cost));
            }
            Err(ComposeErr::Conflict(i, j)) => {
                if part.comps.len() <= 1 {
                    return Err(ComposeErr::Invalid);
                }
                match j {
                    Some(j) => merge_at(&mut part.comps, i, j),
                    None => merge_with_neighbor(&mut part.comps, i),
                };
            }
            Err(e) => return Err(e),
        }
    }
}

/// Per-trigger locked-loss counts in the final state of `ops`: for each deadline
/// pair `(M, D, _)`, count it when `M` is present and `D` is absent — a once-only
/// achievement that fired without delivering (a permanently lost preference). The
/// returned map is keyed by trigger; summing its values gives the total violation.
fn measure_violation(
    task: &PackedTask,
    ops: &[usize],
    base: &[(u32, u32, i64)],
) -> FxHashMap<u32, i64> {
    let mut s = task.initial();
    for &oi in ops {
        s = task.apply(oi, &s);
    }
    let mut v: FxHashMap<u32, i64> = FxHashMap::default();
    for &(m, d, _) in base {
        if crate::bitset::test(&s.bits, m as usize) && !crate::bitset::test(&s.bits, d as usize) {
            *v.entry(m).or_insert(0) += 1;
        }
    }
    v
}

/// Solve to (local) cost optimum under the penalties currently baked into `sat`:
/// an anytime branch-and-bound that starts at `bound0` (`INFINITY` for the
/// monolithic iterations, so the first pass always returns a measurable plan;
/// the partitioned incumbent's cost for the leftover-budget polish, so it only
/// spends time on strictly-cheaper plans) and tightens until no strictly-cheaper
/// plan is found or the inner cap is hit. Returns the best plan + its true
/// metric, or None if no plan beats `bound0` within budget under this penalty
/// setting.
#[allow(clippy::too_many_arguments)]
fn solve_under_penalties(
    task: &PackedTask,
    cost_fluent: usize,
    sat: &SatGuidance,
    threads: usize,
    cfg: SearchCfg,
    deadline: Instant,
    bound0: f64,
) -> Option<(Vec<usize>, f64)> {
    const INNER_MAX: usize = 200;
    let init = task.initial();
    let mut bound = bound0;
    let mut best: Option<(Vec<usize>, f64)> = None;
    for i in 0..INNER_MAX {
        // Budget guard BETWEEN bounded calls: never start another (multi-second)
        // tightening pass once the wall-clock budget is spent. The first pass (i==0,
        // unbounded) always runs so this call yields a measurable plan; subsequent
        // tightening is what gets cut. A single in-flight search is not preemptible,
        // so the budget is honored to within one bounded-call duration.
        if i > 0 && Instant::now() >= deadline {
            break;
        }
        let (opt, _evaluated, _capped) = solve_subgoal_bounded(
            task,
            &init,
            &task.goal_pos,
            &task.goal_num,
            cost_fluent,
            bound,
            threads,
            cfg,
            Some(sat),
        );
        match opt {
            Some(ops) => {
                let cost = plan_cost(task, &ops, cost_fluent);
                best = Some((ops, cost));
                if cost <= 0.0 {
                    break;
                }
                bound = cost; // next plan must be strictly cheaper
            }
            None => break,
        }
    }
    best
}

/// Run the ESPC penalty-resolution outer loop. `sat` must be preloaded with the
/// satisfaction/resource guidance and the static deadline pairs `(M, D, base_val)`;
/// this function owns the per-trigger λ schedule and mutates `sat.deadline`'s
/// effective weights in place each iteration. `seed` is the stage-1 incumbent (used
/// as the anytime starting point and never lost). When `part` supplies ≥2
/// components (and `FF_ESPC_MONO` is unset), each iteration runs the partitioned
/// composition instead of the monolithic tightening B&B — far cheaper per
/// iteration, so the default outer cap rises 16 → 64 and many more λ adaptations
/// fit in the same budget. Returns the best plan found.
pub fn espc_optimize(
    task: &PackedTask,
    cost_fluent: usize,
    sat: &mut SatGuidance,
    seed: Option<(Vec<usize>, f64)>,
    part: Option<EspcPartition>,
    threads: usize,
    cfg: SearchCfg,
) -> Option<EspcResult> {
    // Static (M, D, base_val) snapshot; sat.deadline[i].2 is overwritten per pass.
    let base: Vec<(u32, u32, i64)> = sat.deadline.clone();
    if base.is_empty() {
        return seed.map(|(ops, cost)| EspcResult {
            ops,
            cost,
            iterations: 0,
        });
    }

    // Tunable schedule (defaults chosen near the Phase-0 measured sweet spot, all
    // env-overridable — the exact schedule is a documented reverse-engineering
    // target, see docs/sgplan6-spec.md §4 / espc-preferences-spec.md:83-85).
    // λ0 defaults to 0 ON PURPOSE: iteration 0 then runs the plain (penalty-free)
    // B&B, establishing the default-quality incumbent as a hard floor before any
    // penalty exploration — so ESPC can only improve on it, never regress.
    // Partitioned mode engages only with a usable partition (≥2 components) and
    // without the FF_ESPC_MONO debug hatch (which reproduces the monolithic loop
    // exactly, default outer cap included).
    let mut part = part.filter(|p| p.comps.len() >= 2 && std::env::var("FF_ESPC_MONO").is_err());

    let lambda0 = env_i64("FF_ESPC_LAMBDA0", 0).max(0);
    let rate0 = env_i64("FF_ESPC_RATE", 20).max(1);
    let outer_default = if part.is_some() { 64 } else { 16 };
    let outer_max = env_i64("FF_ESPC_OUTER", outer_default).max(1) as usize;
    let k_bump = env_i64("FF_ESPC_K", 2).max(1); // consecutive-violation rate bump
    let stall_max = env_i64("FF_ESPC_STALL", 4).max(1) as usize;
    // Default budget kept conservative so ESPC reliably RETURNS its incumbent well
    // inside common harness timeouts (run.py uses 30s): it is anytime internally but
    // is killed wholesale by an EXTERNAL timeout, losing even the floor. Raise it
    // (with more threads) for the headline-quality runs. Tunable via FF_ESPC_TIME_MS.
    let time_ms = env_i64("FF_ESPC_TIME_MS", 15_000).max(0) as u64;
    let debug = std::env::var("FF_RES_DEBUG").is_ok();

    // Per-trigger penalty, monotone non-decreasing.
    let mut lambda: FxHashMap<u32, i64> = FxHashMap::default();
    for &(m, _, _) in &base {
        lambda.entry(m).or_insert(lambda0);
    }
    sat.deadline_weight = 1; // effective weights live in sat.deadline[i].2

    if debug {
        match &part {
            Some(p) => eprintln!(
                "[ESPC] partitioned: {} component(s), tail closes {} preference(s)",
                p.comps.len(),
                p.tail.prefs.len()
            ),
            None => eprintln!("[ESPC] monolithic"),
        }
    }

    let mut best = seed;
    let mut rate = rate0;
    let mut consec = 0i64;
    let mut stall = 0usize;
    let mut iterations = 0usize;
    let deadline = Instant::now() + Duration::from_millis(time_ms);
    // In partitioned mode the λ loop gets HALF the budget; the rest is reserved
    // for the monolithic incumbent-bounded polish below, which restores the
    // default-quality floor (partitioned iterations don't subsume the plain B&B
    // the way monolithic iteration 0 does).
    let loop_deadline = if part.is_some() {
        Instant::now() + Duration::from_millis(time_ms / 2)
    } else {
        deadline
    };

    for outer in 0..outer_max {
        // Don't START another outer iteration once the budget is spent — but iter 0
        // (the penalty-free floor) always runs, so we never return worse than the
        // plain B&B even under a tight budget.
        if outer > 0 && Instant::now() >= loop_deadline {
            break;
        }
        iterations += 1;
        // Bake the current λ into the (concrete-state) deadline penalty.
        for (pair, b) in sat.deadline.iter_mut().zip(&base) {
            let lam = *lambda.get(&b.0).unwrap_or(&0);
            pair.2 = lam.saturating_mul(b.2);
        }

        let solved = match part.as_mut() {
            Some(p) => {
                match solve_partitioned(task, cost_fluent, sat, p, threads, cfg, loop_deadline) {
                    Ok(r) => Some(r),
                    Err(ComposeErr::Budget) => None,
                    // Persistent composition failure under this penalty setting:
                    // one monolithic pass for the iteration (incumbent-safe either
                    // way; the merged-down partition persists for later iterations).
                    Err(_) => solve_under_penalties(
                        task,
                        cost_fluent,
                        sat,
                        threads,
                        cfg,
                        loop_deadline,
                        f64::INFINITY,
                    ),
                }
            }
            None => solve_under_penalties(
                task,
                cost_fluent,
                sat,
                threads,
                cfg,
                deadline,
                f64::INFINITY,
            ),
        };
        let Some((ops, cost)) = solved else {
            break; // no plan under this penalty setting (within budget)
        };
        let viol = measure_violation(task, &ops, &base);
        let total_v: i64 = viol.values().sum();

        let improved = best.as_ref().map_or(true, |(_, c)| cost < *c - 1e-9);
        if improved {
            best = Some((ops, cost));
            stall = 0;
            consec = 0;
        } else {
            stall += 1;
            consec += 1;
        }
        if debug {
            let comps = part.as_ref().map_or(1, |p| p.comps.len());
            eprintln!(
                "[ESPC] iter {outer}: cost={cost} violations={total_v} best={} rate={rate} comps={comps}",
                best.as_ref().map(|(_, c)| *c).unwrap_or(f64::INFINITY)
            );
        }

        if total_v == 0 {
            break; // saddle point: every deliverable landed under the current penalties
        }
        if stall >= stall_max {
            break; // penalties no longer buy improvement
        }

        // Penalty update: raise λ only on triggers whose deliveries were missed
        // (commutative per key, so iteration order does not affect the result).
        for (&m, &v) in &viol {
            if v > 0 {
                let e = lambda.entry(m).or_insert(lambda0);
                *e = e.saturating_add(rate.saturating_mul(v));
            }
        }
        if consec >= k_bump {
            rate = rate.saturating_mul(2);
            consec = 0;
        }
    }

    // Partitioned-mode POLISH: spend whatever budget remains on the monolithic
    // tightening B&B bounded by the incumbent (under the last-baked λ, which
    // only reorders the open list). Strictly improving — it either beats the
    // incumbent or leaves it — and it is what restores the "never worse than
    // the plain B&B" floor that monolithic iteration 0 provides.
    if part.is_some() && Instant::now() < deadline {
        let bound0 = best.as_ref().map_or(f64::INFINITY, |(_, c)| *c);
        if bound0 > 0.0 {
            if let Some((ops, cost)) =
                solve_under_penalties(task, cost_fluent, sat, threads, cfg, deadline, bound0)
            {
                if best.as_ref().map_or(true, |(_, c)| cost < *c - 1e-9) {
                    if debug {
                        eprintln!("[ESPC] polish: {bound0} -> {cost}");
                    }
                    best = Some((ops, cost));
                }
            }
        }
    }

    best.map(|(ops, cost)| EspcResult {
        ops,
        cost,
        iterations,
    })
}
