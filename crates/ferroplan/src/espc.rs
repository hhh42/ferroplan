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

use crate::hash::FxHashMap;
use crate::packed::PackedTask;
use crate::pddl3::plan_cost;
use crate::search::{solve_subgoal_bounded, SatGuidance, SearchCfg};

pub struct EspcResult {
    pub ops: Vec<usize>,
    pub cost: f64,
    pub iterations: usize,
}

fn env_i64(key: &str, default: i64) -> i64 {
    std::env::var(key)
        .ok()
        .and_then(|s| s.parse::<i64>().ok())
        .unwrap_or(default)
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
/// an anytime branch-and-bound that starts unbounded (so it always returns a
/// measurable plan) and tightens until no strictly-cheaper plan is found or the
/// inner cap is hit. Returns the best plan + its true metric, or None if no plan
/// exists within budget under this penalty setting.
fn solve_under_penalties(
    task: &PackedTask,
    cost_fluent: usize,
    sat: &SatGuidance,
    threads: usize,
    cfg: SearchCfg,
    deadline: Instant,
) -> Option<(Vec<usize>, f64)> {
    const INNER_MAX: usize = 200;
    let init = task.initial();
    let mut bound = f64::INFINITY;
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
        let (opt, _capped) = solve_subgoal_bounded(
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
/// as the anytime starting point and never lost). Returns the best plan found.
pub fn espc_optimize(
    task: &PackedTask,
    cost_fluent: usize,
    sat: &mut SatGuidance,
    seed: Option<(Vec<usize>, f64)>,
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
    let lambda0 = env_i64("FF_ESPC_LAMBDA0", 0).max(0);
    let rate0 = env_i64("FF_ESPC_RATE", 20).max(1);
    let outer_max = env_i64("FF_ESPC_OUTER", 16).max(1) as usize;
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

    let mut best = seed;
    let mut rate = rate0;
    let mut consec = 0i64;
    let mut stall = 0usize;
    let mut iterations = 0usize;
    let deadline = Instant::now() + Duration::from_millis(time_ms);

    for outer in 0..outer_max {
        // Don't START another outer iteration once the budget is spent — but iter 0
        // (the penalty-free floor) always runs, so we never return worse than the
        // plain B&B even under a tight budget.
        if outer > 0 && Instant::now() >= deadline {
            break;
        }
        iterations += 1;
        // Bake the current λ into the (concrete-state) deadline penalty.
        for (pair, b) in sat.deadline.iter_mut().zip(&base) {
            let lam = *lambda.get(&b.0).unwrap_or(&0);
            pair.2 = lam.saturating_mul(b.2);
        }

        let Some((ops, cost)) =
            solve_under_penalties(task, cost_fluent, sat, threads, cfg, deadline)
        else {
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
            eprintln!(
                "[ESPC] iter {outer}: cost={cost} violations={total_v} best={} rate={rate}",
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

    best.map(|(ops, cost)| EspcResult {
        ops,
        cost,
        iterations,
    })
}
