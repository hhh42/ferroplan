//! `Mode::Optimal` (0.19 Phase 2; LM-cut + the conditional-effect
//! admissibility repair 0.20 Phase 2) — sequential-optimal planning: A*
//! with the admissible LM-cut heuristic (h^max behind `FF_NO_LMCUT=1`),
//! over the same packed task as every other mode. The fence "seq-opt:
//! out of scope by design" comes down.
//!
//! The mode is PROOF-OR-NOTHING: it returns a plan only with an
//! optimality certificate (A* with an admissible heuristic and
//! re-opening — the first goal POP is optimal). A node-cap or budget
//! exhaustion returns INCONCLUSIVE, never a best-effort incumbent —
//! anytime reporting is a satisficing feature and lives in the other
//! modes.
//!
//! Honest v1 scope, recorded:
//! - Costs are per-op CONSTANTS: unit cost without a metric fluent, else
//!   the sum of the op's `(increase (total-cost) <const>)` effects
//!   (ops without a cost effect cost 0, the `:action-costs` semantics).
//!   A state-dependent or non-increase cost effect REJECTS the problem
//!   from this mode with a named note — certifying optima under
//!   state-dependent costs needs machinery v1 does not have.
//! - Numeric preconditions/goals are handled EXACTLY in expansion and
//!   the goal test, and IGNORED by h^max (dropping constraints relaxes,
//!   so admissibility holds).
//! - Search is serial and deterministic: f-ties break on lower h, then
//!   insertion order.

use std::cmp::Reverse;
use std::collections::BinaryHeap;

use crate::bitset;
use crate::hash::FxHashMap;
use crate::heuristic::RpgExit;
use crate::packed::{PackedTask, State, StateKey};
use crate::types::AssignOp;

/// The certificate-bearing result of an optimal solve.
pub struct OptOutcome {
    /// The optimal plan (empty = the initial state satisfies the goal).
    pub ops: Option<Vec<usize>>,
    /// Its certified cost (metric value with a cost fluent, else length).
    pub cost: f64,
    pub expanded: usize,
    pub evaluated: usize,
    /// `true` iff `ops`/`cost` carry an optimality certificate; `false`
    /// means INCONCLUSIVE (cap hit) — no plan is reported.
    pub proven: bool,
    /// The problem shape is outside the mode's certified scope.
    pub reject: Option<String>,
    /// Which admissible heuristic produced this outcome's certificate
    /// ("h^max" from the sprint rung or `FF_NO_LMCUT`, else "LM-cut";
    /// "+numRPG" appended when the 0.22 Phase 4 numeric arm rode along) —
    /// surfaced in the PROVEN note so the record names its prover(s).
    pub heuristic: &'static str,
    /// This inconclusive came from the WALL DEADLINE, not the node cap
    /// (0.22 Phase 2 lever 2): the node-cap refill must never re-enter
    /// after a clock trip — with the teardown reserve, a clock trip can
    /// leave >10% of the wall standing on purpose, so the remaining-wall
    /// test alone no longer discriminates. Always false on proven or
    /// rejected outcomes.
    pub clock_tripped: bool,
}

fn inconclusive(expanded: usize, evaluated: usize, heuristic: &'static str) -> OptOutcome {
    OptOutcome {
        ops: None,
        cost: 0.0,
        expanded,
        evaluated,
        proven: false,
        reject: None,
        heuristic,
        clock_tripped: false,
    }
}

/// Extract each op's constant cost from its effects on the cost fluent
/// `cf`. A cost expression that reads only STATIC fluents (written by no
/// op's effects, defined at init) is a constant in disguise — IPC's
/// `(increase (total-cost) (travel-slow ?f1 ?f2))` pattern — and is
/// evaluated against init. `Err` names the first op whose cost the mode
/// genuinely cannot certify. A CONDITIONAL effect on the cost fluent is
/// state-dependent by construction and rejects the same way.
fn op_costs(task: &PackedTask, cf: Option<usize>) -> Result<Vec<f64>, String> {
    let Some(cf) = cf else {
        return Ok(vec![1.0; task.n_ops]);
    };
    let mut written = vec![false; task.fv0.len()];
    for oi in 0..task.n_ops {
        for ne in task.num_eff.slice(oi) {
            written[ne.target as usize] = true;
        }
        for ce in task.cond.slice(oi) {
            for ne in &ce.num {
                written[ne.target as usize] = true;
            }
        }
    }
    let mut reads = Vec::new();
    let mut costs = vec![0.0; task.n_ops];
    for (oi, cost) in costs.iter_mut().enumerate() {
        if task
            .cond
            .slice(oi)
            .iter()
            .any(|ce| ce.num.iter().any(|ne| ne.target as usize == cf))
        {
            return Err(format!(
                "op `{}` has a conditional cost effect — \
                 outside optimal mode's certified scope",
                task.op_display[oi]
            ));
        }
        for ne in task.num_eff.slice(oi) {
            if ne.target as usize != cf {
                continue;
            }
            let certified = if ne.op == AssignOp::Increase {
                reads.clear();
                ne.value.collect_fluents(&mut reads);
                let static_reads = reads
                    .iter()
                    .all(|&f| !written[f as usize] && task.fdef0[f as usize]);
                if static_reads {
                    ne.value.eval(&task.fv0, &task.fdef0).filter(|c| *c >= 0.0)
                } else {
                    None
                }
            } else {
                None
            };
            match certified {
                Some(c) => *cost += c,
                None => {
                    return Err(format!(
                        "op `{}` has a state-dependent or non-increase cost effect — \
                         outside optimal mode's certified scope",
                        task.op_display[oi]
                    ))
                }
            }
        }
    }
    Ok(costs)
}

/// One relaxed ACHIEVER: `pre` ⊢ `add` at `cost[op]`. Every op contributes
/// its unconditional entry (pre_pos ⊢ add); each conditional effect with
/// adds contributes another (pre_pos ∪ cond-pos-pre ⊢ cond adds) — the
/// delete relaxation of the conditional effect, with negative conditions
/// dropped (both are relaxations, so lower bounds survive).
///
/// This is the 0.20 admissibility repair: 0.19's h^max iterated only the
/// unconditional adds, so a goal reachable ONLY through a conditional
/// effect was labeled unreachable — an OVERestimate, and A* certified
/// wrong optima on the `(when ...)` domains (cave-diving, city-car,
/// maintenance; the two-op fixture in tests/optimal.rs pins the exact
/// failure: "PROVEN cost 100" where the true optimum is 11).
struct Achiever {
    op: u32,
    pre: Vec<u32>,
    add: Vec<u32>,
}

