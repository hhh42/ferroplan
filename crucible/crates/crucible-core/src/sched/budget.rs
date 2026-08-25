//! Core accounting: may this board run at this throttle level, and does the
//! manifest even fit on this box?
//!
//! # What this deliberately is NOT
//!
//! `crucible-spec.md` §5.2 describes a budget in "P-core equivalents" that Runs
//! are packed into, and read casually that invites a bin-packing scheduler
//! filling spare cores with work from another board. **That is impossible
//! here, and the reason is not performance.** `jobs` is part of a row's
//! IDENTITY: `ipc67.py` stamps it on every record, the resume gate compares it
//! EXACTLY, and the manifest's `[[board]]` table exists to hold it. Two boards
//! running at once means every row on both was measured against neighbours its
//! own `jobs` field does not describe -- a chimera, and one that would pass
//! every schema check in the system. For the `--threads N` boards it is worse
//! still: the competition scores wall time on a fixed box however many cores a
//! planner burns, so those boards run ONE instance at a time, and the manifest
//! asserts it rather than inferring it. A packer that put a second board beside
//! an mco board would break the rule the board's whole methodology rests on.
//!
//! So the budget has exactly two jobs, and both are gates rather than
//! optimisers:
//!
//! 1. **Admission.** One board, one level: does its declared demand fit the
//!    capacity this throttle level allows? SUSPENDED has no capacity at all,
//!    which is what suspension means.
//! 2. **Manifest validation, at STARTUP.** A board whose demand exceeds even
//!    FULL capacity can never be admitted. Left to the scheduler that is a
//!    deadlock: a board sitting in the queue forever while the driver waits for
//!    a quiet window that would not help. Checked here it is a startup error
//!    naming the board, the demand and the box.
//!
//! # The exception, which is real and is recorded rather than excused
//!
//! `ipc7-mco-t8` and `ipc2014-mco-t8` demand eight cores against this box's
//! four P-cores, and they are supposed to. `standings.py` has said so since
//! 0.16: the board "is oversubscribed by construction and is recorded as such,
//! not excused", and it renders that sentence into the published methodology
//! column. Refusing to run it would be wrong; running it silently would be
//! worse. So oversubscription must be DECLARED by the caller
//! ([`Oversubscribed`]), and a declared board reports a `warning:` line that
//! travels with the run instead of an error that stops it.
//!
//! The declaration is caller-supplied rather than a manifest field because
//! `manifest::BoardSpec` is `deny_unknown_fields` and its comment is explicit
//! that "renaming a field here re-identifies every row already on disk". It is
//! also not derived from the numbers: a board with `jobs = 1` that demands more
//! than the box has is IRREDUCIBLE, but auto-excusing it would turn a typo
//! (`threads = 64`) into a silent 16x overcommit. The error message names which
//! kind it is, because the two need different fixes -- lower `jobs`, or accept
//! the board as oversubscribed -- and never guesses which one was meant.

use crate::monitor::Level;
use crate::platform::Topology;
use crucible_publish::manifest::{BoardSpec, Defaults, Manifest, WARNING};
use std::collections::BTreeSet;

/// `[scheduler] reserve_p_cores` in `crucible-spec.md` §12 (Configuration):
/// cores held back from the FULL budget. Zero, because on this box the E-cores
/// already absorb the OS and the harness.
pub const DEFAULT_RESERVE_P_CORES: u32 = 0;

/// What one board asks of the box.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Demand {
    /// Instances in flight at once. Board-declared, stamped on every row.
    /// Zero and one are the same request, for the same reason `threads` is:
    /// a board that runs no instances is a typo, not a board that costs
    /// nothing, and reading it as nothing would let `jobs = 0` sail through
    /// [`Accountant::validate`] and be admitted at every level.
    pub jobs: u32,
    /// `ff --threads`. Zero and one are the same request.
    pub threads: u32,
}

impl Demand {
    pub fn new(jobs: u32, threads: u32) -> Self {
        Demand { jobs, threads }
    }

    /// The board's demand as declared in the manifest, defaults applied.
    pub fn of(b: &BoardSpec, d: &Defaults) -> Self {
        Demand {
            jobs: b.jobs.unwrap_or(d.jobs),
            threads: b.threads.unwrap_or(d.threads),
        }
    }

    /// `max(jobs, 1) * max(threads, 1)` -- cores, not processes.
    ///
    /// Both floors are the same rule: zero is not a smaller request than one,
    /// it is a malformed one, and the only reading that cannot silently
    /// under-charge the box is the larger. `tier::eta` already divides by
    /// `jobs.max(1)`, so this is also what stops the two modules disagreeing
    /// about what a board costs.
    pub fn cores(self) -> u32 {
        self.jobs.max(1) * self.threads.max(1)
    }
}

