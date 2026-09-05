//! What the dashboard knows.
//!
//! A SNAPSHOT, deliberately. The UI never reaches into scheduler state, never
//! takes a lock a runner wants, and never blocks a measurement. The sweep
//! publishes progress; the UI thread builds one of these per tick from it;
//! the worst a wedged terminal can do is stop repainting. That constraint is
//! why this file holds plain data and no behaviour beyond navigation and
//! formatting helpers.
//!
//! R2 (`crucible-spec.md` R2.4): the unit on screen is the INSTANCE. Every
//! one of the sweep's rows is a cell in its board's strip; a board is a row
//! of the grid; Enter drills to the board's instances, Enter again to one
//! instance's attempts and the box's timeline across its window.

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

/// One instance's state, as a cell in its board's strip.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Cell {
    #[default]
    Queued,
    Running,
    /// Solved, timing clean.
    SolvedClean,
    /// Solved, measured under a cloud (timing dirty). Coverage is coverage.
    SolvedDirty,
    /// Unsolved and BANKED: the referee saw the process had its core.
    TimeoutBanked,
    /// Unsolved and OWED: starved, a clock jump, swap, a slow box. Re-run.
    Owed,
    /// Crashed, rejected by VAL, or otherwise not a measurement.
    Error,
}

impl Cell {
    /// Worst first, for a strip that has to show several instances in one
    /// column: what still needs attention beats what is settled.
    pub fn rank(self) -> u8 {
        match self {
            Cell::Owed => 6,
            Cell::Running => 5,
            Cell::Error => 4,
            Cell::TimeoutBanked => 3,
            Cell::SolvedDirty => 2,
            Cell::SolvedClean => 1,
            Cell::Queued => 0,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Cell::Queued => "queued",
            Cell::Running => "running",
            Cell::SolvedClean => "solved",
            Cell::SolvedDirty => "solved (dirty timing)",
            Cell::TimeoutBanked => "unsolved, banked",
            Cell::Owed => "unsolved, OWED",
            Cell::Error => "error",
        }
    }

    pub fn banked(self) -> bool {
        matches!(
            self,
            Cell::SolvedClean | Cell::SolvedDirty | Cell::TimeoutBanked
        )
    }
}

#[derive(Debug, Clone, Default)]
pub struct InstanceCell {
    pub variant: String,
    pub label: String,
    pub cell: Cell,
    /// What the comparable predecessor (the promoted raw) said.
    pub prev_solved: Option<bool>,
    pub prev_secs: Option<f64>,
    /// What this sweep's latest row says, once it has one.
    pub this_solved: Option<bool>,
    pub this_secs: Option<f64>,
    pub rho: Option<f64>,
    pub verdict: Option<String>,
    pub attempt: u32,
}

impl InstanceCell {
    /// Solved on the predecessor, banked unsolved here. The loud case.
    pub fn regression(&self) -> bool {
        self.prev_solved == Some(true) && self.this_solved == Some(false) && self.cell.banked()
    }

    pub fn gain(&self) -> bool {
        self.prev_solved == Some(false) && self.this_solved == Some(true)
    }

    pub fn delta_secs(&self) -> Option<f64> {
        Some(self.this_secs? - self.prev_secs?)
    }
}

#[derive(Debug, Clone, Default)]
pub struct BoardRow {
    pub id: String,
    pub label: String,
    pub budget_secs: u64,
    pub threads: u32,
    pub cells: Vec<InstanceCell>,
}

impl BoardRow {
    pub fn total(&self) -> usize {
        self.cells.len()
    }
    pub fn banked(&self) -> usize {
        self.cells.iter().filter(|c| c.cell.banked()).count()
    }
    pub fn owed(&self) -> usize {
        self.total() - self.banked()
    }
    pub fn solved(&self) -> usize {
        self.cells
            .iter()
            .filter(|c| matches!(c.cell, Cell::SolvedClean | Cell::SolvedDirty))
            .count()
    }
    pub fn prev_solved(&self) -> Option<usize> {
        if self.cells.iter().all(|c| c.prev_solved.is_none()) {
            None
        } else {
            Some(
                self.cells
                    .iter()
                    .filter(|c| c.prev_solved == Some(true))
                    .count(),
            )
        }
    }
    pub fn regressions(&self) -> usize {
        self.cells.iter().filter(|c| c.regression()).count()
    }
    pub fn gains(&self) -> usize {
        self.cells.iter().filter(|c| c.gain()).count()
    }
    pub fn running(&self) -> bool {
        self.cells.iter().any(|c| c.cell == Cell::Running)
    }
    pub fn done(&self) -> bool {
        self.owed() == 0
    }
    /// Solve times, for the histogram against the wall.
    pub fn solve_secs(&self) -> Vec<f64> {
        self.cells
            .iter()
            .filter(|c| c.this_solved == Some(true))
            .filter_map(|c| c.this_secs)
            .collect()
    }
    /// Solves at or past three quarters of the budget: where the flips live.
    pub fn near_wall(&self) -> usize {
        let line = self.budget_secs as f64 * 0.75;
        self.solve_secs().iter().filter(|s| **s >= line).count()
    }
}

