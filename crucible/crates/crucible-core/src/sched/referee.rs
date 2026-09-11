//! The verdict on one measured row: does it BANK, or is it OWED again?
//!
//! R1 judged a row by the BOX: any watcher sample inside the run's window
//! with foreign load over the clean line re-owed the row, whatever the row
//! said. On the 0.26 cut sweep that re-owed ~1,800 timeouts of which nine in
//! ten had used >= 90% of their wall as CPU -- the planner had its core, and
//! the referee was looking at the wrong thing (`crucible-spec.md` R2.0).
//!
//! R2 judges the RUN, from what the kernel says about that process:
//!
//! * A **solve** banks, always. Coverage is coverage; a plan found under
//!   contention is a plan, and it could only have been found faster.
//! * An **unsolved** row banks when the process was not starved: its CPU time
//!   over its effective wall (`rho`) is at least `rho_min`, the clock did
//!   not jump, swap did not grow past the line, and the box was not
//!   thermally throttled. `rho_min` is DERIVED, not tuned: the 0.27 Phase 0
//!   sitting read the corrected instrument's own distribution (median 0.995,
//!   p5 0.975 on >= 10 s runs, box at load ~2.5) and the pre-registered rule
//!   -- p5 rounded down to 0.05, floored at 0.85 -- gave 0.95.
//! * `threads > 1` keeps the R1 rule whole. `rho` is not meaningful for a
//!   planner that may not saturate its threads, and those boards are the
//!   competition's wall-clock rule: solo, box-wide window, as before.
//!
//! What `rho` cannot see -- memory bandwidth, a down-clocked core -- the
//! canary covers (`Facts::clock_factor`), and the packing calibration
//! MEASURED it: four planners on the four P-cores, each with its core (packed
//! rho 0.99), ran a median 1.73x slower than solo. A process can have its
//! core and still be slowed by its neighbours; only a clock beside it can
//! tell. So the canary is the referee's second input, not a nicety.
//! What it MUST not see is a row with no trustworthy CPU reading: a row the
//! R1 runner wrote has no `cpu_instrument` stamp, and its `cpu_ms` is a Mach
//! count read as nanoseconds. Those rows are CPU-unknown and, unsolved, are
//! owed -- exactly as R1 judged them. Nothing already banked moves.

use crate::db::{Cleanliness, TimingQuality};

/// What the runner stamps on a row whose CPU time came from `wait4(2)`.
/// Kept in sync with `exec::CPU_INSTRUMENT` by a test.
pub const TRUSTED_CPU_INSTRUMENT: &str = "wait4";

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Rule {
    /// The starvation line: cpu / effective wall below this and an unsolved
    /// row is owed. `[referee] cpu_ratio_min`.
    pub rho_min: f64,
    /// Swap growth across the run's window, in MB, past which an unsolved
    /// row is owed. `[referee] swap_growth_mb`.
    pub swap_growth_mb: f64,
    /// The canary's clock factor (its wall over its solo baseline) above
    /// which the box was too slow for a timeout to count.
    /// `[referee] canary_max_factor`.
    pub canary_max_factor: f64,
    /// THE FLOOR BELOW WHICH rho IS NOT A STATISTIC (0.28).
    ///
    /// rho is cpu over effective wall, and a process's wall includes a fixed
    /// cost -- fork, exec, dynamic linking, teardown -- during which the
    /// child burns no CPU. Over a 60 s run that overhead is noise. Over a
    /// 0.4 s run it IS the measurement: such a row reads rho 0.015 on a
    /// perfectly idle box, so `rho < rho_min` condemns it no matter what the
    /// machine was doing, forever.
    ///
    /// That is not hypothetical. The 0.27 sweep produced 84 sub-second rows
    /// on ipc2023-numeric-opt alone -- grounding-level unsolvability proofs
    /// and honest out-of-scope refusals, both delivered in 0.4 s -- and
    /// refused every one as starved across four passes. The 0.26 sweep
    /// banked the same 21 rows through the window path. A rule that can
    /// never be satisfied is not a strict rule, it is a livelock: the set
    /// could not reach a terminal state.
    ///
    /// `[referee] rho_floor_ms`.
    pub rho_floor_ms: u64,
    /// THE FIXED COST OF RUNNING A PROCESS AT ALL, in milliseconds: fork,
    /// exec, dynamic linking, teardown. Wall the child spends existing
    /// without burning CPU, and therefore a permanent subtraction from rho.
    ///
    /// MEASURED, not assumed. Over 6,989 solo single-threaded rows of the
    /// 0.27 sweep, wall minus cpu is ~0.29 s under 2 s, ~0.31 s at 2-5 s and
    /// ~0.35 s at 5-10 s -- flat, which is what makes it a fixed cost rather
    /// than contention. The first cut of the floor guessed 50-100 ms and was
    /// wrong by a factor of three.
    ///
    /// It sets the floor rather than being one: rho can only reach `rho_min`
    /// once this is a small enough share of the wall, so the honest floor is
    /// `overhead / (1 - rho_min)` -- 8 s at the measured 0.4 s and the
    /// measured 0.95. Below that, no box however idle can produce a passing
    /// rho, and `rho < rho_min` is a statement about arithmetic rather than
    /// about the machine.
    pub rho_overhead_ms: u64,
}