impl std::fmt::Display for Demand {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "jobs {} x threads {}",
            self.jobs.max(1),
            self.threads.max(1)
        )
    }
}

/// Boards whose demand exceeds the box BY CONSTRUCTION, declared by the caller.
///
/// A set rather than a predicate over the numbers: see the module header on why
/// `jobs = 1` is not on its own an excuse.
#[derive(Debug, Clone, Default)]
pub struct Oversubscribed(BTreeSet<String>);

impl Oversubscribed {
    pub fn none() -> Self {
        Oversubscribed(BTreeSet::new())
    }

    pub fn from_ids<I: IntoIterator<Item = String>>(ids: I) -> Self {
        Oversubscribed(ids.into_iter().collect())
    }

    pub fn contains(&self, board_id: &str) -> bool {
        self.0.contains(board_id)
    }

    pub fn ids(&self) -> impl Iterator<Item = &str> {
        self.0.iter().map(String::as_str)
    }
}

/// Why a board may not start now.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Denial {
    /// The throttle is holding everything. Not a capacity question.
    Suspended,
    /// More cores than this level allows, and the board is not declared
    /// oversubscribed.
    Overcommit,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Admission {
    Admit {
        demand: u32,
        capacity: u32,
    },
    /// Over capacity, but declared: it runs, and the run is recorded as
    /// oversubscribed rather than passed off as a clean fit.
    Oversubscribed {
        demand: u32,
        capacity: u32,
    },
    Deny {
        demand: u32,
        capacity: u32,
        why: Denial,
    },
}

impl Admission {
    pub fn is_admitted(&self) -> bool {
        matches!(
            self,
            Admission::Admit { .. } | Admission::Oversubscribed { .. }
        )
    }
}

/// The box, and how much of it the scheduler may spend.
#[derive(Debug, Clone, Copy)]
pub struct Accountant {
    pub topology: Topology,
    pub reserve_p_cores: u32,
}

impl Accountant {
    pub fn new(topology: Topology) -> Self {
        Accountant {
            topology,
            reserve_p_cores: DEFAULT_RESERVE_P_CORES,
        }
    }

    pub fn with_reserve(topology: Topology, reserve_p_cores: u32) -> Self {
        Accountant {
            topology,
            reserve_p_cores,
        }
    }

    /// Cores available at a throttle level.
    ///
    /// POLITE is the E-cores because that is literally where the work goes:
    /// macOS has no affinity API, and `platform::demote` moves a running child
    /// into the background scheduling band, which confines it to the efficiency
    /// cores. The capacity number and the mechanism therefore agree by
    /// construction rather than by convention.
    pub fn capacity(&self, level: Level) -> u32 {
        match level {
            Level::Full => self.topology.p_cores.saturating_sub(self.reserve_p_cores),
            Level::Polite => self.topology.e_cores,
            // Not "no room". Held.
            Level::Suspended => 0,
        }
    }

    /// One board, one level.
    pub fn admit(&self, level: Level, demand: Demand, declared: bool) -> Admission {
        let capacity = self.capacity(level);
        let cores = demand.cores();
        if level == Level::Suspended {
            // Suspension is not negotiable, and a declared board does not get
            // to ignore a game or a memory-pressure hold.
            return Admission::Deny {
                demand: cores,
                capacity,
                why: Denial::Suspended,
            };
        }
        if cores <= capacity {
            Admission::Admit {
                demand: cores,
                capacity,
            }
        } else if declared {
            Admission::Oversubscribed {
                demand: cores,
                capacity,
            }
        } else {
            Admission::Deny {
                demand: cores,
                capacity,
                why: Denial::Overcommit,
            }
        }
    }

    /// Every board that cannot fit this box, checked ONCE at startup.
    ///
    /// Returns one flat list in board order; lines prefixed [`WARNING`] are
    /// legitimate states that must be visible rather than fatal, matching
    /// `Manifest::validate`. Filter with [`errors`].
    pub fn validate(&self, m: &Manifest, declared: &Oversubscribed) -> Vec<String> {
        let full = self.capacity(Level::Full);
        let mut out = Vec::new();
        let mut hit: BTreeSet<&str> = BTreeSet::new();
        for b in &m.boards {
            let d = Demand::of(b, &m.defaults);
            let cores = d.cores();
            if cores <= full {
                if declared.contains(&b.id) {
                    // The declaration list is one more place that can disagree
                    // with the manifest, so it is checked in both directions.
                    out.push(format!(
                        "{WARNING}board `{}` is declared oversubscribed but fits \
                         ({cores} of {full} P-cores): drop the declaration",
                        b.id
                    ));
                }
                continue;
            }
            hit.insert(&b.id);
            if declared.contains(&b.id) {
                out.push(format!(
                    "{WARNING}board `{}` demands {cores} cores ({d}) against \
                     {full} P-cores: OVERSUBSCRIBED BY CONSTRUCTION, and \
                     recorded as such rather than excused",
                    b.id
                ));
            } else if d.jobs > 1 {
                out.push(format!(
                    "board `{}` demands {cores} cores ({d}) against {full} \
                     P-cores: lower `jobs` -- a board that cannot be admitted \
                     is a queue that never drains",
                    b.id
                ));
            } else {
                out.push(format!(
                    "board `{}` demands {cores} cores ({d}) against {full} \
                     P-cores with `jobs` already at 1: it cannot be reduced, so \
                     either the thread count is wrong or the board must be \
                     declared oversubscribed",
                    b.id
                ));
            }
        }
        for id in declared.ids() {
            if hit.contains(id) {
                continue;
            }
            if !m.boards.iter().any(|b| b.id == id) {
                out.push(format!(
                    "{WARNING}board `{id}` is declared oversubscribed but is not \
                     in the manifest"
                ));
            }
        }
        out
    }
}

