//! `Mode::Optimal` (0.19 Phase 2) — sequential-optimal planning: A* with
//! the admissible h^max heuristic, over the same packed task as every
//! other mode. The fence "seq-opt: out of scope by design" comes down.
//!
//! The mode is PROOF-OR-NOTHING: it returns a plan only with an
//! optimality certificate (A* with a consistent admissible heuristic —
//! the first goal expansion is optimal). A node-cap or budget exhaustion
//! returns INCONCLUSIVE, never a best-effort incumbent — anytime
//! reporting is a satisficing feature and lives in the other modes.
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
}

fn inconclusive(expanded: usize, evaluated: usize) -> OptOutcome {
    OptOutcome {
        ops: None,
        cost: 0.0,
        expanded,
        evaluated,
        proven: false,
        reject: None,
    }
}

/// Extract each op's constant cost from its effects on the cost fluent
/// `cf`. A cost expression that reads only STATIC fluents (written by no
/// op's effects, defined at init) is a constant in disguise — IPC's
/// `(increase (total-cost) (travel-slow ?f1 ?f2))` pattern — and is
/// evaluated against init. `Err` names the first op whose cost the mode
/// genuinely cannot certify.
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

/// Admissible h^max: cost-labeled relaxed fixpoint. `label[f]` is a lower
/// bound on the cost to make fact `f` true from `s`; h = max over goal
/// facts. Numeric preconditions are ignored (a relaxation — admissible).
/// `f64::INFINITY` on any goal fact = relaxed-unreachable = safe prune.
fn hmax(task: &PackedTask, s: &State, costs: &[f64], label: &mut [f64]) -> f64 {
    label.fill(f64::INFINITY);
    for (f, l) in label.iter_mut().enumerate() {
        if bitset::test(&s.bits, f) {
            *l = 0.0;
        }
    }
    // Bellman-Ford-style relaxation to the fixpoint: op fires when all its
    // positive preconditions are labeled; its adds improve toward
    // op_cost + max(pre labels). Iteration count is bounded by the layer
    // depth; tasks here are the packed classical ones (thousands of ops).
    loop {
        let mut changed = false;
        for (oi, &op_cost) in costs.iter().enumerate() {
            let mut pre_max = 0.0f64;
            let mut reachable = true;
            for &p in task.pre_pos.slice(oi) {
                let l = label[p as usize];
                if l.is_infinite() {
                    reachable = false;
                    break;
                }
                pre_max = pre_max.max(l);
            }
            if !reachable {
                continue;
            }
            let val = op_cost + pre_max;
            for &a in task.add.slice(oi) {
                if val < label[a as usize] {
                    label[a as usize] = val;
                    changed = true;
                }
            }
        }
        if !changed {
            break;
        }
    }
    let mut h = 0.0f64;
    for &g in task.goal_pos.iter() {
        h = h.max(label[g as usize]);
    }
    h
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

/// A* with h^max. `cf` is the metric cost fluent (None = unit cost).
/// `max_nodes` bounds STORED nodes (the retained-memory model, like the
/// satisficing searches); hitting it returns inconclusive.
pub fn solve(task: &PackedTask, cf: Option<usize>, max_nodes: usize) -> OptOutcome {
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
            }
        }
    };

    let init = task.initial();
    // nodes: (state, parent index, op from parent)
    let mut nodes: Vec<(State, usize, usize)> = vec![(init, usize::MAX, usize::MAX)];
    let mut best_g: FxHashMap<StateKey, f64> = FxHashMap::default();
    best_g.insert(task.state_key(&nodes[0].0), 0.0);
    let mut label = vec![f64::INFINITY; task.fact_names.len()];
    let mut open: BinaryHeap<Reverse<K>> = BinaryHeap::new();
    let h0 = hmax(task, &nodes[0].0, &costs, &mut label);
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
            // consistent admissible h ⇒ the first goal EXPANSION is optimal
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
            };
        }
        expanded += 1;
        for oi in 0..task.n_ops {
            if !task.op_applicable(oi, &nodes[ni].0) {
                continue;
            }
            let succ = task.apply(oi, &nodes[ni].0);
            let sg = g + costs[oi];
            let skey = task.state_key(&succ);
            if best_g.get(&skey).copied().unwrap_or(f64::INFINITY) <= sg {
                continue;
            }
            if nodes.len() >= max_nodes {
                return inconclusive(expanded, evaluated);
            }
            let h = hmax(task, &succ, &costs, &mut label);
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
    }
}