impl Rule {
    /// The wall below which `rho_min` is unreachable, so rho must not judge.
    ///
    /// Derived, so that moving `rho_min` moves this with it: a stricter
    /// starvation line needs a longer run before it can be met, and a
    /// hand-set floor would silently stop matching.
    pub fn rho_floor_ms(&self) -> u64 {
        let derived = if self.rho_min >= 1.0 {
            u64::MAX
        } else {
            // Rounded UP: the floor is the first wall at which rho_min is
            // reachable, and 400/(1-0.95) lands at 7999.999 in binary
            // floating point.
            (self.rho_overhead_ms as f64 / (1.0 - self.rho_min)).ceil() as u64
        };
        derived.max(self.rho_floor_ms)
    }
}

impl Default for Rule {
    fn default() -> Self {
        Rule {
            rho_min: 0.95,
            swap_growth_mb: 512.0,
            canary_max_factor: 1.15,
            // A hard minimum; the operative floor is derived from the
            // overhead below (0.4 / 0.05 = 8 s at the defaults).
            rho_floor_ms: 2_000,
            rho_overhead_ms: 400,
        }
    }
}

/// Everything the verdict is allowed to look at. Assembled by the runner;
/// judged here, so the rule can be tested branch by branch without a
/// database or a child process.
#[derive(Debug, Clone, PartialEq)]
pub struct Facts {
    pub solved: bool,
    pub threads: u32,
    /// `Some("wait4")` from the R2 runner; anything else is CPU-unknown.
    pub cpu_instrument: Option<String>,
    pub cpu_ms: u64,
    /// Wall minus suspension: what the deadline was compared against.
    pub effective_ms: u64,
    /// The monotonic clock jumped mid-run (the machine slept).
    pub clock_jump: bool,
    /// The R1 box-wide window verdict. Decides `threads > 1` rows and the
    /// TIMING quality of every row; no longer decides banking for `threads =
    /// 1`.
    pub window: Cleanliness,
    /// Swap growth over the run's window, when the watcher covered it.
    pub swap_growth_mb: Option<f64>,
    /// The run was moved into the background scheduling band at any point.
    /// `rho` CANNOT see this -- it is CPU share, and a demoted process has
    /// all the share of a slow core (measured: 4.52 s normal, 59.28 s
    /// demoted, rho 0.956).
    pub demoted: bool,
    /// The worst canary clock factor across the run's window, when one was
    /// measured. Above `Rule::canary_max_factor` the row is `Owe::Thermal`
    /// (the name predates the calibration; it covers every way a box gets
    /// slower than its baseline, clocks and bandwidth alike).
    pub clock_factor: Option<f64>,
    /// Our own concurrent planners at the time (0 until the scheduler packs).
    pub neighbours: u32,
    /// The predecessor (the promoted raw, or an earlier engine on this box)
    /// SOLVED this instance. A timeout that contradicts a prior solve is
    /// suspect until a second solo run confirms it -- the 0.27 cut27 sweep
    /// banked openstacks i28 (a 9 s solve on 0.26) as a rho-0.956 timeout
    /// while macOS thrashed the box in ways neither rho nor a 20-minute
    /// canary could see.
    pub prior_solved: bool,
    /// Which SOLO attempt this is for the instance under this engine
    /// (1-based; packed attempts do not count -- a miss beside neighbours
    /// is nobody's verdict, so it cannot confirm one either).
    pub solo_attempt: u32,
}