/// The relaxation graph, built once per solve: achievers with SORTED,
/// DEDUPED preconditions (the Dijkstra counters below count distinct
/// facts), indexed by precondition fact and by added fact.
struct RelaxGraph {
    ach: Vec<Achiever>,
    /// Achievers with fact `f` among their preconditions.
    by_pre: Vec<Vec<u32>>,
    /// Achievers with no preconditions (fire immediately at cost).
    pre_free: Vec<u32>,
    /// Achievers adding fact `f` (the goal-zone walk's incidence).
    by_add: Vec<Vec<u32>>,
}

impl RelaxGraph {
    fn new(task: &PackedTask) -> Self {
        let mut ach = Vec::with_capacity(task.n_ops);
        for oi in 0..task.n_ops {
            let mut pre = task.pre_pos.slice(oi).to_vec();
            pre.sort_unstable();
            pre.dedup();
            ach.push(Achiever {
                op: oi as u32,
                pre,
                add: task.add.slice(oi).to_vec(),
            });
            for ce in task.cond.slice(oi) {
                if ce.add.is_empty() {
                    continue;
                }
                let mut pre = task.pre_pos.slice(oi).to_vec();
                pre.extend_from_slice(&ce.cond_pos);
                pre.sort_unstable();
                pre.dedup();
                ach.push(Achiever {
                    op: oi as u32,
                    pre,
                    add: ce.add.clone(),
                });
            }
        }
        let n_facts = task.fact_names.len();
        let mut by_pre: Vec<Vec<u32>> = vec![Vec::new(); n_facts];
        let mut by_add: Vec<Vec<u32>> = vec![Vec::new(); n_facts];
        let mut pre_free = Vec::new();
        for (ai, a) in ach.iter().enumerate() {
            if a.pre.is_empty() {
                pre_free.push(ai as u32);
            }
            for &p in &a.pre {
                by_pre[p as usize].push(ai as u32);
            }
            for &f in &a.add {
                by_add[f as usize].push(ai as u32);
            }
        }
        RelaxGraph {
            ach,
            by_pre,
            pre_free,
            by_add,
        }
    }
}

/// Heap key over an f64 label (total order via `total_cmp`).
#[derive(PartialEq)]
struct HK(f64, u32);
impl Eq for HK {}
impl PartialOrd for HK {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}
impl Ord for HK {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.0.total_cmp(&other.0).then(self.1.cmp(&other.1))
    }
}

/// Admissible h^max labels via the standard counter Dijkstra: a fact
/// settles at its final label in nondecreasing order; an achiever fires
/// when its last precondition settles (pre_max = that label — the settle
/// order guarantees it is the max). O(edges · log facts) per call — this
/// is the hot loop (LM-cut re-labels once per round per state).
/// `label[f]` is a lower bound on the cost to make fact `f` true from
/// `s`; numeric preconditions are ignored (a relaxation — admissible).
/// `f64::INFINITY` on any goal fact = relaxed-unreachable = safe prune.
fn hmax_labels(g: &RelaxGraph, s: &State, w: &mut CutSpace) {
    w.label.fill(f64::INFINITY);
    w.heap.clear();
    for (ai, a) in g.ach.iter().enumerate() {
        w.counts[ai] = a.pre.len() as u32;
    }
    w.settled.iter_mut().for_each(|b| *b = false);
    for f in 0..w.label.len() {
        if bitset::test(&s.bits, f) {
            w.label[f] = 0.0;
            w.heap.push(Reverse(HK(0.0, f as u32)));
        }
    }
    // Precondition-free achievers fire immediately at their own cost.
    for i in 0..g.pre_free.len() {
        let ai = g.pre_free[i] as usize;
        let a = &g.ach[ai];
        let val = w.cost[a.op as usize];
        for &f in &a.add {
            if val < w.label[f as usize] {
                w.label[f as usize] = val;
                w.heap.push(Reverse(HK(val, f)));
            }
        }
    }
    while let Some(Reverse(HK(l, f))) = w.heap.pop() {
        let fi = f as usize;
        if w.settled[fi] || l > w.label[fi] {
            continue;
        }
        w.settled[fi] = true;
        for &ai in &g.by_pre[fi] {
            let c = &mut w.counts[ai as usize];
            *c -= 1;
            if *c > 0 {
                continue;
            }
            let a = &g.ach[ai as usize];
            let val = w.cost[a.op as usize] + l;
            for &v in &a.add {
                if val < w.label[v as usize] {
                    w.label[v as usize] = val;
                    w.heap.push(Reverse(HK(val, v)));
                }
            }
        }
    }
}

/// h^max under the BASE costs (the plain heuristic; LM-cut calls
/// [`hmax_labels`] directly under its decremented per-round costs).
fn hmax(task: &PackedTask, s: &State, g: &RelaxGraph, costs: &[f64], w: &mut CutSpace) -> f64 {
    w.cost.copy_from_slice(costs);
    hmax_labels(g, s, w);
    let mut h = 0.0f64;
    for &gf in task.goal_pos.iter() {
        h = h.max(w.label[gf as usize]);
    }
    h
}

/// Reusable per-solve buffers for [`lmcut`] (one A* evaluates thousands of
/// states; the workspace keeps the per-state cost to allocations-free).
/// `by_add` (which achievers add fact f — static across a solve) makes the
/// goal-zone walk O(incident edges); `by_pcf` (which achievers this
/// round's supporter roots at fact f — rebuilt per round) does the same
/// for the before-zone walk.
struct CutSpace {
    label: Vec<f64>,
    cost: Vec<f64>,
    /// Supporter (precondition-choice function) per achiever:
    /// the max-label precondition, `u32::MAX` = no preconditions
    /// (a virtual always-true init fact — always in the before zone).
    pcf: Vec<u32>,
    goal_zone: Vec<bool>,
    before: Vec<bool>,
    stack: Vec<u32>,
    by_pcf: Vec<Vec<u32>>,
    heap: BinaryHeap<Reverse<HK>>,
    counts: Vec<u32>,
    settled: Vec<bool>,
    /// This round's virtual-init-rooted achievers (empty preconditions).
    init_rooted: Vec<u32>,
    in_cut: Vec<bool>,
}

impl CutSpace {
    fn new(n_facts: usize, n_ops: usize, g: &RelaxGraph) -> Self {
        CutSpace {
            label: vec![f64::INFINITY; n_facts],
            cost: vec![0.0; n_ops],
            pcf: vec![u32::MAX; g.ach.len()],
            goal_zone: vec![false; n_facts],
            before: vec![false; n_facts],
            stack: Vec::new(),
            by_pcf: vec![Vec::new(); n_facts],
            heap: BinaryHeap::new(),
            counts: vec![0; g.ach.len()],
            settled: vec![false; n_facts],
            init_rooted: Vec::new(),
            in_cut: vec![false; n_ops],
        }
    }
}

