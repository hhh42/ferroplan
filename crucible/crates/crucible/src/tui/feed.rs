//! From what the sweep publishes to what the screen shows.
//!
//! Runs on the UI thread. Reads the [`Progress`] the runner writes, the
//! [`Shared`] throttle state the watcher writes, and -- for the drill-downs
//! only -- the database, through its own reader on its own connection, so a
//! slow query here can never hold a lock the runner wants.

use super::app::*;
use crate::sweep::{Progress, Shared};
use crucible_core::db::Reader;
use crucible_core::monitor::Level as CoreLevel;
use crucible_core::platform::{self, Platform};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

pub struct Feed {
    pub progress: Arc<Mutex<Progress>>,
    pub shared: Arc<Shared>,
    reader: Option<Reader>,
    plat: platform::Host,
    /// The instance the cached detail is for: `(board, variant, label)`.
    detail_key: Option<(usize, String, String)>,
    detail: Option<InstanceDetail>,
    timeline_at: Option<Instant>,
    timeline: Timeline,
    competitors_at: Option<Instant>,
    competitors: Vec<(String, f64)>,
    p_cores: u32,
}

fn now_epoch() -> f64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs_f64())
        .unwrap_or(0.0)
}

impl Feed {
    pub fn new(progress: Arc<Mutex<Progress>>, shared: Arc<Shared>) -> Feed {
        let plat = platform::host();
        let p_cores = plat.topology().p_cores.max(1);
        Feed {
            progress,
            shared,
            reader: None,
            plat,
            detail_key: None,
            detail: None,
            timeline_at: None,
            timeline: Timeline::default(),
            competitors_at: None,
            competitors: Vec::new(),
            p_cores,
        }
    }

    fn reader(&mut self) -> Option<&Reader> {
        if self.reader.is_none() {
            let path = self.progress.lock().unwrap().db_path.clone()?;
            self.reader = Reader::open(&path).ok();
        }
        self.reader.as_ref()
    }

    /// One snapshot, given the previous one (for the cursor and the view).
    pub fn next(&mut self, prev: &Snapshot) -> Option<Snapshot> {
        let (boards, running, engine_ver, engine_hash, started, banked_at, finished, ids) = {
            let g = self.progress.lock().unwrap();
            (
                g.boards.clone(),
                g.running
                    .iter()
                    .map(|r| (r.slot, r.board, r.inst, r.pid, r.started, r.budget))
                    .collect::<Vec<_>>(),
                g.engine_ver.clone(),
                g.engine_hash.clone(),
                g.started,
                g.banked_at.clone(),
                g.finished,
                g.ids.clone(),
            )
        };
        if finished {
            return None;
        }
        let level = self.shared.level();
        let reason = self.shared.reason();
        let suspended = level == CoreLevel::Suspended;

        // The slots: every run in flight, with what the kernel says about it.
        let mut slots: Vec<Slot> = running
            .into_iter()
            .map(|(slot, b, i, pid, at, budget)| {
                let (variant, instance) = boards
                    .get(b)
                    .and_then(|br| br.cells.get(i))
                    .map(|c| (c.variant.clone(), c.label.clone()))
                    .unwrap_or_default();
                let effective = at.elapsed();
                let cpu_ms = pid.and_then(|p| self.plat.cpu_ms(p));
                let rss = pid.and_then(|p| self.plat.rss_bytes(p));
                Slot {
                    index: slot,
                    what: Some(SlotRun {
                        board: boards.get(b).map(|br| br.id.clone()).unwrap_or_default(),
                        variant,
                        instance,
                        effective,
                        suspended,
                        budget,
                        rho: cpu_ms
                            .filter(|_| effective.as_millis() > 500)
                            .map(|c| c as f64 / effective.as_millis().max(1) as f64),
                        rss_mb: rss.map(|r| r as f64 / 1048576.0),
                        last_stderr: None,
                    }),
                }
            })
            .collect();
        slots.sort_by_key(|s| s.index);
        if slots.is_empty() {
            slots.push(Slot {
                index: 0,
                what: None,
            });
        }

        // Throughput: banked per minute over the last half hour, in 2-minute
        // buckets, most recent last.
        let mut throughput = Vec::new();
        let now = Instant::now();
        for k in (0..15).rev() {
            let hi = now - Duration::from_secs(k * 120);
            let lo = hi - Duration::from_secs(120);
            let n = banked_at.iter().filter(|t| **t > lo && **t <= hi).count();
            throughput.push(n as f64 / 2.0);
        }
        let rate_per_min = banked_at
            .iter()
            .filter(|t| now.duration_since(**t) < Duration::from_secs(1800))
            .count() as f64
            / 30.0;

        // Refresh the top competitors every few seconds from the watcher.
        if self
            .competitors_at
            .map_or(true, |t| t.elapsed() > Duration::from_secs(10))
        {
            let now_ts = now_epoch();
            if let Some(r) = self.reader() {
                self.competitors = r
                    .competitors_between(now_ts - 45.0, now_ts)
                    .unwrap_or_default()
                    .into_iter()
                    .map(|(n, total)| (n, total / 2.0))
                    .collect();
            }
            self.competitors_at = Some(now);
        }

        // The whole-sweep timeline, refreshed every 30 s when it is on screen.
        if prev.view == View::Timeline
            && self
                .timeline_at
                .map_or(true, |t| t.elapsed() > Duration::from_secs(30))
        {
            let span_start = started
                .map(|s| now_epoch() - s.elapsed().as_secs_f64())
                .unwrap_or(0.0);
            let engine_id = ids.iter().flatten().next().map(|(_, e)| *e);
            self.timeline = self.timeline_between(span_start, now_epoch(), engine_id);
            self.timeline_at = Some(now);
        }

        // The selected instance's detail, on demand and cached by key.
        let mut detail = None;
        if prev.view == View::Instance {
            if let Some(b) = boards.get(prev.sel_board) {
                let order = {
                    let tmp = Snapshot {
                        boards: boards.clone(),
                        sel_board: prev.sel_board,
                        sort: prev.sort,
                        ..Default::default()
                    };
                    tmp.sorted_instances()
                };
                if let Some(c) = order.get(prev.sel_inst).and_then(|i| b.cells.get(*i)) {
                    let key = (prev.sel_board, c.variant.clone(), c.label.clone());
                    if self.detail_key.as_ref() != Some(&key) || c.cell == Cell::Running {
                        self.detail =
                            self.read_detail(&key, ids.get(prev.sel_board).copied().flatten());
                        self.detail_key = Some(key);
                    }
                    detail = self.detail.clone();
                }
            }
        }

        let mut s = Snapshot {
            engine_ver,
            engine_hash,
            level: LevelState {
                level: match level {
                    CoreLevel::Full => Level::Full,
                    CoreLevel::Polite => Level::Polite,
                    CoreLevel::Suspended => Level::Suspended,
                },
                reason,
            },
            uptime: started.map(|s| s.elapsed()).unwrap_or_default(),
            quiet_in: None,
            sweep: SweepProgress {
                delta_vs: "promoted".into(),
                ..Default::default()
            },
            boards,
            slots,
            p_cores: self.p_cores,
            log: crate::out::recent(200)
                .into_iter()
                .map(|(at, text)| LogLine {
                    kind: if text.contains("REGRESSION") {
                        LogKind::Regression
                    } else if text.starts_with("!!") || text.contains("DEGRADED") {
                        LogKind::Warn
                    } else if text.contains("banked") || text.contains("SWEEP COMPLETE") {
                        LogKind::Good
                    } else {
                        LogKind::Info
                    },
                    at,
                    text,
                })
                .collect(),
            toasts: Vec::new(),
            throughput,
            canary: self.shared.canary(),
            competitors: self.competitors.clone(),
            view: prev.view,
            sel_board: prev.sel_board,
            sel_inst: prev.sel_inst,
            sort: prev.sort,
            detail,
            timeline: self.timeline.clone(),
        };
        s.tally();
        if rate_per_min > 0.0 {
            s.sweep.eta = Some(Duration::from_secs_f64(
                s.sweep.owed as f64 / rate_per_min * 60.0,
            ));
        }
        Some(s)
    }