impl Facts {
    /// cpu / effective wall. `None` when either is unknown or the run was too
    /// short for the ratio to mean anything.
    pub fn rho(&self) -> Option<f64> {
        if self.cpu_instrument.as_deref() != Some(TRUSTED_CPU_INSTRUMENT) || self.effective_ms == 0
        {
            return None;
        }
        Some(self.cpu_ms as f64 / self.effective_ms as f64)
    }
}

/// Why a row banked.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Bank {
    Solved,
    /// Unsolved, and the process had its core.
    Rho,
    /// Unsolved on a `threads > 1` board with a clean box-wide window: the R1
    /// rule, kept whole for those boards.
    Window,
}

/// Why a row is owed again.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Owe {
    /// `rho < rho_min`: the process did not get its core.
    Starved,
    /// No trustworthy CPU reading on the row (an R1 row, or no child ran).
    CpuUnknown,
    ClockJump,
    Swap,
    Thermal,
    /// `threads > 1` with a dirty box-wide window (the R1 verdict).
    Contended,
    /// `threads > 1` with no watcher coverage (fail closed, as R1 did).
    Uncovered,
    /// Unsolved where the predecessor solved, on the first attempt: not a
    /// verdict until a second solo run says the same.
    Suspect,
    /// Measured in the background scheduling band -- an efficiency core at
    /// a fraction of the speed, with the CPU SHARE of a healthy run. Not a
    /// measurement of anything.
    Demoted,
    /// Unsolved beside our own planners. Not a verdict on the instance at
    /// all -- it is re-run with fewer neighbours, then solo, in the same
    /// pass. Packing can waste time; it can never lose a row.
    Packed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Verdict {
    Banked(Bank),
    Owed(Owe),
}

impl Verdict {
    pub fn banked(self) -> bool {
        matches!(self, Verdict::Banked(_))
    }

    /// The `run.verdict` column's spelling.
    pub fn as_str(self) -> &'static str {
        match self {
            Verdict::Banked(Bank::Solved) => "solved",
            Verdict::Banked(Bank::Rho) => "rho",
            Verdict::Banked(Bank::Window) => "window",
            Verdict::Owed(Owe::Starved) => "starved",
            Verdict::Owed(Owe::CpuUnknown) => "cpu-unknown",
            Verdict::Owed(Owe::ClockJump) => "clock-jump",
            Verdict::Owed(Owe::Swap) => "swap",
            Verdict::Owed(Owe::Thermal) => "thermal",
            Verdict::Owed(Owe::Contended) => "contended",
            Verdict::Owed(Owe::Uncovered) => "uncovered",
            Verdict::Owed(Owe::Packed) => "packed",
            Verdict::Owed(Owe::Suspect) => "suspect",
            Verdict::Owed(Owe::Demoted) => "demoted",
        }
    }

    /// The row was owed because of the BOX, not because of the row. A pass
    /// that owes every row this way is a reason to wait for the box; one
    /// that owes rows any other way is a reason to look at the runner.
    pub fn box_fault(self) -> bool {
        matches!(
            self,
            Verdict::Owed(Owe::Starved)
                | Verdict::Owed(Owe::Swap)
                | Verdict::Owed(Owe::Thermal)
                | Verdict::Owed(Owe::Contended)
                | Verdict::Owed(Owe::Uncovered)
                | Verdict::Owed(Owe::ClockJump)
                | Verdict::Owed(Owe::Packed)
                | Verdict::Owed(Owe::Suspect)
                | Verdict::Owed(Owe::Demoted)
        )
    }
}

