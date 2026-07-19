//! Sequential portfolio scheduler (ferroplan-roadmap.md Phase 6).
//!
//! Run a small set of COMPLEMENTARY classical configurations on the same
//! problem under one shared, deterministic eval budget, instead of betting
//! the whole budget on one. Members restart per round with doubling slices
//! (the classic sequential-portfolio schedule): a config that would win
//! quickly wins in an early round at small cost; a config that needs depth
//! gets it in later rounds. The budget is an EVALUATED-STATE pool — never
//! wall clock — charged by each member's actual count, so the whole
//! schedule is thread-count and machine independent (the house determinism
//! contract).
//!
//! v1 members (all existing machinery, fixed order):
//!   1. `ladder`  — EHC → bounded LAMA rung → weighted best-first (the
//!      library default; wins most instances in round 0).
//!   2. `lama`    — the landmark/preferred-operator rung alone, uncapped
//!      within its slice (wins plateau domains: barman, parking,
//!      floortile).
//!   3. `bfs-w3`  — plain weighted best-first at w_h = 3 (middle greed, no
//!      EHC detour — wins where helpful-action pruning misleads).
//!   4. `bfs-w1`  — near-uniform best-first (depth-quality leaning).
//!
//! Coverage-first: the first plan any member finds is returned, tagged
//! with the winner's name (the portfolio-level anytime "global best" over
//! METRICS stays with the downstream cost/length sweeps, which run on the
//! returned plan exactly as for any single config). A member whose
//! COMPLETE search proves exhaustion (un-capped Unsolvable) settles the
//! whole task as unsolvable early.

use crate::packed::PackedTask;
use crate::search::{plan, search_from, PlanResult, SearchCfg};

/// First slice per member; doubles each round.
const SLICE0: usize = 50_000;

pub struct Outcome {
    pub ops: Option<Vec<usize>>,
    pub evaluated: usize,
    /// Name of the member that produced the plan (for the report note).
    pub winner: Option<&'static str>,
}

pub fn solve(task: &PackedTask, threads: usize, cfg: SearchCfg) -> Outcome {
    let names: [&'static str; 4] = ["ladder", "lama", "bfs-w3", "bfs-w1"];
    let mut alive = [true; 4];
    let mut pool = cfg.max_eval;
    let mut evaluated = 0usize;
    let mut round = 0u32;

    // Budget-aware phase A (the settled Phase 6 verdict): the DEFAULT member
    // runs to its NATURAL END on the FULL pool before diversification spends
    // anything. The doubling schedule preempted the ladder and net-LOST 11
    // corpus instances (sokoban −7, visit-all −4 — domains where the ladder
    // needs the whole budget; diversification won only +2). Ladder-first
    // makes portfolio coverage ≥ default BY CONSTRUCTION: the ladder sees
    // exactly the default's budget, and the others run only on what it left
    // behind (an early internal wall — node cap, LAMA cap, dead end).
    // `FF_PORTFOLIO_SLICED=1` restores the pure doubling schedule.
    if std::env::var("FF_PORTFOLIO_SLICED").is_err() {
        let (ops, used, _) = run_member(task, 0, threads, cfg, pool);
        evaluated += used;
        pool = pool.saturating_sub(used.max(1));
        if let Some(ops) = ops {
            return Outcome {
                ops: Some(ops),
                evaluated,
                winner: Some(names[0]),
            };
        }
        alive[0] = false;
    }

    while pool > 0 && alive.iter().any(|&a| a) {
        let slice = (SLICE0 << round).min(pool);
        for (m, &name) in names.iter().enumerate() {
            if !alive[m] || pool == 0 {
                continue;
            }
            let budget = slice.min(pool);
            let (ops, used, proven_unsolvable) = run_member(task, m, threads, cfg, budget);
            evaluated += used;
            pool = pool.saturating_sub(used.max(1));
            if let Some(ops) = ops {
                return Outcome {
                    ops: Some(ops),
                    evaluated,
                    winner: Some(name),
                };
            }
            if proven_unsolvable {
                // A COMPLETE member exhausted the space under no bound:
                // the task is unsolvable, no schedule can change that.
                return Outcome {
                    ops: None,
                    evaluated,
                    winner: None,
                };
            }
            // A member that spent less than its slice without a plan or a
            // proof hit an internal wall (node cap, dead end): re-running
            // it bigger cannot help less than a fresh slice can — keep it
            // alive only if it actually consumed the slice (more budget
            // could genuinely reach further).
            if used < budget {
                alive[m] = false;
            }
        }
        round += 1;
    }
    Outcome {
        ops: None,
        evaluated,
        winner: None,
    }
}

/// One member, one bounded run. Returns (plan, evals used, proven-unsolvable).
fn run_member(
    task: &PackedTask,
    member: usize,
    threads: usize,
    cfg: SearchCfg,
    budget: usize,
) -> (Option<Vec<usize>>, usize, bool) {
    match member {
        0 => {
            let o = plan(
                task,
                threads,
                SearchCfg {
                    max_eval: budget,
                    ..cfg
                },
                true,
            );
            // plan() folds EHC + LAMA + best-first; its None is budget-capped
            // in practice — never claim a proof through the wrapper.
            (o.ops, o.evaluated.max(1), false)
        }
        1 => match crate::lama::search(task, threads, budget, &[]) {
            Some((ops, ev)) => (Some(ops), ev.max(1), false),
            None => (None, budget, false), // lama doesn't report evals on failure
        },
        _ => {
            let wh = if member == 2 { 3.0 } else { 1.0 };
            match search_from(
                task,
                &task.initial(),
                &task.goal_pos,
                &task.goal_num,
                None,
                f64::INFINITY,
                threads,
                SearchCfg::from_weights(1.0, wh, Some(budget)),
                &[],
                None,
                None,
            ) {
                PlanResult::Plan { ops, evaluated, .. } => (Some(ops), evaluated.max(1), false),
                PlanResult::Unsolvable { evaluated, capped } => (None, evaluated.max(1), !capped),
            }
        }
    }
}
