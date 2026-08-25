//! What the dashboard knows.
//!
//! A SNAPSHOT, deliberately. The UI never reaches into scheduler state, never
//! takes a lock a runner wants, and never blocks a measurement. The scheduler
//! publishes one of these when something changes; the worst a wedged terminal
//! can do is stop repainting. That constraint is why this file holds plain
//! data and no behaviour beyond formatting helpers.

use std::time::Duration;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Level {
    Full,
    Polite,
    Suspended,
}

impl Level {
    pub fn label(self) -> &'static str {
        match self {
            Level::Full => "FULL",
            Level::Polite => "POLITE",
            Level::Suspended => "SUSPENDED",
        }
    }
}

/// A run in flight, or a slot with nothing in it.
#[derive(Debug, Clone)]
pub struct Slot {
    pub index: usize,
    pub what: Option<SlotRun>,
}

#[derive(Debug, Clone)]
pub struct SlotRun {
    pub variant: String,
    pub instance: String,
    pub tier: char,
    /// Wall minus suspension. What the deadline is compared against, and the
    /// number that explains why a stopped run is not about to time out.
    pub effective: Duration,
    pub suspended: bool,
    pub budget: Duration,
}

#[derive(Debug, Clone)]
pub struct DomainProgress {
    pub name: String,
    /// The bar tracks SOLVED, not "ran": coverage is the metric, and a full bar
    /// beside `12/30` would say the opposite of the truth.
    pub solved: usize,
    pub total: usize,
    /// Solved on the previous release but not here. The loud case.
    pub regressions: usize,
}

