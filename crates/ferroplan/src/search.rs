//! Data-parallel weighted best-first search.
//!
//! Each round pops a *batch* of the lowest-f nodes, expands all their applicable
//! successors, dedups against the visited set, then evaluates the FF heuristic
//! for the whole batch of successors IN PARALLEL (the dominant cost). Because
//! `par_map` preserves order and all control flow is on one thread, the plan
//! found is identical regardless of the worker count — only the wall-clock of
//! heuristic evaluation changes.

use std::cmp::Reverse;
use std::collections::BinaryHeap;

use crate::hash::FxHashSet;
use crate::heuristic::{relaxed_to, Scratch};
use crate::types::NumPre;
use crate::packed::{PackedTask, State, StateKey};
use crate::par;

const W_G: i32 = 1;
const W_H: i32 = 5;
/// Frontier batch size — FIXED (independent of thread count) so the search
/// expansion order, and thus the plan AND the evaluated-state count, are
/// identical for any worker count; threads only split each batch's h-eval.
const BATCH: usize = 256;
/// Safety cap on evaluated states (deterministic; preserves thread-determinism).
const MAX_EVAL: usize = 5_000_000;

pub enum PlanResult {
    Plan {
        ops: Vec<usize>,
        advance: Vec<i32>,
        evaluated: usize,
        max_g: usize,
    },
    Unsolvable {
        evaluated: usize,
        capped: bool, // true if the MAX_EVAL safety cap was hit (not proven unsolvable)
    },
}

struct Node {
    state: State,
    father: usize,
    op: usize,
    g: usize,
}

/// Solve toward an ARBITRARY (sub)goal from an arbitrary start state over a
/// shared grounded task — the reusable subplanner entry point for SGPlan-style
/// partition-and-resolve. `search` is the whole-task convenience wrapper.
pub fn search_from(
    task: &PackedTask,
    start: &State,
    goal_pos: &[u32],
    goal_num: &[NumPre],
    cost_fluent: Option<usize>,
    cost_bound: f64,
    threads: usize,
) -> PlanResult {
    let batch = BATCH;

    let init = start.clone();
    // early dead-end check: if the initial state is a relaxed dead end, unsolvable
    if relaxed_to(task, &mut Scratch::new(task), &init.bits, &init.fv, &init.fdef, goal_pos, goal_num).is_none() {
        return PlanResult::Unsolvable { evaluated: 1, capped: false };
    }

    let mut nodes: Vec<Node> = vec![Node {
        state: init.clone(),
        father: usize::MAX,
        op: usize::MAX,
        g: 0,
    }];
    // Deferred evaluation: a node's priority is set from its PARENT's h at
    // insertion; its own h is computed only when it is popped. Many inserted
    // nodes are never popped, so far fewer heuristic evaluations are done.
    // The visited key excludes irrelevant fluents (termination); under
    // branch-and-bound it also appends the cost fluent so equal-fact/different-cost
    // states stay distinct (see PackedTask::state_key_with_cost).
    let mut heap: BinaryHeap<Reverse<(i32, usize)>> = BinaryHeap::new();
    heap.push(Reverse((0, 0))); // init popped first
    let mut visited: FxHashSet<StateKey> = FxHashSet::default();
    visited.insert(task.state_key_with_cost(&init, cost_fluent));

    let mut evaluated = 0usize;
    let mut best = i32::MAX;
    let mut advance: Vec<i32> = Vec::new();
    let mut max_g = 0usize;

    while !heap.is_empty() {
        // pop a batch of lowest-priority nodes
        let mut popped: Vec<usize> = Vec::with_capacity(batch);
        for _ in 0..batch {
            match heap.pop() {
                Some(Reverse((_, ni))) => popped.push(ni),
                None => break,
            }
        }

        // goal check (cheap, before any heuristic work)
        for &ni in &popped {
            max_g = max_g.max(nodes[ni].g);
            if task.goal_met_with(&nodes[ni].state, goal_pos, goal_num) {
                return PlanResult::Plan {
                    ops: reconstruct(&nodes, ni),
                    advance,
                    evaluated,
                    max_g,
                };
            }
        }

        // PARALLEL: evaluate h for the popped batch (the only evaluations),
        // each worker reusing one Scratch across its chunk.
        let hs: Vec<Option<i32>> = par::par_map_with(
            &popped,
            threads,
            || Scratch::new(task),
            |sc, &ni| relaxed_to(task, sc, &nodes[ni].state.bits, &nodes[ni].state.fv, &nodes[ni].state.fdef, goal_pos, goal_num),
        );
        evaluated += popped.len();
        if evaluated > MAX_EVAL {
            return PlanResult::Unsolvable { evaluated, capped: true };
        }
        for h in hs.iter().flatten() {
            if *h < best {
                best = *h;
                advance.push(*h);
            }
        }

        // PARALLEL: expand non-dead-end popped nodes; successors carry the
        // parent's h as their (deferred) priority key.
        let live: Vec<(usize, i32)> = popped
            .iter()
            .zip(hs.iter())
            .filter_map(|(&ni, h)| h.map(|h| (ni, h)))
            .collect();
        let cand_chunks: Vec<Vec<(usize, usize, State, StateKey, i32)>> =
            par::par_map(&live, threads, |&(ni, ph)| {
                let st = &nodes[ni].state;
                let mut v = Vec::new();
                for oi in 0..task.n_ops {
                    if task.op_applicable(oi, st) {
                        let ns = task.apply(oi, st);
                        if let Some(cf) = cost_fluent {
                            if ns.fdef[cf] && ns.fv[cf] >= cost_bound {
                                continue; // cost already >= bound: cannot beat incumbent
                            }
                        }
                        let k = task.state_key_with_cost(&ns, cost_fluent);
                        v.push((ni, oi, ns, k, ph));
                    }
                }
                v
            });

        // SERIAL: dedup + insert (deterministic order, independent of threads).
        for chunk in cand_chunks {
            for (pi, oi, s, k, ph) in chunk {
                if visited.insert(k) {
                    let g = nodes[pi].g + 1;
                    let idx = nodes.len();
                    nodes.push(Node {
                        state: s,
                        father: pi,
                        op: oi,
                        g,
                    });
                    heap.push(Reverse((W_G * g as i32 + W_H * ph, idx)));
                }
            }
        }
    }

    PlanResult::Unsolvable { evaluated, capped: false }
}