/// LM-cut (Helmert & Domshlak 2009), the 0.20 Phase 2 centerpiece:
/// admissible like h^max, but pays per-fact cost only once per landmark.
/// Each round: h^max labels under the CURRENT (decremented) costs pick a
/// supporter per achiever; the zero-cost goal zone is traced backward from
/// the goal; the before zone forward from the state's facts (stopping at
/// the goal zone); every op with an achiever edge crossing before → goal
/// zone forms a disjunctive action landmark, its minimum cost joins the
/// heuristic and is decremented from every cut member. Terminates because
/// each round zeroes at least one op. Sums of landmark minima never exceed
/// the true cost (the standard cost-partitioning argument over the relaxed
/// task, which the achiever list makes a faithful relaxation of the real
/// one), so the value is ADMISSIBLE — but not consistent; the A* below
/// re-opens on cheaper routes, which keeps the first goal pop optimal.
fn lmcut(task: &PackedTask, s: &State, g: &RelaxGraph, costs: &[f64], w: &mut CutSpace) -> f64 {
    w.cost.copy_from_slice(costs);
    let mut total = 0.0f64;
    // Safety bound: each round zeroes ≥1 op, so n_ops+1 rounds cannot be
    // reached; the bound guards float-edge stalls (min cut cost rounding
    // to 0) from looping forever.
    for _ in 0..=task.n_ops {
        hmax_labels(g, s, w);
        let mut hg = 0.0f64;
        let mut goal_support = u32::MAX;
        for &gf in task.goal_pos.iter() {
            let l = w.label[gf as usize];
            if l > hg || goal_support == u32::MAX {
                hg = l;
                goal_support = gf;
            }
        }
        if hg == 0.0 || task.goal_pos.is_empty() {
            return total;
        }
        if hg.is_infinite() {
            return f64::INFINITY;
        }
        // Supporters under the current labels: the max-label precondition
        // (first wins ties — deterministic). Bucketed by supporter fact as
        // we go, so the before-zone walk below is worklist-driven.
        for b in w.by_pcf.iter_mut() {
            b.clear();
        }
        w.init_rooted.clear();
        for (ai, a) in g.ach.iter().enumerate() {
            let mut best = u32::MAX;
            let mut best_l = -1.0f64;
            let mut reachable = true;
            for &p in &a.pre {
                let l = w.label[p as usize];
                if l.is_infinite() {
                    reachable = false;
                    break;
                }
                if l > best_l {
                    best_l = l;
                    best = p;
                }
            }
            w.pcf[ai] = if !reachable {
                u32::MAX - 1
            } else if best == u32::MAX {
                w.init_rooted.push(ai as u32);
                u32::MAX
            } else {
                w.by_pcf[best as usize].push(ai as u32);
                best
            };
        }
        // Goal zone: backward from the goal's supporter through ZERO-cost
        // achiever edges (the virtual goal op costs 0, so its supporter
        // seeds the zone). `by_add` narrows each pop to incident edges.
        w.goal_zone.iter_mut().for_each(|b| *b = false);
        w.stack.clear();
        w.goal_zone[goal_support as usize] = true;
        w.stack.push(goal_support);
        while let Some(v) = w.stack.pop() {
            for &ai in &g.by_add[v as usize] {
                let a = &g.ach[ai as usize];
                if w.cost[a.op as usize] != 0.0 {
                    continue;
                }
                let u = w.pcf[ai as usize];
                if u >= u32::MAX - 1 || w.goal_zone[u as usize] {
                    continue;
                }
                w.goal_zone[u as usize] = true;
                w.stack.push(u);
            }
        }
        // Before zone: forward from the state's facts (plus the virtual
        // init fact rooting precondition-free achievers) through supporter
        // edges, never entering the goal zone. Each achiever is processed
        // exactly once — when its supporter is popped — and the cut is
        // collected on the fly: an edge from the before zone into the
        // goal zone.
        w.before.iter_mut().for_each(|b| *b = false);
        w.stack.clear();
        for f in 0..w.before.len() {
            if bitset::test(&s.bits, f) && !w.goal_zone[f] {
                w.before[f] = true;
                w.stack.push(f as u32);
            }
        }
        let mut m = f64::INFINITY;
        let mut cut: Vec<u32> = Vec::new();
        let visit = |ai: u32,
                     w_cost: &[f64],
                     w_goal: &[bool],
                     before: &mut [bool],
                     stack: &mut Vec<u32>,
                     in_cut: &mut [bool],
                     cut: &mut Vec<u32>,
                     m: &mut f64| {
            let a = &g.ach[ai as usize];
            for &v in &a.add {
                if w_goal[v as usize] {
                    let op = a.op as usize;
                    let oc = w_cost[op];
                    if oc > 0.0 {
                        if !in_cut[op] {
                            in_cut[op] = true;
                            cut.push(a.op);
                        }
                        *m = m.min(oc);
                    }
                } else if !before[v as usize] {
                    before[v as usize] = true;
                    stack.push(v);
                }
            }
        };
        for i in 0..w.init_rooted.len() {
            let ai = w.init_rooted[i];
            visit(
                ai,
                &w.cost,
                &w.goal_zone,
                &mut w.before,
                &mut w.stack,
                &mut w.in_cut,
                &mut cut,
                &mut m,
            );
        }
        while let Some(u) = w.stack.pop() {
            for i in 0..w.by_pcf[u as usize].len() {
                let ai = w.by_pcf[u as usize][i];
                visit(
                    ai,
                    &w.cost,
                    &w.goal_zone,
                    &mut w.before,
                    &mut w.stack,
                    &mut w.in_cut,
                    &mut cut,
                    &mut m,
                );
            }
        }
        for &op in &cut {
            w.in_cut[op as usize] = false;
        }
        if cut.is_empty() || !m.is_finite() || m <= 0.0 {
            // Float-edge degenerate round (should be unreachable: hg > 0
            // guarantees a positive-cost crossing edge). Fall back to the
            // admissible sum collected so far.
            return total;
        }
        total += m;
        for &op in &cut {
            w.cost[op as usize] -= m;
            if w.cost[op as usize] < 0.0 {
                w.cost[op as usize] = 0.0;
            }
        }
    }
    total
}

