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
    /// ("h^max" from the sprint rung or `FF_NO_LMCUT`, else "LM-cut") —
    /// surfaced in the PROVEN note so the record names its prover.
    pub heuristic: &'static str,
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

/// The optimal LADDER (0.20): an h^max SPRINT on a quarter of the node
/// budget, then LM-cut over the full budget. The differential priced both
/// heuristics honestly — LM-cut collapses the expansion count where h^max
/// walls (floor-tile 1.95M -> 58k), but its per-node cost LOSES races
/// h^max wins easily (barman-opt i1: h^max proves cost 90 in 22 s,
/// LM-cut does not finish in 100 s). The sprint keeps every cheap h^max
/// certificate at a bounded cost (≤ a quarter of the memory budget, and
/// its nodes/sec is ~30x LM-cut's); a PROVEN verdict either way returns
/// immediately, only an inconclusive cap falls through to LM-cut.
/// `FF_NO_LMCUT=1` = h^max only (full budget); `FF_NO_HMAX_SPRINT=1`
/// skips the sprint (LM-cut only) — the two discriminator hatches.
pub fn solve(task: &PackedTask, cf: Option<usize>, max_nodes: usize) -> OptOutcome {
    let use_lmcut = std::env::var("FF_NO_LMCUT").is_err();
    if !use_lmcut {
        return astar(task, cf, max_nodes, false);
    }
    let sprint = std::env::var("FF_NO_HMAX_SPRINT").is_err();
    if sprint {
        let o = astar(task, cf, (max_nodes / 4).max(2), false);
        if o.proven || o.reject.is_some() {
            return o;
        }
        let mut full = astar(task, cf, max_nodes, true);
        full.expanded += o.expanded;
        full.evaluated += o.evaluated;
        return full;
    }
    astar(task, cf, max_nodes, true)
}

/// One A* pass with the chosen admissible heuristic. `max_nodes` bounds
/// STORED nodes (the retained-memory model, like the satisficing
/// searches); hitting it returns inconclusive.
///
/// LM-cut is admissible but NOT consistent, so the A* re-opens closed
/// states on cheaper routes (the `best_g` map already allows it): with an
/// admissible h and re-opening, some node on an optimal path always sits
/// in open with its optimal g, so the first goal POP still carries the
/// optimality certificate.
fn astar(task: &PackedTask, cf: Option<usize>, max_nodes: usize, use_lmcut: bool) -> OptOutcome {
    let hname: &'static str = if use_lmcut { "LM-cut" } else { "h^max" };
    let costs = match op_costs(task, cf) {
        Ok(c) => c,
        Err(why) => {
            return OptOutcome {
                ops: None,
                cost: 0.0,
                expanded: 0,
                evaluated: 0,
                proven: false,
                reject: Some(why),
                heuristic: hname,
            }
        }
    };
    let graph = RelaxGraph::new(task);
    let mut cutspace = CutSpace::new(task.fact_names.len(), task.n_ops, &graph);
    let eval_h = |s: &State, w: &mut CutSpace| {
        if use_lmcut {
            lmcut(task, s, &graph, &costs, w)
        } else {
            hmax(task, s, &graph, &costs, w)
        }
    };

    let init = task.initial();
    // nodes: (state, parent index, op from parent)
    let mut nodes: Vec<(State, usize, usize)> = vec![(init, usize::MAX, usize::MAX)];
    let mut best_g: FxHashMap<StateKey, f64> = FxHashMap::default();
    best_g.insert(task.state_key(&nodes[0].0), 0.0);
    let mut open: BinaryHeap<Reverse<K>> = BinaryHeap::new();
    let h0 = eval_h(&nodes[0].0, &mut cutspace);
    let mut evaluated = 1usize;
    if h0.is_finite() {
        open.push(Reverse(K(h0, h0, 0)));
    }
    let mut g_of: Vec<f64> = vec![0.0];
    let mut expanded = 0usize;

    while let Some(Reverse(K(_, _, ni))) = open.pop() {
        let g = g_of[ni];
        let key = task.state_key(&nodes[ni].0);
        // stale entry: a cheaper route to this state was expanded already
        if best_g.get(&key).copied().unwrap_or(f64::INFINITY) < g {
            continue;
        }
        if task.goal_met(&nodes[ni].0) {
            // admissible h + re-opening ⇒ the first goal POP is optimal
            let mut ops = Vec::new();
            let mut cur = ni;
            while nodes[cur].1 != usize::MAX {
                ops.push(nodes[cur].2);
                cur = nodes[cur].1;
            }
            ops.reverse();
            return OptOutcome {
                ops: Some(ops),
                cost: g,
                expanded,
                evaluated,
                proven: true,
                reject: None,
                heuristic: hname,
            };
        }
        expanded += 1;
        for (oi, &op_cost) in costs.iter().enumerate() {
            if !task.op_applicable(oi, &nodes[ni].0) {
                continue;
            }
            let succ = task.apply(oi, &nodes[ni].0);
            let sg = g + op_cost;
            let skey = task.state_key(&succ);
            if best_g.get(&skey).copied().unwrap_or(f64::INFINITY) <= sg {
                continue;
            }
            if nodes.len() >= max_nodes {
                return inconclusive(expanded, evaluated, hname);
            }
            let h = eval_h(&succ, &mut cutspace);
            evaluated += 1;
            if h.is_infinite() {
                continue; // relaxed-unreachable goal: safe prune
            }
            best_g.insert(skey, sg);
            nodes.push((succ, ni, oi));
            g_of.push(sg);
            open.push(Reverse(K(sg + h, h, nodes.len() - 1)));
        }
    }
    // open exhausted without a goal: the task is PROVEN unsolvable (under
    // exact expansion; h prunes only relaxed-unreachable states)
    OptOutcome {
        ops: None,
        cost: 0.0,
        expanded,
        evaluated,
        proven: true,
        reject: None,
        heuristic: hname,
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
