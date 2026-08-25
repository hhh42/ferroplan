//! When a board may START. `wait_quiet`, moved onto one currency.
//!
//! The shell driver's `wait_quiet` (`benchmarks/cut25-sweeps.sh:66-75`, over
//! the `idle_pct` helper at 61-64) holds a board until it has seen TWO
//! consecutive samples of at least 70% whole-machine IDLE. The same driver
//! then judges the finished board on `competitors_total_pcpu < 25` -- the
//! watcher's verdict. Two gates, two
//! different quantities, and the mismatch has a name in this repo: the 0.24
//! verdict change.
//!
//! `monitor::sample` records why the verdict moved. `idle_pct` is
//! whole-machine and includes the sweep's OWN threads, so an mco `--threads 8`
//! board burns 40-80% of this ten-core box BY DESIGN and reads 38-40% idle in
//! an empty room. Both DEGRADED records on this box are mco boards, and the two
//! fixtures say precisely why the currency had to change: the t4 record
//! (`tests/fixtures/conditions/degraded-old-idle-rule-mco-t4.json`) was failed
//! at 64.3% median idle against 2.6% of real competing load -- a false
//! positive of the idle rule and nothing else -- while the t8 record beside it
//! carries `spotlightknowledged` at 52.5% and was genuinely contended. One
//! rule, one right answer and one wrong one; the competitor currency gets both.
//! The start gate has exactly the same defect, in the same direction, and one
//! extra one of its own: a 70% idle floor also refuses to start while the
//! PREVIOUS board's planner is winding down, so the driver waits a full minute
//! on load it created itself and will never be charged for.
//!
//! So the floor goes, and admission is stated in the competitor currency the
//! verdict already uses: a board may start when the throttle has been at
//! [`Level::Full`] for an admit dwell. That is not a re-derivation of "is the
//! box busy" -- `monitor::throttle` owns that question, with hysteresis, and
//! FULL already means foreign load has been under [`SAMPLE_CLEAN_PCPU`]
//! (`crate::monitor::SAMPLE_CLEAN_PCPU`). The dwell on top is the shell's "two
//! consecutive samples", kept because it is the thing that stops a board
//! starting into the trailing edge of a contention window.
//!
//! [`Config::min_idle_pct`] keeps the old floor available and DEFAULTS TO OFF.
//! A box whose core layout makes whole-machine idle meaningful again is a
//! configuration change rather than a rewrite -- but on this one, turning it on
//! reintroduces a documented incident, so admission records which rule let the
//! board through ([`Rule`]) and a board that started under the floor says so.

use crate::monitor::{Level, Sample};
use std::time::{Duration, Instant};

/// Two samples at the watcher's default 20 s interval -- the shell's
/// `got -lt 2`, in seconds rather than in sample counts, so a change of
/// interval cannot silently change the wait.
pub const DEFAULT_ADMIT_DWELL: Duration = Duration::from_secs(40);

/// How long to wait before asking again. The shell's `sleep 60`.
pub const DEFAULT_RETRY_AFTER: Duration = Duration::from_secs(60);

/// The floor the 0.24 change abandoned, kept only so it can be turned back on
/// deliberately. `QUIET=70` in every driver that had one.
pub const LEGACY_IDLE_FLOOR_PCT: f64 = 70.0;

#[derive(Debug, Clone)]
pub struct Config {
    /// How long the throttle must have been FULL before a board may start.
    pub admit_dwell: Duration,
    /// OFF by default; see the module header on why it is off and not gone.
    pub min_idle_pct: Option<f64>,
    /// Poll interval while waiting, for the caller's sleep.
    pub retry_after: Duration,
}

impl Default for Config {
    fn default() -> Self {
        Config {
            admit_dwell: DEFAULT_ADMIT_DWELL,
            min_idle_pct: None,
            retry_after: DEFAULT_RETRY_AFTER,
        }
    }
}

/// Which rule admitted the board. Recorded, not inferred: a board that started
/// under the legacy floor must be able to say so afterwards.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Rule {
    /// FULL for the admit dwell. The only rule in the default configuration.
    CompetitorDwell,
    /// FULL for the admit dwell AND above the configured idle floor.
    CompetitorDwellAndIdleFloor,
}

