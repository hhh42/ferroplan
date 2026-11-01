//! THE PARALLEL RUNG PORTFOLIO (0.31 Lane A′, docs/roadmap-0.31.md).
//!
//! What the matrix said (Phase 0.5: 936 cells × 5 rungs, each rung given
//! the whole wall; Lane B's three rungs after): the banked ladder solves
//! 662, the oracle over its five rungs 706 and over all eight 759, and the
//! ladder solves no cell that no single rung does. The cells between are
//! reachable only by running rungs SIDE BY SIDE — and beside the ladder,
//! not instead of it (the ladder with LAMA-novelty beside it 712; with the
//! refreshing driver as well 747; with EHC and bfs as well 753; see
//! [`members`]). That is this module: at `--threads N ≥ 2`, under
//! `FF_PARALLEL_RUNGS=1`, the ladder runs as worker 0 exactly as it would
//! alone (one h-eval thread, its own slices, its own hand-downs), and N−1
//! of its rungs run beside it with the whole wall each; the first plan
//! wins and the others are cancelled through the per-call budget's flag,
//! which every rung already polls at its batch boundary.
//!
//! THE CONTRACT IT BREAKS, by name: "byte-identical for any worker count"
//! (`search.rs`, the module header). Which member wins is a race on wall
//! time, so the plan can differ from run to run and from the one-thread
//! ladder's. Opt-in until the mco subset sweep prices it (band +4 % of the
//! mco cells, kill < +20 rows or any t2 board losing more than it gains);
//! the notes on every solve say it ran and which member won.
//!
//! MEMORY: the workers share one process and one declared budget
//! (`FF_MEM_BUDGET_GB`). A side rung's measured wall (`MemWall::arm_rung`)
//! reads the PROCESS resident set, so the bets die first when the sum
//! crosses the trip line, and hand their arenas back; the ladder worker
//! keeps the ladder's own rules. The runner's cap is still the runner's
//! cap: a t8 row that fit alone can be lost here, and that is what the
//! kill clause watches.

use crate::packed::PackedTask;
use crate::search::{self, PlanOutcome, Rung, SearchCfg};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{mpsc, Arc};
use std::time::Duration;

/// Opt-in: `FF_PARALLEL_RUNGS=1`.
pub fn enabled() -> bool {
    std::env::var("FF_PARALLEL_RUNGS").is_ok_and(|v| v.trim() == "1")
}

/// One worker of the portfolio.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Member {
    /// The whole ladder, as it runs alone (one h-eval thread).
    Ladder,
    /// One rung with the whole wall — the matrix hatch's shape.
    Rung(Rung),
    /// The best-first fallback with its refill loop and nothing before it
    /// (the matrix's `bfs`).
    BestFirst,
}

impl Member {
    pub fn name(self) -> &'static str {
        match self {
            Member::Ladder => "ladder",
            Member::BestFirst => "bfs",
            Member::Rung(r) => r.name(),
        }
    }
}

/// The members for `threads` workers: the ladder first, then the rungs in
/// the order of their MARGINAL cover beside everything before them on the
/// matrix, greedy (the Lane B read of 2026-11-01, eight rungs on 936
/// cells: the ladder 662; + LAMA-novelty 712; + the driver with R refresh
/// and forgetting 747; + EHC 750; + bfs 753; + novelty-light 755; + the
/// front end 757; + plain LAMA 758; + the plain driver 759 = the oracle).
/// The two Lane B rungs stack — 50 cells each beyond the ladder, 15 shared,
/// 85 in union — and the plain forms of LAMA and the driver come last
/// because their Lane B forms cover them (98 cells to 9, 112 to 28). The
/// plain driver is the ninth and has no seat. More threads than members
/// run the members; one thread is the ladder.
pub fn members(threads: usize) -> Vec<Member> {
    const ORDER: [Member; 8] = [
        Member::Ladder,
        Member::Rung(Rung::LamaNov),
        Member::Rung(Rung::DriverX),
        Member::Rung(Rung::Ehc),
        Member::BestFirst,
        Member::Rung(Rung::Novlight),
        Member::Rung(Rung::Front),
        Member::Rung(Rung::Lama),
    ];
    ORDER[..threads.clamp(1, ORDER.len())].to_vec()
}

pub struct Report {
    pub outcome: PlanOutcome,
    pub winner: Option<Member>,
    /// What the winning member narrated — for the ladder, the rung inside
    /// it that solved.
    pub rung: Option<String>,
    pub workers: usize,
}

thread_local! {
    /// The report line of the portfolio that last ran on this thread,
    /// taken by `api.rs` into the solve's notes (solved or not: a sweep row
    /// must say the mode ran).
    static NOTE: std::cell::RefCell<Option<String>> = const { std::cell::RefCell::new(None) };
}

/// The portfolio's report line for this thread's last solve, cleared by
/// the read.
pub fn take_note() -> Option<String> {
    NOTE.with(|n| n.borrow_mut().take())
}

/// How often the parent looks at its own caller's flag while it waits.
const POLL: Duration = Duration::from_millis(20);
/// A worker runs a whole search: the main thread's 8 MiB, not a spawned
/// thread's 2 (virtual; nothing is touched until used).
const STACK: usize = 64 << 20;