/// f64 ordering key for the open list (total order via `total_cmp`).
#[derive(PartialEq)]
struct K(f64, f64, usize);
impl Eq for K {}
impl PartialOrd for K {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}
impl Ord for K {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.0
            .total_cmp(&other.0)
            .then(self.1.total_cmp(&other.1))
            .then(self.2.cmp(&other.2))
    }
}

/// Fraction knob: `var` read as a fraction of remaining wall, (0, 1] only.
fn frac_env(var: &str, default: f64) -> f64 {
    std::env::var(var)
        .ok()
        .and_then(|v| v.trim().parse::<f64>().ok())
        .filter(|f| f.is_finite() && *f > 0.0 && *f <= 1.0)
        .unwrap_or(default)
}

/// The numeric-admissible arm (0.22 Phase 4 L1/L2): `Some(root bound)`
/// when every arming condition holds, `None` otherwise — and `None` means
/// every search below is byte-identical to the unarmed engine.
///
/// Armed on: a genuinely numeric task (numeric goal or preconditions),
/// certificate currency LENGTH (no active `:metric` — the layer bound
/// counts STEPS, so it is only admissible against unit costs; all three
/// ipc2026-opt domains qualify), a passing
/// [`crate::heuristic::numeric_interval_audit`] (fail closed — the audit
/// rejects by name the shapes that break containment), and the root-drop
/// guard (L2): a root bound of 0 means the widened RPG closes the goal
/// without a single step — the bound buys nothing anywhere, so disarm for
/// the solve and skip the per-eval RPG tax (dropping a max-component is
/// always sound). `FF_OPT_NO_NUMH=1` disarms unconditionally, on every
/// path — the pure-hatch discriminator.
fn num_arm_root(task: &PackedTask, cf: Option<usize>) -> Option<f64> {
    if std::env::var("FF_OPT_NO_NUMH").is_ok() || cf.is_some() {
        return None;
    }
    if task.goal_num.is_empty() && task.pre_num.flat.is_empty() {
        return None;
    }
    if crate::heuristic::numeric_interval_audit(task).is_err() {
        return None;
    }
    let init = task.initial();
    let mut sc = crate::heuristic::Scratch::new(task);
    match crate::heuristic::admissible_goal_layers(task, &mut sc, &init.bits, &init.fv, &init.fdef)
    {
        RpgExit::GoalAt(0) => None,
        RpgExit::GoalAt(l) => Some(l as f64),
        RpgExit::Fixpoint => Some(f64::INFINITY),
        RpgExit::Cap => Some(crate::heuristic::LAYER_CAP as f64),
    }
}

