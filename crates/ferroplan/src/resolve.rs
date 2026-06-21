//! Partition-and-resolve control loop (the SGPlan core, adapted to numeric
//! STRIPS — see docs/sgplan6-spec.md §5,§9).
//!
//! Each outer iteration:
//!   Phase A (PARALLEL, coarse): solve every group's subgoal from the INITIAL
//!     state independently — one `ffdp` subplanner per group, run concurrently.
//!   Phase B (sequential, validated compose): replay each group's subplan on the
//!     evolving real state; if it no longer applies, re-solve from the current
//!     state with the full data-parallel subplanner. Reject a step that breaks an
//!     already-achieved sibling's subgoal.
//!   Resolution: on any stuck/conflict, MERGE the offending group with a neighbor
//!     (coarsen granularity) and retry. When merging collapses to one group the
//!     subproblem IS the whole problem, i.e. a monolithic `ffdp` fallback — so
//!     `sgp` is solvable exactly when `ffdp` is.

use crate::packed::{PackedTask, State};
use crate::par;
use crate::search::solve_subgoal;

use crate::partition::{interaction_partition, merge_at, merge_with_neighbor, Subgoal};

#[derive(Clone, Copy)]
pub struct Stats {
    pub init_groups: usize,
    pub final_groups: usize,
    pub merges: usize,
    pub fallback: bool, // collapsed to a single (monolithic) group
}

pub enum Solved {
    Plan(Vec<usize>, Stats),
    Unsolvable,
}

/// Does op-sequence `ops` apply from `state` and achieve `g`? (cheap replay).
fn replay_ok(task: &PackedTask, state: &State, ops: &[usize], g: &Subgoal) -> bool {
    let mut s = state.clone();
    for &oi in ops {
        if !task.op_applicable(oi, &s) {
            return false;
        }
        s = task.apply(oi, &s);
    }
    task.goal_met_with(&s, &g.pos, &g.num)
}

pub fn solve(
    task: &PackedTask,
    threads: usize,
    cfg: crate::search::SearchCfg,
    mutex_groups: &[Vec<u32>],
) -> Solved {
    let init = task.initial();
    // Seed from the goal-interaction graph over guidance variables (mutex groups);
    // falls back to the finest partition when no groups are supplied.
    let mut groups = interaction_partition(task, mutex_groups);
    let init_groups = groups.len();
    let mut merges = 0usize;

    loop {
        let monolithic = groups.len() == 1;
        // Phase A — coarse parallel: solve each group from the initial state.
        // One thread per subplanner when there are many groups (coarse
        // parallelism); all threads for the single monolithic fallback.
        let sub_threads = if monolithic { threads } else { 1 };
        let subplans: Vec<Option<Vec<usize>>> = par::par_map(&groups, threads, |g| {
            if g.is_empty() {
                Some(Vec::new())
            } else {
                solve_subgoal(task, &init, &g.pos, &g.num, sub_threads, cfg)
            }
        });

        // A group unsolvable in isolation → it likely needs a sibling's effects
        // first; merge and retry (or, if monolithic, genuinely unsolvable).
        if let Some(i) = subplans.iter().position(|s| s.is_none()) {
            if monolithic {
                return Solved::Unsolvable;
            }
            merge_with_neighbor(&mut groups, i);
            merges += 1;
            continue;
        }

        // Phase B — sequential validated composition on the evolving state.
        let mut state = init.clone();
        let mut plan: Vec<usize> = Vec::new();
        let mut done = vec![false; groups.len()];
        // (stuck group, the specific sibling it broke if any)
        let mut conflict: Option<(usize, Option<usize>)> = None;

        for i in 0..groups.len() {
            if task.goal_met_with(&state, &groups[i].pos, &groups[i].num) {
                done[i] = true;
                continue; // already achieved (e.g. by a sibling's subplan)
            }
            // reuse the from-init subplan if it still applies, else re-solve
            let pre = subplans[i].as_ref().unwrap();
            let ops = if replay_ok(task, &state, pre, &groups[i]) {
                pre.clone()
            } else {
                match solve_subgoal(task, &state, &groups[i].pos, &groups[i].num, threads, cfg) {
                    Some(o) => o,
                    None => {
                        conflict = Some((i, None));
                        break;
                    }
                }
            };
            // apply, but reject if it breaks an already-achieved sibling
            let mut ns = state.clone();
            for &oi in &ops {
                ns = task.apply(oi, &ns);
            }
            let breaker = (0..i)
                .find(|&j| done[j] && !task.goal_met_with(&ns, &groups[j].pos, &groups[j].num));
            if let Some(j) = breaker {
                conflict = Some((i, Some(j)));
                break;
            }
            state = ns;
            plan.extend(ops);
            done[i] = true;
        }

        if conflict.is_none() && task.goal_met_with(&state, &task.goal_pos, &task.goal_num) {
            return Solved::Plan(
                plan,
                Stats {
                    init_groups,
                    final_groups: groups.len(),
                    merges,
                    fallback: monolithic,
                },
            );
        }

        // Resolve: coalesce the actual conflicting pair (semantic merge); else the
        // stuck group with a neighbor.
        if monolithic {
            // single group already = whole goal but compose didn't satisfy it:
            // treat as unsolvable (matches ffdp on the full problem).
            return Solved::Unsolvable;
        }
        let last = groups.len() - 1;
        match conflict {
            Some((i, Some(j))) => merge_at(&mut groups, i, j),
            Some((i, None)) => merge_with_neighbor(&mut groups, i),
            None => merge_with_neighbor(&mut groups, last),
        };
        merges += 1;
    }
}