/// Run the members side by side; the first plan wins.
pub fn run(
    task: &PackedTask,
    threads: usize,
    cfg: SearchCfg,
    orbit: Option<&crate::orbits::OrbitMap>,
) -> Report {
    let members = members(threads);
    // The workers' should-continue flag, the polarity of `Options::
    // should_continue` (true = go on; `search::cancelled` reads a false as
    // the withdrawal): dropped to false once a member wins or the caller
    // withdraws. The parent's deadline travels to every worker with its
    // clock already running.
    let go = Arc::new(AtomicBool::new(true));
    let parent = search::call_budget();
    let handed = search::take_ladder_base();
    let (tx, rx) = mpsc::channel::<(usize, PlanOutcome, Option<String>)>();
    let mut winner: Option<Member> = None;
    let mut rung: Option<String> = None;
    let mut ops: Option<Vec<usize>> = None;
    let mut fell_back = true;
    let mut evaluated = 0usize;
    let mut ladder_capped: Option<bool> = None;
    let debug = std::env::var("FF_WALL_DEBUG").is_ok();
    let t0 = crate::clock::Clock::now();
    if debug {
        let names: Vec<&str> = members.iter().map(|m| m.name()).collect();
        eprintln!("parallel: {} workers ({})", members.len(), names.join(", "));
    }
    std::thread::scope(|scope| {
        let mut handles = Vec::with_capacity(members.len());
        for (i, &m) in members.iter().enumerate() {
            let tx = tx.clone();
            let go = Arc::clone(&go);
            let deadline = parent.deadline;
            let h = std::thread::Builder::new()
                .name(format!("ff-{}", m.name()))
                .stack_size(STACK)
                .spawn_scoped(scope, move || {
                    let _budget = search::arm_call_budget_inherited(deadline, Some(go));
                    if let (Member::Ladder, Some(base)) = (m, handed) {
                        search::hand_ladder_wall(base);
                    }
                    let o = run_member(task, cfg, m, orbit);
                    let narrated = search::take_solved_by();
                    let _ = tx.send((i, o, narrated));
                })
                .expect("spawn a rung worker");
            handles.push(h);
        }
        drop(tx);
        let mut done = 0;
        while done < members.len() {
            match rx.recv_timeout(POLL) {
                Ok((i, o, narrated)) => {
                    done += 1;
                    evaluated += o.evaluated;
                    if members[i] == Member::Ladder {
                        ladder_capped = Some(o.capped);
                    }
                    if o.ops.is_some() && winner.is_none() {
                        winner = Some(members[i]);
                        rung = narrated;
                        ops = o.ops;
                        fell_back = o.ehc_fell_back;
                        go.store(false, Ordering::Relaxed);
                        if debug {
                            eprintln!(
                                "parallel: {} won at {:.2}s, cancelling {} worker(s)",
                                members[i].name(),
                                t0.elapsed_secs(),
                                members.len() - done
                            );
                        }
                    }
                }
                Err(mpsc::RecvTimeoutError::Timeout) => {
                    if parent.cancelled() {
                        go.store(false, Ordering::Relaxed);
                    }
                }
                // Every sender is gone: a worker that panicked sent nothing.
                Err(mpsc::RecvTimeoutError::Disconnected) => break,
            }
        }
        for h in handles {
            // A worker that panicked is a worker without a plan; the
            // others' verdicts stand.
            let _ = h.join();
        }
    });
    let names: Vec<&str> = members.iter().map(|m| m.name()).collect();
    let verdict = match (winner, &rung) {
        (Some(w), Some(r)) => format!("won by {} ({r}) at {:.2}s", w.name(), t0.elapsed_secs()),
        (Some(w), None) => format!("won by {} at {:.2}s", w.name(), t0.elapsed_secs()),
        (None, _) => format!("no member solved in {:.2}s", t0.elapsed_secs()),
    };
    NOTE.with(|n| {
        *n.borrow_mut() = Some(format!(
            "parallel rungs: {} workers ({}), timing-dependent; {verdict}",
            members.len(),
            names.join(", ")
        ))
    });
    if let Some(w) = winner {
        search::note_solved_by(&match &rung {
            Some(r) => format!("parallel/{} ({r})", w.name()),
            None => format!("parallel/{}", w.name()),
        });
    }
    // Unsolved: the ladder's own verdict is the verdict (the complete
    // search); a side rung's `None` is a hand-down, never a proof.
    let capped = ops.is_none() && ladder_capped.unwrap_or(true);
    Report {
        outcome: PlanOutcome {
            ops,
            evaluated,
            ehc_fell_back: fell_back,
            capped,
        },
        winner,
        rung,
        workers: members.len(),
    }
}

fn run_member(
    task: &PackedTask,
    cfg: SearchCfg,
    m: Member,
    orbit: Option<&crate::orbits::OrbitMap>,
) -> PlanOutcome {
    match m {
        Member::Ladder => search::ladder(task, 1, cfg, true, &[], orbit),
        Member::BestFirst => search::ladder(task, 1, cfg, false, &[], orbit),
        Member::Rung(r) => match search::run_rung(task, 1, cfg, &[], r) {
            Some((ops, evaluated)) => PlanOutcome {
                ops: Some(ops),
                evaluated,
                ehc_fell_back: r != Rung::Ehc,
                capped: false,
            },
            None => PlanOutcome {
                ops: None,
                evaluated: 0,
                ehc_fell_back: r != Rung::Ehc,
                capped: true,
            },
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn members_follow_the_matrix_order() {
        let names =
            |t: usize| -> Vec<&'static str> { members(t).iter().map(|m| m.name()).collect() };
        assert_eq!(names(1), ["ladder"]);
        assert_eq!(names(2), ["ladder", "LAMA-novelty"]);
        assert_eq!(
            names(4),
            ["ladder", "LAMA-novelty", "novelty-driverx", "EHC"]
        );
        assert_eq!(names(64).len(), 8);
        assert_eq!(names(0), ["ladder"]);
    }
}
