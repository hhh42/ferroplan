//! Read-only access, on a connection that is physically forbidden to write.
//!
//! `PRAGMA query_only=ON` rather than `SQLITE_OPEN_READONLY`, and the
//! difference matters: a read-only connection cannot recover a hot WAL, so
//! opening one against a database whose writer just died gives a stale or
//! failed read exactly when the answer matters most. The connection is opened
//! read-write so SQLite can do its recovery, and is then forbidden to write --
//! which is a guarantee about this handle, not about the file.
//!
//! # Export order
//!
//! `export_rows` orders by `variant.ipc, variant.name, instance.sort_key`, and
//! that order is a promise, not a convenience. `ipc67.py` writes a board in
//! corpus order -- IPC directory, then `sorted(os.listdir())` over variant
//! directories, then instances by numeric tuple -- while the crucible scheduler
//! reorders execution freely (tiering, retries, the clean-timing pass). Without
//! a canonical export order a crucible-produced raw could not be diffed against
//! a Python-produced one at all, and "the numbers agree" would have to be taken
//! on trust.
//!
//! Two details make that ORDER BY equal the Python's:
//!
//! * every `ipcs` list in `benchmarks/manifest.toml` is already in ascending
//!   order, so ordering by the ipc STRING agrees with iterating the manifest's
//!   list; and
//! * `sorted()` on `str` is codepoint order, which is byte order in UTF-8, so
//!   SQLite's default BINARY collation on variant names agrees with Python's.
//!
//! # `val` is a tristate
//!
//! Every query in this file that touches `val` uses `IS NULL` for unavailable
//! and `= 0` only for a genuine rejection. NULL is not a verdict. Reading it as
//! one is the 0.20, 0.21 and 0.23 incidents.

use super::model::*;
use super::DbError;
use crucible_publish::raw::{Notes, Present, RawRow};
use rusqlite::{params, Connection, OpenFlags, OptionalExtension};
use std::path::Path;

/// A read-only handle. Each one owns its own connection, which is what lets a
/// renderer, the TUI and a `crucible db export` all read while a sweep writes.
pub struct Reader {
    conn: Connection,
}

impl Reader {
    /// Open `path` for reading. The file must already exist: a reader that
    /// creates an empty database answers every question with "nothing here",
    /// which is indistinguishable from a genuinely empty sweep and far worse
    /// than an error.
    pub fn open(path: &Path) -> Result<Reader, DbError> {
        let conn = Connection::open_with_flags(
            path,
            OpenFlags::SQLITE_OPEN_READ_WRITE | OpenFlags::SQLITE_OPEN_NO_MUTEX,
        )?;
        conn.execute_batch("PRAGMA foreign_keys=ON; PRAGMA query_only=ON;")?;
        Ok(Reader { conn })
    }

    /// Escape hatch for callers that need a query this module does not offer.
    /// Still `query_only`.
    pub fn conn(&self) -> &Connection {
        &self.conn
    }

    /// Every board row carrying this name, oldest first. More than one means
    /// the board was measured under more than one identity -- a tier move, a
    /// mode change -- and the caller has to say which one it means.
    pub fn boards_named(&self, name: &str) -> Result<Vec<i64>, DbError> {
        let mut st = self
            .conn
            .prepare("SELECT id FROM board WHERE name = ?1 ORDER BY id")?;
        let rows = st.query_map(params![name], |r| r.get::<_, i64>(0))?;
        Ok(rows.collect::<Result<Vec<_>, _>>()?)
    }

    /// The engines that have contributed rows to this board, oldest first.
    pub fn engines_for_board(&self, board_id: i64) -> Result<Vec<i64>, DbError> {
        let mut st = self
            .conn
            .prepare("SELECT DISTINCT engine_id FROM run WHERE board_id = ?1 ORDER BY engine_id")?;
        let rows = st.query_map(params![board_id], |r| r.get::<_, i64>(0))?;
        Ok(rows.collect::<Result<Vec<_>, _>>()?)
    }