/// One column of a strip: the worst cell among the instances it stands for,
/// and whether any of them regressed or gained.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StripCol {
    pub cell: Cell,
    pub regression: bool,
    pub gain: bool,
    pub count: usize,
}

/// Fold a board's cells into at most `width` columns. One instance per
/// column when it fits; otherwise each column stands for a run of
/// neighbouring instances: anything that needs attention (owed, running, an
/// error) wins the column, and a settled column shows the MAJORITY of what
/// it holds -- a banked timeout is an ordinary result, not a warning, and a
/// board that is 40 % timeouts must not read as a wall of them.
pub fn strip(cells: &[InstanceCell], width: usize) -> Vec<StripCol> {
    if cells.is_empty() || width == 0 {
        return Vec::new();
    }
    let per = cells.len().div_ceil(width).max(1);
    cells
        .chunks(per)
        .map(|chunk| {
            let worst = chunk
                .iter()
                .map(|c| c.cell)
                .max_by_key(|c| c.rank())
                .unwrap_or_default();
            let cell = if worst.rank() >= Cell::Error.rank() {
                worst
            } else {
                let count = |f: fn(Cell) -> bool| chunk.iter().filter(|c| f(c.cell)).count();
                let solved = count(|c| matches!(c, Cell::SolvedClean | Cell::SolvedDirty));
                let timeouts = count(|c| c == Cell::TimeoutBanked);
                let queued = count(|c| c == Cell::Queued);
                if queued > solved + timeouts {
                    Cell::Queued
                } else if timeouts > solved {
                    Cell::TimeoutBanked
                } else if solved > 0 && chunk.iter().any(|c| c.cell == Cell::SolvedDirty) {
                    // Dirty timing shows through when the column is solved.
                    Cell::SolvedDirty
                } else {
                    Cell::SolvedClean
                }
            };
            StripCol {
                cell,
                regression: chunk.iter().any(|c| c.regression()),
                gain: chunk.iter().any(|c| c.gain()),
                count: chunk.len(),
            }
        })
        .collect()
}

/// A run in flight, or a slot with nothing in it.
#[derive(Debug, Clone)]
pub struct Slot {
    pub index: usize,
    pub what: Option<SlotRun>,
}