/// [`Accountant::validate`] minus the [`WARNING`] lines -- the fatal half.
pub fn errors(lines: &[String]) -> Vec<&str> {
    lines
        .iter()
        .filter(|l| !l.starts_with(WARNING))
        .map(String::as_str)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    /// The box every number in this repo's release record was measured on.
    fn air() -> Topology {
        Topology {
            p_cores: 4,
            e_cores: 6,
            logical: 10,
            mem_bytes: 16 << 30,
        }
    }

    /// FULL is the P-cores less the reserve; POLITE is the E-cores, because
    /// that is where `platform::demote` actually puts the work; SUSPENDED has
    /// no capacity, which is what suspension means.
    #[test]
    fn capacity_follows_the_throttle_level() {
        let a = Accountant::new(air());
        assert_eq!(a.capacity(Level::Full), 4);
        assert_eq!(a.capacity(Level::Polite), 6);
        assert_eq!(a.capacity(Level::Suspended), 0);
        let held = Accountant::with_reserve(air(), 1);
        assert_eq!(held.capacity(Level::Full), 3);
    }

    /// Cores, not processes -- and `threads = 0` is the same request as
    /// `threads = 1`, because `ff` gets one thread either way.
    #[test]
    fn demand_is_jobs_times_threads() {
        assert_eq!(Demand::new(2, 1).cores(), 2);
        assert_eq!(Demand::new(1, 8).cores(), 8);
        assert_eq!(Demand::new(2, 0).cores(), 2);
    }

    /// `jobs = 0` is a typo, not a board that costs nothing. Charged as one --
    /// otherwise a `jobs = 0` board demands zero cores, clears
    /// [`Accountant::validate`] silently and is admitted at every level, while
    /// `tier::eta` is over in the next module dividing its estimate by 1.
    #[test]
    fn a_zero_jobs_board_is_charged_as_one_not_as_nothing() {
        assert_eq!(Demand::new(0, 4).cores(), 4);
        assert_eq!(Demand::new(0, 0).cores(), 1);
        assert_eq!(Demand::new(0, 1).to_string(), "jobs 1 x threads 1");
        let a = Accountant::new(air());
        assert!(
            !a.admit(Level::Full, Demand::new(0, 8), false).is_admitted(),
            "a zero-jobs t8 board must not slip in on a demand of nothing"
        );
    }

    /// Suspension is not a capacity question, so a declared-oversubscribed
    /// board does not get to ignore a game or a memory-pressure hold.
    #[test]
    fn suspension_denies_even_a_declared_board() {
        let a = Accountant::new(air());
        assert_eq!(
            a.admit(Level::Suspended, Demand::new(1, 8), true),
            Admission::Deny {
                demand: 8,
                capacity: 0,
                why: Denial::Suspended
            }
        );
    }

    /// THE STANDING EXCEPTION. `standings.py` since 0.16: t8 "is oversubscribed
    /// by construction and is recorded as such, not excused". Undeclared it is
    /// a deadlock; declared it runs and says so.
    #[test]
    fn the_t8_board_is_denied_undeclared_and_recorded_when_declared() {
        let a = Accountant::new(air());
        let t8 = Demand::new(1, 8);
        assert_eq!(
            a.admit(Level::Full, t8, false),
            Admission::Deny {
                demand: 8,
                capacity: 4,
                why: Denial::Overcommit
            }
        );
        let declared = a.admit(Level::Full, t8, true);
        assert_eq!(
            declared,
            Admission::Oversubscribed {
                demand: 8,
                capacity: 4
            }
        );
        assert!(declared.is_admitted(), "it must actually run");
    }

    /// A board that fits FULL can still be too big for POLITE -- but POLITE is
    /// a degradation, not a scheduling target, so the answer is "wait", not
    /// "this manifest is broken".
    #[test]
    fn a_normal_board_fits_both_levels_and_t4_fits_exactly() {
        let a = Accountant::new(air());
        assert!(a.admit(Level::Full, Demand::new(2, 1), false).is_admitted());
        assert!(a
            .admit(Level::Polite, Demand::new(2, 1), false)
            .is_admitted());
        // The mco t4 board is exactly the box: 4 threads, one instance.
        assert!(a.admit(Level::Full, Demand::new(1, 4), false).is_admitted());
    }

    /// A startup ERROR, not a silent deadlock -- and the message says which fix
    /// applies, because "lower jobs" and "declare it" are different decisions.
    #[test]
    fn an_unfittable_board_names_the_fix_that_applies() {
        let a = Accountant::new(air());
        let m = manifest(&[
            ("packed", "jobs = 4\nthreads = 2\n"),
            ("t8", "threads = 8\njobs = 1\n"),
        ]);
        let lines = a.validate(&m, &Oversubscribed::none());
        let errs = errors(&lines);
        assert_eq!(errs.len(), 2, "{lines:?}");
        assert!(errs[0].contains("lower `jobs`"), "{}", errs[0]);
        assert!(
            errs[1].contains("cannot be reduced"),
            "an irreducible board needs a different fix: {}",
            errs[1]
        );
    }

    /// Declared, and therefore visible rather than fatal.
    #[test]
    fn a_declared_board_downgrades_to_a_warning() {
        let a = Accountant::new(air());
        let m = manifest(&[("t8", "threads = 8\njobs = 1\n")]);
        let lines = a.validate(&m, &Oversubscribed::from_ids(["t8".to_string()]));
        assert!(errors(&lines).is_empty(), "{lines:?}");
        assert!(
            lines[0].contains("OVERSUBSCRIBED BY CONSTRUCTION"),
            "{lines:?}"
        );
    }

    /// The declaration list is one more registry that can disagree with the
    /// manifest, which is the failure this whole port exists to end -- so it is
    /// checked in both directions.
    #[test]
    fn a_stale_declaration_is_reported() {
        let a = Accountant::new(air());
        let m = manifest(&[("small", "jobs = 2\n")]);
        let lines = a.validate(
            &m,
            &Oversubscribed::from_ids(["small".to_string(), "ghost".to_string()]),
        );
        assert!(errors(&lines).is_empty(), "{lines:?}");
        assert!(lines.iter().any(|l| l.contains("but fits")), "{lines:?}");
        assert!(
            lines.iter().any(|l| l.contains("not in the manifest")),
            "{lines:?}"
        );
    }

    /// THE REAL INSTRUMENT on the real box: exactly two boards exceed four
    /// P-cores, both are the `--threads 8` mco boards, and with those two
    /// declared the manifest is clean. If a future board arrives that cannot
    /// run here, this test fails at startup instead of the queue stalling
    /// forever at 3am.
    #[test]
    fn the_committed_manifest_fits_this_box() {
        let p = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../benchmarks/manifest.toml");
        let Ok(m) = Manifest::load(&p) else {
            return; // the manifest is not part of this crate's fixtures
        };
        let a = Accountant::new(air());
        let over: Vec<&str> = m
            .boards
            .iter()
            .filter(|b| Demand::of(b, &m.defaults).cores() > a.capacity(Level::Full))
            .map(|b| b.id.as_str())
            .collect();
        assert_eq!(over, vec!["ipc7-mco-t8", "ipc2014-mco-t8"]);
        let declared = Oversubscribed::from_ids(over.iter().map(|s| s.to_string()));
        assert!(errors(&a.validate(&m, &declared)).is_empty());
    }

    /// A minimal manifest carrying just the boards a test needs. Built as TOML
    /// rather than by hand so the defaults and the parse rules are the real
    /// ones.
    fn manifest(boards: &[(&str, &str)]) -> Manifest {
        let mut s = String::from(
            "schema = 1\n\
             [corpus]\nroot = \".ipc-corpus\"\ndomain_shared = \"domain.pddl\"\n\
             domain_per_instance = \"domains/domain-{first}.pddl\"\n\
             [defaults]\ntimeout_secs = 60\njobs = 2\nthreads = 1\n\
             mode = \"auto\"\nmem_gb = 6.0\n",
        );
        for (id, extra) in boards {
            s.push_str(&format!(
                "[[board]]\nid = \"{id}\"\nraw = \"{id}.jsonl\"\nmd = \"{id}.md\"\n\
                 label = \"{id}\"\ncompetition = \"ipc7\"\nbudget_secs = 60\n\
                 track = \"seq-sat\"\n{extra}"
            ));
        }
        Manifest::parse(&s, "test").expect("test manifest parses")
    }
}