    /// One board's rows, in the canonical order, ready for `write_row`.
    ///
    /// Only the highest DONE attempt per instance is returned, and both halves
    /// of that are load-bearing:
    ///
    /// * a retried instance contributes one row to the raw, the same way
    ///   `ipc67.py`'s in-process retry does; and
    /// * a row that has not finished is not a row. `ipc67.py` gets this for
    ///   free -- `rec` reaches the raw only after `run_instance` returns, so an
    ///   instance still running, queued, or abandoned by a killed runner is
    ///   simply absent from the file. Here those rows exist in the table with
    ///   `solved = 0`, and exporting one would publish an in-flight instance as
    ///   a failure.
    ///
    /// The `done` test is inside the MAX(attempt) subquery as well as outside
    /// it. Filtering only the outer query would let a `running` attempt 2 mask
    /// a finished attempt 1 and drop the instance from the board entirely,
    /// which is worse than the bug it was fixing: a shrunken denominator is a
    /// wrong published number, and nothing in the output would say so.
    pub fn export_rows(&self, board_id: i64, engine_id: i64) -> Result<Vec<RawRow>, DbError> {
        let mut st = self.conn.prepare(
            "SELECT v.ipc, v.name, i.label, i.label_is_int,
                    r.solved, r.time_json, r.metric, r.length, r.val, r.notes_json,
                    r.budget_secs, r.ver, r.mode, r.jobs, r.threads_json,
                    r.start_ts, r.end_ts, r.makespan, r.resumed_clean,
                    r.present_ipc, r.present_budget, r.present_stamps,
                    r.present_makespan, r.present_resumed_clean, r.extra_json
               FROM run r
               JOIN instance i ON i.id = r.instance_id
               JOIN variant  v ON v.id = i.variant_id
              WHERE r.board_id = ?1 AND r.engine_id = ?2
                AND r.state = 'done'
                AND r.attempt = (SELECT MAX(r2.attempt) FROM run r2
                                  WHERE r2.board_id = r.board_id
                                    AND r2.instance_id = r.instance_id
                                    AND r2.engine_id = r.engine_id
                                    AND r2.state = 'done')
              ORDER BY v.ipc, v.name, i.sort_key",
        )?;
        let rows = st.query_map(params![board_id, engine_id], |row| {
            let label: String = row.get(2)?;
            let label_is_int: bool = row.get(3)?;
            let time_json: Option<String> = row.get(5)?;
            let notes_json: Option<String> = row.get(9)?;
            let threads_json: Option<String> = row.get(14)?;
            let extra_json: Option<String> = row.get(24)?;
            Ok(RawRow {
                ipc: row.get(0)?,
                variant: row.get(1)?,
                instance: InstanceKey {
                    label,
                    label_is_int,
                }
                .to_instance(),
                solved: row.get(4)?,
                time: time_json.as_deref().and_then(parse_number),
                metric: row.get(6)?,
                length: row.get::<_, Option<i64>>(7)?.map(|v| v as u64),
                val: row.get::<_, Option<i64>>(8)?.map(|v| v != 0),
                notes: notes_json.as_deref().and_then(parse_notes),
                budget: row.get(10)?,
                ver: row.get(11)?,
                mode: row.get(12)?,
                jobs: row.get::<_, Option<i64>>(13)?.map(|v| v as u32),
                threads: threads_json.as_deref().and_then(parse_value),
                start_ts: row.get(15)?,
                end_ts: row.get(16)?,
                makespan: row.get(17)?,
                resumed_clean: row.get(18)?,
                extra: extra_json
                    .as_deref()
                    .and_then(parse_object)
                    .unwrap_or_default(),
                present: Present {
                    ipc: row.get(19)?,
                    budget: row.get(20)?,
                    stamps: row.get(21)?,
                    makespan: row.get(22)?,
                    resumed_clean: row.get(23)?,
                },
            })
        })?;
        Ok(rows.collect::<Result<Vec<_>, _>>()?)
    }

