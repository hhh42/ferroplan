//! The sweep: what the shell drivers did, without the shell.
//!
//! `cut25-sweeps.sh` is 194 lines that wait for a quiet box, start a contention
//! watcher, hand a board to a Python runner, read a verdict back out of a JSON
//! file through three separate `python3 -c` invocations, and either touch a
//! `.done` marker or leave the board for the next pass. This is that, with the
//! atom moved from a BOARD to an INSTANCE -- which is the whole point, because
//! the contention that kills a two-hour board is usually a ten-minute window in
//! the middle of it, and everything measured either side of that window was
//! fine.
//!
//! The organising rule, stated once: **contention may cost a timing number; it
//! must never cost hours of computation.** So every row measured is kept and
//! written, marked dirty when the box was not quiet, and a board is banked only
//! when every one of its instances has a CLEAN row. Nothing is discarded; work
//! is owed, not lost.
//!
//! # The database is the truth; the JSONL is an export
//!
//! Every measured instance is committed to the database in its own
//! transaction the moment its run finishes -- BEFORE the box is asked whether
//! the run was clean, before the artifacts are rewritten, before anything else
//! happens. That commit is the `kill -9` receipt: a restarted sweep opens the
//! same database, reads back every row and every clean verdict, and owes
//! exactly the instances that never got one. The stage's `.jsonl` is
//! regenerated from those rows, which is what makes it an export rather than a
//! second record that could disagree.
//!
//! Cleanliness is the per-sample window intersection over the watcher's
//! box-wide timeline (`Reader::window_gate`), the same rule `ipc67.py`'s
//! `load_resume` applies to a conditions file -- and not the before/after
//! sample pair the first cut of this driver used, which could not see a
//! ten-minute spike in the middle of a five-minute instance.
//!
//! `--no-db` restores that first cut exactly: no database, no watcher thread,
//! no engine stamp on the rows, pair-judged cleanliness. It is the hatch, and
//! it is kept so the off-path artifacts stay bit-identical to what the
//! pre-database binary wrote.

use anyhow::Context;
use crucible_core::corpus;
use crucible_core::db::{self, Db, Reader};
use crucible_core::exec::{self, orphan, Ctl};
use crucible_core::monitor::{self, Level, Sample, Throttle};
use crucible_core::platform::{self, Pid, Platform};
use crucible_core::sched::{self, referee, Attempt, BoardState, Event, LoopConfig, Next, Runner};
use crucible_core::sweep::{BoardCfg, Engine as SweepEngine};
use crucible_publish::manifest::{BoardSpec, Manifest};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{mpsc, Arc, Mutex};
use std::time::{Duration, Instant};

/// One board's work: its spec, its instances, and which of them still owe a
/// clean row.
pub(crate) struct Board {
    spec: BoardSpec,
    position: usize,
    instances: Vec<(String, String, corpus::Instance)>,
    /// Instance keys whose latest row BANKED under the referee
    /// (`sched::referee`): a solve, or an unsolved row the process was not
    /// starved on. An owed row is written and kept, but does not remove the
    /// instance from the owed set.
    ///
    /// Keyed by the row's full address -- `ipc`, variant, label -- and never by
    /// the label alone. A multi-variant board carries instance "1" in every
    /// variant, and a set keyed on labels would count them once and never
    /// reach zero.
    banked: std::collections::BTreeSet<String>,
    /// Every row measured, keyed so a later clean measurement SUPERSEDES an
    /// earlier dirty one. Nothing is ever dropped -- a dirty row is the record
    /// that the instance was attempted and what the box was doing.
    rows: std::collections::BTreeMap<String, crucible_publish::RawRow>,
    /// The database's names for this board and engine, once resolved.
    ids: Option<(i64, i64)>,
    /// The identity every receipt for this board is written under.
    key: db::BoardKey,
    facts: db::BoardFacts,
    /// Rows this process did not measure: read back from the database at
    /// startup. Reported on the pass row, the way `ipc67.py`'s `.md` reports
    /// what it stitched.
    reused: usize,
}

impl Board {
    fn remaining(&self) -> usize {
        self.instances.len() - self.banked.len()
    }
}

/// The packed scheduler's knobs, from `[scheduler]`.
#[derive(Debug, Clone, Copy)]
pub struct Pack {
    pub width: usize,
    pub narrow_width: usize,
    pub max_frac: f64,
    pub narrow_max_frac: f64,
    pub mem_reserve_bytes: u64,
    pub rss_headroom: f64,
}

impl Pack {
    pub fn from_config(c: &crate::config::Scheduler) -> Pack {
        let logical = platform::host().topology().logical.max(1) as usize;
        Pack {
            width: if c.pack_width == 0 {
                logical
            } else {
                (c.pack_width as usize).max(1)
            },
            narrow_width: (c.pack_narrow_width as usize).max(1),
            max_frac: c.pack_max_frac,
            narrow_max_frac: c.pack_narrow_max_frac,
            mem_reserve_bytes: (c.mem_reserve_gb.max(0.0) * (1u64 << 30) as f64) as u64,
            rss_headroom: c.rss_headroom.max(1.0),
        }
    }

    /// Everything solo: the R1 shape, for tests and for `--quiet-only`.
    pub fn solo() -> Pack {
        Pack {
            width: 1,
            narrow_width: 1,
            max_frac: 0.0,
            narrow_max_frac: 0.0,
            mem_reserve_bytes: 0,
            rss_headroom: 1.0,
        }
    }
}

#[derive(Default)]
struct PassStats {
    banked: usize,
    ran: usize,
    all_dirty: bool,
    tally: std::collections::BTreeMap<&'static str, usize>,
}

impl PassStats {
    fn default() -> Self {
        PassStats {
            banked: 0,
            ran: 0,
            all_dirty: true,
            tally: Default::default(),
        }
    }
}

/// One instance's measurement and verdict, back from a worker.
struct Done {
    i: usize,
    key: String,
    row: crucible_publish::RawRow,
    banked: bool,
    verdict: Option<&'static str>,
    box_fault: bool,
    /// Unsolved beside neighbours: try again with fewer.
    cascade: bool,
    cancelled: bool,
    solved: bool,
    rejected: bool,
    secs: Option<f64>,
    rho: Option<f64>,
}

/// What a worker needs to run one instance: owned or shared, nothing that
/// borrows the runner, so the runner can apply results while workers run.
struct RunCtx {
    engine: SweepEngine,
    cfg: BoardCfg,
    plan_dir: PathBuf,
    val: Option<PathBuf>,
    instances: Arc<Vec<(String, String, corpus::Instance)>>,
    shared: Arc<Shared>,
    progress: Option<ProgressHandle>,
    rule: referee::Rule,
    quiet_only: bool,
    admit_below_full: bool,
    board_idx: usize,
    db: Option<RunDb>,
}

struct RunDb {
    writer: db::WriterHandle,
    path: PathBuf,
    engine: db::EngineKey,
    engine_facts: db::EngineFacts,
    board: db::BoardKey,
    facts: db::BoardFacts,
    ids: (i64, i64),
    interval: f64,
}

impl<'a> SweepRunner<'a> {
    /// Wide / narrow / solo, from what the predecessor (the promoted raw)
    /// said and what this sweep has already seen. `threads > 1` is always
    /// solo; so is anything this sweep already measured unsolved.
    fn classify(
        &self,
        idx: usize,
        cfg: &BoardCfg,
        todo: &[usize],
    ) -> (Vec<usize>, Vec<usize>, Vec<usize>) {
        let b = &self.boards[idx];
        let prior = prior_rows(&self.repo, &b.spec.raw);
        let budget = cfg.timeout_secs as f64;
        let (mut wide, mut narrow, mut solo) = (Vec::new(), Vec::new(), Vec::new());
        for &i in todo {
            let (ipc, variant, inst) = &b.instances[i];
            let key = instance_key(ipc, variant, &inst.label);
            let seen_miss = b.rows.get(&key).is_some_and(|r| !r.solved);
            let class = if cfg.threads > 1 || seen_miss || self.pack.width <= 1 {
                2
            } else {
                match prior.get(&format!("{variant}/{}", inst.label)) {
                    Some((true, Some(secs))) if *secs <= self.pack.max_frac * budget => 0,
                    Some((true, Some(secs))) if *secs <= self.pack.narrow_max_frac * budget => 1,
                    _ => 2,
                }
            };
            match class {
                0 => wide.push(i),
                1 => narrow.push(i),
                _ => solo.push(i),
            }
        }
        (wide, narrow, solo)
    }

    /// What each instance is expected to take in memory: its prior peak RSS
    /// on this box with headroom, or the board's cap where nothing is known.
    /// The workers draw these from one byte budget (the box after the
    /// reserve), so a memory-hungry instance throttles itself and not the
    /// whole batch -- sizing a batch by its worst member ran
    /// ipc5-metric-time 3-wide on ten cores.
    fn expected_bytes(&self, idx: usize, cfg: &BoardCfg, items: &[usize]) -> Vec<(usize, u64)> {
        let cap = (cfg.mem_gb * (1u64 << 30) as f64) as u64;
        items
            .iter()
            .map(|&i| {
                let (_, variant, inst) = &self.boards[idx].instances[i];
                let prior = self
                    .db
                    .as_ref()
                    .and_then(|d| d.reader.prior_peak_rss(variant, &inst.label).ok().flatten())
                    .map(|r| (r as f64 * self.pack.rss_headroom) as u64)
                    .unwrap_or(cap);
                (i, prior.max(64 << 20))
            })
            .collect()
    }

    fn mem_budget(&self) -> u64 {
        self.plat
            .topology()
            .mem_bytes
            .saturating_sub(self.pack.mem_reserve_bytes)
            .max(1 << 30)
    }

