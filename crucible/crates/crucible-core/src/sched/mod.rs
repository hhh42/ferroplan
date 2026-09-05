//! What runs next, and what does not need to run again.
//!
//! The four modules under this one answer one question each, and the loop below
//! spends them:
//!
//! * [`resume`] -- may a prior measurement stand, or must it be re-measured?
//! * [`quiet`] -- may a board start yet?
//! * [`tier`] -- in what order, and how long will it take?
//! * [`budget`] -- does this board fit the box at all?
//!
//! # MAX_PASSES is gone, and that is the point
//!
//! The shell driver (`benchmarks/cut25-sweeps.sh:173-189`, with its
//! `remaining()` at 158-165) makes whole-board passes: each pass retries every
//! board not yet banked clean, and after eight of them it prints "gave up"
//! (line 179) and exits 1 (line 181). That structure was forced by its
//! atom. A board contaminated anywhere was refused everywhere, so a pass could
//! end with the remaining set exactly as large as it started -- three passes
//! and nine board-hours on 2026-08-21, re-measuring rows that were clean the
//! first time -- and with no guarantee of shrinking, the only defence against
//! looping forever was a counter.
//!
//! With per-instance resume (`PER-INSTANCE-RETRY.md`) the atom is an INSTANCE,
//! and the shape of the problem changes underneath the loop. Every attempt
//! banks the instances it measured cleanly, so the remaining set strictly
//! shrinks unless the whole attempt was dirty -- which means the loop
//! terminates on its own, and a pass counter would only ever fire on a box that
//! is genuinely too busy to measure anything. Exiting 1 there is precisely
//! backwards: a sweep left overnight on a laptop that is busy by day and free
//! by night should still be running in the morning, not dead since 22:15 with
//! nine boards to go.
//!
//! So the counter becomes a stall guard. Zero-progress attempts are counted;
//! after [`LoopConfig::stall_after`] consecutive ones the loop emits
//! [`Event::Stalled`] -- visible, so a human can act -- and backs off
//! exponentially to a cap of about an hour. It never gives up, because there is
//! nothing to give up ON: the work that remains is the work that remains, and
//! the box will be free eventually.
//!
//! # Order
//!
//! The first pass keeps the manifest's order, which is cheapest-first for the
//! reason the driver states: "a driver that dies early banks something".
//! Later passes prefer the board with the FEWEST remaining instances, so a
//! board sitting on three dirty rows finishes and leaves the queue instead of
//! waiting behind a board with two hundred. Both orders are total -- ties break
//! on manifest position -- so a resumed sweep schedules identically to the one
//! it resumed.
//!
//! # No database in here
//!
//! Queue state arrives through [`Runner`], a four-method trait over plain
//! structs. The loop's whole behaviour -- ordering, progress accounting, the
//! stall guard, the back-off curve -- is therefore testable in microseconds
//! against a fake, which matters because the failure it defends against takes
//! eight hours to reproduce for real.

pub mod budget;
pub mod quiet;
pub mod referee;
pub mod resume;
pub mod tier;

pub use budget::{Accountant, Demand, Oversubscribed};
pub use quiet::Gate;
pub use referee::{Bank, Facts, Owe, Rule, Verdict};
pub use resume::{Conditions, InstanceKey, Reject, Resume, RowKey, RunParams};
pub use tier::{Scheduled, Thresholds, Tier};

use std::time::Duration;

/// One board as the loop sees it: an identity, a place in the manifest, and how
/// much of it is still owed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BoardState {
    pub id: String,
    /// Index in the manifest's board list. The manifest's order is
    /// cheapest-first and its file order is load-bearing elsewhere, so it is
    /// carried rather than re-derived.
    pub position: usize,
    /// Instances with no clean row yet. Zero means banked.
    pub remaining: usize,
}

/// What one attempt at one board achieved.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Attempt {
    /// Instances that banked a clean row this time.
    pub banked: usize,
    /// What is still owed afterwards.
    pub remaining: usize,
    /// The attempt was measured under contention throughout. Nothing banked,
    /// and it is not the board's fault -- the distinction matters because a
    /// dirty attempt is a reason to wait for the box, while an unproductive
    /// clean attempt is a reason to look at the board.
    pub dirty: bool,
}

/// Whether the loop should keep going after a back-off.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Next {
    Continue,
    /// The operator asked to stop. The remaining work is still remaining; it is
    /// not failed, and the next run picks it up.
    Stop,
}