    /// The instances of this board, under this engine, whose latest DONE
    /// attempt banked a CLEAN timing -- the set a restarted sweep owes nothing
    /// for. Keys are `(ipc, variant, label)`, the row's own address.
    ///
    /// Same latest-done-attempt rule as [`Reader::export_rows`], for the same
    /// reason: a clean attempt 1 followed by a dirty attempt 2 is an instance
    /// that was clean and then re-measured under load, and the LATER verdict
    /// is the one in force. Anything else lets a stale clean row bank an
    /// instance the runner deliberately re-opened.
    pub fn clean_instances(
        &self,
        board_id: i64,
        engine_id: i64,
    ) -> Result<Vec<(Option<String>, String, String)>, DbError> {
        let mut st = self.conn.prepare(
            "SELECT v.ipc, v.name, i.label
               FROM run r
               JOIN instance i ON i.id = r.instance_id
               JOIN variant  v ON v.id = i.variant_id
              WHERE r.board_id = ?1 AND r.engine_id = ?2
                AND r.state = 'done' AND r.timing_quality = 'clean'
                AND r.attempt = (SELECT MAX(r2.attempt) FROM run r2
                                  WHERE r2.board_id = r.board_id
                                    AND r2.instance_id = r.instance_id
                                    AND r2.engine_id = r.engine_id
                                    AND r2.state = 'done')
              ORDER BY v.ipc, v.name, i.sort_key",
        )?;
        let rows = st.query_map(params![board_id, engine_id], |r| {
            Ok((r.get(0)?, r.get(1)?, r.get(2)?))
        })?;
        Ok(rows.collect::<Result<Vec<_>, _>>()?)
    }

    /// The instances whose latest done attempt BANKED under the referee --
    /// what a restart owes nothing for. Same latest-attempt rule as
    /// [`Reader::clean_instances`]; `banked` is the R2 column, and on a
    /// migrated database it carries the v3 backfill (solves and clean rows).
    pub fn banked_instances(
        &self,
        board_id: i64,
        engine_id: i64,
    ) -> Result<Vec<(Option<String>, String, String)>, DbError> {
        let mut st = self.conn.prepare(
            "SELECT v.ipc, v.name, i.label
               FROM run r
               JOIN instance i ON i.id = r.instance_id
               JOIN variant  v ON v.id = i.variant_id
              WHERE r.board_id = ?1 AND r.engine_id = ?2
                AND r.state = 'done' AND r.banked = 1
                AND r.attempt = (SELECT MAX(r2.attempt) FROM run r2
                                  WHERE r2.board_id = r.board_id
                                    AND r2.instance_id = r.instance_id
                                    AND r2.engine_id = r.engine_id
                                    AND r2.state = 'done')
              ORDER BY v.ipc, v.name, i.sort_key",
        )?;
        let rows = st.query_map(params![board_id, engine_id], |r| {
            Ok((r.get(0)?, r.get(1)?, r.get(2)?))
        })?;
        Ok(rows.collect::<Result<Vec<_>, _>>()?)
    }

    /// How much swap grew across a window: the last reading minus the first,
    /// over the live watcher's samples. `None` when no sample with a swap
    /// reading covers the window.
    pub fn swap_growth_between(&self, start_ts: f64, end_ts: f64) -> Result<Option<f64>, DbError> {
        let mut st = self.conn.prepare(
            "SELECT swap_mb FROM sample
              WHERE pass_id IS NULL AND at >= ?1 AND at <= ?2 AND swap_mb IS NOT NULL
              ORDER BY at",
        )?;
        let vals: Vec<f64> = st
            .query_map(params![start_ts, end_ts], |r| r.get(0))?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(match (vals.first(), vals.last()) {
            (Some(a), Some(b)) => Some(b - a),
            _ => None,
        })
    }

    /// The worst clock factor the canary reported across a window, from the
    /// live watcher's samples. `None` when no sample in the window carries
    /// one -- the sweep's first twenty minutes, or a pre-canary database.
    pub fn canary_max_between(&self, start_ts: f64, end_ts: f64) -> Result<Option<f64>, DbError> {
        Ok(self
            .conn
            .query_row(
                "SELECT MAX(canary_factor) FROM sample
                  WHERE pass_id IS NULL AND at >= ?1 AND at <= ?2",
                params![start_ts, end_ts],
                |r| r.get::<_, Option<f64>>(0),
            )
            .optional()?
            .flatten())
    }