/// Why not yet. Every variant is "ask again in [`Config::retry_after`]".
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Denied {
    /// The throttle is POLITE or SUSPENDED: foreign load, a game, or memory
    /// pressure. Starting a board here would measure the contention.
    Throttled(Level),
    /// Quiet, but not for long enough yet.
    Dwell { have: Duration, need: Duration },
    /// The legacy floor, when it is switched on. `have: None` means the box
    /// did not report idle at all, and unknown is not quiet.
    IdleFloor { have: Option<f64>, need: f64 },
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Admission {
    Admit(Rule),
    Wait(Denied),
}

impl Admission {
    pub fn is_admitted(&self) -> bool {
        matches!(self, Admission::Admit(_))
    }
}

/// The start gate. One per sweep, not one per board: once the box is quiet it
/// stays quiet, and making each board re-earn forty seconds of stillness costs
/// measurement time for nothing. Anything that disturbs the box moves the
/// throttle off FULL, which clears the dwell on its own.
#[derive(Debug)]
pub struct Gate {
    cfg: Config,
    full_since: Option<Instant>,
}

impl Gate {
    pub fn new(cfg: Config) -> Self {
        Gate {
            cfg,
            full_since: None,
        }
    }

    pub fn config(&self) -> &Config {
        &self.cfg
    }

    /// Make the next board earn a fresh dwell -- after an operator hold, or
    /// after anything that leaves the box in a state the throttle cannot see.
    pub fn reset(&mut self) {
        self.full_since = None;
    }

    /// May a board start now?
    ///
    /// `level` is the throttle's, and it is authoritative: this gate never
    /// re-derives "is the box busy" from the sample, because two implementations
    /// of that question are exactly what this module exists to end.
    pub fn poll(&mut self, level: Level, sample: &Sample, now: Instant) -> Admission {
        if level != Level::Full {
            self.full_since = None;
            return Admission::Wait(Denied::Throttled(level));
        }
        let since = *self.full_since.get_or_insert(now);
        let have = now.saturating_duration_since(since);
        if have < self.cfg.admit_dwell {
            return Admission::Wait(Denied::Dwell {
                have,
                need: self.cfg.admit_dwell,
            });
        }
        match self.cfg.min_idle_pct {
            None => Admission::Admit(Rule::CompetitorDwell),
            Some(need) => match sample.idle_pct {
                // Unknown is not quiet -- the same rule the resume gate applies
                // to a sample with no competitor total.
                Some(have) if have >= need => Admission::Admit(Rule::CompetitorDwellAndIdleFloor),
                have => Admission::Wait(Denied::IdleFloor { have, need }),
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn at(t0: Instant, secs: u64) -> Instant {
        t0 + Duration::from_secs(secs)
    }

    fn quiet_box() -> Sample {
        Sample {
            idle_pct: Some(92.0),
            competitors_total: 3.0,
            ..Default::default()
        }
    }

    /// The shell's "two consecutive samples", in the new currency: quiet is not
    /// enough, quiet for a while is.
    #[test]
    fn full_admits_only_after_the_dwell() {
        let t0 = Instant::now();
        let mut g = Gate::new(Config::default());
        assert!(matches!(
            g.poll(Level::Full, &quiet_box(), t0),
            Admission::Wait(Denied::Dwell { .. })
        ));
        assert!(matches!(
            g.poll(Level::Full, &quiet_box(), at(t0, 39)),
            Admission::Wait(Denied::Dwell { .. })
        ));
        assert_eq!(
            g.poll(Level::Full, &quiet_box(), at(t0, 40)),
            Admission::Admit(Rule::CompetitorDwell)
        );
    }

    /// A blip mid-dwell starts the clock again. This is the thing the shell's
    /// `got=0` reset was for: a board must not start into the trailing edge of
    /// a contention window.
    #[test]
    fn any_demotion_restarts_the_dwell() {
        let t0 = Instant::now();
        let mut g = Gate::new(Config::default());
        g.poll(Level::Full, &quiet_box(), t0);
        assert!(matches!(
            g.poll(Level::Polite, &quiet_box(), at(t0, 30)),
            Admission::Wait(Denied::Throttled(Level::Polite))
        ));
        // 40s have passed since the FIRST full sample, but only 10 since the
        // box came back.
        assert!(matches!(
            g.poll(Level::Full, &quiet_box(), at(t0, 40)),
            Admission::Wait(Denied::Dwell { .. })
        ));
        assert!(g.poll(Level::Full, &quiet_box(), at(t0, 80)).is_admitted());
    }

    /// THE 0.24 INCIDENT, from the start-gate end. An mco `--threads 8` board
    /// reads 38-40% idle in an empty room, so the old 70% floor could never
    /// start one -- and both DEGRADED records on this box are mco boards.
    #[test]
    fn a_thread_heavy_board_starts_in_an_empty_room() {
        let t0 = Instant::now();
        let mut g = Gate::new(Config::default());
        let mco = Sample {
            idle_pct: Some(38.0),
            competitors_total: 4.5,
            ..Default::default()
        };
        g.poll(Level::Full, &mco, t0);
        assert_eq!(
            g.poll(Level::Full, &mco, at(t0, 40)),
            Admission::Admit(Rule::CompetitorDwell),
            "38% idle with 4.5% of foreign load is an empty room, not a busy box"
        );
    }

    /// The floor is still available, still wrong on this box, and says which
    /// rule it was when it admits -- so a board measured under it is traceable.
    #[test]
    fn the_legacy_idle_floor_is_off_by_default_and_names_itself_when_on() {
        assert!(Config::default().min_idle_pct.is_none());
        let t0 = Instant::now();
        let mut g = Gate::new(Config {
            min_idle_pct: Some(LEGACY_IDLE_FLOOR_PCT),
            ..Default::default()
        });
        let mco = Sample {
            idle_pct: Some(38.0),
            competitors_total: 4.5,
            ..Default::default()
        };
        g.poll(Level::Full, &mco, t0);
        assert_eq!(
            g.poll(Level::Full, &mco, at(t0, 40)),
            Admission::Wait(Denied::IdleFloor {
                have: Some(38.0),
                need: 70.0
            }),
            "which is the incident, reproduced on demand"
        );
        assert_eq!(
            g.poll(Level::Full, &quiet_box(), at(t0, 60)),
            Admission::Admit(Rule::CompetitorDwellAndIdleFloor)
        );
    }

    /// Unknown is not quiet: a box that did not report idle has not cleared a
    /// floor denominated in idle. The same rule the resume gate applies to a
    /// sample with no competitor total.
    #[test]
    fn an_unreported_idle_does_not_clear_the_floor() {
        let t0 = Instant::now();
        let mut g = Gate::new(Config {
            min_idle_pct: Some(LEGACY_IDLE_FLOOR_PCT),
            ..Default::default()
        });
        let blind = Sample {
            idle_pct: None,
            competitors_total: 1.0,
            ..Default::default()
        };
        g.poll(Level::Full, &blind, t0);
        assert_eq!(
            g.poll(Level::Full, &blind, at(t0, 40)),
            Admission::Wait(Denied::IdleFloor {
                have: None,
                need: 70.0
            })
        );
    }

    /// Suspension is not a slow start, it is a stop. The gate reports the level
    /// so the operator log can say WHICH -- a game and a Time Machine run need
    /// different responses from a human.
    #[test]
    fn suspension_denies_and_names_the_level() {
        let t0 = Instant::now();
        let mut g = Gate::new(Config::default());
        assert_eq!(
            g.poll(Level::Suspended, &quiet_box(), t0),
            Admission::Wait(Denied::Throttled(Level::Suspended))
        );
    }

    /// Once the box is quiet the SECOND board starts immediately: making every
    /// board re-earn forty seconds of stillness costs measurement time for
    /// nothing, and anything that disturbs the box moves the throttle off FULL.
    #[test]
    fn a_quiet_box_admits_the_next_board_at_once() {
        let t0 = Instant::now();
        let mut g = Gate::new(Config::default());
        g.poll(Level::Full, &quiet_box(), t0);
        assert!(g.poll(Level::Full, &quiet_box(), at(t0, 40)).is_admitted());
        assert!(
            g.poll(Level::Full, &quiet_box(), at(t0, 41)).is_admitted(),
            "the box did not become less quiet by being used"
        );
        // ...unless the operator asks for a fresh one.
        g.reset();
        assert!(!g.poll(Level::Full, &quiet_box(), at(t0, 42)).is_admitted());
    }
}