#[derive(Debug, Clone)]
pub struct TrackProgress {
    pub name: String,
    pub done: usize,
    pub total: usize,
    pub solved: usize,
    pub delta: Option<i64>,
    pub domains: Vec<DomainProgress>,
    pub expanded: bool,
    pub finished: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LogKind {
    Info,
    Good,
    Warn,
    /// A problem solved on the previous release and not on this one. Red,
    /// pinned, and it does not scroll away.
    Regression,
}

#[derive(Debug, Clone)]
pub struct LogLine {
    pub at: String,
    pub kind: LogKind,
    pub text: String,
}

#[derive(Debug, Clone)]
pub struct Toast {
    pub text: String,
    pub kind: LogKind,
    /// A regression toast is sticky: it must be dismissed, not waited out.
    pub sticky: bool,
    pub age: Duration,
}

#[derive(Debug, Clone, Default)]
pub struct Snapshot {
    pub engine_ver: String,
    pub engine_hash: String,
    pub level: LevelState,
    pub uptime: Duration,
    pub quiet_in: Option<Duration>,
    pub sweep: SweepProgress,
    pub tracks: Vec<TrackProgress>,
    pub slots: Vec<Slot>,
    pub p_cores: u32,
    pub log: Vec<LogLine>,
    pub toasts: Vec<Toast>,
    /// Runs per minute, most recent last. Drives the throughput sparkline --
    /// the one line that makes the dashboard worth leaving on screen, because
    /// it shows a demotion happening rather than merely reporting it.
    pub throughput: Vec<f64>,
    pub selected: usize,
}

#[derive(Debug, Clone)]
pub struct LevelState {
    pub level: Level,
    pub reason: Option<String>,
}

impl Default for LevelState {
    fn default() -> Self {
        Self {
            level: Level::Full,
            reason: None,
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct SweepProgress {
    pub done: usize,
    pub total: usize,
    pub solved: usize,
    pub delta: Option<i64>,
    pub delta_vs: String,
    pub regressions: usize,
    /// Rows measured while the box was not quiet. They are KEPT -- nothing is
    /// discarded -- but the board cannot be banked until they are re-measured
    /// clean, so this is a count of work still owed, not of work lost.
    pub dirty: usize,
    pub eta: Option<Duration>,
}

impl SweepProgress {
    pub fn frac(&self) -> f64 {
        if self.total == 0 {
            0.0
        } else {
            self.done as f64 / self.total as f64
        }
    }
}

impl Snapshot {
    /// Rows the dashboard can show for the track list, honouring the
    /// progressive-disclosure rule: a finished track collapses to one line, the
    /// active one expands, a queued one is a dim single line.
    pub fn visible_tracks(&self) -> Vec<(usize, Option<usize>)> {
        let mut out = Vec::new();
        for (i, t) in self.tracks.iter().enumerate() {
            out.push((i, None));
            if t.expanded && !t.finished {
                for (j, _) in t.domains.iter().enumerate() {
                    out.push((i, Some(j)));
                }
            }
        }
        out
    }

    pub fn toggle_selected(&mut self) {
        let rows = self.visible_tracks();
        if let Some(&(i, None)) = rows.get(self.selected) {
            if let Some(t) = self.tracks.get_mut(i) {
                t.expanded = !t.expanded;
            }
        }
    }

    /// `z` -- collapse everything that is finished, which is the state you want
    /// after a track completes and its thirty domains are still on screen.
    pub fn collapse_finished(&mut self) {
        for t in self.tracks.iter_mut().filter(|t| t.finished) {
            t.expanded = false;
        }
        self.clamp_selection();
    }

    pub fn move_selection(&mut self, delta: isize) {
        let n = self.visible_tracks().len();
        if n == 0 {
            return;
        }
        let next = self.selected as isize + delta;
        self.selected = next.clamp(0, n as isize - 1) as usize;
    }

    fn clamp_selection(&mut self) {
        let n = self.visible_tracks().len();
        self.selected = self.selected.min(n.saturating_sub(1));
    }

    /// Toasts dwell for four seconds; a regression's does not expire at all.
    pub fn expire_toasts(&mut self, dwell: Duration) {
        self.toasts.retain(|t| t.sticky || t.age < dwell);
    }

    pub fn dismiss_toasts(&mut self) {
        self.toasts.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn track(name: &str, finished: bool, expanded: bool, domains: usize) -> TrackProgress {
        TrackProgress {
            name: name.into(),
            done: 1,
            total: 2,
            solved: 1,
            delta: None,
            domains: (0..domains)
                .map(|i| DomainProgress {
                    name: format!("d{i}"),
                    solved: 0,
                    total: 30,
                    regressions: 0,
                })
                .collect(),
            expanded,
            finished,
        }
    }

    /// A finished track collapses to a single line even when it is marked
    /// expanded -- otherwise a completed 30-domain track keeps the active one
    /// off screen for the rest of the sweep.
    #[test]
    fn a_finished_track_never_expands() {
        let s = Snapshot {
            tracks: vec![track("IPC5", true, true, 30)],
            ..Default::default()
        };
        assert_eq!(s.visible_tracks().len(), 1);
    }

    #[test]
    fn an_active_expanded_track_shows_its_domains() {
        let s = Snapshot {
            tracks: vec![track("IPC6", false, true, 3)],
            ..Default::default()
        };
        assert_eq!(s.visible_tracks().len(), 4);
    }

    /// Selection must survive a collapse that removes the row it was on.
    #[test]
    fn collapsing_clamps_the_selection_instead_of_dangling() {
        let mut s = Snapshot {
            tracks: vec![track("IPC5", true, true, 10)],
            ..Default::default()
        };
        s.selected = 8;
        s.collapse_finished();
        assert_eq!(s.selected, 0);
    }

    #[test]
    fn selection_cannot_leave_the_list() {
        let mut s = Snapshot {
            tracks: vec![track("A", false, false, 0), track("B", false, false, 0)],
            ..Default::default()
        };
        s.move_selection(-5);
        assert_eq!(s.selected, 0);
        s.move_selection(99);
        assert_eq!(s.selected, 1);
    }

    #[test]
    fn moving_within_an_empty_list_is_harmless() {
        let mut s = Snapshot::default();
        s.move_selection(1);
        s.toggle_selected();
        assert_eq!(s.selected, 0);
    }

    /// A REGRESSION toast must not scroll away on a four-second timer -- it is
    /// the one thing on this screen nobody may miss.
    #[test]
    fn a_regression_toast_is_sticky() {
        let mut s = Snapshot {
            toasts: vec![
                Toast {
                    text: "resumed".into(),
                    kind: LogKind::Info,
                    sticky: false,
                    age: Duration::from_secs(9),
                },
                Toast {
                    text: "REGRESSION".into(),
                    kind: LogKind::Regression,
                    sticky: true,
                    age: Duration::from_secs(9999),
                },
            ],
            ..Default::default()
        };
        s.expire_toasts(Duration::from_secs(4));
        assert_eq!(s.toasts.len(), 1);
        assert_eq!(s.toasts[0].kind, LogKind::Regression);
        s.dismiss_toasts();
        assert!(s.toasts.is_empty());
    }

    #[test]
    fn an_empty_sweep_does_not_divide_by_zero() {
        assert_eq!(SweepProgress::default().frac(), 0.0);
    }
}