    /// Every attempt of one instance under one engine, oldest first, with
    /// what the supervisor measured. The dashboard's instance view.
    pub fn attempts_for(
        &self,
        board_id: i64,
        engine_id: i64,
        variant: &str,
        label: &str,
    ) -> Result<Vec<AttemptRec>, DbError> {
        let mut st = self.conn.prepare(
            "SELECT r.attempt, r.solved, r.time_secs, r.wall_ms, r.cpu_ms, r.suspended_ms,
                    r.peak_rss, r.timing_quality, r.verdict, r.started_at, r.finished_at
               FROM run r
               JOIN instance i ON i.id = r.instance_id
               JOIN variant  v ON v.id = i.variant_id
              WHERE r.board_id = ?1 AND r.engine_id = ?2 AND v.name = ?3 AND i.label = ?4
                AND r.state = 'done'
              ORDER BY r.attempt",
        )?;
        let rows = st.query_map(params![board_id, engine_id, variant, label], |r| {
            Ok(AttemptRec {
                attempt: r.get::<_, i64>(0)? as u32,
                solved: r.get::<_, i64>(1)? != 0,
                secs: r.get(2)?,
                wall_ms: r.get::<_, Option<i64>>(3)?.map(|v| v as u64),
                cpu_ms: r.get::<_, Option<i64>>(4)?.map(|v| v as u64),
                suspended_ms: r.get::<_, Option<i64>>(5)?.map(|v| v as u64),
                peak_rss: r.get::<_, Option<i64>>(6)?.map(|v| v as u64),
                timing: r.get(7)?,
                verdict: r.get(8)?,
                started_at: r.get(9)?,
                finished_at: r.get(10)?,
            })
        })?;
        Ok(rows.collect::<Result<Vec<_>, _>>()?)
    }

    /// The live watcher's samples across a span, oldest first.
    pub fn samples_between(&self, start_ts: f64, end_ts: f64) -> Result<Vec<SamplePoint>, DbError> {
        let mut st = self.conn.prepare(
            "SELECT at, competitors_total, canary_factor, swap_mb, mem_pressure
               FROM sample
              WHERE pass_id IS NULL AND at >= ?1 AND at <= ?2
              ORDER BY at",
        )?;
        let rows = st.query_map(params![start_ts, end_ts], |r| {
            Ok(SamplePoint {
                at: r.get(0)?,
                foreign: r.get(1)?,
                canary: r.get(2)?,
                swap_mb: r.get(3)?,
                mem_pressure: r.get::<_, Option<i64>>(4)?.map(|v| v as u32),
            })
        })?;
        Ok(rows.collect::<Result<Vec<_>, _>>()?)
    }

    /// Throttle windows overlapping a span: `(started_at, ended_at, level)`.
    pub fn throttle_windows_between(
        &self,
        start_ts: f64,
        end_ts: f64,
    ) -> Result<Vec<(f64, Option<f64>, String)>, DbError> {
        let mut st = self.conn.prepare(
            "SELECT started_at, ended_at, level FROM throttle_window
              WHERE started_at <= ?2 AND (ended_at IS NULL OR ended_at >= ?1)
              ORDER BY started_at",
        )?;
        let rows = st.query_map(params![start_ts, end_ts], |r| {
            Ok((r.get(0)?, r.get(1)?, r.get(2)?))
        })?;
        Ok(rows.collect::<Result<Vec<_>, _>>()?)
    }

    /// Runs under one engine that overlap a span: `(started, finished, banked)`.
    pub fn runs_between(
        &self,
        engine_id: i64,
        start_ts: f64,
        end_ts: f64,
    ) -> Result<Vec<(f64, f64, bool)>, DbError> {
        let mut st = self.conn.prepare(
            "SELECT started_at, finished_at, banked FROM run
              WHERE engine_id = ?1 AND state = 'done'
                AND started_at IS NOT NULL AND finished_at IS NOT NULL
                AND started_at <= ?3 AND finished_at >= ?2
              ORDER BY started_at",
        )?;
        let rows = st.query_map(params![engine_id, start_ts, end_ts], |r| {
            Ok((r.get(0)?, r.get(1)?, r.get::<_, i64>(2)? != 0))
        })?;
        Ok(rows.collect::<Result<Vec<_>, _>>()?)
    }

