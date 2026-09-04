//! One thread owns the only read-write connection. Everything else asks it.
//!
//! SQLite in WAL mode tolerates many readers and exactly one writer, and the
//! cheapest way to guarantee the second half is to have only one. A single
//! writer also buys three things this harness specifically needs:
//!
//! * **Resolution is race-free by construction.** `variant`, `instance`,
//!   `board` and `engine` are all resolved SELECT-then-INSERT. On one thread
//!   that is atomic enough; on several it is a lost-update bug waiting for the
//!   first busy sweep -- and one of those tables (`variant`) has a UNIQUE that
//!   SQLite does not enforce over NULLs, so the database would not even catch
//!   it.
//! * **Batching without a queue of its own.** Samples arrive every twenty
//!   seconds and events in bursts; committing each one is a fsync the box does
//!   not need to do. They are collected up to [`BATCH_MAX`] or [`BATCH_WINDOW`]
//!   and written as one transaction.
//! * **A run row is never batched.** A completed run is the expensive thing --
//!   minutes to hours of computation that cannot be recovered by re-reading a
//!   file -- and never losing one to a crash is the entire premise of this
//!   project. It goes into its own transaction, immediately, and the caller
//!   waits for the commit. Sample and event rows are cheap and are allowed to
//!   be lost; a run row is not.
//!
//! The asymmetry is deliberate and is the reason the API is shaped the way it
//! is: [`WriterHandle::run`] returns a `Result` because it waited, while
//! [`WriterHandle::sample`] returns nothing because it did not.

use super::model::*;
use super::DbError;
use rusqlite::{params, Connection, OptionalExtension};
use std::collections::HashMap;
use std::sync::mpsc::{Receiver, RecvTimeoutError, Sender};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

/// Rows of cheap telemetry to collect before committing.
pub const BATCH_MAX: usize = 64;
/// ...or this long, whichever comes first. Short enough that a crash loses at
/// most a fifth of a second of telemetry, long enough that a burst of events
/// is one fsync rather than sixty.
pub const BATCH_WINDOW: Duration = Duration::from_millis(200);

/// What the writer thread accepts. Everything that carries a reply channel is
/// committed immediately in its own transaction; `Sample` and `Event` are the
/// only batched variants, because they are the only ones cheap enough to lose.
enum Cmd {
    Run(Box<RunRecord>, Sender<Result<i64, DbError>>),
    Sample(Box<SampleRec>),
    Event(Box<EventRec>),
    ThrottleOpen(ThrottleWindowRec, Sender<Result<i64, DbError>>),
    ThrottleClose {
        id: i64,
        ended_at: f64,
        reply: Sender<Result<(), DbError>>,
    },
    ChildSpawned(LiveChild, Sender<Result<(), DbError>>),
    ChildStopped {
        pid: i32,
        stopped: bool,
        reply: Sender<Result<(), DbError>>,
    },
    ChildGone {
        pid: i32,
        reply: Sender<Result<(), DbError>>,
    },
    Pass(Box<BoardPassRec>, Sender<Result<i64, DbError>>),
    Resolve(
        Box<(BoardKey, BoardFacts, EngineKey, EngineFacts)>,
        Sender<Result<(i64, i64), DbError>>,
    ),
    Flush(Sender<Result<(), DbError>>),
    Stop,
}

/// The caller's end of the writer thread. Cheap to clone; every clone talks to
/// the same connection.
#[derive(Clone)]
pub struct WriterHandle {
    tx: Sender<Cmd>,
    /// Where a batched write's failure goes, since there is nobody waiting to
    /// be told. A writer that swallows these silently is how a table quietly
    /// stops growing, so it is kept and [`WriterHandle::take_error`] hands it
    /// over.
    last_error: Arc<Mutex<Option<String>>>,
}

impl WriterHandle {
    /// Record one completed instance. Blocks until it is committed.
    pub fn run(&self, rec: RunRecord) -> Result<i64, DbError> {
        self.ask(|reply| Cmd::Run(Box::new(rec), reply))
    }