    /// Run `items` `width` at a time and apply every result as it lands.
    /// Returns the instances that missed beside neighbours, for the next
    /// rung of the cascade. A stop leaves the rest owed.
    fn run_batch(
        &mut self,
        idx: usize,
        cfg: &BoardCfg,
        plan_dir: &Path,
        items: Vec<(usize, u64)>,
        width: usize,
        stats: &mut PassStats,
    ) -> Vec<usize> {
        let mut cascade = Vec::new();
        if items.is_empty() || self.stop || exec::interrupted() {
            self.stop |= exec::interrupted();
            return cascade;
        }
        let ctx = RunCtx {
            engine: self.engine.clone(),
            cfg: cfg.clone(),
            plan_dir: plan_dir.to_path_buf(),
            val: self.val.clone(),
            instances: Arc::new(self.boards[idx].instances.clone()),
            shared: Arc::clone(&self.shared),
            progress: self.progress.clone(),
            rule: self.rule,
            quiet_only: self.quiet_only,
            admit_below_full: self.admit_below_full,
            board_idx: idx,
            db: self.db.as_ref().map(|d| RunDb {
                writer: d.db.writer().clone(),
                path: d.db.path().to_path_buf(),
                engine: d.engine.clone(),
                engine_facts: d.engine_facts.clone(),
                board: self.boards[idx].key.clone(),
                facts: self.boards[idx].facts.clone(),
                ids: self.boards[idx]
                    .ids
                    .expect("a board with a database has resolved ids"),
                interval: d.interval,
            }),
        };
        let width = width.clamp(1, items.len());
        let budget = self.mem_budget();
        let queue = Mutex::new(std::collections::VecDeque::from(items));
        let in_use = Mutex::new(0u64);
        let stop = AtomicBool::new(false);
        std::thread::scope(|sc| {
            let (tx, rx) = mpsc::channel::<Done>();
            for w in 0..width {
                let tx = tx.clone();
                let (ctx, queue, stop, in_use) = (&ctx, &queue, &stop, &in_use);
                sc.spawn(move || {
                    let plat = platform::host();
                    let reader = ctx.db.as_ref().and_then(|d| Reader::open(&d.path).ok());
                    loop {
                        if stop.load(Ordering::Relaxed) || exec::interrupted() {
                            break;
                        }
                        let Some((i, bytes)) = queue.lock().unwrap().pop_front() else {
                            break;
                        };
                        // The byte budget: wait until this instance fits
                        // beside what is running. An instance larger than
                        // the whole budget runs alone.
                        loop {
                            {
                                let mut used = in_use.lock().unwrap();
                                if *used == 0 || *used + bytes <= budget {
                                    *used += bytes;
                                    break;
                                }
                            }
                            if exec::interrupted() || stop.load(Ordering::Relaxed) {
                                break;
                            }
                            std::thread::sleep(Duration::from_millis(200));
                        }
                        // Admission (R2.2): SUSPENDED always waits; POLITE
                        // starts the run demoted unless FULL was asked for.
                        loop {
                            let level = ctx.shared.level();
                            let wait = level == Level::Suspended
                                || (level != Level::Full
                                    && (ctx.quiet_only || !ctx.admit_below_full));
                            if !wait || exec::interrupted() || stop.load(Ordering::Relaxed) {
                                break;
                            }
                            std::thread::sleep(Duration::from_secs(5));
                        }
                        if exec::interrupted() || stop.load(Ordering::Relaxed) {
                            // Back on the queue is pointless: the pass ends.
                            break;
                        }
                        if exec::interrupted() || stop.load(Ordering::Relaxed) {
                            break;
                        }
                        // The canary is reading the box: do not spawn into it.
                        while ctx.shared.held() {
                            std::thread::sleep(Duration::from_millis(100));
                        }
                        let d = run_one(ctx, w, i, (width - 1) as u32, &plat, reader.as_ref());
                        *in_use.lock().unwrap() -= bytes;
                        let cancelled = d.cancelled;
                        let _ = tx.send(d);
                        if cancelled {
                            stop.store(true, Ordering::Relaxed);
                            break;
                        }
                    }
                });
            }
            drop(tx);
            for d in rx {
                if d.cancelled {
                    self.stop = true;
                    continue;
                }
                if d.cascade {
                    cascade.push(d.i);
                }
                self.apply(idx, d, stats);
            }
        });
        cascade
    }

    /// One result into the runner's state: the row, the banked set, the
    /// pass statistics and the dashboard cell.
    fn apply(&mut self, idx: usize, d: Done, stats: &mut PassStats) {
        stats.ran += 1;
        if let Some(v) = d.verdict {
            *stats.tally.entry(v).or_default() += 1;
        }
        if !d.banked && !d.box_fault {
            stats.all_dirty = false;
        }
        let (banked, solved, rejected, secs, rho, verdict) =
            (d.banked, d.solved, d.rejected, d.secs, d.rho, d.verdict);
        self.progress_cell(idx, d.i, |c| {
            use crate::tui::app::Cell;
            c.this_solved = Some(solved);
            c.this_secs = secs;
            c.rho = rho;
            c.attempt += 1;
            c.cell = match (solved, banked) {
                _ if rejected => Cell::Error,
                (true, _) => Cell::SolvedClean,
                (false, true) => Cell::TimeoutBanked,
                (false, false) => Cell::Owed,
            };
            if let Some(v) = verdict {
                c.verdict = Some(v.to_string());
            }
        });
        if banked {
            if let Some(p) = &self.progress {
                p.lock().unwrap().banked_at.push(Instant::now());
            }
            self.boards[idx].banked.insert(d.key.clone());
            stats.banked += 1;
            stats.all_dirty = false;
        }
        // Written either way. The board is not banked, but the work is
        // not lost -- and a later banked row supersedes this one under the
        // same key.
        self.boards[idx].rows.insert(d.key, d.row);
    }
}

/// Measure one instance and judge it. Runs on a worker thread with its own
/// reader; everything it touches is in `ctx`.
fn run_one(
    ctx: &RunCtx,
    slot: usize,
    i: usize,
    neighbours: u32,
    plat: &platform::Host,
    reader: Option<&Reader>,
) -> Done {
    let (ipc, variant, inst) = ctx.instances[i].clone();
    let key = instance_key(&ipc, &variant, &inst.label);
    let dirty_now = match &ctx.db {
        None => !sample_box(plat).is_clean(),
        Some(_) => ctx.shared.level() != Level::Full,
    };
    let (tx, rx) = mpsc::channel::<Ctl>();
    let attached = ctx.shared.attach(tx);
    if let Some(p) = &ctx.progress {
        let mut g = p.lock().unwrap();
        g.running.retain(|r| r.slot != slot);
        g.running.push(Running {
            slot,
            board: ctx.board_idx,
            inst: i,
            pid: None,
            started: Instant::now(),
            budget: Duration::from_secs(ctx.cfg.timeout_secs),
        });
        if let Some(c) = g
            .boards
            .get_mut(ctx.board_idx)
            .and_then(|b| b.cells.get_mut(i))
        {
            c.cell = crate::tui::app::Cell::Running;
        }
    }

    // The live-child record goes to disk the moment the child exists: a
    // `kill -9` of this process between here and the run's end leaves a row
    // the next startup reaps by identity, instead of a planner nobody owns
    // burning a core until the wall.
    let register = |pid: Pid, at: f64| {
        if let Some(p) = &ctx.progress {
            if let Some(r) = p
                .lock()
                .unwrap()
                .running
                .iter_mut()
                .find(|r| r.slot == slot)
            {
                r.pid = Some(pid);
            }
        }
        let Some(d) = &ctx.db else {
            return;
        };
        let Some(id) = plat.proc_identity(pid) else {
            return;
        };
        let child = db::LiveChild {
            pid,
            pgid: pid,
            run_id: None,
            binary_path: id.path.clone(),
            proc_start_tvsec: id.start_tvsec,
            spawned_at: at,
            stopped: false,
        };
        if let Err(e) = d.writer.child_spawned(child) {
            eprintln!("!! could not register child {pid}: {e}");
        }
    };
    let m = crucible_core::sweep::measure(
        &ctx.engine,
        &ctx.cfg,
        &ipc,
        &variant,
        &inst,
        ctx.val.as_deref(),
        &ctx.plan_dir,
        plat,
        &rx,
        ctx.db.as_ref().map(|_| &register as &dyn Fn(Pid, f64)),
    );
    ctx.shared.detach(attached);
    if let Some(p) = &ctx.progress {
        p.lock().unwrap().running.retain(|r| r.slot != slot);
    }

    let mut done = Done {
        i,
        key: key.clone(),
        row: m.row.clone(),
        banked: false,
        verdict: None,
        box_fault: true,
        cascade: false,
        cancelled: m.cancelled,
        solved: m.row.solved,
        rejected: m.row.val == Some(false),
        secs: m.row.time.as_ref().and_then(|t| t.as_f64()),
        rho: m.cpu_instrument.map(|_| {
            m.cpu_ms as f64 / m.wall.saturating_sub(m.suspended).as_millis().max(1) as f64
        }),
    };
    if m.cancelled {
        if let (Some(d), Some(pid)) = (&ctx.db, m.pid) {
            let _ = d.writer.child_gone(pid);
        }
        crate::say!("   interrupted mid-instance ({key}); the row is not written");
        return done;
    }

    match (&ctx.db, reader) {
        (Some(d), Some(reader)) => {
            let w = &d.writer;
            if let Some(pid) = m.pid {
                let _ = w.child_gone(pid);
            }
            let (bid, eid) = d.ids;
            let attempt = reader
                .next_attempt(bid, eid, Some(&ipc), &variant, &inst.label)
                .unwrap_or(1);
            let mut rec = db::RunRecord {
                board: d.board.clone(),
                board_facts: d.facts.clone(),
                engine: d.engine.clone(),
                engine_facts: d.engine_facts.clone(),
                attempt,
                state: db::RunState::Done,
                timing: db::TimingQuality::Unknown,
                banked: false,
                verdict: None,
                val_reason: m.val_reason.and_then(db::ValReason::parse),
                row: m.row.clone(),
                measured: db::Measured {
                    started_at: m.row.start_ts,
                    finished_at: m.row.end_ts,
                    wall_ms: Some(m.wall.as_millis() as u64),
                    cpu_ms: Some(m.cpu_ms),
                    cpu_instrument: m.cpu_instrument.map(str::to_string),
                    neighbours: Some(neighbours),
                    suspended_ms: Some(m.suspended.as_millis() as u64),
                    peak_rss: Some(m.peak_rss),
                    mem_instrument: Some(m.mem_instrument.to_string()),
                    exit_code: m.exit_code,
                    term_signal: m.term_signal,
                    pid: m.pid,
                    pgid: m.pgid,
                },
            };
            // THE RECEIPT. Committed before the verdict is asked for, in its
            // own transaction, and this call waits for it.
            if let Err(e) = w.run(rec.clone()) {
                eprintln!("!! could not commit {key}: {e}");
            }
            let _ = w.flush();
            let window = match (m.row.start_ts, m.row.end_ts) {
                (Some(st), Some(en)) => Some((st, en)),
                _ => None,
            };
            let gate = window
                .map(|(st, en)| {
                    reader
                        .window_gate(st, en, d.interval, None)
                        .unwrap_or(db::Cleanliness::Uncovered)
                })
                .unwrap_or(db::Cleanliness::Uncovered);
            let swap =
                window.and_then(|(st, en)| reader.swap_growth_between(st, en).ok().flatten());
            let clock_factor =
                window.and_then(|(st, en)| reader.canary_max_between(st, en).ok().flatten());
            // THE R2 REFEREE (sched::referee): the row is judged by what the
            // kernel says about ITS process; beside our own planners only a
            // solve counts.
            let facts = referee::Facts {
                solved: m.row.solved,
                threads: ctx.cfg.threads,
                cpu_instrument: m.cpu_instrument.map(str::to_string),
                cpu_ms: m.cpu_ms,
                effective_ms: m.wall.saturating_sub(m.suspended).as_millis() as u64,
                clock_jump: !m.clock_jump.is_zero(),
                window: gate,
                swap_growth_mb: swap,
                clock_factor,
                neighbours,
            };
            let verdict = referee::judge(&ctx.rule, &facts);
            rec.timing = referee::timing(&facts);
            rec.banked = verdict.banked();
            rec.verdict = Some(verdict.as_str().to_string());
            if let Err(e) = w.run(rec) {
                eprintln!("!! could not record the verdict for {key}: {e}");
            }
            done.banked = verdict.banked();
            done.verdict = Some(verdict.as_str());
            done.box_fault = verdict.box_fault();
            done.cascade = verdict == referee::Verdict::Owed(referee::Owe::Packed);
        }
        _ => {
            // The pre-database rule, kept bit for bit under --no-db: a
            // before/after pair, and nothing in between.
            let after = sample_box(plat);
            done.banked = !dirty_now && after.is_clean() && m.clock_jump.is_zero();
        }
    }
    done
}