#[derive(Debug, Clone)]
pub struct SlotRun {
    pub board: String,
    pub variant: String,
    pub instance: String,
    /// Wall minus suspension. What the deadline is compared against, and the
    /// number that explains why a stopped run is not about to time out.
    pub effective: Duration,
    pub suspended: bool,
    pub budget: Duration,
    /// Live: the process's CPU share of its effective wall so far.
    pub rho: Option<f64>,
    pub rss_mb: Option<f64>,
    pub last_stderr: Option<String>,
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum View {
    #[default]
    Grid,
    Board,
    Instance,
    Timeline,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Sort {
    #[default]
    Corpus,
    Time,
    Delta,
    Rho,
    State,
}

impl Sort {
    pub fn next(self) -> Sort {
        match self {
            Sort::Corpus => Sort::Time,
            Sort::Time => Sort::Delta,
            Sort::Delta => Sort::Rho,
            Sort::Rho => Sort::State,
            Sort::State => Sort::Corpus,
        }
    }
    pub fn label(self) -> &'static str {
        match self {
            Sort::Corpus => "corpus order",
            Sort::Time => "time",
            Sort::Delta => "delta vs prev",
            Sort::Rho => "rho",
            Sort::State => "state",
        }
    }
}

/// One watcher sample, for the timeline.
#[derive(Debug, Clone, Default)]
pub struct TimelinePoint {
    pub at: f64,
    pub foreign: Option<f64>,
    pub canary: Option<f64>,
    pub swap_mb: Option<f64>,
    pub mem_pressure: Option<u32>,
}

#[derive(Debug, Clone, Default)]
pub struct Timeline {
    pub points: Vec<TimelinePoint>,
    /// `(started_at, ended_at, level)` throttle windows.
    pub windows: Vec<(f64, Option<f64>, String)>,
    /// `(started_at, finished_at, banked)` runs across the span.
    pub runs: Vec<(f64, f64, bool)>,
}

/// One attempt of the selected instance, as the database has it.
#[derive(Debug, Clone, Default)]
pub struct AttemptRow {
    pub attempt: u32,
    pub solved: bool,
    pub secs: Option<f64>,
    pub wall_ms: Option<u64>,
    pub cpu_ms: Option<u64>,
    pub suspended_ms: Option<u64>,
    pub peak_rss: Option<u64>,
    pub timing: String,
    pub verdict: Option<String>,
    pub started_at: Option<f64>,
    pub finished_at: Option<f64>,
}

impl AttemptRow {
    pub fn rho(&self) -> Option<f64> {
        let eff = self.wall_ms?.saturating_sub(self.suspended_ms.unwrap_or(0));
        if eff == 0 {
            return None;
        }
        Some(self.cpu_ms? as f64 / eff as f64)
    }
}

#[derive(Debug, Clone, Default)]
pub struct InstanceDetail {
    pub attempts: Vec<AttemptRow>,
    /// Competitors across the latest attempt's window, busiest first.
    pub competitors: Vec<(String, f64)>,
    pub canary_max: Option<f64>,
    pub swap_growth_mb: Option<f64>,
    pub timeline: Timeline,
}

#[derive(Debug, Clone, Default)]
pub struct Snapshot {
    pub engine_ver: String,
    pub engine_hash: String,
    pub level: LevelState,
    pub uptime: Duration,
    pub quiet_in: Option<Duration>,
    pub sweep: SweepProgress,
    pub boards: Vec<BoardRow>,
    pub slots: Vec<Slot>,
    pub p_cores: u32,
    pub log: Vec<LogLine>,
    pub toasts: Vec<Toast>,
    /// Runs per minute, most recent last. Drives the throughput sparkline --
    /// the one line that makes the dashboard worth leaving on screen, because
    /// it shows a demotion happening rather than merely reporting it.
    pub throughput: Vec<f64>,
    /// The canary's latest clock factor.
    pub canary: Option<f64>,
    /// The top competitors right now, busiest first.
    pub competitors: Vec<(String, f64)>,
    pub view: View,
    pub sel_board: usize,
    pub sel_inst: usize,
    pub sort: Sort,
    /// Filled by the UI thread for the selected instance, on demand.
    pub detail: Option<InstanceDetail>,
    pub timeline: Timeline,
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
    /// Instances with a banked row.
    pub done: usize,
    pub total: usize,
    pub solved: usize,
    pub delta: Option<i64>,
    pub delta_vs: String,
    pub regressions: usize,
    /// Rows measured and OWED again. They are KEPT -- nothing is discarded --
    /// but the instance is not banked until the referee accepts a row.
    pub owed: usize,
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
    /// The selected board, if any.
    pub fn board(&self) -> Option<&BoardRow> {
        self.boards.get(self.sel_board)
    }

    /// The board view's instance order under the current sort.
    pub fn sorted_instances(&self) -> Vec<usize> {
        let Some(b) = self.board() else {
            return Vec::new();
        };
        let mut idx: Vec<usize> = (0..b.cells.len()).collect();
        let key = |i: usize| -> (u8, f64) {
            let c = &b.cells[i];
            match self.sort {
                Sort::Corpus => (0, i as f64),
                Sort::Time => (0, -c.this_secs.unwrap_or(-1.0)),
                Sort::Delta => (0, -c.delta_secs().unwrap_or(f64::NEG_INFINITY)),
                Sort::Rho => (0, c.rho.unwrap_or(2.0)),
                Sort::State => (255 - c.cell.rank(), i as f64),
            }
        };
        idx.sort_by(|&a, &c| {
            let (ka, kb) = (key(a), key(c));
            ka.0.cmp(&kb.0)
                .then(ka.1.partial_cmp(&kb.1).unwrap_or(std::cmp::Ordering::Equal))
        });
        idx
    }

    /// The instance the cursor is on in the board view.
    pub fn selected_instance(&self) -> Option<&InstanceCell> {
        let order = self.sorted_instances();
        let i = *order.get(self.sel_inst)?;
        self.board()?.cells.get(i)
    }