    /// The most memory this box has ever seen the instance take, on any
    /// engine, from the supervisor's RSS watchdog. Sizes a packed batch.
    pub fn prior_peak_rss(&self, variant: &str, label: &str) -> Result<Option<u64>, DbError> {
        Ok(self
            .conn
            .query_row(
                "SELECT MAX(r.peak_rss) FROM run r
                   JOIN instance i ON i.id = r.instance_id
                   JOIN variant  v ON v.id = i.variant_id
                  WHERE v.name = ?1 AND i.label = ?2 AND r.state = 'done'",
                params![variant, label],
                |r| r.get::<_, Option<i64>>(0),
            )
            .optional()?
            .flatten()
            .map(|v| v as u64))
    }

    /// The fastest this box has ever run the canary instance, solo.
    pub fn canary_best(&self, label: &str) -> Result<Option<f64>, DbError> {
        Ok(self
            .conn
            .query_row(
                "SELECT MIN(secs) FROM canary WHERE label = ?1 AND solo = 1",
                params![label],
                |r| r.get::<_, Option<f64>>(0),
            )
            .optional()?
            .flatten())
    }

    /// The attempt number a NEW run of this instance should carry: one past
    /// the highest already recorded, in any state. `run` is UNIQUE on
    /// (board, instance, engine, attempt) and the insert upserts, so reusing a
    /// number would overwrite the receipt it is meant to follow.
    pub fn next_attempt(
        &self,
        board_id: i64,
        engine_id: i64,
        ipc: Option<&str>,
        variant: &str,
        label: &str,
    ) -> Result<i64, DbError> {
        Ok(self.conn.query_row(
            "SELECT ifnull(MAX(r.attempt), 0) + 1
               FROM run r
               JOIN instance i ON i.id = r.instance_id
               JOIN variant  v ON v.id = i.variant_id
              WHERE r.board_id = ?1 AND r.engine_id = ?2
                AND v.ipc IS ?3 AND v.name = ?4 AND i.label = ?5",
            params![board_id, engine_id, ipc, variant, label],
            |r| r.get(0),
        )?)
    }

    /// How many rows this board holds for this engine, and the highest attempt
    /// among them. A second import that produced `attempt = 2` rows would show
    /// up here as a doubled count.
    pub fn run_census(&self, board_id: i64, engine_id: i64) -> Result<(i64, i64), DbError> {
        Ok(self.conn.query_row(
            "SELECT COUNT(*), ifnull(MAX(attempt),0) FROM run
              WHERE board_id = ?1 AND engine_id = ?2",
            params![board_id, engine_id],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )?)
    }

    /// Plans VAL actually rejected. `= 0`, and only `= 0`.
    pub fn val_rejected(&self, board_id: i64, engine_id: i64) -> Result<i64, DbError> {
        Ok(self.conn.query_row(
            "SELECT COUNT(*) FROM run WHERE board_id=?1 AND engine_id=?2 AND val = 0",
            params![board_id, engine_id],
            |r| r.get(0),
        )?)
    }

    /// Plans VAL accepted.
    pub fn val_ok(&self, board_id: i64, engine_id: i64) -> Result<i64, DbError> {
        Ok(self.conn.query_row(
            "SELECT COUNT(*) FROM run WHERE board_id=?1 AND engine_id=?2 AND val = 1",
            params![board_id, engine_id],
            |r| r.get(0),
        )?)
    }

    /// Rows where validation was UNAVAILABLE. `IS NULL`, never `= 0` -- the
    /// confusion between these two counts is three published-number incidents.
    pub fn val_unavailable(&self, board_id: i64, engine_id: i64) -> Result<i64, DbError> {
        Ok(self.conn.query_row(
            "SELECT COUNT(*) FROM run WHERE board_id=?1 AND engine_id=?2 AND val IS NULL",
            params![board_id, engine_id],
            |r| r.get(0),
        )?)
    }

