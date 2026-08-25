//! `~/.config/crucible/config.toml` -- the operator's settings, as opposed to
//! `benchmarks/manifest.toml`, which is the INSTRUMENT and is versioned with
//! the planner.
//!
//! The split matters. Anything that changes what a measurement MEANS -- which
//! boards exist, their budgets, their job counts, their env -- lives in the
//! manifest, in the repo, in git history, next to the code it measures. Only
//! things that change how politely the machine behaves live here. An operator
//! editing this file must not be able to silently alter a published number.
//!
//! Every default below is either the value the shell drivers actually used or
//! a value the record justifies, and each says which.

use serde::Deserialize;
use std::path::PathBuf;

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Config {
    pub repo: Repo,
    pub sweep: Sweep,
    pub scheduler: Scheduler,
    pub quiet_hours: QuietHours,
    pub contention: Contention,
    pub ui: Ui,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Repo {
    /// The working tree crucible sweeps and publishes into.
    pub local: PathBuf,
    /// Poll interval for new tags, in seconds. Tag polling is the BACKFILL
    /// path, not the trigger: this project sweeps the cut candidate BEFORE the
    /// tag exists, so a tag-driven harness could only ever re-verify history.
    pub tag_poll_secs: u64,
    /// How many built tag worktrees to keep before garbage-collecting.
    pub keep_tags: usize,
    /// Where crucible's own worktrees live. Deliberately NOT the sibling
    /// `~/ferroplan-backfill-*` convention the operator uses by hand -- sharing
    /// that prefix would eventually garbage-collect somebody's manual checkout.
    pub worktree_dir: PathBuf,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Sweep {
    /// Which box these numbers were measured on. LOAD-BEARING: deltas are only
    /// ever computed between snapshots sharing a box, because coverage at a
    /// fixed time budget is a property of the hardware as much as the engine.
    pub box_id: String,
    /// External validator. Honoured for continuity with `$FERROPLAN_VAL`.
    pub validator: Option<PathBuf>,
    /// Seconds before VAL is considered hung. A timeout is NOT a rejected plan.
    pub val_timeout_secs: u64,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Scheduler {
    /// P-cores held back from the budget, for the OS and for crucible itself.
    pub reserve_p_cores: u32,
    /// Consecutive FULL time before a board may START. Replaces the shell's
    /// "two samples at 70% idle", which was denominated in a different currency
    /// from the verdict that judges the same board at the end.
    pub admit_dwell_secs: u64,
    /// An optional whole-machine idle floor. OFF by default: it is exactly the
    /// rule the 0.24 verdict change abandoned, because a `--threads 8` board
    /// burns 40-80% of this box by design and can never clear one.
    pub min_idle_pct: Option<f64>,
    /// Consecutive zero-progress attempts on a board before backing off. The
    /// shell gave up after 8 passes and exited 1; an instance-level queue
    /// converges instead, so this only guards a genuine stall.
    pub stall_attempts: u32,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct QuietHours {
    pub start: String,
    pub end: String,
    /// Quiet hours steer SCHEDULING and skip the game check. They must NEVER
    /// move a contention threshold: a Time Machine run at 3am depresses
    /// coverage exactly as much as one at 3pm, and the numbers have to be
    /// comparable regardless of the hour.
    pub skip_game_check: bool,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Contention {
    pub polite_dwell_secs: u64,
    pub suspend_threshold_pct: f64,
    pub resume_dwell_secs: u64,
    pub game_cpu_threshold_pct: f64,
    pub game_dwell_secs: u64,
    pub swap_pressure_mb: f64,
    pub known_games: Vec<String>,
    pub steam_process_names: Vec<String>,
    /// Sampling interval. 20s is what every committed conditions file was
    /// written at, and the resume gate's window padding is denominated in it,
    /// so changing it changes how a historical board is judged.
    pub sample_interval_secs: u64,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Ui {
    pub fps: u32,
    pub theme: String,
    pub banner_text: String,
}

impl Default for Repo {
    fn default() -> Self {
        Self {
            local: dirs_home().join("ferroplan"),
            tag_poll_secs: 300,
            keep_tags: 5,
            worktree_dir: dirs_home().join(".crucible/worktrees"),
        }
    }
}

impl Default for Sweep {
    fn default() -> Self {
        Self {
            // The value every committed snapshot carries.
            box_id: "m5-air".into(),
            validator: None,
            // ipc67.py's VAL wall.
            val_timeout_secs: 120,
        }
    }
}

impl Default for Scheduler {
    fn default() -> Self {
        Self {
            reserve_p_cores: 0,
            admit_dwell_secs: 40,
            min_idle_pct: None,
            stall_attempts: 3,
        }
    }
}

impl Default for QuietHours {
    fn default() -> Self {
        Self {
            start: "21:00".into(),
            end: "06:00".into(),
            skip_game_check: true,
        }
    }
}

impl Default for Contention {
    fn default() -> Self {
        Self {
            polite_dwell_secs: 20,
            suspend_threshold_pct: 60.0,
            resume_dwell_secs: 60,
            game_cpu_threshold_pct: 30.0,
            game_dwell_secs: 10,
            // 2 jobs x 6 GiB against 16 GiB physical leaves real headroom;
            // past this the box is thrashing and the numbers are worthless.
            swap_pressure_mb: 12_000.0,
            // Simulation-heavy and will absolutely fight a sweep for cores.
            known_games: vec!["Timberborn".into()],
            steam_process_names: vec!["steam_osx".into(), "steamwebhelper".into()],
            sample_interval_secs: 20,
        }
    }
}

impl Default for Ui {
    fn default() -> Self {
        Self {
            fps: 4,
            theme: "forge".into(),
            banner_text: "CRUCIBLE".into(),
        }
    }
}

fn dirs_home() -> PathBuf {
    std::env::var_os("HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("."))
}

impl Config {
    pub fn path() -> PathBuf {
        std::env::var_os("CRUCIBLE_CONFIG")
            .map(PathBuf::from)
            .unwrap_or_else(|| dirs_home().join(".config/crucible/config.toml"))
    }

    /// Load, or fall back to defaults when the file is absent.
    ///
    /// A MALFORMED file is an error, not a silent default: running a
    /// three-day sweep under settings the operator thought they had changed is
    /// exactly the kind of quiet wrongness this project exists to stop.
    /// `deny_unknown_fields` makes a typo'd key loud for the same reason.
    pub fn load(path: &std::path::Path) -> anyhow::Result<Config> {
        if !path.exists() {
            return Ok(Config::default());
        }
        let src = std::fs::read_to_string(path)?;
        toml::from_str(&src).map_err(|e| anyhow::anyhow!("{}: {e}", path.display()))
    }

    /// Parse "HH:MM" into minutes past midnight.
    fn hhmm(s: &str) -> Option<u32> {
        let (h, m) = s.split_once(':')?;
        Some(h.trim().parse::<u32>().ok()? * 60 + m.trim().parse::<u32>().ok()?)
    }

    /// Is `minutes_past_midnight` inside the quiet window?
    ///
    /// Read by the sweep's admission check to prefer long boards overnight and
    /// to skip the game detector -- quiet hours steer SCHEDULING and nothing
    /// else. A contention threshold never moves with the clock: a Time Machine
    /// run at 3am depresses coverage exactly as much as one at 3pm, and the
    /// numbers have to be comparable regardless of the hour. The window normally
    /// wraps midnight, which a naive `start <= t < end` gets exactly backwards.
    pub fn in_quiet_hours(&self, minutes_past_midnight: u32) -> bool {
        let (Some(a), Some(b)) = (
            Self::hhmm(&self.quiet_hours.start),
            Self::hhmm(&self.quiet_hours.end),
        ) else {
            return false;
        };
        if a <= b {
            (a..b).contains(&minutes_past_midnight)
        } else {
            minutes_past_midnight >= a || minutes_past_midnight < b
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 21:00-06:00 wraps midnight. A naive range check reports the window
    /// backwards -- awake all night, polite all day.
    #[test]
    fn the_quiet_window_wraps_midnight() {
        let c = Config::default();
        assert!(c.in_quiet_hours(21 * 60), "21:00 is quiet");
        assert!(c.in_quiet_hours(3 * 60), "03:00 is quiet");
        assert!(c.in_quiet_hours(0), "midnight is quiet");
        assert!(!c.in_quiet_hours(12 * 60), "noon is not");
        assert!(!c.in_quiet_hours(6 * 60), "06:00 is the exclusive end");
        assert!(c.in_quiet_hours(5 * 60 + 59));
    }

    #[test]
    fn a_non_wrapping_window_still_works() {
        let mut c = Config::default();
        c.quiet_hours.start = "01:00".into();
        c.quiet_hours.end = "05:00".into();
        assert!(c.in_quiet_hours(3 * 60));
        assert!(!c.in_quiet_hours(23 * 60));
    }

    #[test]
    fn a_missing_file_is_defaults_not_an_error() {
        let c = Config::load(std::path::Path::new("/nonexistent/crucible.toml")).unwrap();
        assert_eq!(c.ui.fps, 4);
        assert_eq!(c.sweep.box_id, "m5-air");
    }

    /// A typo'd key must be loud. Running a three-day sweep under settings the
    /// operator believed they had changed is the quiet wrongness this whole
    /// project exists to stop.
    #[test]
    fn an_unknown_key_is_refused() {
        let dir = std::env::temp_dir().join("crucible-cfg-test");
        std::fs::create_dir_all(&dir).unwrap();
        let p = dir.join("typo.toml");
        std::fs::write(&p, "[ui]\nfpsx = 9\n").unwrap();
        assert!(Config::load(&p).is_err());
        let p2 = dir.join("ok.toml");
        std::fs::write(&p2, "[ui]\nfps = 9\n").unwrap();
        assert_eq!(Config::load(&p2).unwrap().ui.fps, 9);
    }

    /// The defaults are the values the shell drivers actually used, and the
    /// sampling interval is one the resume gate's arithmetic depends on.
    #[test]
    fn the_defaults_match_the_recorded_practice() {
        let c = Config::default();
        assert_eq!(c.contention.sample_interval_secs, 20);
        assert_eq!(c.sweep.val_timeout_secs, 120);
        assert!(c.scheduler.min_idle_pct.is_none());
        assert!(c.contention.known_games.iter().any(|g| g == "Timberborn"));
    }
}