    pub fn move_selection(&mut self, delta: isize) {
        match self.view {
            View::Grid | View::Timeline => {
                let n = self.boards.len();
                if n > 0 {
                    self.sel_board =
                        (self.sel_board as isize + delta).clamp(0, n as isize - 1) as usize;
                }
            }
            View::Board | View::Instance => {
                let n = self.board().map_or(0, |b| b.cells.len());
                if n > 0 {
                    self.sel_inst =
                        (self.sel_inst as isize + delta).clamp(0, n as isize - 1) as usize;
                    // Moving in the instance view moves to another instance's
                    // detail, which the UI thread refills on the next tick.
                    if self.view == View::Instance {
                        self.detail = None;
                    }
                }
            }
        }
    }

    pub fn jump(&mut self, to_end: bool) {
        match self.view {
            View::Grid | View::Timeline => {
                self.sel_board = if to_end {
                    self.boards.len().saturating_sub(1)
                } else {
                    0
                };
            }
            View::Board | View::Instance => {
                self.sel_inst = if to_end {
                    self.board().map_or(0, |b| b.cells.len().saturating_sub(1))
                } else {
                    0
                };
                self.detail = None;
            }
        }
    }

    /// Enter: one level deeper.
    pub fn enter(&mut self) {
        match self.view {
            View::Grid | View::Timeline => {
                if self.board().is_some() {
                    self.view = View::Board;
                    self.sel_inst = self
                        .sel_inst
                        .min(self.board().map_or(0, |b| b.cells.len().saturating_sub(1)));
                }
            }
            View::Board => {
                if self.selected_instance().is_some() {
                    self.view = View::Instance;
                    self.detail = None;
                }
            }
            View::Instance => {}
        }
    }

    /// Esc / b: one level up.
    pub fn back(&mut self) {
        self.view = match self.view {
            View::Instance => View::Board,
            View::Board | View::Timeline => View::Grid,
            View::Grid => View::Grid,
        };
    }

    pub fn toggle_timeline(&mut self) {
        self.view = if self.view == View::Timeline {
            View::Grid
        } else {
            View::Timeline
        };
    }

    pub fn cycle_sort(&mut self) {
        if matches!(self.view, View::Board | View::Instance) {
            self.sort = self.sort.next();
            self.sel_inst = 0;
        }
    }

    /// Toasts dwell for four seconds; a regression's does not expire at all.
    pub fn expire_toasts(&mut self, dwell: Duration) {
        self.toasts.retain(|t| t.sticky || t.age < dwell);
    }

    pub fn dismiss_toasts(&mut self) {
        self.toasts.clear();
    }