/// The predecessor's rows for a board: `variant/label -> (solved, secs)` from
/// the promoted raw under `benchmarks/`. Empty when there is none.
fn prior_rows(repo: &Path, raw: &str) -> std::collections::BTreeMap<String, (bool, Option<f64>)> {
    let mut out = std::collections::BTreeMap::new();
    let path = repo.join("benchmarks").join(raw);
    let Ok(src) = std::fs::read_to_string(&path) else {
        return out;
    };
    if let Ok(rows) = crucible_publish::parse_rows(&src, &path.display().to_string()) {
        for r in rows {
            let label = db::InstanceKey::of(&r.instance).label;
            out.insert(
                format!("{}/{label}", r.variant),
                (r.solved, r.time.as_ref().and_then(|t| t.as_f64())),
            );
        }
    }
    out
}

/// The row's address inside a board: the same three fields `run` is keyed by.
fn instance_key(ipc: &str, variant: &str, label: &str) -> String {
    format!("{ipc}\u{1}{variant}\u{1}{label}")
}

/// The open database and everything a receipt is stamped with.
pub struct DbCtx {
    pub db: Db,
    pub reader: Reader,
    pub engine: db::EngineKey,
    pub engine_facts: db::EngineFacts,
    /// The watcher's cadence, and the padding either side of a run's window.
    pub interval: f64,
}

/// The runner's construction parameters.
pub struct Setup<'s> {
    pub repo: &'s Path,
    pub set: &'s str,
    pub engine: SweepEngine,
    pub val: Option<PathBuf>,
    pub quiet_only: bool,
    pub max_passes: Option<u32>,
    /// The throttle level the watcher publishes, and the channel to the
    /// running child it drives.
    pub shared: Arc<Shared>,
    pub rule: referee::Rule,
    /// Start under POLITE rather than waiting for FULL (`[referee]`).
    pub admit_below_full: bool,
    /// The dashboard's feed, when one is on screen.
    pub progress: Option<ProgressHandle>,
    pub pack: Pack,
    /// Whether this engine can run a given `--mode`. A board it cannot run is
    /// skipped with ZERO rows, never measured as zero coverage.
    pub capable: &'s dyn Fn(&str) -> bool,
    /// `None` is the `--no-db` path.
    pub db: Option<DbCtx>,
    /// Where the artifacts go. `None` is the set's own stage; a backfill
    /// stages under `benchmarks/air-<ver>/` instead, because the set names
    /// the CANDIDATE's stage and an old engine must never write there.
    pub stage: Option<PathBuf>,
}

pub struct SweepRunner<'a> {
    stage: PathBuf,
    repo: PathBuf,
    manifest: &'a Manifest,
    engine: SweepEngine,
    val: Option<PathBuf>,
    pub(crate) boards: Vec<Board>,
    shared: Arc<Shared>,
    rule: referee::Rule,
    admit_below_full: bool,
    progress: Option<ProgressHandle>,
    pack: Pack,
    plat: platform::Host,
    /// Set when the operator interrupts. The remaining work stays remaining --
    /// it is not failed, and the next run picks it up.
    stop: bool,
    quiet_only: bool,
    /// Stop after this many passes. `None` is the resident behaviour: a board
    /// that cannot bank because the box is never quiet is not FAILING, it is
    /// waiting, and a harness meant to live in a pane for three days should go
    /// on waiting. A bounded run is for a smoke test, or for "make one pass
    /// tonight and show me".
    max_passes: Option<u32>,
    passes: u32,
    db: Option<DbCtx>,
}

/// A board's config, assembled once so the row-identity tuple travels together.
fn board_cfg(m: &Manifest, b: &BoardSpec) -> BoardCfg {
    let threads = b.threads.unwrap_or(m.defaults.threads);
    BoardCfg {
        // The board sweeps at its own timeout, which may differ from the budget
        // it is SCORED at -- that is a tier move in flight, and the row carries
        // the wall it actually ran under.
        timeout_secs: b.timeout_secs.unwrap_or(b.budget_secs as u64),
        mode: b.mode.clone(),
        // The mco wall-clock rule: a board carrying --threads runs ONE instance
        // at a time whatever the default says.
        jobs: if threads > 1 {
            1
        } else {
            b.jobs.unwrap_or(m.defaults.jobs)
        },
        threads,
        mem_gb: m.defaults.mem_gb,
        env: b.env.clone(),
        extra_args: b.extra_args.clone(),
    }
}

/// The identity a LIVE sweep writes its receipts under: the manifest's, with
/// the armed wall and the declared environment filled in. A rebuilt board's
/// `env` is empty because the artifacts do not record it, so a live board gets
/// its own row -- which is correct, not unfortunate: a measurement whose
/// environment is known is not the same measurement as one whose environment
/// is not.
fn board_key_for_sweep(m: &Manifest, spec: &BoardSpec, cfg: &BoardCfg) -> db::BoardKey {
    let mut k = db::board_key_from_manifest(m, spec);
    k.budget_secs = cfg.timeout_secs as f64;
    k.mode = cfg.mode.clone().unwrap_or_else(|| "auto".into());
    k.jobs = cfg.jobs;
    k.threads = cfg.threads.to_string();
    // A BTreeMap serialises with sorted keys, which is the canonical form the
    // `board` table's UNIQUE needs: one environment, one identity.
    k.env = serde_json::to_string(&cfg.env).unwrap_or_else(|_| "{}".into());
    k.args = serde_json::to_string(&cfg.extra_args).unwrap_or_else(|_| "[]".into());
    k
}

/// Sample the box once. The verdict is named-competitor load, never idle:
/// a `--threads 8` board burns most of this machine by design.
fn sample_box(plat: &platform::Host) -> Sample {
    let ps = std::process::Command::new("ps")
        .args(["-Ao", "pcpu,comm", "-r"])
        .output()
        .map(|o| String::from_utf8_lossy(&o.stdout).into_owned())
        .unwrap_or_default();
    let mine = plat.descendants(std::process::id() as i32);
    let competitors = monitor::sample::attribute(&ps, &|cmd| {
        // Exclude our own tree by PID rather than by name. The Python
        // matched substrings and so never excluded `Validate`, which meant
        // VAL's bursts of a full core counted as foreign competition on
        // every temporal board.
        let _ = &mine;
        cmd.contains("crucible") || cmd.contains("Validate") || cmd.ends_with("/ff")
    });
    let total = competitors.values().sum();
    Sample {
        at: now_epoch(),
        idle_pct: None,
        competitors,
        competitors_total: total,
        loadavg1: None,
        swap_mb: plat.swap_used_mb(),
        cpu_speed_limit: plat.cpu_speed_limit(),
        mem_pressure: plat.memory_pressure_level(),
    }
}

fn now_epoch() -> f64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| (d.as_secs_f64() * 10.0).round() / 10.0)
        .unwrap_or(0.0)
}