/// The optimal LADDER (0.20; wall-denominated 0.21 Phase 4; allocation
/// repriced + resumption 0.22 Phase 3): an h^max SPRINT, then LM-cut,
/// then — new this cycle — h^max RESUMED. The differential priced both
/// heuristics honestly — LM-cut collapses the expansion count where h^max
/// walls (floor-tile 1.95M -> 58k), but its per-node cost LOSES races
/// h^max wins easily (barman-opt i1: h^max proves cost 90 in 22 s,
/// LM-cut does not finish in 100 s). The sprint keeps every cheap h^max
/// certificate at a bounded cost; a PROVEN verdict either way returns
/// immediately, only an inconclusive cap falls through.
///
/// With no armed `FF_TIME_LIMIT` the split is the 0.20 node-split
/// (sprint = a quarter of the node budget), bit-identical — dev boxes
/// and every existing test are out of blast range by construction.
/// Under an armed wall the ladder is denominated in WALL SECONDS:
/// - ROOT GATE first (0.21), now MARGIN-shaped (0.22 Phase 3 lever 1):
///   city-car's six v0.19 proofs gated c-branch at LM-cut/h^max ratios
///   1.09–1.36 while every probed TRUE-c domain sits at 2.2–6.0, so the
///   binary `lc > hm` was handing thin margins to the wrong rung.
///   Ratio < `FF_OPT_GATE_MARGIN` (default 1.4) ⇒ h^max keeps the FULL
///   node budget and the whole remaining wall (margin 1.0 restores the
///   0.21 binary gate exactly).
/// - MARGIN-SCALED sprint slice (lever 2): ratio ≥ 2 means the landmark
///   structure is unambiguous (the scanalyzer/elevator class, where every
///   0.21 LM-cut certificate paid the full 24 s sprint first — min cert
///   24.12 s, three within 1.7 s of the kill line) — the sprint shrinks
///   to `FF_OPT_SPRINT_FRAC_HI` (default 0.1) of the remaining wall;
///   below 2 it keeps `FF_OPT_SPRINT_FRAC` (default 0.4 — a 25% slice
///   would kill the 22 s barman class).
/// - SPRINT-RESUME (lever 3, the new machinery): the ten h^max
///   certificates the 0.21 slice killed have root ratios 2.59–3.5 —
///   inside the true-c range, no threshold saves them; they need the
///   h^max seconds the ladder used to throw away at handover. The
///   sprint's A* state is KEPT; LM-cut runs a bounded probe
///   (`FF_OPT_LMCUT_PROBE_FRAC`, default 0.33 of the remaining wall, half
///   the node budget — a probe, and its arena drops mid-flight, far from
///   the wall); on LM-cut failure h^max RESUMES its own open list with
///   the leftover wall and the FULL node budget (plus the one cap-raise
///   re-entry, in place — no re-tread). `FF_OPT_NO_RESUME=1` restores
///   the 0.21 throw-away handover (sprint dropped, LM-cut gets the whole
///   remainder with the refill).
///
/// `FF_NO_LMCUT=1` = h^max only (full budget); `FF_NO_HMAX_SPRINT=1`
/// = LM-cut only (the gate never resurrects the sprint) — the two
/// discriminator hatches keep their pure-rung meanings on every path.
/// `FF_OPT_NO_ROOTGATE=1` restores the unconditional 0.21
/// sprint-then-LM-cut ladder under a wall (no margin, no resume).
/// The numeric arm ([`num_arm_root`], 0.22 Phase 4) composes with every
/// branch via max() and never touches the gate's h^max-vs-LM-cut verdict.
pub fn solve(task: &PackedTask, cf: Option<usize>, max_nodes: usize) -> OptOutcome {
    let wall = crate::search::wall_remaining_secs();
    let narm = num_arm_root(task, cf);
    let num = narm.is_some();
    if std::env::var("FF_NO_LMCUT").is_ok() {
        return astar_wall_refill(task, cf, max_nodes, false, num, wall);
    }
    if std::env::var("FF_NO_HMAX_SPRINT").is_ok() {
        return astar_wall_refill(task, cf, max_nodes, true, num, wall);
    }
    let Some(remaining) = wall else {
        let o = astar(task, cf, (max_nodes / 4).max(2), false, num, None);
        if o.proven || o.reject.is_some() {
            return o;
        }
        let mut full = astar(task, cf, max_nodes, true, num, None);
        full.expanded += o.expanded;
        full.evaluated += o.evaluated;
        return full;
    };
    let rootgate = std::env::var("FF_OPT_NO_ROOTGATE").is_err();
    let mut ratio_hi = false;
    if rootgate {
        // An op_costs Err falls through: the sprint below surfaces the
        // identical reject.
        if let Ok(costs) = op_costs(task, cf) {
            let graph = RelaxGraph::new(task);
            let mut w = CutSpace::new(task.fact_names.len(), task.n_ops, &graph);
            let s = task.initial();
            let hm = hmax(task, &s, &graph, &costs, &mut w);
            let lc = lmcut(task, &s, &graph, &costs, &mut w);
            let margin = std::env::var("FF_OPT_GATE_MARGIN")
                .ok()
                .and_then(|v| v.trim().parse::<f64>().ok())
                .filter(|m| m.is_finite() && *m > 0.0)
                .unwrap_or(1.4);
            let ratio = if hm > 0.0 {
                lc / hm
            } else if lc > 0.0 {
                f64::INFINITY
            } else {
                1.0
            };
            let informative = lc > hm && ratio >= margin;
            ratio_hi = informative && ratio >= 2.0;
            // The probe eyes (FF_WALL_DEBUG's pattern): narrate the
            // verdict on stderr, never affect the search.
            if std::env::var("FF_WALL_DEBUG").is_ok() {
                let numroot = narm.map(|n| format!(", num root {n}")).unwrap_or_default();
                eprintln!(
                    "wall: opt root gate h^max {hm} vs LM-cut {lc} \
                     (ratio {ratio:.2}, margin {margin}{numroot}) -> {}",
                    if informative {
                        if ratio_hi {
                            "LM-cut earns the remainder (sprint 0.1-class)"
                        } else {
                            "LM-cut earns the remainder"
                        }
                    } else {
                        "h^max holds the wall"
                    }
                );
            }
            if !informative {
                return astar_wall_refill(task, cf, max_nodes, false, num, Some(remaining));
            }
        }
    }
    let frac = if ratio_hi {
        frac_env("FF_OPT_SPRINT_FRAC_HI", 0.1)
    } else {
        frac_env("FF_OPT_SPRINT_FRAC", 0.4)
    };
    if rootgate && std::env::var("FF_OPT_NO_RESUME").is_err() {
        // Lever 3: the resumable ladder. The sprint engine LIVES through
        // the LM-cut probe; nothing is re-expanded on resume.
        let mut sprint = match AstarState::new(task, cf, false, num) {
            Ok(e) => e,
            Err(reject) => return reject,
        };
        let o = sprint.run((max_nodes / 4).max(2), Some(frac * remaining));
        if o.proven || o.reject.is_some() {
            return o;
        }
        let probe_frac = frac_env("FF_OPT_LMCUT_PROBE_FRAC", 0.33);
        let probe_wall = crate::search::wall_remaining_secs().unwrap_or(0.0);
        let mut probe = astar(
            task,
            cf,
            (max_nodes / 2).max(2),
            true,
            num,
            Some(probe_frac * probe_wall),
        );
        if probe.proven || probe.reject.is_some() {
            probe.expanded += o.expanded;
            probe.evaluated += o.evaluated;
            return probe;
        }
        if std::env::var("FF_WALL_DEBUG").is_ok() {
            eprintln!(
                "wall: LM-cut probe inconclusive — h^max resumes its open list \
                 ({:.1}s remaining)",
                crate::search::wall_remaining_secs().unwrap_or(0.0)
            );
        }
        let mut out = sprint.run(max_nodes, crate::search::wall_remaining_secs());
        // The node-cap refill (0.22 Phase 2 lever 2) on the resumed
        // engine: the raise happens IN PLACE — the open list continues,
        // no re-tread — under the same conditions and hatch as
        // astar_wall_refill's fresh-pass form.
        if !out.proven
            && out.reject.is_none()
            && !out.clock_tripped
            && std::env::var("FF_NO_NODECAP_REFILL").is_err()
            && crate::search::wall_remaining_frac().is_some_and(|f| f > 0.10)
        {
            if std::env::var("FF_WALL_DEBUG").is_ok() {
                eprintln!(
                    "wall: optimal node cap raised x2 in place ({:.1}s remaining)",
                    crate::search::wall_remaining_secs().unwrap_or(0.0)
                );
            }
            out = sprint.run(
                max_nodes.saturating_mul(2),
                crate::search::wall_remaining_secs(),
            );
        }
        out.expanded += probe.expanded;
        out.evaluated += probe.evaluated;
        return out;
    }
    // The 0.21 throw-away handover (FF_OPT_NO_RESUME, or the whole
    // unconditional ladder under FF_OPT_NO_ROOTGATE).
    let o = astar(
        task,
        cf,
        (max_nodes / 4).max(2),
        false,
        num,
        Some(frac * remaining),
    );
    if o.proven || o.reject.is_some() {
        return o;
    }
    let mut full = astar_wall_refill(
        task,
        cf,
        max_nodes,
        true,
        num,
        crate::search::wall_remaining_secs(),
    );
    full.expanded += o.expanded;
    full.evaluated += o.evaluated;
    full
}

/// The full-budget pass plus the node-cap refill (0.22 Phase 2 lever 2).
/// The sailing-wind receipt: 9 early-exit rows on ipc2026-opt died at
/// the old cap with 20–40 s of a 60 s wall LEFT. With the cap now on
/// the honest optimal model (`opt_per_node_model_bytes`), a node-capped
/// pass with wall remaining re-enters ONCE with the cap doubled — one
/// doubling spends the actual headroom the conservative 8 GiB default
/// leaves on the sweep box, and under the runner's `FF_MEM_BUDGET_GB`
/// the raise stays inside watchdog territory. The 0.20 refill pattern,
/// on the proof ladder; `clock_tripped` keeps it from re-entering after
/// a deadline trip. No armed wall (`deadline` None) ⇒ one pass,
/// bit-identical; `FF_NO_NODECAP_REFILL=1` restores the fixed cap.
fn astar_wall_refill(
    task: &PackedTask,
    cf: Option<usize>,
    max_nodes: usize,
    use_lmcut: bool,
    num: bool,
    deadline: Option<f64>,
) -> OptOutcome {
    let mut out = astar(task, cf, max_nodes, use_lmcut, num, deadline);
    if deadline.is_none() {
        return out;
    }
    let mut raise = 1usize;
    while !out.proven
        && out.ops.is_none()
        && out.reject.is_none()
        && !out.clock_tripped
        && raise < 2
        && std::env::var("FF_NO_NODECAP_REFILL").is_err()
        && crate::search::wall_remaining_frac().is_some_and(|f| f > 0.10)
    {
        raise *= 2;
        if std::env::var("FF_WALL_DEBUG").is_ok() {
            eprintln!(
                "wall: optimal node cap raised x{raise} ({:.1}s remaining)",
                crate::search::wall_remaining_secs().unwrap_or(0.0)
            );
        }
        let o = astar(
            task,
            cf,
            max_nodes.saturating_mul(raise),
            use_lmcut,
            num,
            crate::search::wall_remaining_secs(),
        );
        out = OptOutcome {
            expanded: out.expanded + o.expanded,
            evaluated: out.evaluated + o.evaluated,
            ..o
        };
    }
    out
}