/// The verdict table of `crucible-spec.md` R2.1, in the order it is written.
pub fn judge(rule: &Rule, f: &Facts) -> Verdict {
    if f.solved {
        return Verdict::Banked(Bank::Solved);
    }
    // Beside our own planners nothing about the box is knowable from this
    // process: the calibration measured +73 % wall with rho 0.99. A packed
    // miss is re-run narrower, then solo, in the same pass.
    if f.neighbours > 0 {
        return Verdict::Owed(Owe::Packed);
    }
    if f.prior_solved && f.solo_attempt < 2 {
        return Verdict::Owed(Owe::Suspect);
    }
    if f.clock_jump {
        return Verdict::Owed(Owe::ClockJump);
    }
    if f.threads > 1 {
        return match f.window {
            Cleanliness::Clean => Verdict::Banked(Bank::Window),
            Cleanliness::Dirty => Verdict::Owed(Owe::Contended),
            Cleanliness::Uncovered => Verdict::Owed(Owe::Uncovered),
        };
    }
    // Before every box-wide signal: this one is about THIS process, and no
    // box-wide instrument can see it. The canary runs at normal priority,
    // so it reads a healthy box while every planner crawls on an E-core.
    if f.demoted {
        return Verdict::Owed(Owe::Demoted);
    }
    if f.clock_factor.is_some_and(|c| c > rule.canary_max_factor) {
        return Verdict::Owed(Owe::Thermal);
    }
    if f.swap_growth_mb.is_some_and(|g| g > rule.swap_growth_mb) {
        return Verdict::Owed(Owe::Swap);
    }
    // TOO SHORT FOR rho TO MEAN ANYTHING (0.28). Judge it the way 0.26 did,
    // and the way a `threads > 1` row still is: by whether the box-wide
    // WINDOW was clean. That is a statement about the machine that does not
    // divide by a wall too small to divide by.
    //
    // Deliberately AFTER demoted, thermal, swap and clock-jump: those are
    // reasons to doubt a row that hold however briefly it ran, and a short
    // run must not be a way around them.
    //
    // No rho AT ALL -- an untrusted instrument, or a run with no measurable
    // wall -- is a different and older answer, and it comes first: the floor
    // is about a ratio that exists and does not mean anything, not about a
    // ratio we never had.
    let Some(r) = f.rho() else {
        return Verdict::Owed(Owe::CpuUnknown);
    };
    if f.effective_ms < rule.rho_floor_ms() {
        return match f.window {
            Cleanliness::Clean => Verdict::Banked(Bank::Window),
            Cleanliness::Dirty => Verdict::Owed(Owe::Contended),
            Cleanliness::Uncovered => Verdict::Owed(Owe::Uncovered),
        };
    }
    if r >= rule.rho_min {
        Verdict::Banked(Bank::Rho)
    } else {
        Verdict::Owed(Owe::Starved)
    }
}