impl<'a> SweepRunner<'a> {
    /// Everything the runner needs that is not the manifest itself. Grouped
    /// because nine positional arguments is more than anyone can keep straight,
    /// and three of them are booleans.
    pub fn new(manifest: &'a Manifest, setup: Setup<'_>) -> anyhow::Result<Self> {
        let Setup {
            repo,
            set,
            engine,
            val,
            quiet_only,
            max_passes,
            shared,
            rule,
            admit_below_full,
            progress,
            pack,
            capable,
            db,
            stage,
        } = setup;
        let spec = manifest
            .set(set)
            .with_context(|| format!("no set {set:?} in the manifest"))?;
        let corpus_dir = std::env::var_os("FERROPLAN_IPC_CORPUS")
            .map(PathBuf::from)
            .unwrap_or_else(|| repo.join("benchmarks/.ipc-corpus"));

        let mut boards = Vec::new();
        let mut warnings = Vec::new();
        let mut absent = Vec::new();
        for (position, id) in spec.boards.iter().enumerate() {
            let Some(b) = manifest.board(id) else {
                continue;
            };
            // A board this engine cannot run is SKIPPED, with ZERO rows
            // written -- never a board of zeroes. "The feature does not exist,
            // and recording a zero would be a lie the standings would then
            // average." Old tags predate Mode::Optimal, and a stale binary can
            // predate a whole track. The skip gets a `feature-absent` pass row
            // (0.26 F6 Part 2) so it has provenance, not just an absence.
            if let Some(mode) = &b.mode {
                if !capable(mode) {
                    crate::say!(
                        "SKIP {id}: this engine has no --mode {mode} -- \
                         feature-absent, not zero coverage"
                    );
                    let cfg = board_cfg(manifest, b);
                    absent.push((
                        board_key_for_sweep(manifest, b, &cfg),
                        db::board_facts(b, manifest, None),
                    ));
                    continue;
                }
            }
            let Some(track) = manifest.track(&b.track) else {
                continue;
            };
            let sel = track.selector().map_err(|e| anyhow::anyhow!("{e}"))?;
            let walk = corpus::variants(&corpus_dir, &track.ipcs, &|v| sel.is_match(v));
            warnings.extend(walk.warnings);
            let mut instances = Vec::new();
            for v in &walk.variants {
                for i in corpus::instances(v, 0, &mut warnings) {
                    instances.push((v.ipc.clone(), v.name.clone(), i));
                }
            }
            let cfg = board_cfg(manifest, b);
            boards.push(Board {
                spec: b.clone(),
                position,
                instances,
                banked: Default::default(),
                rows: Default::default(),
                ids: None,
                key: board_key_for_sweep(manifest, b, &cfg),
                facts: db::board_facts(b, manifest, None),
                reused: 0,
            });
        }
        for w in &warnings {
            eprintln!("WARN {w}");
        }

        // Feature-absent boards get their pass row now, before anything is
        // measured: the skip is a verdict with provenance, written once per
        // run (the `''` source identity, like the live pass).
        if let Some(ctx) = &db {
            for (key, facts) in absent {
                let rec = db::BoardPassRec {
                    board: key,
                    board_facts: facts,
                    engine: ctx.engine.clone(),
                    engine_facts: ctx.engine_facts.clone(),
                    started_at: Some(format!("{:.1}", now_epoch())),
                    ended_at: Some(format!("{:.1}", now_epoch())),
                    verdict: db::PassVerdict::FeatureAbsent,
                    ran: 0,
                    reused: 0,
                    done_marker: None,
                    raw_path: None,
                    conditions_path: None,
                    sample_interval: Some(ctx.interval),
                    source_path: None,
                };
                if let Err(e) = ctx.db.writer().board_pass(rec) {
                    eprintln!("!! could not record the feature-absent pass: {e}");
                }
            }
        }

        let mut runner = SweepRunner {
            stage: stage.unwrap_or_else(|| repo.join(&spec.stage)),
            repo: repo.to_path_buf(),
            manifest,
            engine,
            val,
            boards,
            shared,
            rule,
            admit_below_full,
            progress,
            pack,
            plat: platform::host(),
            stop: false,
            quiet_only,
            max_passes,
            passes: 0,
            db,
        };
        runner.seed_from_db()?;
        runner.publish_boards();
        Ok(runner)
    }

    /// Build the dashboard's board rows from what the runner knows: every
    /// instance, its predecessor's row from the promoted raw, and the row
    /// this sweep already holds for it.
    fn publish_boards(&self) {
        let Some(p) = &self.progress else {
            return;
        };
        use crate::tui::app::{BoardRow, Cell, InstanceCell};
        let mut rows = Vec::new();
        let mut ids = Vec::new();
        for b in &self.boards {
            let cfg = board_cfg(self.manifest, &b.spec);
            let prev = prior_rows(&self.repo, &b.spec.raw);
            let cells = b
                .instances
                .iter()
                .map(|(ipc, variant, inst)| {
                    let key = instance_key(ipc, variant, &inst.label);
                    let mut c = InstanceCell {
                        variant: variant.clone(),
                        label: inst.label.clone(),
                        ..Default::default()
                    };
                    if let Some((solved, secs)) = prev.get(&format!("{variant}/{}", inst.label)) {
                        c.prev_solved = Some(*solved);
                        c.prev_secs = *secs;
                    }
                    if let Some(r) = b.rows.get(&key) {
                        c.this_solved = Some(r.solved);
                        c.this_secs = r.time.as_ref().and_then(|t| t.as_f64());
                        let banked = b.banked.contains(&key);
                        c.cell = match (r.solved, banked) {
                            _ if r.val == Some(false) => Cell::Error,
                            (true, _) => Cell::SolvedClean,
                            (false, true) => Cell::TimeoutBanked,
                            (false, false) => Cell::Owed,
                        };
                        c.attempt = 1;
                    }
                    c
                })
                .collect();
            rows.push(BoardRow {
                id: b.spec.id.clone(),
                label: b.spec.label.clone(),
                budget_secs: cfg.timeout_secs,
                threads: cfg.threads,
                cells,
            });
            ids.push(b.ids);
        }
        let mut g = p.lock().unwrap();
        g.boards = rows;
        g.ids = ids;
    }

    fn progress_cell(
        &self,
        idx: usize,
        i: usize,
        f: impl FnOnce(&mut crate::tui::app::InstanceCell),
    ) {
        if let Some(p) = &self.progress {
            let mut g = p.lock().unwrap();
            if let Some(c) = g.boards.get_mut(idx).and_then(|b| b.cells.get_mut(i)) {
                f(c);
            }
        }
    }

    /// The restart: every row and every clean verdict the database already
    /// holds for these boards under THIS engine comes back, and the stage is
    /// regenerated from them. Rows measured by another binary do not resolve
    /// to this `(board, engine)` and are invisible, which is the BLAKE3 gate
    /// at database granularity. Rows imported from artifacts carry timing
    /// `unknown`, never `clean`, so they are kept and re-run -- fail closed;
    /// a needless re-run costs sixty seconds.
    fn seed_from_db(&mut self) -> anyhow::Result<()> {
        let Some(ctx) = &self.db else {
            return Ok(());
        };
        let mut seeded = Vec::new();
        for (idx, b) in self.boards.iter_mut().enumerate() {
            let (bid, eid) = ctx
                .db
                .writer()
                .resolve(
                    b.key.clone(),
                    b.facts.clone(),
                    ctx.engine.clone(),
                    ctx.engine_facts.clone(),
                )
                .context("resolving the board in the database")?;
            b.ids = Some((bid, eid));
            let rows = ctx.reader.export_rows(bid, eid)?;
            let clean = ctx.reader.banked_instances(bid, eid)?;
            if rows.is_empty() {
                continue;
            }
            // Only instances this sweep actually enumerates count: a row for
            // an instance the corpus no longer has is kept in the database and
            // ignored here, exactly as an export would ignore it.
            let known: std::collections::BTreeSet<String> = b
                .instances
                .iter()
                .map(|(ipc, v, i)| instance_key(ipc, v, &i.label))
                .collect();
            for r in rows {
                let key = instance_key(
                    r.ipc.as_deref().unwrap_or(""),
                    &r.variant,
                    &db::InstanceKey::of(&r.instance).label,
                );
                if known.contains(&key) {
                    b.rows.insert(key, r);
                }
            }
            for (ipc, variant, label) in clean {
                let key = instance_key(ipc.as_deref().unwrap_or(""), &variant, &label);
                if known.contains(&key) {
                    b.banked.insert(key);
                }
            }
            b.reused = b.rows.len();
            if b.reused > 0 {
                crate::say!(
                    "resume  {:<22} {} row(s) read back, {} banked -- {} still owed",
                    b.spec.id,
                    b.reused,
                    b.banked.len(),
                    b.remaining()
                );
                seeded.push(idx);
            }
        }
        for idx in seeded {
            if let Err(e) = self.write_artifacts(idx) {
                eprintln!("!! could not write {}: {e}", self.boards[idx].spec.id);
            }
        }
        Ok(())
    }

    pub fn total_instances(&self) -> usize {
        self.boards.iter().map(|b| b.instances.len()).sum()
    }