/// One A* pass with the chosen admissible heuristic: one engine, one
/// `run`. `max_nodes` bounds STORED nodes (the retained-memory model,
/// like the satisficing searches); hitting it returns inconclusive.
/// `deadline` (0.21 Phase 4) is wall seconds from entry; a trip returns
/// the same inconclusive shape as a node-cap hit, so the ladder proceeds
/// identically. `None` never trips (the no-wall path stays bit-identical).
fn astar(
    task: &PackedTask,
    cf: Option<usize>,
    max_nodes: usize,
    use_lmcut: bool,
    num: bool,
    deadline: Option<f64>,
) -> OptOutcome {
    match AstarState::new(task, cf, use_lmcut, num) {
        Ok(mut e) => e.run(max_nodes, deadline),
        Err(reject) => reject,
    }
}

/// Evaluate the admissible heuristic on one state: h^max or LM-cut, and —
/// with the numeric arm ([`num_arm_root`], 0.22 Phase 4 L1) — max() with
/// the interval-RPG layer bound (max of admissible bounds is admissible).
/// The propositional ∞ short-circuits: the state is already a safe prune,
/// so the RPG tax is skipped there.
#[allow(clippy::too_many_arguments)]
fn eval_state(
    task: &PackedTask,
    use_lmcut: bool,
    graph: &RelaxGraph,
    costs: &[f64],
    w: &mut CutSpace,
    numsc: &mut Option<crate::heuristic::Scratch>,
    s: &State,
) -> f64 {
    let ph = if use_lmcut {
        lmcut(task, s, graph, costs, w)
    } else {
        hmax(task, s, graph, costs, w)
    };
    let Some(sc) = numsc else { return ph };
    if ph.is_infinite() {
        return ph;
    }
    let nh = match crate::heuristic::admissible_goal_layers(task, sc, &s.bits, &s.fv, &s.fdef) {
        RpgExit::GoalAt(l) => l as f64,
        RpgExit::Fixpoint => f64::INFINITY,
        RpgExit::Cap => crate::heuristic::LAYER_CAP as f64,
    };
    ph.max(nh)
}

/// The resumable A* engine (0.22 Phase 3 lever 3). One `run` is exactly
/// the historical one-shot pass — same pop order, same counters, same
/// trip shapes — but the open list, arena, and `best_g` memo SURVIVE the
/// return, so a later `run` continues where the trip happened instead of
/// re-treading the prefix. Resume soundness rides `pending`: both trip
/// shapes fire after a node was popped (clock: before its successor loop;
/// node cap: mid-loop), which would silently LOSE that node's remaining
/// successors on resume — so the tripped node's index is parked, and the
/// next `run` re-drives its successor loop first (idempotent: already-
/// inserted successors dedup through `best_g` at equal g; the node's
/// `expanded` count is not double-charged).
///
/// LM-cut is admissible but NOT consistent, so the A* re-opens closed
/// states on cheaper routes (the `best_g` map already allows it): with an
/// admissible h and re-opening, some node on an optimal path always sits
/// in open with its optimal g, so the first goal POP still carries the
/// optimality certificate — across resumes too, because nothing that was
/// reachable at the trip is forgotten.
struct AstarState<'t> {
    task: &'t PackedTask,
    use_lmcut: bool,
    num: bool,
    hname: &'static str,
    costs: Vec<f64>,
    per_node: usize,
    reserve_on: bool,
    // Primed lazily on the first run() call, AFTER its deadline check —
    // an already-spent deadline must return before any graph is built
    // (the 0.21 shape, kept byte-identical).
    graph: Option<RelaxGraph>,
    cutspace: Option<CutSpace>,
    numsc: Option<crate::heuristic::Scratch>,
    /// (state, parent index, op from parent)
    nodes: Vec<(State, usize, usize)>,
    /// Per stored state: best g AND its h (0.21 Phase 4 rider). h is a
    /// pure function of the state, so a cheaper route to a known state —
    /// an open-list decrease or a genuine re-open — reuses the memo
    /// instead of re-paying eval_state.
    best_g: FxHashMap<StateKey, (f64, f64)>,
    open: BinaryHeap<Reverse<K>>,
    g_of: Vec<f64>,
    expanded: usize,
    evaluated: usize,
    /// A popped node whose successor loop was interrupted by a trip;
    /// the next run() re-drives it before popping anything.
    pending: Option<usize>,
}

