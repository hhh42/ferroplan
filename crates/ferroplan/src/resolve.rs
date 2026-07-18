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
use crate::search::{solve_subgoal, solve_subgoal_avoiding};

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

/// The per-subgoal LAMA rung (the second half of the 0.9 text-path
/// unification): when a subgoal's plain best-first solve fails, try the
/// bounded landmark/preferred-operator search for exactly this
/// (start, subgoal) pair before conceding a merge. Landmarks are recomputed
/// per call, so the count is sound for the piece being solved. Same gate
/// and cap discipline as the library ladder's rung (`FF_NO_LAMA=1`
/// removes it); a None here costs one bounded search and the cascade
/// merges exactly as before.
fn lama_rung(
    task: &PackedTask,
    start: &State,
    g: &Subgoal,
    threads: usize,
    cfg: crate::search::SearchCfg,
) -> Option<Vec<usize>> {
    if std::env::var("FF_NO_LAMA").is_ok() {
        return None;
    }
    const LAMA_CAP: usize = 400_000;
    crate::lama::search_subgoal(
        task,
        start,
        &g.pos,
        &g.num,
        threads,
        LAMA_CAP.min(cfg.max_eval),
        &[],
    )
    .map(|(ops, _)| ops)
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

    // Per-SUBGOAL solves are BOUNDED probes (the 0.9 text-path unification,
    // second half): a subgoal unsolvable in isolation used to burn the full
    // eval budget proving it before every merge — on barman11 p01 the
    // cascade (9 groups, 7 merges) never finished at any tested budget.
    // A bounded probe just merges EARLIER, which is heuristic territory:
    // completeness is untouched because the monolithic endpoint below runs
    // the complete full-budget ladder. Measured: barman11 p01 text path
    // never-finishes -> solves.
    const SUB_CAP: usize = 100_000;
    let sub_cfg = crate::search::SearchCfg {
        max_eval: SUB_CAP.min(cfg.max_eval),
        ..cfg
    };

    loop {
        let monolithic = groups.len() == 1;
        // Phase A — coarse parallel: solve each group from the initial state.
        // One thread per subplanner when there are many groups (coarse
        // parallelism); all threads for the single monolithic fallback.
        //
        // The MONOLITHIC case is the whole problem, so it gets the full
        // library ladder — EHC, the bounded LAMA rung (landmark counting +
        // preferred operators), then the complete weighted best-first —
        // instead of bare best-first. This closes the 0.9 scope cut ("the
        // text path lacks the LAMA rung"): the partition path now solves
        // exactly when the library path does, which is the module's stated
        // doctrine. Per-SUBGOAL landmark guidance for the non-monolithic
        // groups remains future work (goal_landmarks is whole-goal today).
        let subplans: Vec<Option<Vec<usize>>> = if monolithic {
            vec![crate::search::plan(task, threads, cfg, true).ops]
        } else {
            par::par_map(&groups, threads, |g| {
                if g.is_empty() {
                    Some(Vec::new())
                } else {
                    solve_subgoal(task, &init, &g.pos, &g.num, 1, sub_cfg)
                        .or_else(|| lama_rung(task, &init, g, 1, sub_cfg))
                }
            })
        };

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
            // Protect already-achieved siblings: forbid ops that would delete one
            // of their goal facts (the ESPC hard-constraint / ∞-penalty form). If
            // the subgoal is solvable under that protection it provably can't break
            // a sibling; only if it's infeasible do we relax and let the conflict
            // check below trigger a merge.
            let protected: crate::hash::FxHashSet<u32> = (0..i)
                .filter(|&j| done[j])
                .flat_map(|j| groups[j].pos.iter().copied())
                .collect();
            let forbidden: Vec<bool> = if protected.is_empty() {
                Vec::new()
            } else {
                (0..task.n_ops)
                    .map(|oi| task.del.slice(oi).iter().any(|f| protected.contains(f)))
                    .collect()
            };

            let pre = subplans[i].as_ref().unwrap();
            let ops = if protected.is_empty() && replay_ok(task, &state, pre, &groups[i]) {
                pre.clone() // no siblings to protect yet — reuse the from-init plan
            } else {
                let protected_solve = solve_subgoal_avoiding(
                    task,
                    &state,
                    &groups[i].pos,
                    &groups[i].num,
                    &forbidden,
                    threads,
                    sub_cfg,
                );
                match protected_solve
                    .or_else(|| {
                        solve_subgoal(
                            task,
                            &state,
                            &groups[i].pos,
                            &groups[i].num,
                            threads,
                            sub_cfg,
                        )
                    })
                    .or_else(|| lama_rung(task, &state, &groups[i], threads, sub_cfg))
                {
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