    /// Is the machine in its known-unattended window right now?
    /// Write the board's raw and its summary, in the corpus's canonical order.
    ///
    /// After EVERY attempt, not only at the end. `ipc67.py` opens its raw with
    /// `"w"`, which truncates -- which is why the shell drivers copy the file
    /// aside before each pass. Here the rows are held and rewritten, so a
    /// sweep killed mid-board still leaves everything it measured.
    fn write_artifacts(&self, idx: usize) -> std::io::Result<()> {
        let b = &self.boards[idx];
        std::fs::create_dir_all(&self.stage)?;

        // Canonical order: the order the instances were enumerated in, which is
        // the order every committed board raw is written in, regardless of the
        // order the scheduler measured them.
        let mut jsonl = String::new();
        let mut ordered: Vec<&crucible_publish::RawRow> = Vec::new();
        for (ipc, variant, i) in &b.instances {
            if let Some(r) = b.rows.get(&instance_key(ipc, variant, &i.label)) {
                crucible_publish::write_row(r, &mut jsonl);
                jsonl.push('\n');
                ordered.push(r);
            }
        }
        std::fs::write(self.stage.join(format!("{}.jsonl", b.spec.id)), &jsonl)?;

        let cfg = board_cfg(self.manifest, &b.spec);
        let owned: Vec<crucible_publish::RawRow> = ordered.into_iter().cloned().collect();
        let md = crucible_core::artifact::board_md::render(
            &crucible_core::artifact::board_md::BoardHeader {
                track: b.spec.track.clone(),
                timeout_s: cfg.timeout_secs as i64,
                jobs: cfg.jobs,
                mode: cfg.mode.clone(),
                val: self.val.is_some(),
                reused_total: b.reused,
                resume_raw: None,
            },
            &crucible_core::artifact::board_md::summarize_variants(&owned, None),
            None,
        );
        std::fs::write(self.stage.join(format!("{}.md", b.spec.id)), md)?;

        // The zero-byte marker, and the whole board-level checkpoint. Written
        // only when every instance has a CLEAN row -- refuse-not-bank, at row
        // granularity instead of board granularity.
        let done = self.stage.join(format!("{}.done", b.spec.id));
        if b.remaining() == 0 {
            std::fs::write(&done, "")?;
        } else if done.exists() {
            std::fs::remove_file(&done)?;
        }
        Ok(())
    }

    /// The `.done` marker's provenance: what this attempt measured, what it
    /// reused, and whether the board is banked. `''` as the source is the
    /// live-pass identity, so re-recording after every attempt updates one
    /// row rather than adding one per pass.
    fn record_pass(&self, idx: usize, ran: usize, started_at: f64) {
        let Some(ctx) = &self.db else {
            return;
        };
        let b = &self.boards[idx];
        let rec = db::BoardPassRec {
            board: b.key.clone(),
            board_facts: b.facts.clone(),
            engine: ctx.engine.clone(),
            engine_facts: ctx.engine_facts.clone(),
            started_at: Some(format!("{started_at:.1}")),
            ended_at: Some(format!("{:.1}", now_epoch())),
            verdict: if b.remaining() == 0 {
                db::PassVerdict::Clean
            } else {
                db::PassVerdict::Degraded
            },
            ran: ran as i64,
            reused: b.reused as i64,
            done_marker: (b.remaining() == 0).then(|| {
                self.stage
                    .join(format!("{}.done", b.spec.id))
                    .display()
                    .to_string()
            }),
            raw_path: Some(
                self.stage
                    .join(format!("{}.jsonl", b.spec.id))
                    .display()
                    .to_string(),
            ),
            conditions_path: None,
            sample_interval: Some(ctx.interval),
            source_path: None,
        };
        if let Err(e) = ctx.db.writer().board_pass(rec) {
            eprintln!("!! could not record the pass for {}: {e}", b.spec.id);
        }
    }
}

impl Runner for SweepRunner<'_> {
    fn boards(&mut self) -> Vec<BoardState> {
        self.boards
            .iter()
            .map(|b| BoardState {
                id: b.spec.id.clone(),
                position: b.position,
                remaining: b.remaining(),
            })
            .collect()
    }

    fn attempt(&mut self, board: &BoardState) -> Attempt {
        let Some(idx) = self.boards.iter().position(|b| b.spec.id == board.id) else {
            return Attempt::default();
        };
        let cfg = board_cfg(self.manifest, &self.boards[idx].spec);
        let plan_dir = self.stage.join("plans").join(&self.boards[idx].spec.id);
        let _ = std::fs::create_dir_all(&self.stage);
        let pass_started = now_epoch();

        let todo: Vec<usize> = (0..self.boards[idx].instances.len())
            .filter(|i| {
                let (ipc, variant, inst) = &self.boards[idx].instances[*i];
                !self.boards[idx]
                    .banked
                    .contains(&instance_key(ipc, variant, &inst.label))
            })
            .collect();

        // THE CASCADE (decision 2026-09-05): what the predecessor solved fast
        // runs packed as wide as the cores and the memory allow; a packed
        // miss falls to the narrow batch; a narrow miss runs solo. All in
        // this pass, so packing can only ever cost time.
        let (wide, mut narrow, mut solo) = self.classify(idx, &cfg, &todo);
        let mut stats = PassStats::default();
        if !wide.is_empty() {
            crate::say!(
                "   {:<22} packed: {} wide x{} ({:.1} GB budget), {} narrow, {} solo",
                self.boards[idx].spec.id,
                wide.len(),
                self.pack.width,
                self.mem_budget() as f64 / (1u64 << 30) as f64,
                narrow.len(),
                solo.len()
            );
        }
        let wide = self.expected_bytes(idx, &cfg, &wide);
        let missed = self.run_batch(idx, &cfg, &plan_dir, wide, self.pack.width, &mut stats);
        narrow.extend(missed);
        let narrow = self.expected_bytes(idx, &cfg, &narrow);
        let missed = self.run_batch(
            idx,
            &cfg,
            &plan_dir,
            narrow,
            self.pack.narrow_width,
            &mut stats,
        );
        solo.extend(missed);
        let solo = self.expected_bytes(idx, &cfg, &solo);
        let _ = self.run_batch(idx, &cfg, &plan_dir, solo, 1, &mut stats);

        if !stats.tally.is_empty() {
            let line: Vec<String> = stats
                .tally
                .iter()
                .map(|(k, v)| format!("{k} {v}"))
                .collect();
            crate::say!(
                "   {:<22} verdicts: {}",
                self.boards[idx].spec.id,
                line.join(", ")
            );
        }
        if let Err(e) = self.write_artifacts(idx) {
            eprintln!("!! could not write {}: {e}", self.boards[idx].spec.id);
        }
        self.record_pass(idx, stats.ran, pass_started);

        Attempt {
            banked: stats.banked,
            remaining: self.boards[idx].remaining(),
            dirty: stats.all_dirty && stats.banked == 0,
        }
    }

    fn stopped(&mut self) -> bool {
        if exec::interrupted() {
            self.stop = true;
        }
        self.stop
    }

    fn wait(&mut self, backoff: Duration) -> Next {
        if self.stop {
            return Next::Stop;
        }
        if let Some(max) = self.max_passes {
            if self.passes >= max {
                crate::say!("   (--max-passes {max} reached)");
                return Next::Stop;
            }
        }
        let until = Instant::now() + backoff.min(Duration::from_secs(60));
        while Instant::now() < until {
            if exec::interrupted() {
                self.stop = true;
                return Next::Stop;
            }
            std::thread::sleep(Duration::from_secs(1));
        }
        Next::Continue
    }

    fn event(&mut self, event: Event) {
        // The loop neither prints nor logs, so that its behaviour can be
        // asserted rather than scraped. This is where a human gets told.
        match event {
            Event::PassStarted { pass, boards } => {
                self.passes = pass;
                crate::say!("== pass {pass} -- {} board(s) outstanding", boards.len())
            }
            Event::Attempted {
                board,
                banked,
                before,
                after,
                dirty,
            } => crate::say!(
                "   {board:<22} banked {banked:>4}   {before} -> {after}{}",
                if dirty {
                    "   [DEGRADED -- not banked, work owed]"
                } else {
                    ""
                }
            ),
            Event::Unproductive { board, remaining } => {
                crate::say!(
                    "!! {board}: no progress and the box was quiet -- {remaining} still owed"
                )
            }
            Event::Grew {
                board,
                before,
                after,
            } => {
                crate::say!("!! {board}: remaining GREW {before} -> {after} -- a runner bug")
            }
            Event::Stalled {
                consecutive,
                backoff,
                remaining,
            } => crate::say!(
                "!! stalled after {consecutive} passes; backing off {backoff:?}, {remaining} owed"
            ),
            Event::Finished { passes, banked } => {
                crate::say!("SWEEP COMPLETE -- {banked} banked in {passes} pass(es)")
            }
            Event::Stopped { passes, remaining } => crate::say!(
                "stopped after {passes} pass(es); {remaining} still owed -- \
                 the next run picks them up"
            ),
        }
    }
}

/// Run a set's sweep to completion, or until the operator stops it.
/// How this invocation differs from a plain resident sweep.
pub struct Opts<'a> {
    pub set: &'a str,
    /// Print the log instead of hosting the dashboard. The default when
    /// stdout is not a terminal.
    pub headless: bool,
    /// Refuse unless the binary reports this. The gate every sweep driver opens
    /// with: measure the CANDIDATE, not whatever happens to be built. `None`
    /// defers to the set's own `requires_version`.
    pub require_version: Option<&'a str>,
    pub quiet_only: bool,
    pub dry_run: bool,
    /// `None` is the resident behaviour: a board that cannot bank because the
    /// box is never quiet is waiting, not failing.
    pub max_passes: Option<u32>,
    /// The restore hatch: the pre-database path, bit for bit.
    pub no_db: bool,
}

/// What the sweep publishes for the dashboard: every board's cells, the
/// run in flight, and where the database is. Written by the runner at each
/// instance's start and end, read by the UI thread once per tick. Nothing
/// here blocks a measurement: the lock is held for a map update.
#[derive(Default)]
pub struct Progress {
    pub boards: Vec<crate::tui::app::BoardRow>,
    /// Every run in flight, one per worker slot.
    pub running: Vec<Running>,
    pub db_path: Option<PathBuf>,
    pub engine_ver: String,
    pub engine_hash: String,
    pub started: Option<Instant>,
    /// Instances banked, with the time they banked at: the throughput line.
    pub banked_at: Vec<Instant>,
    /// `(board id, engine id)` per board, for the UI's own reader.
    pub ids: Vec<Option<(i64, i64)>>,
    pub finished: bool,
}

pub struct Running {
    pub slot: usize,
    pub board: usize,
    pub inst: usize,
    pub pid: Option<Pid>,
    pub started: Instant,
    pub budget: Duration,
}

pub type ProgressHandle = Arc<Mutex<Progress>>;