    /// How many of this board's runs carry each timing verdict.
    pub fn timing_census(
        &self,
        board_id: i64,
        engine_id: i64,
    ) -> Result<Vec<(String, i64)>, DbError> {
        let mut st = self.conn.prepare(
            "SELECT timing_quality, COUNT(*) FROM run
              WHERE board_id=?1 AND engine_id=?2
              GROUP BY timing_quality ORDER BY timing_quality",
        )?;
        let rows = st.query_map(params![board_id, engine_id], |r| Ok((r.get(0)?, r.get(1)?)))?;
        Ok(rows.collect::<Result<Vec<_>, _>>()?)
    }

    /// The resume gate, as a range query over the contention timeline.
    ///
    /// This is `ipc67.py:load_resume`'s per-sample rule, verbatim:
    ///
    /// * the window `[start_ts, end_ts]` must lie inside the sampled span,
    ///   padded one interval each side -- otherwise nothing was watching and
    ///   the answer is [`Cleanliness::Uncovered`];
    /// * every sample within one interval of the window must be under
    ///   [`crate::monitor::SAMPLE_CLEAN_PCPU`], and a sample whose competitor
    ///   total could not be attributed counts as DIRTY;
    /// * an empty overlap is uncovered, not clean.
    ///
    /// Per-sample, never a median over the run: a clean median across a dirty
    /// stretch is exactly the lie the whole-board retry existed to prevent.
    ///
    /// `pass` scopes the query to one imported conditions file; `None` uses the
    /// box-wide watcher's samples, which is the point of keeping them -- the
    /// Python could only ever see one prior pass's span.
    pub fn window_gate(
        &self,
        start_ts: f64,
        end_ts: f64,
        interval: f64,
        pass: Option<i64>,
    ) -> Result<Cleanliness, DbError> {
        let span: Option<(f64, f64)> = self
            .conn
            .query_row(
                "SELECT MIN(at), MAX(at) FROM sample
                  WHERE ((?1 IS NULL AND pass_id IS NULL) OR pass_id = ?1)",
                params![pass],
                |r| Ok((r.get::<_, Option<f64>>(0)?, r.get::<_, Option<f64>>(1)?)),
            )
            .optional()?
            .and_then(|(a, b)| Some((a?, b?)));
        let Some((first, last)) = span else {
            return Ok(Cleanliness::Uncovered);
        };
        // The Python reads tl[0] and tl[-1] and trusts the list to be in time
        // order. MIN/MAX say the same thing without the assumption.
        if start_ts < first - interval || end_ts > last + interval {
            return Ok(Cleanliness::Uncovered);
        }
        let (overlapping, dirty): (i64, i64) = self.conn.query_row(
            "SELECT COUNT(*),
                    SUM(CASE WHEN competitors_total IS NULL OR competitors_total >= ?4
                             THEN 1 ELSE 0 END)
               FROM sample
              WHERE ((?1 IS NULL AND pass_id IS NULL) OR pass_id = ?1)
                AND at >= ?2 AND at <= ?3",
            params![
                pass,
                start_ts - interval,
                end_ts + interval,
                crate::monitor::SAMPLE_CLEAN_PCPU
            ],
            |r| Ok((r.get(0)?, r.get::<_, Option<i64>>(1)?.unwrap_or(0))),
        )?;
        Ok(if overlapping == 0 {
            Cleanliness::Uncovered
        } else if dirty > 0 {
            Cleanliness::Dirty
        } else {
            Cleanliness::Clean
        })
    }