    /// Record one contention sample. Fire-and-forget: batched, and allowed to
    /// be lost.
    pub fn sample(&self, s: SampleRec) {
        let _ = self.tx.send(Cmd::Sample(Box::new(s)));
    }

    /// Append one log line. Fire-and-forget, same reasoning.
    pub fn event(&self, e: EventRec) {
        let _ = self.tx.send(Cmd::Event(Box::new(e)));
    }

    /// Open a throttle window, returning its id so it can be closed later.
    /// Immediate: this is the record of why a run is about to be dirty, and it
    /// has to survive the crash that the contention may well be causing.
    pub fn throttle_open(&self, w: ThrottleWindowRec) -> Result<i64, DbError> {
        self.ask(|reply| Cmd::ThrottleOpen(w, reply))
    }

    pub fn throttle_close(&self, id: i64, ended_at: f64) -> Result<(), DbError> {
        self.ask(|reply| Cmd::ThrottleClose {
            id,
            ended_at,
            reply,
        })
    }

    /// Register a spawned child. Immediate, because the whole value of the row
    /// is that it is on disk BEFORE the process that could crash does.
    pub fn child_spawned(&self, c: LiveChild) -> Result<(), DbError> {
        self.ask(|reply| Cmd::ChildSpawned(c, reply))
    }

    pub fn child_stopped(&self, pid: i32, stopped: bool) -> Result<(), DbError> {
        self.ask(|reply| Cmd::ChildStopped {
            pid,
            stopped,
            reply,
        })
    }

    pub fn child_gone(&self, pid: i32) -> Result<(), DbError> {
        self.ask(|reply| Cmd::ChildGone { pid, reply })
    }

    /// Record (or update) a board pass. Re-recording the same `source_path`
    /// updates the row rather than adding one, and drops the samples the
    /// previous import of that file brought with it.
    pub fn board_pass(&self, p: BoardPassRec) -> Result<i64, DbError> {
        self.ask(|reply| Cmd::Pass(Box::new(p), reply))
    }

    /// Interning only: get (board_id, engine_id) for an identity, creating the
    /// rows if this is the first time either has been seen. Resolution has to
    /// go through the writer thread even when nothing is being written,
    /// because SELECT-then-INSERT is only atomic while one thread does it.
    pub fn resolve(
        &self,
        board: BoardKey,
        board_facts: BoardFacts,
        engine: EngineKey,
        engine_facts: EngineFacts,
    ) -> Result<(i64, i64), DbError> {
        self.ask(|reply| Cmd::Resolve(Box::new((board, board_facts, engine, engine_facts)), reply))
    }

    /// Commit whatever is batched. Needed by anything that is about to read
    /// its own writes through a separate reader connection.
    pub fn flush(&self) -> Result<(), DbError> {
        self.ask(Cmd::Flush)
    }

    /// The most recent failure from a batched write, cleared by reading it.
    pub fn take_error(&self) -> Option<String> {
        self.last_error.lock().ok().and_then(|mut e| e.take())
    }

    fn ask<T, F>(&self, make: F) -> Result<T, DbError>
    where
        F: FnOnce(Sender<Result<T, DbError>>) -> Cmd,
    {
        let (tx, rx) = std::sync::mpsc::channel();
        self.tx.send(make(tx)).map_err(|_| DbError::WriterGone)?;
        rx.recv().map_err(|_| DbError::WriterGone)?
    }
}

/// Owns the writer thread and stops it on drop.
pub struct Writer {
    handle: WriterHandle,
    join: Option<std::thread::JoinHandle<()>>,
}

impl Writer {
    /// Take ownership of the one read-write connection and start serving.
    pub fn start(conn: Connection) -> Writer {
        let (tx, rx) = std::sync::mpsc::channel();
        let last_error = Arc::new(Mutex::new(None));
        let err_slot = Arc::clone(&last_error);
        let join = std::thread::Builder::new()
            .name("crucible-db".into())
            .spawn(move || serve(conn, rx, err_slot))
            .expect("spawning the database writer thread");
        Writer {
            handle: WriterHandle { tx, last_error },
            join: Some(join),
        }
    }

    pub fn handle(&self) -> &WriterHandle {
        &self.handle
    }
}