/// What the watcher thread publishes and the runner reads: the throttle
/// level, and the control channel of the child that is running right now.
/// R1 computed the throttle and never delivered it -- `attempt()` built its
/// channel as `let (_tx, rx)` and dropped the sender on the spot, so
/// SUSPENDED never reached a planner (`crucible-spec.md` R2.0). This is the
/// sender, kept.
pub struct Shared {
    level: Mutex<(Level, Option<String>)>,
    /// Every running child's control channel, by attachment id. A throttle
    /// transition or a canary pause reaches all of them.
    children: Mutex<Vec<(u64, mpsc::Sender<Ctl>)>>,
    next_id: std::sync::atomic::AtomicU64,
    /// The canary is reading: nothing of ours may START until it is done.
    /// Pausing the attached children is not enough with ten workers cycling
    /// through short instances -- the next spawn lands inside the reading.
    hold: AtomicBool,
    /// The canary's latest clock factor and when it was read.
    canary: Mutex<Option<(f64, Instant)>>,
}

impl Shared {
    pub fn new() -> Arc<Shared> {
        Arc::new(Shared {
            level: Mutex::new((Level::Full, None)),
            children: Mutex::new(Vec::new()),
            next_id: std::sync::atomic::AtomicU64::new(1),
            hold: AtomicBool::new(false),
            canary: Mutex::new(None),
        })
    }

    pub fn hold(&self, on: bool) {
        self.hold.store(on, Ordering::Relaxed);
    }

    pub fn held(&self) -> bool {
        self.hold.load(Ordering::Relaxed)
    }

    /// How many of our own planners are attached right now.
    pub fn attached(&self) -> usize {
        self.children.lock().unwrap().len()
    }

    pub fn canary(&self) -> Option<f64> {
        self.canary.lock().unwrap().map(|(f, _)| f)
    }

    pub fn set_canary(&self, factor: f64) {
        *self.canary.lock().unwrap() = Some((factor, Instant::now()));
    }

    pub fn level(&self) -> Level {
        self.level.lock().unwrap().0
    }

    pub fn reason(&self) -> Option<String> {
        self.level.lock().unwrap().1.clone()
    }

    pub fn set_level(&self, level: Level, reason: Option<String>) {
        *self.level.lock().unwrap() = (level, reason);
    }

    /// Register the running child's channel. A child that starts while the
    /// box is already POLITE is told so at once, rather than at the next
    /// transition.
    pub fn attach(&self, tx: mpsc::Sender<Ctl>) -> u64 {
        for c in ctl_for(Level::Full, self.level()) {
            let _ = tx.send(c);
        }
        let id = self.next_id.fetch_add(1, Ordering::Relaxed);
        self.children.lock().unwrap().push((id, tx));
        id
    }

    pub fn detach(&self, id: u64) {
        self.children.lock().unwrap().retain(|(i, _)| *i != id);
    }

    /// To every attached child.
    pub fn send(&self, c: Ctl) {
        for (_, tx) in self.children.lock().unwrap().iter() {
            let _ = tx.send(c);
        }
    }
}

/// What a throttle transition tells the running child.
pub fn ctl_for(from: Level, to: Level) -> Vec<Ctl> {
    match (from, to) {
        (Level::Suspended, Level::Suspended) => vec![],
        (_, Level::Suspended) => vec![Ctl::Stop],
        (Level::Suspended, Level::Polite) => vec![Ctl::Cont, Ctl::Demote],
        (Level::Suspended, Level::Full) => vec![Ctl::Cont, Ctl::Promote],
        (Level::Full, Level::Polite) => vec![Ctl::Demote],
        (Level::Polite, Level::Full) => vec![Ctl::Promote],
        (Level::Full, Level::Full) | (Level::Polite, Level::Polite) => vec![],
    }
}

fn level_str(l: Level) -> &'static str {
    match l {
        Level::Full => "full",
        Level::Polite => "polite",
        Level::Suspended => "suspended",
    }
}

/// The throttle's configuration, from the operator's `[contention]` table.
/// The polite threshold is the clean line itself, so the throttle and the
/// window gate cannot disagree about what "busy" means.
pub fn throttle_config(c: &crate::config::Contention) -> monitor::Config {
    let secs = Duration::from_secs;
    monitor::Config {
        polite_threshold_pct: monitor::SAMPLE_CLEAN_PCPU,
        polite_dwell: secs(c.polite_dwell_secs),
        suspend_threshold_pct: c.suspend_threshold_pct,
        resume_dwell: secs(c.resume_dwell_secs),
        game_cpu_threshold_pct: c.game_cpu_threshold_pct,
        game_dwell: secs(c.game_dwell_secs),
        swap_pressure_mb: c.swap_pressure_mb,
    }
}

/// The busiest game process, if any. Presence alone is never enough --
/// Steam idles in the background for weeks, and suspending a three-day
/// sweep because a launcher is open would be its own kind of failure.
fn games_now(plat: &platform::Host) -> monitor::GameState {
    let ps = std::process::Command::new("ps")
        .args(["-Ao", "pid,ppid,pcpu,comm"])
        .output()
        .map(|o| String::from_utf8_lossy(&o.stdout).into_owned())
        .unwrap_or_default();
    let procs = monitor::games::snapshot(plat, &ps);
    monitor::GameState {
        busiest: monitor::GameRules::default().busiest(&procs),
    }
}

/// The canary (`crucible-spec.md` R2.3): one fixed, fast, low-variance
/// instance, run solo `baseline_n` times before the first child to set the
/// baseline (the FASTEST of them -- the least-disturbed reading), then once
/// every `interval` beside whatever is running. Its wall over the baseline
/// is the box's clock factor, stamped on every sample the watcher takes
/// until the next reading, and read back by the referee over a run's
/// window. This is the instrument for what `rho` cannot see: the packing
/// calibration put four planners on four P-cores, each at rho 0.99, and
/// they ran 1.73x slower than solo.
pub struct Canary {
    argv: Vec<String>,
    envs: Vec<(std::ffi::OsString, std::ffi::OsString)>,
    program: PathBuf,
    baseline: Option<Duration>,
    baseline_n: u32,
    interval: Duration,
    max_factor: f64,
    label: String,
}

impl Canary {
    /// `None` when the configured instance is not on disk: a sweep on a box
    /// without the corpus variant runs without a clock, and says so.
    /// The label carries the engine's hash: the canary's time is a
    /// property of the ENGINE as much as the box (the 0.27 successor
    /// generator solved it 2.3x faster than 0.26), so its history and its
    /// baseline are per engine, never shared across them.
    pub fn resolve(
        repo: &Path,
        engine: &Path,
        engine_hash: &str,
        r: &crate::config::Referee,
    ) -> Option<Canary> {
        let corpus_dir = std::env::var_os("FERROPLAN_IPC_CORPUS")
            .map(PathBuf::from)
            .unwrap_or_else(|| repo.join("benchmarks/.ipc-corpus"));
        let vdir = corpus_dir
            .join(&r.canary_ipc)
            .join("domains")
            .join(&r.canary_variant);
        let problem = vdir
            .join("instances")
            .join(format!("instance-{}.pddl", r.canary_instance));
        let mut domain = vdir.join("domain.pddl");
        if !domain.exists() {
            let first: String = r
                .canary_instance
                .chars()
                .take_while(|c| c.is_ascii_digit())
                .collect();
            domain = vdir.join("domains").join(format!("domain-{first}.pddl"));
        }
        if !problem.exists() || !domain.exists() {
            return None;
        }
        let cfg = BoardCfg {
            timeout_secs: 30,
            mode: None,
            jobs: 1,
            threads: 1,
            mem_gb: 2.0,
            env: Default::default(),
            extra_args: vec![],
        };
        Some(Canary {
            argv: crucible_core::sweep::argv(&cfg, &domain, &problem),
            envs: crucible_core::exec::env::build(30, 2.0, &Default::default()),
            program: engine.to_path_buf(),
            baseline: None,
            baseline_n: r.canary_baseline_n.max(1),
            interval: Duration::from_secs(r.canary_interval_secs.max(60)),
            max_factor: r.canary_max_factor,
            label: format!("{}/{}@{engine_hash}", r.canary_variant, r.canary_instance),
        })
    }

    fn run_once(&self) -> Option<Duration> {
        let (_tx, rx) = mpsc::channel::<Ctl>();
        let plat = platform::host();
        let out = exec::run(
            &exec::RunRequest {
                program: &self.program,
                args: &self.argv,
                envs: &self.envs,
                timeout: Duration::from_secs(30),
                mem_cap: crucible_core::platform::MemCap::Off,
                on_spawn: None,
            },
            &plat,
            &rx,
        )
        .ok()?;
        if out.killed.is_some() || out.exit_code != Some(0) {
            return None;
        }
        Some(out.effective)
    }

    /// The baseline: the FASTEST this box has ever run the instance -- the
    /// database's best (`prior_best`, every earlier solo run on this box)
    /// against `baseline_n` solo runs now, each of which is recorded through
    /// `record`. A baseline that is only "the fastest of five at start" is
    /// lenient for the whole sweep whenever the sweep starts on a slow
    /// morning; one taken over the box's history is not.
    pub fn calibrate(&mut self, prior_best: Option<f64>, record: &dyn Fn(f64)) -> Option<Duration> {
        let mut best: Option<f64> = prior_best;
        for _ in 0..self.baseline_n {
            if let Some(d) = self.run_once() {
                let secs = d.as_secs_f64();
                record(secs);
                best = Some(best.map_or(secs, |b| b.min(secs)));
            }
        }
        self.baseline = best.map(Duration::from_secs_f64);
        self.baseline
    }

    /// One reading: `(secs, secs / baseline)`. A faster run than the
    /// baseline becomes the baseline.
    pub fn read(&mut self) -> Option<(f64, f64)> {
        let base = self.baseline?;
        let secs = self.run_once()?.as_secs_f64();
        if secs < base.as_secs_f64() {
            self.baseline = Some(Duration::from_secs_f64(secs));
        }
        Some((secs, secs / base.as_secs_f64().max(0.001)))
    }