    fn timeline_between(&mut self, start: f64, end: f64, engine_id: Option<i64>) -> Timeline {
        let Some(r) = self.reader() else {
            return Timeline::default();
        };
        Timeline {
            points: r
                .samples_between(start, end)
                .unwrap_or_default()
                .into_iter()
                .map(|p| TimelinePoint {
                    at: p.at,
                    foreign: p.foreign,
                    canary: p.canary,
                    swap_mb: p.swap_mb,
                    mem_pressure: p.mem_pressure,
                })
                .collect(),
            windows: r.throttle_windows_between(start, end).unwrap_or_default(),
            runs: engine_id
                .map(|e| r.runs_between(e, start, end).unwrap_or_default())
                .unwrap_or_default(),
        }
    }

    fn read_detail(
        &mut self,
        key: &(usize, String, String),
        ids: Option<(i64, i64)>,
    ) -> Option<InstanceDetail> {
        let (bid, eid) = ids?;
        let (_, variant, label) = key;
        let attempts: Vec<AttemptRow> = {
            let r = self.reader()?;
            r.attempts_for(bid, eid, variant, label)
                .unwrap_or_default()
                .into_iter()
                .map(|a| AttemptRow {
                    attempt: a.attempt,
                    solved: a.solved,
                    secs: a.secs,
                    wall_ms: a.wall_ms,
                    cpu_ms: a.cpu_ms,
                    suspended_ms: a.suspended_ms,
                    peak_rss: a.peak_rss,
                    timing: a.timing,
                    verdict: a.verdict,
                    started_at: a.started_at,
                    finished_at: a.finished_at,
                })
                .collect()
        };
        let window = attempts
            .last()
            .and_then(|a| Some((a.started_at?, a.finished_at?)));
        let (competitors, canary_max, swap_growth_mb, timeline) = match window {
            Some((st, en)) => {
                let (c, cm, sg) = {
                    let r = self.reader()?;
                    (
                        r.competitors_between(st, en).unwrap_or_default(),
                        r.canary_max_between(st, en).ok().flatten(),
                        r.swap_growth_between(st, en).ok().flatten(),
                    )
                };
                let samples = ((en - st) / 20.0).max(1.0);
                let c = c.into_iter().map(|(n, t)| (n, t / samples)).collect();
                (
                    c,
                    cm,
                    sg,
                    self.timeline_between(st - 60.0, en + 60.0, Some(eid)),
                )
            }
            None => (Vec::new(), None, None, Timeline::default()),
        };
        Some(InstanceDetail {
            attempts,
            competitors,
            canary_max,
            swap_growth_mb,
            timeline,
        })
    }
}