impl Drop for Writer {
    fn drop(&mut self) {
        // Flush first, then stop: a Stop that raced the batch would drop the
        // telemetry the last two hundred milliseconds accumulated.
        let _ = self.handle.flush();
        let _ = self.handle.tx.send(Cmd::Stop);
        if let Some(j) = self.join.take() {
            let _ = j.join();
        }
    }
}

/// The hashable form of a [`BoardKey`]: (name, budget, mode, jobs, threads,
/// env, args).
///
/// Worth a name rather than an anonymous seven-tuple, because this IS the
/// comparison the resume gate makes. Two rows may only be stitched together
/// when every one of these matches, so a board that changes any of them -- a
/// tier move from 30s to 60s, say -- is a DIFFERENT board rather than the same
/// board reconfigured.
type BoardCacheKey = (String, u64, String, u32, String, String, String);

/// Ids already resolved this session. Order is never observable through these,
/// so a hash map is safe here in a way it is not in the publication crate.
#[derive(Default)]
struct Ids {
    engine: HashMap<EngineKey, i64>,
    board: HashMap<BoardCacheKey, i64>,
    variant: HashMap<VariantKey, i64>,
    instance: HashMap<(i64, String), i64>,
}

fn serve(conn: Connection, rx: Receiver<Cmd>, err_slot: Arc<Mutex<Option<String>>>) {
    let mut ids = Ids::default();
    let mut samples: Vec<SampleRec> = Vec::new();
    let mut events: Vec<EventRec> = Vec::new();
    let mut since: Option<Instant> = None;

    let note = |e: DbError, slot: &Arc<Mutex<Option<String>>>| {
        if let Ok(mut s) = slot.lock() {
            *s = Some(e.to_string());
        }
    };

    loop {
        let waited = match since {
            Some(t) => rx.recv_timeout(BATCH_WINDOW.saturating_sub(t.elapsed())),
            None => rx.recv().map_err(|_| RecvTimeoutError::Disconnected),
        };

        let cmd = match waited {
            Ok(c) => c,
            Err(RecvTimeoutError::Timeout) => {
                if let Err(e) = drain(&conn, &mut samples, &mut events) {
                    note(e, &err_slot);
                }
                since = None;
                continue;
            }
            Err(RecvTimeoutError::Disconnected) => {
                if let Err(e) = drain(&conn, &mut samples, &mut events) {
                    note(e, &err_slot);
                }
                return;
            }
        };

        match cmd {
            Cmd::Sample(s) => {
                samples.push(*s);
                since.get_or_insert_with(Instant::now);
            }
            Cmd::Event(e) => {
                events.push(*e);
                since.get_or_insert_with(Instant::now);
            }
            other => {
                // Anything immediate flushes what is buffered first, so the
                // telemetry timeline and the rows it explains stay in id order.
                if let Err(e) = drain(&conn, &mut samples, &mut events) {
                    note(e, &err_slot);
                }
                since = None;
                if immediate(&conn, &mut ids, other) {
                    return;
                }
                continue;
            }
        }

        if samples.len() + events.len() >= BATCH_MAX {
            if let Err(e) = drain(&conn, &mut samples, &mut events) {
                note(e, &err_slot);
            }
            since = None;
        }
    }
}

