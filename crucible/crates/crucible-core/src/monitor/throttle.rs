//! FULL / POLITE / SUSPENDED, with hysteresis so it cannot flap.
//!
//! This is the fix for the incident that motivated the whole project: a mail
//! client doing a one-time sync should cost a timing number. It must not, under
//! any circumstances, cost hours of completed computation.
//!
//! De-escalation is deliberately slower than escalation. Going polite early is
//! cheap -- the runs keep running, just on the efficiency cores. Coming back
//! early is expensive, because it re-enters the contention that just triggered.
//!
//! A CORRECTION TO THE SPEC: `crucible-spec.md` §5.3 wants contention detection
//! "relaxed" during quiet hours so a 3am background sync does not demote
//! anything. No. A Time Machine run at 3am depresses coverage exactly as much
//! as one at 3pm, and the numbers have to be comparable regardless of the hour.
//! What quiet hours legitimately change is SCHEDULING -- which boards to
//! prefer, how long to wait for the box to settle, and whether to bother
//! checking for a game. Thresholds never move.

use super::sample::{Sample, SAMPLE_CLEAN_PCPU};
use std::time::{Duration, Instant};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Level {
    Full,
    /// Foreign load is real but tolerable. Children move to the background
    /// scheduling band and KEEP RUNNING. Nothing is lost, only slowed.
    Polite,
    /// A game is actually consuming CPU, or foreign load is overwhelming, or
    /// memory pressure is critical. Children are SIGSTOPped and held.
    Suspended,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Reason {
    Foreign(f64),
    Game { name: String, cpu: f64 },
    MemoryPressure(f64),
    Manual,
    Clear,
}

#[derive(Debug, Clone)]
pub struct Config {
    pub polite_threshold_pct: f64,
    pub polite_dwell: Duration,
    pub suspend_threshold_pct: f64,
    pub resume_dwell: Duration,
    pub game_cpu_threshold_pct: f64,
    pub game_dwell: Duration,
    /// Swap in MiB above which the box is considered to be thrashing. A
    /// swapping box slows search while looking perfectly CPU-idle.
    pub swap_pressure_mb: f64,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            polite_threshold_pct: SAMPLE_CLEAN_PCPU,
            polite_dwell: Duration::from_secs(20),
            suspend_threshold_pct: 60.0,
            resume_dwell: Duration::from_secs(60),
            game_cpu_threshold_pct: 30.0,
            game_dwell: Duration::from_secs(10),
            swap_pressure_mb: 12_000.0,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Transition {
    pub from: Level,
    pub to: Level,
    pub reason: Reason,
}

pub struct Throttle {
    level: Level,
    cfg: Config,
    over_polite_since: Option<Instant>,
    over_suspend_since: Option<Instant>,
    game_since: Option<Instant>,
    clear_since: Option<Instant>,
    manual_hold: bool,
}

/// What the game detector saw this tick.
///
/// Presence alone is NOT enough -- Steam idling in the background is fine, and
/// suspending a three-day sweep because a launcher is open would be its own
/// kind of failure. The trigger is a game process actually burning CPU.
#[derive(Debug, Clone, Default)]
pub struct GameState {
    pub busiest: Option<(String, f64)>,
}

impl Throttle {
    pub fn new(cfg: Config) -> Self {
        Self {
            level: Level::Full,
            cfg,
            over_polite_since: None,
            over_suspend_since: None,
            game_since: None,
            clear_since: None,
            manual_hold: false,
        }
    }

    pub fn level(&self) -> Level {
        self.level
    }

    /// Operator override: hold everything until released.
    pub fn set_manual_hold(&mut self, on: bool) -> Option<Transition> {
        self.manual_hold = on;
        if on && self.level != Level::Suspended {
            return Some(self.go(Level::Suspended, Reason::Manual));
        }
        if !on && self.level == Level::Suspended {
            self.clear_since = None;
            return Some(self.go(Level::Full, Reason::Clear));
        }
        None
    }

    fn go(&mut self, to: Level, reason: Reason) -> Transition {
        let from = self.level;
        self.level = to;
        Transition { from, to, reason }
    }

    pub fn on_sample(&mut self, s: &Sample, g: &GameState, now: Instant) -> Option<Transition> {
        if self.manual_hold {
            return None;
        }
        let load = s.competitors_total;

        let hold = |slot: &mut Option<Instant>, over: bool| -> Option<Duration> {
            if over {
                let since = *slot.get_or_insert(now);
                Some(now.saturating_duration_since(since))
            } else {
                *slot = None;
                None
            }
        };

        let polite_for = hold(
            &mut self.over_polite_since,
            load > self.cfg.polite_threshold_pct,
        );
        let suspend_for = hold(
            &mut self.over_suspend_since,
            load > self.cfg.suspend_threshold_pct,
        );
        let game_busy = g
            .busiest
            .as_ref()
            .filter(|(_, cpu)| *cpu > self.cfg.game_cpu_threshold_pct);
        let game_for = hold(&mut self.game_since, game_busy.is_some());
        // The kernel's level when it has one -- CRITICAL suspends, warn does
        // not -- and the swap-stock line only as a fallback where it does
        // not. Swap in use never comes back down once idle pages are out.
        let swapping = match s.mem_pressure {
            Some(level) => level >= 4,
            None => s.swap_mb.is_some_and(|m| m > self.cfg.swap_pressure_mb),
        };

        // Escalate first, and to the highest level the evidence supports.
        if game_for.is_some_and(|d| d >= self.cfg.game_dwell) {
            if self.level != Level::Suspended {
                let (name, cpu) = game_busy.cloned().unwrap_or_default();
                self.clear_since = None;
                return Some(self.go(Level::Suspended, Reason::Game { name, cpu }));
            }
            self.clear_since = None;
            return None;
        }
        if suspend_for.is_some_and(|d| d >= self.cfg.polite_dwell) || swapping {
            if self.level != Level::Suspended {
                self.clear_since = None;
                let r = if swapping {
                    Reason::MemoryPressure(s.swap_mb.unwrap_or_default())
                } else {
                    Reason::Foreign(load)
                };
                return Some(self.go(Level::Suspended, r));
            }
            self.clear_since = None;
            return None;
        }
        if polite_for.is_some_and(|d| d >= self.cfg.polite_dwell) && self.level == Level::Full {
            self.clear_since = None;
            return Some(self.go(Level::Polite, Reason::Foreign(load)));
        }

        // De-escalate only after a sustained clear stretch, and only ONE level
        // at a time -- SUSPENDED returns to POLITE, which then has to earn FULL
        // separately. Jumping straight back to full throttle is how a
        // borderline box oscillates.
        if load <= self.cfg.polite_threshold_pct && game_busy.is_none() && !swapping {
            let since = *self.clear_since.get_or_insert(now);
            if now.saturating_duration_since(since) >= self.cfg.resume_dwell {
                self.clear_since = Some(now);
                return match self.level {
                    Level::Suspended => Some(self.go(Level::Polite, Reason::Clear)),
                    Level::Polite => Some(self.go(Level::Full, Reason::Clear)),
                    Level::Full => None,
                };
            }
        } else {
            self.clear_since = None;
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn load(pcpu: f64) -> Sample {
        Sample {
            competitors_total: pcpu,
            ..Default::default()
        }
    }

    fn at(t0: Instant, secs: u64) -> Instant {
        t0 + Duration::from_secs(secs)
    }

    /// THE MOTIVATING INCIDENT: a mail sync demotes the sweep to the efficiency
    /// cores. It does not, under any circumstances, discard the work.
    #[test]
    fn sustained_foreign_load_goes_polite_not_suspended() {
        let t0 = Instant::now();
        let mut th = Throttle::new(Config::default());
        assert_eq!(th.on_sample(&load(34.0), &GameState::default(), t0), None);
        let tr = th
            .on_sample(&load(34.0), &GameState::default(), at(t0, 20))
            .expect("20s of load crosses the dwell");
        assert_eq!(tr.to, Level::Polite);
    }

    /// A brief spike must not demote anything -- that is what the dwell is for.
    #[test]
    fn a_spike_shorter_than_the_dwell_changes_nothing() {
        let t0 = Instant::now();
        let mut th = Throttle::new(Config::default());
        assert!(th
            .on_sample(&load(80.0), &GameState::default(), t0)
            .is_none());
        assert!(th
            .on_sample(&load(2.0), &GameState::default(), at(t0, 5))
            .is_none());
        assert!(th
            .on_sample(&load(80.0), &GameState::default(), at(t0, 10))
            .is_none());
        assert_eq!(th.level(), Level::Full);
    }

    /// Presence is not enough: a launcher sitting idle must not suspend a
    /// three-day sweep.
    #[test]
    fn an_idle_game_process_does_not_suspend() {
        let t0 = Instant::now();
        let mut th = Throttle::new(Config::default());
        let idle_game = GameState {
            busiest: Some(("Timberborn".into(), 1.0)),
        };
        for s in [0, 20, 60, 120] {
            assert!(th.on_sample(&load(1.0), &idle_game, at(t0, s)).is_none());
        }
        assert_eq!(th.level(), Level::Full);
    }

    #[test]
    fn a_busy_game_suspends_after_its_dwell() {
        let t0 = Instant::now();
        let mut th = Throttle::new(Config::default());
        let playing = GameState {
            busiest: Some(("Timberborn".into(), 240.0)),
        };
        assert!(th.on_sample(&load(1.0), &playing, t0).is_none());
        let tr = th.on_sample(&load(1.0), &playing, at(t0, 10)).unwrap();
        assert_eq!(tr.to, Level::Suspended);
        assert!(matches!(tr.reason, Reason::Game { .. }));
    }

    /// De-escalation is slower than escalation, and one level at a time.
    #[test]
    fn recovery_is_gradual_and_earns_each_level() {
        let t0 = Instant::now();
        let mut th = Throttle::new(Config::default());
        let playing = GameState {
            busiest: Some(("Timberborn".into(), 240.0)),
        };
        th.on_sample(&load(1.0), &playing, t0);
        th.on_sample(&load(1.0), &playing, at(t0, 10));
        assert_eq!(th.level(), Level::Suspended);

        let quiet = GameState::default();
        assert!(th.on_sample(&load(1.0), &quiet, at(t0, 20)).is_none());
        assert!(
            th.on_sample(&load(1.0), &quiet, at(t0, 40)).is_none(),
            "less than the 60s resume dwell"
        );
        let tr = th.on_sample(&load(1.0), &quiet, at(t0, 85)).unwrap();
        assert_eq!(
            tr.to,
            Level::Polite,
            "suspended returns to polite, not full"
        );
        let tr = th.on_sample(&load(1.0), &quiet, at(t0, 150)).unwrap();
        assert_eq!(tr.to, Level::Full);
    }

    /// A swapping box slows search while looking perfectly CPU-idle, so the
    /// CPU threshold alone would never catch it.
    #[test]
    fn memory_pressure_suspends_even_at_zero_cpu_load() {
        let t0 = Instant::now();
        let mut th = Throttle::new(Config::default());
        let s = Sample {
            competitors_total: 0.0,
            swap_mb: Some(20_000.0),
            ..Default::default()
        };
        let tr = th.on_sample(&s, &GameState::default(), t0).unwrap();
        assert_eq!(tr.to, Level::Suspended);
        assert!(matches!(tr.reason, Reason::MemoryPressure(_)));
    }

    #[test]
    fn a_manual_hold_overrides_everything_until_released() {
        let mut th = Throttle::new(Config::default());
        let tr = th.set_manual_hold(true).unwrap();
        assert_eq!(tr.to, Level::Suspended);
        // Quiet samples must not release it.
        assert!(th
            .on_sample(&load(0.0), &GameState::default(), Instant::now())
            .is_none());
        assert_eq!(th.level(), Level::Suspended);
        assert_eq!(th.set_manual_hold(false).unwrap().to, Level::Full);
    }
}

#[cfg(test)]
mod pressure_tests {
    use super::*;

    fn sample(swap_mb: f64, level: Option<u32>) -> Sample {
        Sample {
            at: 0.0,
            competitors_total: 0.0,
            swap_mb: Some(swap_mb),
            mem_pressure: level,
            ..Default::default()
        }
    }

    /// 15 GB of swap in use with the kernel at "warn" is a box that paged
    /// out its idle pages, not a box under pressure. The first R2 evening
    /// sat SUSPENDED on exactly this until the level replaced the stock.
    #[test]
    fn the_kernel_level_outranks_the_swap_stock() {
        let mut t = Throttle::new(Config::default());
        let g = GameState::default();
        let now = Instant::now();
        assert!(t.on_sample(&sample(15_700.0, Some(2)), &g, now).is_none());
        assert_eq!(t.level(), Level::Full);
        let tr = t
            .on_sample(&sample(15_700.0, Some(4)), &g, now)
            .expect("critical suspends");
        assert_eq!(tr.to, Level::Suspended);
        assert!(matches!(tr.reason, Reason::MemoryPressure(_)));
    }

    /// Without a kernel reading the swap-stock line still applies.
    #[test]
    fn the_swap_stock_is_the_fallback() {
        let mut t = Throttle::new(Config::default());
        let g = GameState::default();
        let tr = t.on_sample(&sample(15_700.0, None), &g, Instant::now());
        assert_eq!(tr.map(|t| t.to), Some(Level::Suspended));
    }
}