impl<'t> AstarState<'t> {
    fn new(
        task: &'t PackedTask,
        cf: Option<usize>,
        use_lmcut: bool,
        num: bool,
    ) -> Result<Self, OptOutcome> {
        // The prover names itself in the PROVEN note; the numeric arm
        // rides along as "+numRPG" so the record names BOTH admissible
        // components (0.22 Phase 4 L1).
        let hname: &'static str = match (use_lmcut, num) {
            (true, true) => "LM-cut+numRPG",
            (true, false) => "LM-cut",
            (false, true) => "h^max+numRPG",
            (false, false) => "h^max",
        };
        let costs = match op_costs(task, cf) {
            Ok(c) => c,
            Err(why) => {
                return Err(OptOutcome {
                    ops: None,
                    cost: 0.0,
                    expanded: 0,
                    evaluated: 0,
                    proven: false,
                    reject: Some(why),
                    heuristic: hname,
                    clock_tripped: false,
                })
            }
        };
        Ok(AstarState {
            task,
            use_lmcut,
            num,
            hname,
            costs,
            // The teardown reserve (0.22 Phase 2): dropping the arena is
            // paid at RETURN, before the caller can report —
            // sailing-wind-opt i9 measured ~13 s of frees for ~4.5M
            // stored 1.2KB states AFTER its deadline trip, blowing a
            // 60 s solo run to 90+. Reserve stored-bytes / 4e8 (the
            // measured give-back rate on the sweep box), capped at half
            // the deadline so small tasks keep the whole wall. Rides the
            // 0.22 checkpoint hatch so the RED overrun stays pinnable.
            per_node: crate::search::opt_per_node_model_bytes(task),
            reserve_on: crate::search::rung_wallcap_on(),
            graph: None,
            cutspace: None,
            numsc: None,
            nodes: Vec::new(),
            best_g: FxHashMap::default(),
            open: BinaryHeap::new(),
            g_of: Vec::new(),
            expanded: 0,
            evaluated: 0,
            pending: None,
        })
    }

    fn prime(&mut self) {
        if self.graph.is_some() {
            return;
        }
        let graph = RelaxGraph::new(self.task);
        self.cutspace = Some(CutSpace::new(
            self.task.fact_names.len(),
            self.task.n_ops,
            &graph,
        ));
        self.graph = Some(graph);
        if self.num {
            self.numsc = Some(crate::heuristic::Scratch::new(self.task));
        }
        let init = self.task.initial();
        let h0 = eval_state(
            self.task,
            self.use_lmcut,
            self.graph.as_ref().unwrap(),
            &self.costs,
            self.cutspace.as_mut().unwrap(),
            &mut self.numsc,
            &init,
        );
        self.best_g.insert(self.task.state_key(&init), (0.0, h0));
        self.nodes.push((init, usize::MAX, usize::MAX));
        self.evaluated = 1;
        if h0.is_finite() {
            self.open.push(Reverse(K(h0, h0, 0)));
        }
        self.g_of.push(0.0);
    }

    fn tripped(&self) -> OptOutcome {
        let mut o = inconclusive(self.expanded, self.evaluated, self.hname);
        o.clock_tripped = true;
        o
    }

    /// Drive node `ni`'s successor loop. `Some(outcome)` = the node cap
    /// tripped mid-loop (ni parked in `pending` for the next run).
    fn expand(&mut self, ni: usize, max_nodes: usize) -> Option<OptOutcome> {
        let g = self.g_of[ni];
        for oi in 0..self.costs.len() {
            let op_cost = self.costs[oi];
            if !self.task.op_applicable(oi, &self.nodes[ni].0) {
                continue;
            }
            let succ = self.task.apply(oi, &self.nodes[ni].0);
            let sg = g + op_cost;
            let skey = self.task.state_key(&succ);
            let prev = self.best_g.get(&skey).copied();
            if prev.is_some_and(|(bg, _)| bg <= sg) {
                continue;
            }
            if self.nodes.len() >= max_nodes {
                self.pending = Some(ni);
                return Some(inconclusive(self.expanded, self.evaluated, self.hname));
            }
            let h = match prev {
                Some((_, ph)) => ph,
                None => {
                    self.evaluated += 1;
                    eval_state(
                        self.task,
                        self.use_lmcut,
                        self.graph.as_ref().unwrap(),
                        &self.costs,
                        self.cutspace.as_mut().unwrap(),
                        &mut self.numsc,
                        &succ,
                    )
                }
            };
            if h.is_infinite() {
                continue; // relaxed-unreachable goal: safe prune
            }
            self.best_g.insert(skey, (sg, h));
            self.nodes.push((succ, ni, oi));
            self.g_of.push(sg);
            self.open.push(Reverse(K(sg + h, h, self.nodes.len() - 1)));
        }
        None
    }

    fn run(&mut self, max_nodes: usize, deadline: Option<f64>) -> OptOutcome {
        let clock = crate::clock::Clock::now();
        if deadline.is_some_and(|d| d <= 0.0) {
            return self.tripped();
        }
        self.prime();
        // Resume repair: finish the successor loop the last trip cut
        // short (its node was already popped, goal-checked and counted).
        if let Some(ni) = self.pending.take() {
            if let Some(o) = self.expand(ni, max_nodes) {
                return o;
            }
        }
        while let Some(Reverse(K(_, _, ni))) = self.open.pop() {
            let g = self.g_of[ni];
            let key = self.task.state_key(&self.nodes[ni].0);
            // stale entry: a cheaper route to this state was expanded already
            if self.best_g.get(&key).map_or(f64::INFINITY, |&(bg, _)| bg) < g {
                continue;
            }
            if self.task.goal_met(&self.nodes[ni].0) {
                // admissible h + re-opening ⇒ the first goal POP is optimal
                let mut ops = Vec::new();
                let mut cur = ni;
                while self.nodes[cur].1 != usize::MAX {
                    ops.push(self.nodes[cur].2);
                    cur = self.nodes[cur].1;
                }
                ops.reverse();
                return OptOutcome {
                    ops: Some(ops),
                    cost: g,
                    expanded: self.expanded,
                    evaluated: self.evaluated,
                    proven: true,
                    reject: None,
                    heuristic: self.hname,
                    clock_tripped: false,
                };
            }
            self.expanded += 1;
            // Checked EVERY pop, not every 1024 (0.22 Phase 2 lever 1): a
            // clock read costs ~25 ns against a µs-plus expansion, and on
            // sailing-wind i9 the count cadence went BLIND exactly when it
            // mattered — past ~6 GB the memory compressor makes each pop
            // cost milliseconds, 1024 pops take minutes, and a 60 s wall
            // ran to a 120 s external kill with the deadline armed the
            // whole time.
            if let Some(d) = deadline {
                let reserve = if self.reserve_on {
                    ((self.nodes.len() * self.per_node) as f64 / 4e8).min(d * 0.5)
                } else {
                    0.0
                };
                if clock.elapsed_secs() >= d - reserve {
                    self.pending = Some(ni);
                    return self.tripped();
                }
            }
            if let Some(o) = self.expand(ni, max_nodes) {
                return o;
            }
        }
        // open exhausted without a goal: the task is PROVEN unsolvable
        // (under exact expansion; h prunes only relaxed-unreachable states)
        OptOutcome {
            ops: None,
            cost: 0.0,
            expanded: self.expanded,
            evaluated: self.evaluated,
            proven: true,
            reject: None,
            heuristic: self.hname,
            clock_tripped: false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn task_of(dom: &str, prb: &str) -> (PackedTask, Option<usize>) {
        let d = crate::parser::parse_domain(dom).unwrap();
        let p = crate::parser::parse_problem(prb).unwrap();
        let cf_desc = crate::costs::metric_fluent(&p);
        match crate::ground::ground(&d, &p, 1) {
            crate::ground::Outcome::Task(t) => {
                let cf = cf_desc.and_then(|desc| t.fluent_id(&desc));
                (t, cf)
            }
            _ => panic!("unexpected grounding outcome"),
        }
    }

    fn h_values(dom: &str, prb: &str) -> (f64, f64) {
        let (task, cf) = task_of(dom, prb);
        let costs = op_costs(&task, cf).unwrap();
        let g = RelaxGraph::new(&task);
        let s = task.initial();
        let mut w = CutSpace::new(task.fact_names.len(), task.n_ops, &g);
        let hm = hmax(&task, &s, &g, &costs, &mut w);
        let lc = lmcut(&task, &s, &g, &costs, &mut w);
        (hm, lc)
    }

    /// The textbook separation: two independent unit-cost goal conjuncts.
    /// h^max sees only the deeper one (1); LM-cut charges one landmark per
    /// conjunct (2 = the true optimum).
    #[test]
    fn lmcut_beats_hmax_on_parallel_goals() {
        let dom = "(define (domain par)
          (:predicates (g1) (g2))
          (:action a1 :parameters () :precondition (and) :effect (g1))
          (:action a2 :parameters () :precondition (and) :effect (g2)))";
        let prb = "(define (problem p) (:domain par) (:init) (:goal (and (g1) (g2))))";
        let (hm, lc) = h_values(dom, prb);
        assert_eq!(hm, 1.0);
        assert_eq!(lc, 2.0);
    }

    /// On a pure cost chain both are exact: h^max = LM-cut = 9 (2+3+4).
    #[test]
    fn lmcut_exact_on_cost_chain() {
        let dom = "(define (domain ch)
          (:requirements :action-costs)
          (:predicates (p0) (p1) (p2) (p3))
          (:functions (total-cost))
          (:action s1 :parameters () :precondition (p0)
            :effect (and (p1) (increase (total-cost) 2)))
          (:action s2 :parameters () :precondition (p1)
            :effect (and (p2) (increase (total-cost) 3)))
          (:action s3 :parameters () :precondition (p2)
            :effect (and (p3) (increase (total-cost) 4))))";
        let prb = "(define (problem p) (:domain ch)
          (:init (p0) (= (total-cost) 0)) (:goal (p3))
          (:metric minimize (total-cost)))";
        let (hm, lc) = h_values(dom, prb);
        assert_eq!(hm, 9.0);
        assert_eq!(lc, 9.0);
    }

    /// The numeric arm's gating table (0.22 Phase 4 L1/L2), pinned at the
    /// unit level: length currency + numeric task + audit Ok + informative
    /// root ⇒ armed with the root bound; an active metric (p01/p03)
    /// disarms on currency (the layer bound counts STEPS — inadmissible
    /// against weighted costs); an audit reject disarms fail-closed; a
    /// classical task never arms (and pays zero audit tax).
    #[test]
    fn num_arm_gates_on_currency_audit_and_root() {
        // p02: pure numeric goal, no metric — armed, root GoalAt(1).
        let (task, cf) = task_of(
            include_str!("../../../benchmarks/bench/numopt-domain.pddl"),
            include_str!("../../../benchmarks/bench/numopt-p02.pddl"),
        );
        assert_eq!(num_arm_root(&task, cf), Some(1.0));
        // p01: same task shape, active :metric — currency disarms.
        let (task, cf) = task_of(
            include_str!("../../../benchmarks/bench/numopt-domain.pddl"),
            include_str!("../../../benchmarks/bench/numopt-p01.pddl"),
        );
        assert!(cf.is_some());
        assert_eq!(num_arm_root(&task, cf), None);
        // p04: the gap-60 pump chain — armed, root exactly 60.
        let (task, cf) = task_of(
            include_str!("../../../benchmarks/bench/numopt-pump-domain.pddl"),
            include_str!("../../../benchmarks/bench/numopt-p04.pddl"),
        );
        assert_eq!(num_arm_root(&task, cf), Some(60.0));
        // sailing-band through Mode::Optimal's arm, admissible side:
        // propositional goal behind numeric bands — pre_num alone arms it,
        // root GoalAt(4) ≤ the 21-step optimum.
        let (task, cf) = task_of(
            include_str!("../../../benchmarks/bench/sailing-band-domain.pddl"),
            include_str!("../../../benchmarks/bench/sailing-band-i1.pddl"),
        );
        assert_eq!(num_arm_root(&task, cf), Some(4.0));
        // scale-up shape: the audit rejects, the arm stays down.
        let (task, cf) = task_of(
            "(define (domain sc) (:requirements :numeric-fluents)
              (:functions (x))
              (:action shrink :parameters ()
                :precondition (and) :effect (scale-up (x) 0.25)))",
            "(define (problem p) (:domain sc)
              (:init (= (x) 1)) (:goal (<= (x) 0.5)))",
        );
        assert_eq!(num_arm_root(&task, cf), None);
        // classical: no numeric anything — never armed.
        let (task, cf) = task_of(
            "(define (domain ch) (:predicates (p) (g))
              (:action a :parameters () :precondition (p) :effect (g)))",
            "(define (problem c) (:domain ch) (:init (p)) (:goal (g)))",
        );
        assert_eq!(num_arm_root(&task, cf), None);
    }

    /// The 0.20 admissibility-repair witness at the heuristic level: the
    /// goal is reachable only through a CONDITIONAL add (or a cost-100
    /// direct op). With cond achievers modeled, both heuristics value the
    /// initial state at exactly the true optimum 11 (setp 10 + a 1);
    /// 0.19's h^max saw only the direct op and said 100 — an
    /// overestimate that certified a wrong plan.
    #[test]
    fn cond_effect_achievers_keep_h_admissible() {
        let dom = "(define (domain co)
          (:requirements :conditional-effects :action-costs)
          (:predicates (p) (g))
          (:functions (total-cost))
          (:action setp :parameters () :precondition (and)
            :effect (and (p) (increase (total-cost) 10)))
          (:action a :parameters () :precondition (and)
            :effect (and (when (p) (g)) (increase (total-cost) 1)))
          (:action direct :parameters () :precondition (and)
            :effect (and (g) (increase (total-cost) 100))))";
        let prb = "(define (problem p) (:domain co)
          (:init (= (total-cost) 0)) (:goal (g))
          (:metric minimize (total-cost)))";
        let (hm, lc) = h_values(dom, prb);
        assert_eq!(hm, 11.0);
        assert_eq!(lc, 11.0);
    }
}