/// Handle one immediate command. Returns true when the thread should stop.
fn immediate(conn: &Connection, ids: &mut Ids, cmd: Cmd) -> bool {
    match cmd {
        Cmd::Run(rec, reply) => {
            let _ = reply.send(txn(conn, |c| insert_run(c, ids, &rec)));
        }
        Cmd::ThrottleOpen(w, reply) => {
            let _ = reply.send(txn(conn, |c| {
                c.execute(
                    "INSERT INTO throttle_window(level,started_at,ended_at,reason)
                     VALUES(?1,?2,?3,?4)",
                    params![w.level, w.started_at, w.ended_at, w.reason],
                )?;
                Ok(c.last_insert_rowid())
            }));
        }
        Cmd::ThrottleClose {
            id,
            ended_at,
            reply,
        } => {
            let _ = reply.send(txn(conn, |c| {
                c.execute(
                    "UPDATE throttle_window SET ended_at = ?2 WHERE id = ?1",
                    params![id, ended_at],
                )?;
                Ok(())
            }));
        }
        Cmd::ChildSpawned(child, reply) => {
            let _ = reply.send(txn(conn, |c| {
                // REPLACE, not INSERT: pids recycle, and a stale row sitting
                // beside a fresh one is exactly the ambiguity that gets a
                // stranger's process group killed at the next startup sweep.
                c.execute(
                    "INSERT OR REPLACE INTO live_child
                       (pid,pgid,run_id,binary_path,proc_start_tvsec,spawned_at,stopped)
                     VALUES(?1,?2,?3,?4,?5,?6,?7)",
                    params![
                        child.pid,
                        child.pgid,
                        child.run_id,
                        child.binary_path,
                        child.proc_start_tvsec,
                        child.spawned_at,
                        child.stopped as i64
                    ],
                )?;
                Ok(())
            }));
        }
        Cmd::ChildStopped {
            pid,
            stopped,
            reply,
        } => {
            let _ = reply.send(txn(conn, |c| {
                c.execute(
                    "UPDATE live_child SET stopped = ?2 WHERE pid = ?1",
                    params![pid, stopped as i64],
                )?;
                Ok(())
            }));
        }
        Cmd::ChildGone { pid, reply } => {
            let _ = reply.send(txn(conn, |c| {
                c.execute("DELETE FROM live_child WHERE pid = ?1", params![pid])?;
                Ok(())
            }));
        }
        Cmd::Pass(p, reply) => {
            let _ = reply.send(txn(conn, |c| insert_pass(c, ids, &p)));
        }
        Cmd::Resolve(what, reply) => {
            let (bk, bf, ek, ef) = *what;
            let _ = reply.send(txn(conn, |c| {
                let eid = engine_id(c, ids, &ek, &ef)?;
                let bid = board_id(c, ids, &bk, &bf)?;
                Ok((bid, eid))
            }));
        }
        Cmd::Flush(reply) => {
            let _ = reply.send(Ok(()));
        }
        Cmd::Stop => return true,
        Cmd::Sample(_) | Cmd::Event(_) => unreachable!("batched commands never reach immediate"),
    }
    false
}

fn txn<T, F>(conn: &Connection, f: F) -> Result<T, DbError>
where
    F: FnOnce(&Connection) -> Result<T, DbError>,
{
    let tx = conn.unchecked_transaction()?;
    let out = f(&tx)?;
    tx.commit()?;
    Ok(out)
}

fn drain(
    conn: &Connection,
    samples: &mut Vec<SampleRec>,
    events: &mut Vec<EventRec>,
) -> Result<(), DbError> {
    if samples.is_empty() && events.is_empty() {
        return Ok(());
    }
    let out = txn(conn, |c| {
        for s in samples.iter() {
            c.execute(
                "INSERT INTO sample
                   (at,idle_pct,competitors_total,loadavg1,swap_mb,cpu_speed_limit,pass_id)
                 VALUES(?1,?2,?3,?4,?5,?6,?7)",
                params![
                    s.at,
                    s.idle_pct,
                    s.competitors_total,
                    s.loadavg1,
                    s.swap_mb,
                    s.cpu_speed_limit.map(|v| v as i64),
                    s.pass_id
                ],
            )?;
            let sid = c.last_insert_rowid();
            for (name, pcpu) in &s.processes {
                c.execute(
                    "INSERT OR REPLACE INTO sample_process(sample_id,name,pcpu)
                     VALUES(?1,?2,?3)",
                    params![sid, name, pcpu],
                )?;
            }
        }
        for e in events.iter() {
            c.execute(
                "INSERT INTO event(at,level,kind,run_id,board_id,message)
                 VALUES(?1,?2,?3,?4,?5,?6)",
                params![e.at, e.level, e.kind, e.run_id, e.board_id, e.message],
            )?;
        }
        Ok(())
    });
    // Whether it committed or not, the buffer is spent: retrying a batch that
    // failed on a constraint would wedge the thread on the same row forever,
    // and telemetry is the one thing here allowed to be lost.
    samples.clear();
    events.clear();
    out
}