    /// Recompute the sweep totals from the boards. The feed publishes boards;
    /// the totals are a view on them, so they cannot disagree.
    pub fn tally(&mut self) {
        let total = self.boards.iter().map(|b| b.total()).sum();
        let done = self.boards.iter().map(|b| b.banked()).sum();
        let solved = self.boards.iter().map(|b| b.solved()).sum::<usize>();
        let regressions = self.boards.iter().map(|b| b.regressions()).sum();
        let prev: Option<usize> = self
            .boards
            .iter()
            .map(|b| b.prev_solved())
            .try_fold(0usize, |acc, p| p.map(|p| acc + p));
        self.sweep.total = total;
        self.sweep.done = done;
        self.sweep.solved = solved;
        self.sweep.regressions = regressions;
        self.sweep.owed = total - done;
        self.sweep.delta = prev.map(|p| solved as i64 - p as i64);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cell(c: Cell, prev: Option<bool>, this: Option<bool>) -> InstanceCell {
        InstanceCell {
            cell: c,
            prev_solved: prev,
            this_solved: this,
            ..Default::default()
        }
    }

    fn board(id: &str, cells: Vec<InstanceCell>) -> BoardRow {
        BoardRow {
            id: id.into(),
            label: id.into(),
            budget_secs: 60,
            threads: 1,
            cells,
        }
    }

    /// Attention wins the column: an owed instance among nine solved ones
    /// is what the operator needs to see. A settled column shows its
    /// majority, so one banked timeout among four solves reads as solved.
    #[test]
    fn a_strip_column_shows_attention_first_and_the_majority_otherwise() {
        let mut cells: Vec<InstanceCell> = (0..10)
            .map(|_| cell(Cell::SolvedClean, None, None))
            .collect();
        cells[7].cell = Cell::Owed;
        cells[1].cell = Cell::TimeoutBanked;
        let s = strip(&cells, 2);
        assert_eq!(s.len(), 2);
        assert_eq!(s[0].cell, Cell::SolvedClean, "four solves, one timeout");
        assert_eq!(s[1].cell, Cell::Owed);
        assert_eq!(s[0].count + s[1].count, 10);
        cells[0].cell = Cell::TimeoutBanked;
        cells[2].cell = Cell::TimeoutBanked;
        assert_eq!(
            strip(&cells, 2)[0].cell,
            Cell::TimeoutBanked,
            "three of five"
        );
        // One per column when it fits.
        assert_eq!(strip(&cells, 40).len(), 10);
        assert!(strip(&cells, 0).is_empty());
    }

    #[test]
    fn a_regression_is_solved_before_and_banked_unsolved_now() {
        let c = cell(Cell::TimeoutBanked, Some(true), Some(false));
        assert!(c.regression());
        let owed = cell(Cell::Owed, Some(true), Some(false));
        assert!(!owed.regression(), "an owed row is not a verdict yet");
        let gain = cell(Cell::SolvedClean, Some(false), Some(true));
        assert!(gain.gain() && !gain.regression());
        assert!(strip(std::slice::from_ref(&c), 1)[0].regression);
    }

    #[test]
    fn the_tally_is_a_view_on_the_boards() {
        let mut s = Snapshot {
            boards: vec![
                board(
                    "a",
                    vec![
                        cell(Cell::SolvedClean, Some(true), Some(true)),
                        cell(Cell::Owed, Some(false), Some(false)),
                        cell(Cell::TimeoutBanked, Some(true), Some(false)),
                    ],
                ),
                board("b", vec![cell(Cell::Queued, Some(true), None)]),
            ],
            ..Default::default()
        };
        s.tally();
        assert_eq!((s.sweep.total, s.sweep.done, s.sweep.owed), (4, 2, 2));
        assert_eq!(s.sweep.solved, 1);
        assert_eq!(s.sweep.regressions, 1);
        assert_eq!(s.sweep.delta, Some(1 - 3));
    }

    #[test]
    fn navigation_drills_and_climbs_and_never_leaves_the_lists() {
        let mut s = Snapshot {
            boards: vec![
                board("a", vec![cell(Cell::Queued, None, None); 3]),
                board("b", vec![cell(Cell::Queued, None, None); 2]),
            ],
            ..Default::default()
        };
        s.move_selection(-5);
        assert_eq!(s.sel_board, 0);
        s.move_selection(99);
        assert_eq!(s.sel_board, 1);
        s.enter();
        assert_eq!(s.view, View::Board);
        s.move_selection(99);
        assert_eq!(s.sel_inst, 1, "board b has two instances");
        s.enter();
        assert_eq!(s.view, View::Instance);
        s.enter();
        assert_eq!(s.view, View::Instance, "nothing deeper than an instance");
        s.back();
        s.back();
        assert_eq!(s.view, View::Grid);
        s.toggle_timeline();
        assert_eq!(s.view, View::Timeline);
        s.toggle_timeline();
        assert_eq!(s.view, View::Grid);
        let mut empty = Snapshot::default();
        empty.move_selection(1);
        empty.enter();
        assert_eq!(empty.view, View::Grid, "no boards, nothing to enter");
    }

    #[test]
    fn sorting_orders_the_board_view() {
        let mut s = Snapshot {
            boards: vec![board(
                "a",
                vec![
                    InstanceCell {
                        this_secs: Some(5.0),
                        rho: Some(0.99),
                        ..cell(Cell::SolvedClean, None, Some(true))
                    },
                    InstanceCell {
                        this_secs: Some(50.0),
                        rho: Some(0.5),
                        ..cell(Cell::Owed, None, Some(false))
                    },
                ],
            )],
            view: View::Board,
            ..Default::default()
        };
        assert_eq!(s.sorted_instances(), vec![0, 1]);
        s.cycle_sort();
        assert_eq!(s.sort, Sort::Time);
        assert_eq!(s.sorted_instances(), vec![1, 0], "slowest first");
        s.sort = Sort::Rho;
        assert_eq!(s.sorted_instances(), vec![1, 0], "most starved first");
        s.sort = Sort::State;
        assert_eq!(s.sorted_instances(), vec![1, 0], "owed before solved");
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
        assert_eq!(AttemptRow::default().rho(), None);
    }
}