/// Everything the loop reports. Consumed by whatever writes the operator log --
/// the loop itself neither prints nor logs, so its behaviour can be asserted
/// rather than scraped.
#[derive(Debug, Clone, PartialEq)]
pub enum Event {
    PassStarted {
        pass: u32,
        boards: Vec<String>,
    },
    Attempted {
        board: String,
        banked: usize,
        before: usize,
        after: usize,
        dirty: bool,
    },
    /// Neither banked nor shrank, and not dirty either: the board is failing to
    /// make progress for a reason the scheduler cannot see. Named separately
    /// because "the box is busy" and "this board is stuck" need different
    /// responses from a human.
    Unproductive {
        board: String,
        remaining: usize,
    },
    /// The remaining set GREW. A runner bug; reported, and never counted as
    /// progress -- counting it would let the loop spin forever without ever
    /// reaching the back-off that exists to stop exactly that.
    Grew {
        board: String,
        before: usize,
        after: usize,
    },
    /// [`LoopConfig::stall_after`] consecutive passes have banked nothing.
    Stalled {
        consecutive: u32,
        backoff: Duration,
        remaining: usize,
    },
    Finished {
        passes: u32,
        banked: usize,
    },
    Stopped {
        passes: u32,
        remaining: usize,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LoopConfig {
    /// Consecutive zero-progress passes before the loop says so out loud.
    pub stall_after: u32,
    /// The first wait after a zero-progress pass. The shell's `sleep 60`.
    pub retry_backoff: Duration,
    /// The ceiling on the doubling. About an hour: long enough not to hammer a
    /// box somebody is working on, short enough to catch the evening.
    pub max_backoff: Duration,
}

impl Default for LoopConfig {
    fn default() -> Self {
        LoopConfig {
            stall_after: 3,
            retry_backoff: Duration::from_secs(60),
            max_backoff: Duration::from_secs(3600),
        }
    }
}

impl LoopConfig {
    /// How long to wait after `consecutive` zero-progress passes.
    ///
    /// Flat at [`Self::retry_backoff`] until the stall threshold -- a single
    /// contended pass is ordinary and does not deserve an hour -- then doubling
    /// to the cap.
    pub fn backoff(&self, consecutive: u32) -> Duration {
        let steps = consecutive.saturating_sub(self.stall_after);
        let mut d = self.retry_backoff;
        for _ in 0..steps {
            if d >= self.max_backoff {
                break;
            }
            d = d.saturating_mul(2);
        }
        d.min(self.max_backoff)
    }
}

/// The queue, and the thing that actually measures.
///
/// Deliberately small: everything the loop needs to decide what runs next, and
/// nothing about how a board is measured or where its rows are stored.
pub trait Runner {
    /// The boards with work outstanding. Called fresh at the top of each pass,
    /// so a board completed by another means simply stops appearing.
    fn boards(&mut self) -> Vec<BoardState>;

    /// Measure this board's remaining instances. The implementation owns
    /// admission ([`quiet`], [`budget`]), ordering ([`tier`]) and reuse
    /// ([`resume`]); the loop only reads what came back.
    fn attempt(&mut self, board: &BoardState) -> Attempt;

    /// Sleep out a back-off, or ask to stop. The only place the loop blocks,
    /// which is what lets a test run the whole thing instantly.
    fn wait(&mut self, backoff: Duration) -> Next {
        let _ = backoff;
        Next::Continue
    }

    /// Report. Default is to say nothing.
    fn event(&mut self, event: Event) {
        let _ = event;
    }

    /// The operator asked to stop, mid-pass. Checked before every attempt,
    /// so an interrupt ends the pass at the board it interrupted instead of
    /// walking every remaining board through a zero-banked attempt.
    fn stopped(&mut self) -> bool {
        false
    }
}

/// How the loop ended.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Outcome {
    pub passes: u32,
    pub attempts: u32,
    pub banked: usize,
    /// True when the queue drained. False means the operator stopped it, with
    /// work still outstanding -- not a failure, and not an exit code.
    pub complete: bool,
    pub remaining: usize,
}

/// Board order for a pass.
///
/// Pass 1 is the manifest's own order: cheapest first, so a driver that dies
/// early banks something. Later passes prefer the fewest remaining instances,
/// so nearly-done boards finish and leave. Both comparators are TOTAL --
/// `position` is unique -- so there is no tie for a sort to resolve arbitrarily
/// and two runs of the same queue schedule identically.
pub fn order_boards(boards: &[BoardState], pass: u32) -> Vec<BoardState> {
    let mut out: Vec<BoardState> = boards.iter().filter(|b| b.remaining > 0).cloned().collect();
    if pass <= 1 {
        out.sort_by_key(|b| b.position);
    } else {
        out.sort_by_key(|b| (b.remaining, b.position));
    }
    out
}

/// Run the queue to completion, or until the operator stops it.
pub fn run(r: &mut dyn Runner, cfg: &LoopConfig) -> Outcome {
    let mut pass = 0u32;
    let mut attempts = 0u32;
    let mut banked = 0usize;
    let mut zero_progress = 0u32;

    loop {
        let boards = r.boards();
        let ordered = order_boards(&boards, pass + 1);
        if ordered.is_empty() {
            r.event(Event::Finished {
                passes: pass,
                banked,
            });
            return Outcome {
                passes: pass,
                attempts,
                banked,
                complete: true,
                remaining: 0,
            };
        }
        pass += 1;
        r.event(Event::PassStarted {
            pass,
            boards: ordered.iter().map(|b| b.id.clone()).collect(),
        });

        let mut shrank_by = 0usize;
        for b in &ordered {
            if r.stopped() {
                let remaining: usize =
                    ordered.iter().map(|b| b.remaining).sum::<usize>() - shrank_by;
                r.event(Event::Stopped {
                    passes: pass,
                    remaining,
                });
                return Outcome {
                    passes: pass,
                    attempts,
                    banked,
                    complete: false,
                    remaining,
                };
            }
            let a = r.attempt(b);
            attempts += 1;
            banked += a.banked;
            r.event(Event::Attempted {
                board: b.id.clone(),
                banked: a.banked,
                before: b.remaining,
                after: a.remaining,
                dirty: a.dirty,
            });
            if a.remaining > b.remaining {
                r.event(Event::Grew {
                    board: b.id.clone(),
                    before: b.remaining,
                    after: a.remaining,
                });
                continue;
            }
            // Progress is measured on the QUEUE, not on the runner's own
            // report: `banked` is what the attempt claims, `before - after` is
            // what the queue actually lost. Only the second can end the loop.
            let shrank = b.remaining - a.remaining;
            shrank_by += shrank;
            if shrank == 0 && !a.dirty {
                r.event(Event::Unproductive {
                    board: b.id.clone(),
                    remaining: a.remaining,
                });
            }
        }

        if shrank_by > 0 {
            // Any progress at all resets both the counter and the curve: the
            // box came back, and the next contended pass should wait a minute,
            // not an hour.
            zero_progress = 0;
            continue;
        }

        zero_progress += 1;
        let backoff = cfg.backoff(zero_progress);
        let remaining: usize = ordered.iter().map(|b| b.remaining).sum();
        if zero_progress >= cfg.stall_after {
            r.event(Event::Stalled {
                consecutive: zero_progress,
                backoff,
                remaining,
            });
        }
        if r.wait(backoff) == Next::Stop {
            r.event(Event::Stopped {
                passes: pass,
                remaining,
            });
            return Outcome {
                passes: pass,
                attempts,
                banked,
                complete: false,
                remaining,
            };
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;

    /// A queue with no database: boards, how much each owes, and a script of
    /// what the next attempt on each will achieve.
    #[derive(Default)]
    struct Fake {
        owed: BTreeMap<String, (usize, usize)>, // id -> (position, remaining)
        /// Instances banked per attempt, by board. Zero means a dirty attempt.
        bank_per_attempt: BTreeMap<String, usize>,
        dirty: bool,
        grow: bool,
        events: Vec<Event>,
        waits: Vec<Duration>,
        stop_after_waits: usize,
    }

    impl Fake {
        fn with(boards: &[(&str, usize, usize)]) -> Self {
            let mut f = Fake {
                stop_after_waits: 100,
                ..Default::default()
            };
            for (i, (id, per, remaining)) in boards.iter().enumerate() {
                f.owed.insert(id.to_string(), (i, *remaining));
                f.bank_per_attempt.insert(id.to_string(), *per);
            }
            f
        }

        fn attempted(&self) -> Vec<String> {
            self.events
                .iter()
                .filter_map(|e| match e {
                    Event::Attempted { board, .. } => Some(board.clone()),
                    _ => None,
                })
                .collect()
        }
    }

    impl Runner for Fake {
        fn boards(&mut self) -> Vec<BoardState> {
            self.owed
                .iter()
                .filter(|(_, (_, r))| *r > 0)
                .map(|(id, (position, remaining))| BoardState {
                    id: id.clone(),
                    position: *position,
                    remaining: *remaining,
                })
                .collect()
        }

        fn attempt(&mut self, board: &BoardState) -> Attempt {
            let per = *self.bank_per_attempt.get(&board.id).unwrap_or(&0);
            let banked = if self.dirty {
                0
            } else {
                per.min(board.remaining)
            };
            let remaining = if self.grow {
                board.remaining + 1
            } else {
                board.remaining - banked
            };
            if let Some(e) = self.owed.get_mut(&board.id) {
                e.1 = remaining;
            }
            Attempt {
                banked,
                remaining,
                dirty: self.dirty,
            }
        }

        fn wait(&mut self, backoff: Duration) -> Next {
            self.waits.push(backoff);
            if self.waits.len() >= self.stop_after_waits {
                Next::Stop
            } else {
                Next::Continue
            }
        }

        fn event(&mut self, event: Event) {
            self.events.push(event);
        }
    }

    /// THE POINT OF THE REWRITE. Twelve instances that bank one per attempt is
    /// twelve passes; the shell driver would have printed "gave up after 8
    /// passes" and exited 1 with the board four rows short.
    #[test]
    fn a_board_needing_more_than_eight_passes_still_finishes() {
        let mut f = Fake::with(&[("slow", 1, 12)]);
        let out = run(&mut f, &LoopConfig::default());
        assert!(out.complete);
        assert_eq!(out.passes, 12);
        assert_eq!(out.banked, 12);
        assert!(f.waits.is_empty(), "progress never waits");
    }

    /// The first pass is the manifest's own: cheapest first, because a driver
    /// that dies early banks something.
    #[test]
    fn the_first_pass_keeps_manifest_order() {
        let boards = vec![
            BoardState {
                id: "big".into(),
                position: 0,
                remaining: 400,
            },
            BoardState {
                id: "small".into(),
                position: 1,
                remaining: 3,
            },
        ];
        let ids: Vec<String> = order_boards(&boards, 1).into_iter().map(|b| b.id).collect();
        assert_eq!(ids, vec!["big", "small"]);
    }

    /// Later passes prefer the board with the fewest remaining instances, so a
    /// board sitting on three dirty rows finishes and leaves the queue instead
    /// of queueing behind two hundred.
    #[test]
    fn later_passes_prefer_the_nearly_finished_board() {
        let boards = vec![
            BoardState {
                id: "big".into(),
                position: 0,
                remaining: 400,
            },
            BoardState {
                id: "small".into(),
                position: 1,
                remaining: 3,
            },
            BoardState {
                id: "mid".into(),
                position: 2,
                remaining: 3,
            },
        ];
        let ids: Vec<String> = order_boards(&boards, 2).into_iter().map(|b| b.id).collect();
        assert_eq!(
            ids,
            vec!["small", "mid", "big"],
            "ties break on manifest position, so the order is total"
        );
    }

    /// Banked boards leave the queue rather than being attempted with nothing
    /// to do.
    #[test]
    fn a_board_with_nothing_remaining_is_not_scheduled() {
        let boards = vec![
            BoardState {
                id: "done".into(),
                position: 0,
                remaining: 0,
            },
            BoardState {
                id: "todo".into(),
                position: 1,
                remaining: 5,
            },
        ];
        assert_eq!(order_boards(&boards, 1).len(), 1);
        assert_eq!(order_boards(&boards, 9).len(), 1);
    }

    /// A box that is busy all night banks nothing and must NOT exit 1 -- it
    /// says so, backs off, and is still there in the morning.
    #[test]
    fn a_dirty_box_stalls_and_backs_off_instead_of_giving_up() {
        let mut f = Fake::with(&[("board", 5, 20)]);
        f.dirty = true;
        f.stop_after_waits = 6;
        let out = run(&mut f, &LoopConfig::default());
        assert!(!out.complete, "stopped by the operator, not failed");
        assert_eq!(out.remaining, 20, "nothing was lost");
        assert_eq!(out.banked, 0);
        let stalls: Vec<&Event> = f
            .events
            .iter()
            .filter(|e| matches!(e, Event::Stalled { .. }))
            .collect();
        assert_eq!(
            stalls.len(),
            4,
            "silent for the first two, then said so every pass: {:?}",
            f.waits
        );
    }

    /// Flat for the first few -- one contended pass is ordinary -- then
    /// doubling to a cap of about an hour.
    #[test]
    fn the_backoff_is_flat_then_doubles_to_the_cap() {
        let cfg = LoopConfig::default();
        assert_eq!(cfg.backoff(1), Duration::from_secs(60));
        assert_eq!(cfg.backoff(3), Duration::from_secs(60));
        assert_eq!(cfg.backoff(4), Duration::from_secs(120));
        assert_eq!(cfg.backoff(5), Duration::from_secs(240));
        assert_eq!(cfg.backoff(9), Duration::from_secs(3600));
        assert_eq!(
            cfg.backoff(u32::MAX),
            Duration::from_secs(3600),
            "the cap holds however long the box stays busy"
        );
    }

    /// The box came back: the counter AND the curve reset, so the next
    /// contended pass waits a minute rather than an hour.
    #[test]
    fn any_progress_resets_the_stall_curve() {
        let mut f = Fake::with(&[("a", 0, 4), ("b", 2, 4)]);
        f.stop_after_waits = 3;
        let out = run(&mut f, &LoopConfig::default());
        // `a` banks nothing ever, `b` banks two per attempt: two productive
        // passes, then nothing but stalls.
        assert!(!out.complete);
        assert_eq!(out.banked, 4);
        assert!(
            f.waits.iter().all(|w| *w == Duration::from_secs(60)),
            "the curve restarted from the floor: {:?}",
            f.waits
        );
    }

    /// A clean attempt that banks nothing is a different problem from a
    /// contended one, and a human needs to know which.
    #[test]
    fn an_unproductive_clean_attempt_is_named_separately() {
        let mut f = Fake::with(&[("stuck", 0, 3)]);
        f.stop_after_waits = 1;
        run(&mut f, &LoopConfig::default());
        assert!(f
            .events
            .iter()
            .any(|e| matches!(e, Event::Unproductive { .. })));
        assert!(!f
            .events
            .iter()
            .any(|e| matches!(e, Event::Attempted { dirty: true, .. })));
    }

    /// A queue that grows is a runner bug. Counting it as progress would let
    /// the loop spin forever and never reach the back-off that exists to stop
    /// exactly that.
    #[test]
    fn a_growing_queue_is_reported_and_is_not_progress() {
        let mut f = Fake::with(&[("odd", 1, 2)]);
        f.grow = true;
        f.stop_after_waits = 1;
        let out = run(&mut f, &LoopConfig::default());
        assert!(!out.complete);
        assert!(f.events.iter().any(|e| matches!(e, Event::Grew { .. })));
        assert_eq!(f.waits.len(), 1, "it backed off rather than spinning");
    }

    /// An empty queue is finished, not stalled -- and it never attempts
    /// anything.
    #[test]
    fn an_empty_queue_finishes_immediately() {
        let mut f = Fake::default();
        let out = run(&mut f, &LoopConfig::default());
        assert!(out.complete);
        assert_eq!((out.passes, out.attempts), (0, 0));
        assert!(f.attempted().is_empty());
    }

    /// An interrupt ends the pass at the board it interrupted: the boards
    /// after it are not walked through zero-banked attempts.
    #[test]
    fn a_stop_ends_the_pass_at_the_interrupted_board() {
        struct Stopper(Fake, u32);
        impl Runner for Stopper {
            fn boards(&mut self) -> Vec<BoardState> {
                self.0.boards()
            }
            fn attempt(&mut self, b: &BoardState) -> Attempt {
                self.1 += 1;
                self.0.attempt(b)
            }
            fn event(&mut self, e: Event) {
                self.0.event(e)
            }
            fn stopped(&mut self) -> bool {
                self.1 >= 1
            }
        }
        let mut s = Stopper(Fake::with(&[("a", 1, 3), ("b", 1, 3), ("c", 1, 3)]), 0);
        let out = run(&mut s, &LoopConfig::default());
        assert!(!out.complete);
        assert_eq!(s.1, 1, "one attempt, then the stop was honoured");
        assert_eq!(out.remaining, 8);
        assert!(s
            .0
            .events
            .iter()
            .any(|e| matches!(e, Event::Stopped { .. })));
    }

    /// Every board gets its attempt in a pass, in the pass's order.
    #[test]
    fn a_pass_attempts_every_board_in_order() {
        let mut f = Fake::with(&[("first", 9, 9), ("second", 9, 9)]);
        run(&mut f, &LoopConfig::default());
        assert_eq!(f.attempted(), vec!["first", "second"]);
        match &f.events[0] {
            Event::PassStarted { pass, boards } => {
                assert_eq!(*pass, 1);
                assert_eq!(boards, &["first".to_string(), "second".to_string()]);
            }
            other => panic!("{other:?}"),
        }
    }
}