// ---------------------------------------------------------------------------
// Identity resolution. SELECT-then-INSERT, safe because there is one writer.
// ---------------------------------------------------------------------------

fn engine_id(
    conn: &Connection,
    ids: &mut Ids,
    key: &EngineKey,
    facts: &EngineFacts,
) -> Result<i64, DbError> {
    if let Some(id) = ids.engine.get(key) {
        return Ok(*id);
    }
    let found: Option<i64> = match &key.blake3 {
        Some(h) => conn
            .prepare_cached("SELECT id FROM engine WHERE blake3 = ?1")?
            .query_row(params![h], |r| r.get(0))
            .optional()?,
        // No hash: this engine was reconstructed from artifacts, where the
        // binary is long gone. The partial unique index keeps one row per
        // version string so a rebuild cannot fan a board out across
        // indistinguishable phantoms.
        None => conn
            .prepare_cached(
                "SELECT id FROM engine
                  WHERE blake3 IS NULL AND ifnull(ver,'') = ifnull(?1,'')",
            )?
            .query_row(params![key.ver], |r| r.get(0))
            .optional()?,
    };
    let id = match found {
        Some(id) => id,
        None => {
            conn.execute(
                "INSERT INTO engine
                   (blake3,ver,tag,commit_sha,binary_path,built_at,build_status,build_log,source)
                 VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9)",
                params![
                    key.blake3,
                    key.ver,
                    facts.tag,
                    facts.commit_sha,
                    facts.binary_path,
                    facts.built_at,
                    facts.build_status,
                    facts.build_log,
                    if facts.rebuilt { "rebuilt" } else { "measured" }
                ],
            )?;
            conn.last_insert_rowid()
        }
    };
    ids.engine.insert(key.clone(), id);
    Ok(id)
}

fn board_id(
    conn: &Connection,
    ids: &mut Ids,
    key: &BoardKey,
    facts: &BoardFacts,
) -> Result<i64, DbError> {
    let ck = key.cache_key();
    if let Some(id) = ids.board.get(&ck) {
        return Ok(*id);
    }
    let found: Option<i64> = conn
        .prepare_cached(
            "SELECT id FROM board
              WHERE name=?1 AND budget_secs=?2 AND mode=?3 AND jobs=?4
                AND threads=?5 AND env=?6 AND args=?7",
        )?
        .query_row(
            params![
                key.name,
                key.budget_secs,
                key.mode,
                key.jobs,
                key.threads,
                key.env,
                key.args
            ],
            |r| r.get(0),
        )
        .optional()?;
    let id = match found {
        Some(id) => id,
        None => {
            conn.execute(
                "INSERT INTO board
                   (name,budget_secs,mode,jobs,threads,env,args,threads_json,
                    label,competition,proof_track)
                 VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11)",
                params![
                    key.name,
                    key.budget_secs,
                    key.mode,
                    key.jobs,
                    key.threads,
                    key.env,
                    key.args,
                    facts.threads_json,
                    facts.label,
                    facts.competition,
                    facts.proof_track as i64
                ],
            )?;
            conn.last_insert_rowid()
        }
    };
    ids.board.insert(ck, id);
    Ok(id)
}

fn variant_id(conn: &Connection, ids: &mut Ids, key: &VariantKey) -> Result<i64, DbError> {
    if let Some(id) = ids.variant.get(key) {
        return Ok(*id);
    }
    // `IS`, not `=`: a row that carried no `ipc` has a NULL here, and `= NULL`
    // is never true. The UNIQUE on the table does not constrain NULLs either,
    // which is why this lookup has to be right rather than merely optimistic.
    let found: Option<i64> = conn
        .prepare_cached("SELECT id FROM variant WHERE ipc IS ?1 AND name = ?2")?
        .query_row(params![key.ipc, key.name], |r| r.get(0))
        .optional()?;
    let id = match found {
        Some(id) => id,
        None => {
            conn.execute(
                "INSERT INTO variant(ipc,name) VALUES(?1,?2)",
                params![key.ipc, key.name],
            )?;
            conn.last_insert_rowid()
        }
    };
    ids.variant.insert(key.clone(), id);
    Ok(id)
}