fn reconstruct(nodes: &[Node], mut ni: usize) -> Vec<usize> {
    let mut ops = Vec::new();
    while nodes[ni].father != usize::MAX {
        ops.push(nodes[ni].op);
        ni = nodes[ni].father;
    }
    ops.reverse();
    ops
}

/// Whole-task search (start = initial state, goal = task goal).
pub fn search(task: &PackedTask, threads: usize) -> PlanResult {
    search_from(task, &task.initial(), &task.goal_pos, &task.goal_num, None, f64::INFINITY, threads)
}

/// Subplanner API: return the op sequence achieving `(goal_pos, goal_num)` from
/// `start`, or None if unsolvable. This is what `sgp` calls per partition.
pub fn solve_subgoal(
    task: &PackedTask,
    start: &State,
    goal_pos: &[u32],
    goal_num: &[NumPre],
    threads: usize,
) -> Option<Vec<usize>> {
    match search_from(task, start, goal_pos, goal_num, None, f64::INFINITY, threads) {
        PlanResult::Plan { ops, .. } => Some(ops),
        PlanResult::Unsolvable { .. } => None,
    }
}

/// Subplanner with a monotone COST upper bound on `cost_fluent`: returns a plan
/// reaching the goal whose final cost is < `bound`, or None if none exists under
/// the bound. The anytime branch-and-bound metric optimizer (sgp) calls this
/// with a tightening bound.
pub fn solve_subgoal_bounded(
    task: &PackedTask,
    start: &State,
    goal_pos: &[u32],
    goal_num: &[NumPre],
    cost_fluent: usize,
    bound: f64,
    threads: usize,
) -> (Option<Vec<usize>>, bool) {
    match search_from(task, start, goal_pos, goal_num, Some(cost_fluent), bound, threads) {
        PlanResult::Plan { ops, .. } => (Some(ops), false),
        PlanResult::Unsolvable { capped, .. } => (None, capped),
    }
}