/// The TIMING quality of a row, which is a different question from whether it
/// banks: R1's rule, unchanged. A row measured beside our own planners will
/// get its own answer when the scheduler packs (R2.2).
pub fn timing(f: &Facts) -> TimingQuality {
    if f.neighbours > 0 {
        return TimingQuality::Dirty;
    }
    match f.window {
        Cleanliness::Clean if !f.clock_jump => TimingQuality::Clean,
        Cleanliness::Clean | Cleanliness::Dirty => TimingQuality::Dirty,
        Cleanliness::Uncovered => TimingQuality::Unknown,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn unsolved() -> Facts {
        Facts {
            solved: false,
            threads: 1,
            cpu_instrument: Some(TRUSTED_CPU_INSTRUMENT.into()),
            cpu_ms: 59_500,
            effective_ms: 60_000,
            clock_jump: false,
            window: Cleanliness::Dirty,
            swap_growth_mb: Some(0.0),
            clock_factor: Some(1.0),
            demoted: false,
            neighbours: 0,
            prior_solved: false,
            solo_attempt: 1,
        }
    }

    #[test]
    fn the_stamp_is_the_runners_stamp() {
        assert_eq!(TRUSTED_CPU_INSTRUMENT, crate::exec::CPU_INSTRUMENT);
    }

    /// THE LIVELOCK, pinned (0.28). rho cannot condemn a row too short to
    /// have a meaningful rho.
    ///
    /// A 0.4 s run spends most of its wall in fork, exec and teardown, during
    /// which the child burns no CPU. It reads rho 0.015 on a perfectly idle
    /// box. Under `rho < rho_min` that row is starved FOREVER -- and the 0.27
    /// sweep proved it, refusing 84 sub-second rows on ipc2023-numeric-opt
    /// across four passes while the set could never reach a terminal state.
    ///
    /// The rows were not failures. They were grounding-level unsolvability
    /// proofs and honest out-of-scope refusals, answered in 0.4 s. The 0.26
    /// instrument banked the same 21 rows through the window path; this
    /// restores that, so the two cycles stay like-for-like.
    #[test]
    fn a_run_too_short_for_rho_is_judged_by_the_window_instead() {
        let brief = |window| Facts {
            cpu_ms: 6,
            effective_ms: 400,
            window,
            ..unsolved()
        };

        // The shape that livelocked: rho 0.015, far under rho_min, on a box
        // whose window was clean.
        let clean = brief(Cleanliness::Clean);
        assert!(
            clean.rho().is_some_and(|r| r < Rule::default().rho_min),
            "the premise: rho alone condemns this row"
        );
        assert_eq!(
            judge(&Rule::default(), &clean),
            Verdict::Banked(Bank::Window),
            "a clean window banks it, exactly as 0.26 did"
        );

        // A dirty box is still a dirty box -- the floor is not an amnesty.
        assert_eq!(
            judge(&Rule::default(), &brief(Cleanliness::Dirty)),
            Verdict::Owed(Owe::Contended)
        );
        assert_eq!(
            judge(&Rule::default(), &brief(Cleanliness::Uncovered)),
            Verdict::Owed(Owe::Uncovered)
        );
    }

    /// The floor must not become a way around the signals that hold however
    /// briefly a row ran. A demoted 0.4 s run is still demoted: the E-core
    /// defect banked 1,709 rows precisely because a box-wide instrument could
    /// not see it, and a short wall does not make it visible.
    #[test]
    fn the_short_run_floor_does_not_override_the_process_signals() {
        let brief = Facts {
            cpu_ms: 6,
            effective_ms: 400,
            window: Cleanliness::Clean,
            ..unsolved()
        };
        let cases = [
            (
                Facts {
                    demoted: true,
                    ..brief.clone()
                },
                Verdict::Owed(Owe::Demoted),
            ),
            (
                Facts {
                    clock_factor: Some(2.0),
                    ..brief.clone()
                },
                Verdict::Owed(Owe::Thermal),
            ),
            (
                Facts {
                    swap_growth_mb: Some(4096.0),
                    ..brief.clone()
                },
                Verdict::Owed(Owe::Swap),
            ),
            (
                Facts {
                    clock_jump: true,
                    ..brief.clone()
                },
                Verdict::Owed(Owe::ClockJump),
            ),
        ];
        for (f, want) in cases {
            assert_eq!(judge(&Rule::default(), &f), want);
        }
    }

    /// THE FLOOR IS DERIVED, NOT GUESSED (0.28).
    ///
    /// rho is cpu over wall, and every process pays a fixed cost -- fork,
    /// exec, linking, teardown -- that is wall without cpu. So the BEST rho a
    /// run can possibly show is `(W - overhead) / W`, and below some wall
    /// even a perfectly served process cannot reach `rho_min`.
    ///
    /// The first cut of the floor guessed that overhead at 50-100 ms and set
    /// 2 s. Measured over 6,989 solo single-threaded rows it is ~0.3 s, so
    /// the guess was wrong by a factor of three and 2-5 s runs went on being
    /// condemned: only 10% of them could reach 0.95, against 69% at 5-10 s
    /// and 96% at 10-30 s. ipc5-complex-pref refused the same twelve rows for
    /// three consecutive passes at rho 0.89-0.93 -- all mem-cap exits at
    /// 3-5 s, none of which could ever have passed.
    #[test]
    fn the_floor_is_where_rho_min_first_becomes_reachable() {
        let r = Rule::default();
        // 0.4 s of overhead against a 0.95 line: 8 s.
        assert_eq!(r.rho_floor_ms(), 8_000);

        // The property that makes it the right floor: just below it, a
        // perfectly served process still fails; just above, it passes.
        let best_rho =
            |wall_ms: u64| (wall_ms.saturating_sub(r.rho_overhead_ms)) as f64 / wall_ms as f64;
        assert!(best_rho(r.rho_floor_ms() - 1) < r.rho_min);
        assert!(best_rho(r.rho_floor_ms()) >= r.rho_min);

        // And it tracks rho_min, rather than being a constant that silently
        // stops matching when the line moves.
        let strict = Rule {
            rho_min: 0.98,
            ..Rule::default()
        };
        assert_eq!(strict.rho_floor_ms(), 20_000);
    }

    /// The shape that stalled: a 3.4 s mem-cap exit at rho 0.90, refused
    /// three passes running, banks on a clean window and stays owed on a
    /// dirty one.
    #[test]
    fn a_mem_cap_exit_under_the_floor_is_judged_by_the_window() {
        let row = |window| Facts {
            cpu_ms: 3_030,
            effective_ms: 3_360,
            window,
            ..unsolved()
        };
        assert!(row(Cleanliness::Clean).rho().is_some_and(|r| r < 0.95));
        assert_eq!(
            judge(&Rule::default(), &row(Cleanliness::Clean)),
            Verdict::Banked(Bank::Window)
        );
        assert_eq!(
            judge(&Rule::default(), &row(Cleanliness::Dirty)),
            Verdict::Owed(Owe::Contended)
        );
    }

    /// And the floor is a FLOOR: an ordinary 60 s timeout is still judged by
    /// rho, which is the whole point of the R2 referee.
    #[test]
    fn a_full_length_run_is_still_judged_by_rho() {
        let f = unsolved();
        assert!(f.effective_ms >= Rule::default().rho_floor_ms);
        assert_eq!(judge(&Rule::default(), &f), Verdict::Banked(Bank::Rho));

        let starved = Facts {
            cpu_ms: 6_000,
            ..unsolved()
        };
        assert_eq!(
            judge(&Rule::default(), &starved),
            Verdict::Owed(Owe::Starved)
        );
    }

    /// THE R2 CASE: a timeout measured while something else was on the box,
    /// but the planner had its core the whole time. R1 owed it; R2 banks it.
    #[test]
    fn a_timeout_with_its_core_banks_under_a_dirty_window() {
        let f = unsolved();
        assert_eq!(f.window, Cleanliness::Dirty);
        assert_eq!(judge(&Rule::default(), &f), Verdict::Banked(Bank::Rho));
        assert_eq!(
            timing(&f),
            TimingQuality::Dirty,
            "banked, but not a clean timing"
        );
    }

    #[test]
    fn a_starved_timeout_is_owed() {
        let f = Facts {
            cpu_ms: 40_000,
            ..unsolved()
        };
        assert_eq!(judge(&Rule::default(), &f), Verdict::Owed(Owe::Starved));
        assert!(judge(&Rule::default(), &f).box_fault());
    }

    #[test]
    fn a_solve_banks_whatever_the_box_did() {
        let f = Facts {
            solved: true,
            cpu_ms: 5,
            window: Cleanliness::Dirty,
            swap_growth_mb: Some(9_000.0),
            clock_factor: Some(4.0),
            ..unsolved()
        };
        assert_eq!(judge(&Rule::default(), &f), Verdict::Banked(Bank::Solved));
    }

    /// An R1 row: no stamp, and a cpu_ms that means nothing. Unsolved, it is
    /// owed -- exactly as R1 judged it; nothing already banked moves.
    #[test]
    fn a_row_without_the_stamp_is_cpu_unknown() {
        let f = Facts {
            cpu_instrument: None,
            ..unsolved()
        };
        assert_eq!(judge(&Rule::default(), &f), Verdict::Owed(Owe::CpuUnknown));
        assert!(!judge(&Rule::default(), &f).box_fault());
        let f = Facts {
            cpu_instrument: Some("pidrusage".into()),
            ..unsolved()
        };
        assert_eq!(judge(&Rule::default(), &f), Verdict::Owed(Owe::CpuUnknown));
    }

    #[test]
    fn the_flags_come_before_rho_in_the_order_written() {
        let r = Rule::default();
        let f = Facts {
            clock_jump: true,
            ..unsolved()
        };
        assert_eq!(judge(&r, &f), Verdict::Owed(Owe::ClockJump));
        let f = Facts {
            clock_factor: Some(1.16),
            ..unsolved()
        };
        assert_eq!(judge(&r, &f), Verdict::Owed(Owe::Thermal));
        let f = Facts {
            clock_factor: Some(1.15),
            ..unsolved()
        };
        assert_ne!(
            judge(&r, &f),
            Verdict::Owed(Owe::Thermal),
            "at the line is not past it"
        );
        let f = Facts {
            clock_factor: None,
            ..unsolved()
        };
        assert_eq!(
            judge(&r, &f),
            Verdict::Banked(Bank::Rho),
            "no canary yet is not a slow box"
        );
        let f = Facts {
            swap_growth_mb: Some(600.0),
            ..unsolved()
        };
        assert_eq!(judge(&r, &f), Verdict::Owed(Owe::Swap));
        let f = Facts {
            swap_growth_mb: Some(512.0),
            ..unsolved()
        };
        assert_eq!(
            judge(&r, &f),
            Verdict::Banked(Bank::Rho),
            "at the line is not past it"
        );
    }

    /// The mco boards keep the R1 rule whole: the box-wide window decides,
    /// and rho is never consulted.
    #[test]
    fn threads_above_one_keep_the_window_rule() {
        let r = Rule::default();
        let f = Facts {
            threads: 4,
            cpu_ms: 1,
            window: Cleanliness::Clean,
            ..unsolved()
        };
        assert_eq!(judge(&r, &f), Verdict::Banked(Bank::Window));
        let f = Facts {
            threads: 4,
            window: Cleanliness::Dirty,
            ..unsolved()
        };
        assert_eq!(judge(&r, &f), Verdict::Owed(Owe::Contended));
        let f = Facts {
            threads: 8,
            window: Cleanliness::Uncovered,
            ..unsolved()
        };
        assert_eq!(judge(&r, &f), Verdict::Owed(Owe::Uncovered));
    }

    #[test]
    fn rho_min_is_the_line_and_the_default_is_the_measured_one() {
        let r = Rule::default();
        assert_eq!(r.rho_min, 0.95);
        let f = Facts {
            cpu_ms: 57_000,
            ..unsolved()
        };
        assert_eq!(judge(&r, &f), Verdict::Banked(Bank::Rho));
        let f = Facts {
            cpu_ms: 56_999,
            ..unsolved()
        };
        assert_eq!(judge(&r, &f), Verdict::Owed(Owe::Starved));
    }

    /// A timeout that contradicts the predecessor's solve is suspect on
    /// the first attempt and a verdict on the second; a solve never is.
    #[test]
    fn a_timeout_where_the_predecessor_solved_needs_a_second_run() {
        let r = Rule::default();
        let f = Facts {
            prior_solved: true,
            solo_attempt: 1,
            ..unsolved()
        };
        assert_eq!(judge(&r, &f), Verdict::Owed(Owe::Suspect));
        assert!(judge(&r, &f).box_fault());
        let f = Facts {
            prior_solved: true,
            solo_attempt: 2,
            ..unsolved()
        };
        assert_eq!(judge(&r, &f), Verdict::Banked(Bank::Rho));
        let f = Facts {
            prior_solved: true,
            solo_attempt: 1,
            solved: true,
            ..unsolved()
        };
        assert_eq!(judge(&r, &f), Verdict::Banked(Bank::Solved));
        let f = Facts {
            prior_solved: false,
            solo_attempt: 1,
            ..unsolved()
        };
        assert_eq!(
            judge(&r, &f),
            Verdict::Banked(Bank::Rho),
            "no prior solve, no suspicion"
        );
    }

    /// THE E-CORE DEFECT (0.27 cut sweep): a run in the background band is
    /// 13x slower with cpu/wall 0.956 -- it clears the starvation line and
    /// banks a five-second instance as a sixty-second timeout. Unsolved and
    /// demoted is owed; SOLVED and demoted still banks, because a solve is
    /// a solve however slowly it arrived.
    #[test]
    fn a_demoted_row_cannot_bank_a_timeout_but_can_bank_a_solve() {
        let r = Rule::default();
        let f = Facts {
            demoted: true,
            cpu_ms: 56_690,
            effective_ms: 59_280,
            ..unsolved()
        };
        assert!(
            f.rho().is_some_and(|v| v > r.rho_min),
            "rho alone would bank it"
        );
        assert_eq!(judge(&r, &f), Verdict::Owed(Owe::Demoted));
        assert!(judge(&r, &f).box_fault());
        let f = Facts {
            demoted: true,
            solved: true,
            ..unsolved()
        };
        assert_eq!(judge(&r, &f), Verdict::Banked(Bank::Solved));
    }

    /// Packed: a solve is a solve; a miss is nobody's verdict.
    #[test]
    fn packed_rows_bank_only_when_solved() {
        let r = Rule::default();
        let f = Facts {
            neighbours: 7,
            ..unsolved()
        };
        assert_eq!(judge(&r, &f), Verdict::Owed(Owe::Packed));
        assert!(judge(&r, &f).box_fault());
        assert_eq!(timing(&f), TimingQuality::Dirty);
        let f = Facts {
            neighbours: 7,
            solved: true,
            ..unsolved()
        };
        assert_eq!(judge(&r, &f), Verdict::Banked(Bank::Solved));
    }

    #[test]
    fn a_zero_length_run_has_no_rho() {
        let f = Facts {
            effective_ms: 0,
            ..unsolved()
        };
        assert_eq!(f.rho(), None);
        assert_eq!(judge(&Rule::default(), &f), Verdict::Owed(Owe::CpuUnknown));
    }

    #[test]
    fn every_verdict_spells_itself() {
        for v in [
            Verdict::Banked(Bank::Solved),
            Verdict::Banked(Bank::Rho),
            Verdict::Banked(Bank::Window),
            Verdict::Owed(Owe::Starved),
            Verdict::Owed(Owe::CpuUnknown),
            Verdict::Owed(Owe::ClockJump),
            Verdict::Owed(Owe::Swap),
            Verdict::Owed(Owe::Thermal),
            Verdict::Owed(Owe::Contended),
            Verdict::Owed(Owe::Uncovered),
            Verdict::Owed(Owe::Packed),
            Verdict::Owed(Owe::Suspect),
            Verdict::Owed(Owe::Demoted),
        ] {
            assert!(!v.as_str().is_empty());
            assert_eq!(v.banked(), matches!(v, Verdict::Banked(_)));
        }
    }
}