fn instance_id(
    conn: &Connection,
    ids: &mut Ids,
    variant: i64,
    key: &InstanceKey,
) -> Result<i64, DbError> {
    let ck = (variant, key.label.clone());
    if let Some(id) = ids.instance.get(&ck) {
        return Ok(*id);
    }
    let found: Option<i64> = conn
        .prepare_cached("SELECT id FROM instance WHERE variant_id = ?1 AND label = ?2")?
        .query_row(params![variant, key.label], |r| r.get(0))
        .optional()?;
    let id = match found {
        Some(id) => id,
        None => {
            conn.execute(
                "INSERT INTO instance(variant_id,label,label_is_int,sort_key)
                 VALUES(?1,?2,?3,?4)",
                params![variant, key.label, key.label_is_int as i64, key.sort_key()],
            )?;
            conn.last_insert_rowid()
        }
    };
    ids.instance.insert(ck, id);
    Ok(id)
}

// ---------------------------------------------------------------------------
// The receipt.
// ---------------------------------------------------------------------------

fn insert_run(conn: &Connection, ids: &mut Ids, rec: &RunRecord) -> Result<i64, DbError> {
    let eid = engine_id(conn, ids, &rec.engine, &rec.engine_facts)?;
    let bid = board_id(conn, ids, &rec.board, &rec.board_facts)?;
    let vid = variant_id(conn, ids, &VariantKey::of(&rec.row))?;
    let iid = instance_id(conn, ids, vid, &InstanceKey::of(&rec.row.instance))?;

    let r = &rec.row;
    let m = &rec.measured;
    // The exact JSON token, not a re-formatted number: `time` is an INTEGER on
    // the hard-timeout path and a two-place float everywhere else, and a
    // storage layer that normalises the two rewrites a measured value.
    let time_json = r.time.as_ref().map(|n| n.to_string());
    let time_secs = r.time.as_ref().and_then(|n| n.as_f64());
    let notes_json = match &r.notes {
        Some(n) => Some(serde_json::to_string(n).map_err(|e| DbError::Encode(e.to_string()))?),
        None => None,
    };
    let threads_json = match &r.threads {
        Some(v) => Some(serde_json::to_string(v).map_err(|e| DbError::Encode(e.to_string()))?),
        None => None,
    };
    let extra_json = if r.extra.is_empty() {
        None
    } else {
        Some(serde_json::to_string(&r.extra).map_err(|e| DbError::Encode(e.to_string()))?)
    };

    let id: i64 = conn
        .prepare_cached(
            "INSERT INTO run
               (board_id,instance_id,engine_id,attempt,state,timing_quality,
                solved,time_secs,time_json,metric,length,val,val_reason,notes_json,
                budget_secs,ver,mode,jobs,threads_json,start_ts,end_ts,makespan,resumed_clean,
                present_ipc,present_budget,present_stamps,present_makespan,present_resumed_clean,
                extra_json,
                started_at,finished_at,wall_ms,cpu_ms,suspended_ms,peak_rss,mem_instrument,
                exit_code,term_signal,pid,pgid,cpu_instrument)
             VALUES
               (?1,?2,?3,?4,?5,?6,
                ?7,?8,?9,?10,?11,?12,?13,?14,
                ?15,?16,?17,?18,?19,?20,?21,?22,?23,
                ?24,?25,?26,?27,?28,
                ?29,
                ?30,?31,?32,?33,?34,?35,?36,
                ?37,?38,?39,?40,?41)
             ON CONFLICT(board_id,instance_id,engine_id,attempt) DO UPDATE SET
                state=excluded.state, timing_quality=excluded.timing_quality,
                solved=excluded.solved, time_secs=excluded.time_secs,
                time_json=excluded.time_json, metric=excluded.metric,
                length=excluded.length, val=excluded.val,
                val_reason=excluded.val_reason, notes_json=excluded.notes_json,
                budget_secs=excluded.budget_secs, ver=excluded.ver,
                mode=excluded.mode, jobs=excluded.jobs,
                threads_json=excluded.threads_json, start_ts=excluded.start_ts,
                end_ts=excluded.end_ts, makespan=excluded.makespan,
                resumed_clean=excluded.resumed_clean,
                present_ipc=excluded.present_ipc,
                present_budget=excluded.present_budget,
                present_stamps=excluded.present_stamps,
                present_makespan=excluded.present_makespan,
                present_resumed_clean=excluded.present_resumed_clean,
                extra_json=excluded.extra_json,
                started_at=excluded.started_at, finished_at=excluded.finished_at,
                wall_ms=excluded.wall_ms, cpu_ms=excluded.cpu_ms,
                suspended_ms=excluded.suspended_ms, peak_rss=excluded.peak_rss,
                mem_instrument=excluded.mem_instrument,
                exit_code=excluded.exit_code, term_signal=excluded.term_signal,
                pid=excluded.pid, pgid=excluded.pgid,
                cpu_instrument=excluded.cpu_instrument
             RETURNING id",
        )?
        .query_row(
            params![
                bid,
                iid,
                eid,
                rec.attempt,
                rec.state.as_str(),
                rec.timing.as_str(),
                r.solved as i64,
                time_secs,
                time_json,
                r.metric,
                r.length.map(|v| v as i64),
                r.val.map(|v| v as i64),
                rec.val_reason.map(|v| v.as_str()),
                notes_json,
                r.budget,
                r.ver,
                r.mode,
                r.jobs.map(|v| v as i64),
                threads_json,
                r.start_ts,
                r.end_ts,
                r.makespan,
                r.resumed_clean as i64,
                r.present.ipc as i64,
                r.present.budget as i64,
                r.present.stamps as i64,
                r.present.makespan as i64,
                r.present.resumed_clean as i64,
                extra_json,
                m.started_at,
                m.finished_at,
                m.wall_ms.map(|v| v as i64),
                m.cpu_ms.map(|v| v as i64),
                m.suspended_ms.map(|v| v as i64),
                m.peak_rss.map(|v| v as i64),
                m.mem_instrument,
                m.exit_code,
                m.term_signal,
                m.pid,
                m.pgid,
                m.cpu_instrument,
            ],
            |row| row.get(0),
        )?;
    Ok(id)
}