    pub fn label(&self) -> &str {
        &self.label
    }
}

/// The box-wide contention timeline: one sample every interval, for as long
/// as the sweep runs, written to the database as telemetry -- batched, and
/// allowed to be lost, unlike a run row. This is what `window_gate` reads.
///
/// Since R2 the watcher also OWNS the throttle: every sample is judged, a
/// transition is published through [`Shared`], delivered to the running
/// child as a `Ctl`, logged as an event and bracketed as a throttle window.
struct Watcher {
    stop: Arc<AtomicBool>,
    join: Option<std::thread::JoinHandle<()>>,
}

impl Watcher {
    fn start(
        writer: db::WriterHandle,
        interval: Duration,
        shared: Arc<Shared>,
        throttle_cfg: monitor::Config,
        quiet_hours: crate::config::QuietHours,
        mut canary: Option<Canary>,
    ) -> Watcher {
        let stop = Arc::new(AtomicBool::new(false));
        let flag = Arc::clone(&stop);
        let join = std::thread::Builder::new()
            .name("crucible-watch".into())
            .spawn(move || {
                let plat = platform::host();
                let mut throttle = Throttle::new(throttle_cfg);
                let mut next = Instant::now();
                let mut window: Option<i64> = None;
                let mut next_canary = canary.as_ref().map(|c| Instant::now() + c.interval);
                while !flag.load(Ordering::Relaxed) {
                    if let (Some(c), Some(t)) = (&mut canary, next_canary) {
                        if Instant::now() >= t {
                            // The canary measures the BOX, so our own planner
                            // is paused for its two seconds: beside one
                            // neighbour it read 1.14x on an idle box (the
                            // calibration's +23 % for a single neighbour),
                            // beside an mco board's eight threads 1.42x.
                            // Suspended time is not charged to the run.
                            let pause = shared.level() != Level::Suspended;
                            let paused = shared.attached();
                            shared.hold(true);
                            if pause {
                                shared.send(Ctl::Stop);
                                std::thread::sleep(Duration::from_millis(400));
                            }
                            let reading = c.read();
                            if pause && shared.level() != Level::Suspended {
                                shared.send(Ctl::Cont);
                            }
                            shared.hold(false);
                            if let Some((secs, _)) = reading {
                                writer.canary(now_epoch(), c.label().to_string(), secs, pause);
                            }
                            if let Some((_, f)) = reading {
                                shared.set_canary(f);
                                let slow = f > c.max_factor;
                                if slow {
                                    crate::say!(
                                        "!! canary {} at {f:.2}x its baseline -- the box is slow",
                                        c.label
                                    );
                                }
                                writer.event(db::EventRec {
                                    at: now_epoch(),
                                    level: if slow { "warn" } else { "info" },
                                    kind: "canary",
                                    run_id: None,
                                    board_id: None,
                                    message: format!(
                                        "{} clock factor {f:.3} ({paused} of ours paused)",
                                        c.label
                                    ),
                                });
                            }
                            next_canary = Some(Instant::now() + c.interval);
                        }
                    }
                    if Instant::now() >= next {
                        let s = sample_box(&plat);
                        let mut rec = db::SampleRec::of(&s);
                        rec.canary_factor = shared.canary();
                        writer.sample(rec);
                        // Overnight the game check is skipped: nobody is
                        // playing at 04:00, so it is a source of false
                        // positives rather than of signal.
                        let qcfg = crate::config::Config {
                            quiet_hours: quiet_hours.clone(),
                            ..Default::default()
                        };
                        let games = if qcfg.in_quiet_hours(minutes_past_midnight())
                            && quiet_hours.skip_game_check
                        {
                            Default::default()
                        } else {
                            games_now(&plat)
                        };
                        if let Some(t) = throttle.on_sample(&s, &games, Instant::now()) {
                            let reason = format!("{:?}", t.reason);
                            shared.set_level(t.to, Some(reason.clone()));
                            for c in ctl_for(t.from, t.to) {
                                shared.send(c);
                            }
                            let at = now_epoch();
                            crate::say!(
                                "!! throttle {} -> {} ({reason})",
                                level_str(t.from),
                                level_str(t.to)
                            );
                            writer.event(db::EventRec {
                                at,
                                level: if t.to == Level::Full { "info" } else { "warn" },
                                kind: "throttle",
                                run_id: None,
                                board_id: None,
                                message: format!(
                                    "{} -> {}: {reason}",
                                    level_str(t.from),
                                    level_str(t.to)
                                ),
                            });
                            if let Some(id) = window.take() {
                                let _ = writer.throttle_close(id, at);
                            }
                            if t.to != Level::Full {
                                window = writer
                                    .throttle_open(db::ThrottleWindowRec {
                                        level: level_str(t.to),
                                        started_at: at,
                                        ended_at: None,
                                        reason: Some(reason),
                                    })
                                    .ok();
                            }
                        }
                        next += interval;
                    }
                    std::thread::sleep(Duration::from_millis(250));
                }
                if let Some(id) = window.take() {
                    let _ = writer.throttle_close(id, now_epoch());
                }
            })
            .expect("spawning the contention watcher");
        Watcher {
            stop,
            join: Some(join),
        }
    }
}

impl Drop for Watcher {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
        if let Some(j) = self.join.take() {
            let _ = j.join();
        }
    }
}

/// Open the database and settle what it owes from before: children a killed
/// supervisor left running are reaped by identity, never by pid alone.
fn open_db(cfg: &crate::config::Config, engine: &crate::repo::Engine) -> anyhow::Result<DbCtx> {
    let db = Db::open(&cfg.db.dir).with_context(|| {
        format!(
            "opening the database at {} (another crucible holding the lock is \
             an answer, not a fault)",
            cfg.db.dir.display()
        )
    })?;
    let reader = db.reader()?;
    let plat = platform::host();
    let orphans = reader.live_children()?;
    if !orphans.is_empty() {
        crate::say!(
            "reap    {} child(ren) recorded by an earlier run",
            orphans.len()
        );
        let children: Vec<orphan::LiveChild> = orphans
            .iter()
            .map(|c| orphan::LiveChild {
                pid: c.pid,
                pgid: c.pgid,
                run_id: c.run_id,
                binary_path: c.binary_path.clone(),
                proc_start_tvsec: c.proc_start_tvsec,
                spawned_at: c.spawned_at,
                stopped: c.stopped,
            })
            .collect();
        for r in orphan::reap(&children, &plat) {
            match &r {
                orphan::Reaped::Killed { pid, pgid } => {
                    crate::say!("        killed {pid} (group {pgid}) -- verified ours")
                }
                orphan::Reaped::Vanished { pid } => crate::say!("        {pid} already gone"),
                orphan::Reaped::Recycled {
                    pid,
                    expected,
                    found,
                } => crate::say!(
                    "        {pid} is somebody else's now ({found}, not {expected}) -- \
                     NOT signalled"
                ),
            }
            // Every row is closed: a killed child is gone, a vanished one was
            // already gone, and a recycled pid is a ghost this table must not
            // keep claiming.
            let _ = db.writer().child_gone(r.pid());
        }
    }
    Ok(DbCtx {
        db,
        reader,
        engine: db::EngineKey {
            blake3: Some(engine.blake3.clone()),
            ver: Some(engine.ver.clone()),
        },
        engine_facts: db::EngineFacts {
            tag: engine.tag.clone(),
            binary_path: Some(engine.path.display().to_string()),
            ..Default::default()
        },
        interval: cfg.contention.sample_interval_secs as f64,
    })
}

pub fn run(repo: &Path, cfg: &crate::config::Config, o: Opts<'_>) -> anyhow::Result<()> {
    let manifest = crate::load_manifest(repo)?;
    let bin = crate::repo::candidate_path(repo);
    let engine = crate::repo::Engine::probe(&bin)?;
    // The gate every sweep driver opens with: measure the CANDIDATE, not
    // whatever happens to be built. The set may name the version itself.
    let set_wants = manifest.set(o.set).and_then(|s| s.requires_version.clone());
    if let Some(want) = o.require_version.map(str::to_string).or(set_wants) {
        engine.require_version(&want)?;
    }
    run_engine(repo, cfg, o, &manifest, engine, None)
}

/// The sweep proper, for an engine already identified: the candidate (above)
/// or a built tag (`backfill`). `stage` overrides the set's own staging dir.
///
/// Hosts the dashboard (`crucible-spec.md` R2.4) unless `--headless` or
/// stdout is not a terminal: the sweep runs on a scoped thread, the UI on
/// this one; `q` cancels the running child the way ^C does and the sweep
/// stops with everything banked kept.
pub fn run_engine(
    repo: &Path,
    cfg: &crate::config::Config,
    o: Opts<'_>,
    manifest: &Manifest,
    engine: crate::repo::Engine,
    stage: Option<PathBuf>,
) -> anyhow::Result<()> {
    let tui = !o.headless && !o.dry_run && std::io::IsTerminal::is_terminal(&std::io::stdout());
    let shared = Shared::new();
    if !tui {
        return sweep_body(repo, cfg, o, manifest, engine, stage, shared, None);
    }
    let progress: ProgressHandle = Arc::new(Mutex::new(Progress {
        engine_ver: engine.ver.clone(),
        engine_hash: engine.short_hash(),
        started: Some(Instant::now()),
        ..Default::default()
    }));
    crate::out::quiet(true);
    let result = std::thread::scope(|sc| {
        let body = {
            let shared = Arc::clone(&shared);
            let progress = Arc::clone(&progress);
            sc.spawn(move || {
                let r = sweep_body(
                    repo,
                    cfg,
                    o,
                    manifest,
                    engine,
                    stage,
                    shared,
                    Some(Arc::clone(&progress)),
                );
                progress.lock().unwrap().finished = true;
                r
            })
        };
        let mut feed = crate::tui::feed::Feed::new(Arc::clone(&progress), Arc::clone(&shared));
        let ui = crate::tui::run::run(
            cfg.ui.fps,
            &cfg.ui.banner_text,
            |prev| feed.next(prev),
            |action, _| {
                if action == crate::tui::run::Action::Quit {
                    exec::set_interrupted(true);
                }
            },
        );
        // The screen is given back before the sweep is joined, so the
        // operator sees the stop happen rather than a frozen frame.
        drop(ui);
        body.join()
            .unwrap_or_else(|_| Err(anyhow::anyhow!("the sweep thread panicked")))
    });
    crate::out::quiet(false);
    crate::out::flush_to_stdout();
    result
}