    /// Children this database believes are still alive, for the startup reap.
    ///
    /// The caller MUST compare [`LiveChild::identity`] against a live
    /// `Platform::proc_identity` before signalling anything: pids recycle, and
    /// killpg on a recycled pgid kills a stranger's work silently.
    pub fn live_children(&self) -> Result<Vec<LiveChild>, DbError> {
        let mut st = self.conn.prepare(
            "SELECT pid,pgid,run_id,binary_path,proc_start_tvsec,spawned_at,stopped
               FROM live_child ORDER BY spawned_at, pid",
        )?;
        let rows = st.query_map([], |r| {
            Ok(LiveChild {
                pid: r.get(0)?,
                pgid: r.get(1)?,
                run_id: r.get(2)?,
                binary_path: r.get(3)?,
                proc_start_tvsec: r.get(4)?,
                spawned_at: r.get(5)?,
                stopped: r.get(6)?,
            })
        })?;
        Ok(rows.collect::<Result<Vec<_>, _>>()?)
    }

    /// The pass record for a board, if one exists.
    pub fn pass_verdict(
        &self,
        board_id: i64,
        engine_id: i64,
    ) -> Result<Option<(PassVerdict, i64, i64)>, DbError> {
        let got: Option<(String, i64, i64)> = self
            .conn
            .query_row(
                "SELECT verdict, ran, reused FROM board_pass
                  WHERE board_id = ?1 AND engine_id = ?2 ORDER BY id DESC LIMIT 1",
                params![board_id, engine_id],
                |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
            )
            .optional()?;
        Ok(got.map(|(v, ran, reused)| (PassVerdict::parse(&v), ran, reused)))
    }

    /// Who else was on the box between two instants, worst first.
    ///
    /// This is what `sample_process` is FOR: the rollup in a conditions file
    /// answers "was the board clean", and only the per-process breakdown
    /// answers "and if not, what was it". Summed per name, because three
    /// browser renderers are one competitor rather than three -- the same
    /// collapse `monitor::attribute` does at sample time, held across samples.
    ///
    /// Ordered by total descending and then by NAME, never by total alone: two
    /// competitors that tie would otherwise swap places between calls and make
    /// a rendered table flicker for no reason.
    pub fn competitors_between(
        &self,
        start_ts: f64,
        end_ts: f64,
    ) -> Result<Vec<(String, f64)>, DbError> {
        let mut st = self.conn.prepare(
            "SELECT p.name, SUM(p.pcpu) AS total
               FROM sample_process p
               JOIN sample s ON s.id = p.sample_id
              WHERE s.at >= ?1 AND s.at <= ?2
              GROUP BY p.name
              ORDER BY total DESC, p.name",
        )?;
        let rows = st.query_map(params![start_ts, end_ts], |r| Ok((r.get(0)?, r.get(1)?)))?;
        Ok(rows.collect::<Result<Vec<_>, _>>()?)
    }

    /// How many samples this database holds, optionally scoped to one imported
    /// pass.
    pub fn sample_count(&self, pass: Option<i64>) -> Result<i64, DbError> {
        Ok(self.conn.query_row(
            "SELECT COUNT(*) FROM sample
              WHERE ((?1 IS NULL AND pass_id IS NULL) OR pass_id = ?1)",
            params![pass],
            |r| r.get(0),
        )?)
    }
}

/// Parse a stored JSON token back into the number it was.
///
/// Via `Value`, which is the path `crucible_publish::parse_rows` already takes
/// and the one the `arbitrary_precision` feature is known to keep exact: the
/// token is preserved rather than round-tripped through an `f64` whose fast
/// parse is one ULP off on real values in this corpus.
fn parse_number(s: &str) -> Option<serde_json::Number> {
    match serde_json::from_str::<serde_json::Value>(s) {
        Ok(serde_json::Value::Number(n)) => Some(n),
        _ => None,
    }
}

fn parse_value(s: &str) -> Option<serde_json::Value> {
    serde_json::from_str::<serde_json::Value>(s).ok()
}

fn parse_notes(s: &str) -> Option<Notes> {
    let v = serde_json::from_str::<serde_json::Value>(s).ok()?;
    serde_json::from_value::<Notes>(v).ok()
}

fn parse_object(s: &str) -> Option<serde_json::Map<String, serde_json::Value>> {
    match serde_json::from_str::<serde_json::Value>(s) {
        Ok(serde_json::Value::Object(m)) => Some(m),
        _ => None,
    }
}