fn insert_pass(conn: &Connection, ids: &mut Ids, p: &BoardPassRec) -> Result<i64, DbError> {
    let eid = engine_id(conn, ids, &p.engine, &p.engine_facts)?;
    let bid = board_id(conn, ids, &p.board, &p.board_facts)?;
    let id: i64 = conn
        .prepare_cached(
            "INSERT INTO board_pass
               (board_id,engine_id,started_at,ended_at,verdict,ran,reused,
                done_marker,raw_path,conditions_path,sample_interval,source_path)
             VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12)
             ON CONFLICT(board_id,engine_id,source_path) DO UPDATE SET
                started_at=excluded.started_at, ended_at=excluded.ended_at,
                verdict=excluded.verdict, ran=excluded.ran, reused=excluded.reused,
                done_marker=excluded.done_marker, raw_path=excluded.raw_path,
                conditions_path=excluded.conditions_path,
                sample_interval=excluded.sample_interval
             RETURNING id",
        )?
        .query_row(
            params![
                bid,
                eid,
                p.started_at,
                p.ended_at,
                p.verdict.as_str(),
                p.ran,
                p.reused,
                p.done_marker,
                p.raw_path,
                p.conditions_path,
                p.sample_interval,
                // '' is the identity of a LIVE pass -- one that came from no
                // file. See the column's comment.
                p.source_path.clone().unwrap_or_default(),
            ],
            |row| row.get(0),
        )?;
    // Re-importing a conditions file must not double its timeline. Only samples
    // that came from THIS pass are dropped; the box-wide watcher's samples
    // carry no pass_id and are never touched.
    conn.execute("DELETE FROM sample WHERE pass_id = ?1", params![id])?;
    Ok(id)
}
