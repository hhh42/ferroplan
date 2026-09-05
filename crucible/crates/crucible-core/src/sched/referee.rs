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
}

impl Default for Rule {
    fn default() -> Self {
        Rule {
            rho_min: 0.95,
            swap_growth_mb: 512.0,
            canary_max_factor: 1.15,
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
    /// The worst canary clock factor across the run's window, when one was
    /// measured. Above `Rule::canary_max_factor` the row is `Owe::Thermal`
    /// (the name predates the calibration; it covers every way a box gets
    /// slower than its baseline, clocks and bandwidth alike).
    pub clock_factor: Option<f64>,
    /// Our own concurrent planners at the time (0 until the scheduler packs).
    pub neighbours: u32,
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
        )
    }
}

/// The verdict table of `crucible-spec.md` R2.1, in the order it is written.
pub fn judge(rule: &Rule, f: &Facts) -> Verdict {
    if f.solved {
        return Verdict::Banked(Bank::Solved);
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
    if f.clock_factor.is_some_and(|c| c > rule.canary_max_factor) {
        return Verdict::Owed(Owe::Thermal);
    }
    if f.swap_growth_mb.is_some_and(|g| g > rule.swap_growth_mb) {
        return Verdict::Owed(Owe::Swap);
    }
    match f.rho() {
        Some(r) if r >= rule.rho_min => Verdict::Banked(Bank::Rho),
        Some(_) => Verdict::Owed(Owe::Starved),
        None => Verdict::Owed(Owe::CpuUnknown),
    }
}

/// The TIMING quality of a row, which is a different question from whether it
/// banks: R1's rule, unchanged. A row measured beside our own planners will
/// get its own answer when the scheduler packs (R2.2).
pub fn timing(f: &Facts) -> TimingQuality {
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
            neighbours: 0,
        }
    }

    #[test]
    fn the_stamp_is_the_runners_stamp() {
        assert_eq!(TRUSTED_CPU_INSTRUMENT, crate::exec::CPU_INSTRUMENT);
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
        ] {
            assert!(!v.as_str().is_empty());
            assert_eq!(v.banked(), matches!(v, Verdict::Banked(_)));
        }
    }
}