#[allow(clippy::too_many_arguments)]
fn sweep_body(
    repo: &Path,
    cfg: &crate::config::Config,
    o: Opts<'_>,
    manifest: &Manifest,
    engine: crate::repo::Engine,
    stage: Option<PathBuf>,
    shared: Arc<Shared>,
    progress: Option<ProgressHandle>,
) -> anyhow::Result<()> {
    let Opts {
        set,
        require_version: _,
        headless: _,
        quiet_only,
        dry_run,
        max_passes,
        no_db,
    } = o;
    let val = crucible_core::validate::find(repo, cfg.sweep.validator.as_deref());
    if val.is_none() {
        eprintln!(
            "note: no validator found -- boards will render VAL-unavailable, \
             which is NOT the same as a rejected plan"
        );
    }
    // A three-day sweep that sleeps at hour four is not a sweep.
    let _awake = platform::host().keep_awake();
    // ^C or SIGTERM cancels the running child (SIGTERM, grace, SIGKILL to
    // its group) and stops the loop with everything banked kept. The 0.26
    // sweep was stopped with SIGTERM and left its planner under pid 1.
    exec::install_interrupt_handler();

    crate::say!("engine  {} [{}]", engine.ver, engine.short_hash());
    let dbctx = if dry_run || no_db {
        None
    } else {
        Some(open_db(cfg, &engine)?)
    };
    // The canary's baseline is taken NOW, solo, before any child exists.
    let canary = if dry_run {
        None
    } else {
        match Canary::resolve(repo, &engine.path, &engine.short_hash(), &cfg.referee) {
            Some(mut c) => {
                let prior = dbctx
                    .as_ref()
                    .and_then(|d| d.reader.canary_best(c.label()).ok().flatten());
                let label = c.label().to_string();
                let record = |secs: f64| {
                    if let Some(d) = &dbctx {
                        d.db.writer().canary(now_epoch(), label.clone(), secs, true);
                    }
                };
                match c.calibrate(prior, &record) {
                    Some(b) => {
                        crate::say!(
                            "canary  {} baseline {:.3} s ({}); read every {} s, slow above {:.2}x",
                            c.label(),
                            b.as_secs_f64(),
                            match prior {
                                Some(p) => format!(
                                    "this box's best of {p:.3} s against {} runs now",
                                    c.baseline_n
                                ),
                                None => format!("fastest of {} runs, no history yet", c.baseline_n),
                            },
                            c.interval.as_secs(),
                            c.max_factor
                        );
                        shared.set_canary(1.0);
                        Some(c)
                    }
                    None => {
                        eprintln!(
                            "note: the canary {} did not solve; sweeping without a clock",
                            c.label()
                        );
                        None
                    }
                }
            }
            None => {
                eprintln!(
                    "note: canary instance {}/{} not in the corpus; sweeping without a clock",
                    cfg.referee.canary_variant, cfg.referee.canary_instance
                );
                None
            }
        }
    };
    if let (Some(p), Some(c)) = (&progress, &dbctx) {
        p.lock().unwrap().db_path = Some(c.db.path().to_path_buf());
    }
    let _watcher = dbctx.as_ref().map(|c| {
        crate::say!("db      {}", c.db.path().display());
        Watcher::start(
            c.db.writer().clone(),
            Duration::from_secs(cfg.contention.sample_interval_secs.max(1)),
            Arc::clone(&shared),
            throttle_config(&cfg.contention),
            cfg.quiet_hours.clone(),
            canary,
        )
    });

    let mut runner = SweepRunner::new(
        manifest,
        Setup {
            repo,
            set,
            engine: SweepEngine {
                path: engine.path.clone(),
                ver: engine.ver.clone(),
                // Under --no-db the rows stay unstamped, as the pre-database
                // binary wrote them.
                blake3: if no_db {
                    String::new()
                } else {
                    engine.blake3.clone()
                },
            },
            val,
            quiet_only,
            max_passes,
            shared: Arc::clone(&shared),
            rule: referee::Rule {
                rho_min: cfg.referee.cpu_ratio_min,
                swap_growth_mb: cfg.referee.swap_growth_mb,
                canary_max_factor: cfg.referee.canary_max_factor,
            },
            admit_below_full: cfg.referee.admit_below_full,
            progress: progress.clone(),
            // --quiet-only is the R1 shape: one at a time, FULL only.
            pack: if quiet_only {
                Pack::solo()
            } else {
                Pack::from_config(&cfg.scheduler)
            },
            capable: &|m| engine.supports_mode(m),
            db: dbctx,
            stage,
        },
    )?;
    crate::say!("set     {set}: {} instances", runner.total_instances());

    if dry_run {
        // Everything up to the first spawn: the boards, their row-identity
        // tuples, and how much work each owes. Enough to check a sweep before
        // committing days of the machine to it.
        crate::say!();
        crate::say!(
            "{:<22} {:>7} {:>5} {:>5} {:>7} {:<10} notes",
            "board",
            "insts",
            "wall",
            "jobs",
            "threads",
            "mode"
        );
        for b in &runner.boards {
            let c = board_cfg(manifest, &b.spec);
            let mut notes = Vec::new();
            if c.threads > 1 {
                notes.push("mco wall-clock rule: jobs forced to 1".to_string());
            }
            if b.spec
                .timeout_secs
                .is_some_and(|t| t as f64 != b.spec.budget_secs)
            {
                notes.push(format!("TIER MOVE: scored at {}s", b.spec.budget_secs));
            }
            if !c.env.is_empty() {
                notes.push(format!("env {:?}", c.env));
            }
            crate::say!(
                "{:<22} {:>7} {:>5} {:>5} {:>7} {:<10} {}",
                b.spec.id,
                b.instances.len(),
                c.timeout_secs,
                c.jobs,
                c.threads,
                c.mode.clone().unwrap_or_else(|| "auto".into()),
                notes.join("; ")
            );
        }
        crate::say!();
        crate::say!("dry run -- nothing measured, nothing written");
        return Ok(());
    }

    let out = sched::run(
        &mut runner,
        &LoopConfig {
            stall_after: cfg.scheduler.stall_attempts,
            ..Default::default()
        },
    );
    if !out.complete {
        // NOT an error. A board that could not bank because the box was never
        // quiet has lost nothing -- every row it measured is on disk, and the
        // next run picks up exactly what is still owed.
        crate::say!(
            "{} instance(s) still owed -- rows are written, nothing is lost",
            out.remaining
        );
    }
    Ok(())
}

/// Local minutes past midnight, without a timezone dependency: the offset comes
/// from the platform's own idea of local time.
fn minutes_past_midnight() -> u32 {
    let secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    // `date +%H%M` is the only portable way to get LOCAL time here without
    // pulling in a timezone crate for one number, and it is read once per
    // instance rather than per sample.
    if let Ok(o) = std::process::Command::new("date").arg("+%H %M").output() {
        let s = String::from_utf8_lossy(&o.stdout);
        let mut it = s.split_whitespace();
        if let (Some(h), Some(m)) = (it.next(), it.next()) {
            if let (Ok(h), Ok(m)) = (h.parse::<u32>(), m.parse::<u32>()) {
                return h * 60 + m;
            }
        }
    }
    ((secs % 86_400) / 60) as u32
}

#[cfg(test)]
mod r2_tests {
    use super::*;

    /// Every transition delivers what the child needs, and nothing else: a
    /// stopped child is continued before it is demoted or promoted, and a
    /// no-op transition sends nothing.
    #[test]
    fn transitions_map_to_control_messages() {
        use Level::*;
        assert_eq!(ctl_for(Full, Suspended), vec![Ctl::Stop]);
        assert_eq!(ctl_for(Polite, Suspended), vec![Ctl::Stop]);
        assert_eq!(ctl_for(Suspended, Polite), vec![Ctl::Cont, Ctl::Demote]);
        assert_eq!(ctl_for(Suspended, Full), vec![Ctl::Cont, Ctl::Promote]);
        assert_eq!(ctl_for(Full, Polite), vec![Ctl::Demote]);
        assert_eq!(ctl_for(Polite, Full), vec![Ctl::Promote]);
        assert!(ctl_for(Full, Full).is_empty());
        assert!(ctl_for(Suspended, Suspended).is_empty());
    }

    /// A child attached while the box is already POLITE is demoted at once;
    /// a later transition reaches it; after detach nothing is sent anywhere.
    #[test]
    fn the_shared_state_delivers_to_the_attached_child() {
        let shared = Shared::new();
        shared.set_level(Level::Polite, Some("test".into()));
        let (tx, rx) = mpsc::channel();
        let id = shared.attach(tx);
        assert_eq!(rx.try_recv(), Ok(Ctl::Demote));
        let (tx2, rx2) = mpsc::channel();
        let id2 = shared.attach(tx2);
        assert_eq!(rx2.try_recv(), Ok(Ctl::Demote));
        shared.send(Ctl::Stop);
        assert_eq!(rx.try_recv(), Ok(Ctl::Stop));
        assert_eq!(
            rx2.try_recv(),
            Ok(Ctl::Stop),
            "a transition reaches every child"
        );
        assert_eq!(shared.attached(), 2);
        shared.detach(id);
        shared.send(Ctl::Cont);
        assert!(rx.try_recv().is_err());
        assert_eq!(rx2.try_recv(), Ok(Ctl::Cont));
        shared.detach(id2);
        assert_eq!(shared.attached(), 0);
    }
}
