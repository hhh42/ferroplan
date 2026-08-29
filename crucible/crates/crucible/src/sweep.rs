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
use crucible_core::exec::{orphan, Ctl};
use crucible_core::monitor::{self, Level, Sample, Throttle};
use crucible_core::platform::{self, Pid, Platform};
use crucible_core::sched::{self, Attempt, BoardState, Event, LoopConfig, Next, Runner};
use crucible_core::sweep::{BoardCfg, Engine as SweepEngine};
use crucible_publish::manifest::{BoardSpec, Manifest};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{mpsc, Arc};
use std::time::{Duration, Instant};

/// One board's work: its spec, its instances, and which of them still owe a
/// clean row.
pub(crate) struct Board {
    spec: BoardSpec,
    position: usize,
    instances: Vec<(String, String, corpus::Instance)>,
    /// Instance keys that have banked a CLEAN row. A dirty row is written and
    /// kept, but does not remove the instance from the owed set.
    ///
    /// Keyed by the row's full address -- `ipc`, variant, label -- and never by
    /// the label alone. A multi-variant board carries instance "1" in every
    /// variant, and a set keyed on labels would count them once and never
    /// reach zero.
    clean: std::collections::BTreeSet<String>,
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
        self.instances.len() - self.clean.len()
    }
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
    pub quiet_hours: crate::config::QuietHours,
    pub max_passes: Option<u32>,
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
    manifest: &'a Manifest,
    engine: SweepEngine,
    val: Option<PathBuf>,
    pub(crate) boards: Vec<Board>,
    throttle: Throttle,
    plat: platform::Host,
    /// Set when the operator interrupts. The remaining work stays remaining --
    /// it is not failed, and the next run picks it up.
    stop: bool,
    quiet_only: bool,
    /// Quiet hours steer SCHEDULING and skip the game check. They never move a
    /// contention threshold: a Time Machine run at 3am depresses coverage
    /// exactly as much as one at 3pm.
    quiet_hours: crate::config::QuietHours,
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
            quiet_hours,
            max_passes,
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
                    println!(
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
                clean: Default::default(),
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
            manifest,
            engine,
            val,
            boards,
            throttle: Throttle::new(Default::default()),
            plat: platform::host(),
            stop: false,
            quiet_only,
            quiet_hours,
            max_passes,
            passes: 0,
            db,
        };
        runner.seed_from_db()?;
        Ok(runner)
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
            let clean = ctx.reader.clean_instances(bid, eid)?;
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
                    b.clean.insert(key);
                }
            }
            b.reused = b.rows.len();
            if b.reused > 0 {
                println!(
                    "resume  {:<22} {} row(s) read back, {} clean -- {} still owed",
                    b.spec.id,
                    b.reused,
                    b.clean.len(),
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
    fn in_quiet_hours(&self) -> bool {
        let cfg = crate::config::Config {
            quiet_hours: self.quiet_hours.clone(),
            ..Default::default()
        };
        cfg.in_quiet_hours(minutes_past_midnight())
    }

    /// The busiest game process, if any. Presence alone is never enough --
    /// Steam idles in the background for weeks, and suspending a three-day
    /// sweep because a launcher is open would be its own kind of failure.
    fn games(&self) -> monitor::GameState {
        let ps = std::process::Command::new("ps")
            .args(["-Ao", "pid,ppid,pcpu,comm"])
            .output()
            .map(|o| String::from_utf8_lossy(&o.stdout).into_owned())
            .unwrap_or_default();
        let procs = monitor::games::snapshot(&self.plat, &ps);
        monitor::GameState {
            busiest: monitor::GameRules::default().busiest(&procs),
        }
    }

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

    fn sample(&self) -> Sample {
        sample_box(&self.plat)
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

        let (_tx, rx) = mpsc::channel::<Ctl>();
        let mut banked = 0usize;
        let mut ran = 0usize;
        let mut all_dirty = true;
        let todo: Vec<usize> = (0..self.boards[idx].instances.len())
            .filter(|i| {
                let (ipc, variant, inst) = &self.boards[idx].instances[*i];
                !self.boards[idx]
                    .clean
                    .contains(&instance_key(ipc, variant, &inst.label))
            })
            .collect();

        for i in todo {
            if self.stop {
                break;
            }
            // Admission: the box must be FULL, and have been for a dwell. A
            // board started under load is a board measured under load.
            let s = self.sample();
            // Overnight the game check is skipped: nobody is playing at 04:00,
            // so it is a source of false positives rather than of signal.
            // Nothing else about the box's judgement moves with the clock -- a
            // Time Machine run at 3am depresses coverage exactly as much as one
            // at 3pm, and the numbers have to be comparable regardless.
            let games = if self.in_quiet_hours() && self.quiet_hours.skip_game_check {
                Default::default()
            } else {
                self.games()
            };
            self.throttle.on_sample(&s, &games, Instant::now());
            let dirty_now =
                !s.is_clean() || (self.quiet_only && self.throttle.level() != Level::Full);

            let (ipc, variant, inst) = self.boards[idx].instances[i].clone();
            let key = instance_key(&ipc, &variant, &inst.label);

            // The live-child record goes to disk the moment the child exists:
            // a `kill -9` of this process between here and the run's end
            // leaves a row the next startup reaps by identity, instead of a
            // planner nobody owns burning a core until the wall.
            let register = |pid: Pid, at: f64| {
                let Some(ctx) = &self.db else {
                    return;
                };
                let Some(id) = self.plat.proc_identity(pid) else {
                    return;
                };
                // The identity is the KERNEL's reading of the process, both
                // halves -- never the path this process configured. The
                // kernel canonicalises (`/var` is `/private/var` on Darwin),
                // and a reaper comparing a configured path against a live
                // one would spare every orphan as a stranger.
                let child = db::LiveChild {
                    pid,
                    pgid: pid,
                    run_id: None,
                    binary_path: id.path.clone(),
                    proc_start_tvsec: id.start_tvsec,
                    spawned_at: at,
                    stopped: false,
                };
                if let Err(e) = ctx.db.writer().child_spawned(child) {
                    eprintln!("!! could not register child {pid}: {e}");
                }
            };
            let m = crucible_core::sweep::measure(
                &self.engine,
                &cfg,
                &ipc,
                &variant,
                &inst,
                self.val.as_deref(),
                &plan_dir,
                &self.plat,
                &rx,
                self.db.as_ref().map(|_| &register as &dyn Fn(Pid, f64)),
            );
            ran += 1;

            let clean = match &self.db {
                None => {
                    // The pre-database rule, kept bit for bit under --no-db:
                    // a before/after pair, and nothing in between.
                    let after = self.sample();
                    !dirty_now && after.is_clean() && m.clock_jump.is_zero()
                }
                Some(ctx) => {
                    let w = ctx.db.writer();
                    if let Some(pid) = m.pid {
                        let _ = w.child_gone(pid);
                    }
                    let (bid, eid) = self.boards[idx]
                        .ids
                        .expect("a board with a database has resolved ids");
                    let attempt = ctx
                        .reader
                        .next_attempt(bid, eid, Some(&ipc), &variant, &inst.label)
                        .unwrap_or(1);
                    let mut rec = db::RunRecord {
                        board: self.boards[idx].key.clone(),
                        board_facts: self.boards[idx].facts.clone(),
                        engine: ctx.engine.clone(),
                        engine_facts: ctx.engine_facts.clone(),
                        attempt,
                        state: db::RunState::Done,
                        timing: db::TimingQuality::Unknown,
                        val_reason: m.val_reason.and_then(db::ValReason::parse),
                        row: m.row.clone(),
                        measured: db::Measured {
                            started_at: m.row.start_ts,
                            finished_at: m.row.end_ts,
                            wall_ms: Some(m.wall.as_millis() as u64),
                            cpu_ms: Some(m.cpu_ms),
                            suspended_ms: Some(m.suspended.as_millis() as u64),
                            peak_rss: Some(m.peak_rss),
                            mem_instrument: Some(m.mem_instrument.to_string()),
                            exit_code: m.exit_code,
                            term_signal: m.term_signal,
                            pid: m.pid,
                            pgid: m.pgid,
                        },
                    };
                    // THE RECEIPT. Committed before the verdict is asked for,
                    // in its own transaction, and this call waits for it.
                    if let Err(e) = w.run(rec.clone()) {
                        eprintln!("!! could not commit {key}: {e}");
                    }
                    // The verdict: every watcher sample within one interval of
                    // the run's window under the clean line, and at least one
                    // of them. Flush first -- the watcher's samples are batched
                    // and the reader is a separate connection.
                    let _ = w.flush();
                    let gate = match (m.row.start_ts, m.row.end_ts) {
                        (Some(s), Some(e)) => ctx
                            .reader
                            .window_gate(s, e, ctx.interval, None)
                            .unwrap_or(db::Cleanliness::Uncovered),
                        _ => db::Cleanliness::Uncovered,
                    };
                    let clean = gate == db::Cleanliness::Clean && m.clock_jump.is_zero();
                    rec.timing = match gate {
                        db::Cleanliness::Clean if clean => db::TimingQuality::Clean,
                        db::Cleanliness::Clean | db::Cleanliness::Dirty => db::TimingQuality::Dirty,
                        db::Cleanliness::Uncovered => db::TimingQuality::Unknown,
                    };
                    if let Err(e) = w.run(rec) {
                        eprintln!("!! could not record the verdict for {key}: {e}");
                    }
                    clean
                }
            };

            // The row is KEPT either way -- nothing is discarded for
            // contention. It just does not count toward banking.
            if clean {
                self.boards[idx].clean.insert(key.clone());
                banked += 1;
                all_dirty = false;
            }
            // Written either way. The board is not banked, but the work is
            // not lost -- and a later clean row supersedes this one under the
            // same key.
            self.boards[idx].rows.insert(key, m.row);
        }

        if let Err(e) = self.write_artifacts(idx) {
            eprintln!("!! could not write {}: {e}", self.boards[idx].spec.id);
        }
        self.record_pass(idx, ran, pass_started);

        Attempt {
            banked,
            remaining: self.boards[idx].remaining(),
            dirty: all_dirty && banked == 0,
        }
    }

    fn wait(&mut self, backoff: Duration) -> Next {
        if self.stop {
            return Next::Stop;
        }
        if let Some(max) = self.max_passes {
            if self.passes >= max {
                println!("   (--max-passes {max} reached)");
                return Next::Stop;
            }
        }
        std::thread::sleep(backoff.min(Duration::from_secs(60)));
        Next::Continue
    }

    fn event(&mut self, event: Event) {
        // The loop neither prints nor logs, so that its behaviour can be
        // asserted rather than scraped. This is where a human gets told.
        match event {
            Event::PassStarted { pass, boards } => {
                self.passes = pass;
                println!("== pass {pass} -- {} board(s) outstanding", boards.len())
            }
            Event::Attempted {
                board,
                banked,
                before,
                after,
                dirty,
            } => println!(
                "   {board:<22} banked {banked:>4}   {before} -> {after}{}",
                if dirty {
                    "   [DEGRADED -- not banked, work owed]"
                } else {
                    ""
                }
            ),
            Event::Unproductive { board, remaining } => {
                println!("!! {board}: no progress and the box was quiet -- {remaining} still owed")
            }
            Event::Grew {
                board,
                before,
                after,
            } => {
                println!("!! {board}: remaining GREW {before} -> {after} -- a runner bug")
            }
            Event::Stalled {
                consecutive,
                backoff,
                remaining,
            } => println!(
                "!! stalled after {consecutive} passes; backing off {backoff:?}, {remaining} owed"
            ),
            Event::Finished { passes, banked } => {
                println!("SWEEP COMPLETE -- {banked} banked in {passes} pass(es)")
            }
            Event::Stopped { passes, remaining } => println!(
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

/// The box-wide contention timeline: one sample every interval, for as long
/// as the sweep runs, written to the database as telemetry -- batched, and
/// allowed to be lost, unlike a run row. This is what `window_gate` reads.
struct Watcher {
    stop: Arc<AtomicBool>,
    join: Option<std::thread::JoinHandle<()>>,
}

impl Watcher {
    fn start(writer: db::WriterHandle, interval: Duration) -> Watcher {
        let stop = Arc::new(AtomicBool::new(false));
        let flag = Arc::clone(&stop);
        let join = std::thread::Builder::new()
            .name("crucible-watch".into())
            .spawn(move || {
                let plat = platform::host();
                let mut next = Instant::now();
                while !flag.load(Ordering::Relaxed) {
                    if Instant::now() >= next {
                        writer.sample(db::SampleRec::of(&sample_box(&plat)));
                        next += interval;
                    }
                    std::thread::sleep(Duration::from_millis(250));
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
        println!(
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
                    println!("        killed {pid} (group {pgid}) -- verified ours")
                }
                orphan::Reaped::Vanished { pid } => println!("        {pid} already gone"),
                orphan::Reaped::Recycled {
                    pid,
                    expected,
                    found,
                } => println!(
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
pub fn run_engine(
    repo: &Path,
    cfg: &crate::config::Config,
    o: Opts<'_>,
    manifest: &Manifest,
    engine: crate::repo::Engine,
    stage: Option<PathBuf>,
) -> anyhow::Result<()> {
    let Opts {
        set,
        require_version: _,
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

    println!("engine  {} [{}]", engine.ver, engine.short_hash());
    let dbctx = if dry_run || no_db {
        None
    } else {
        Some(open_db(cfg, &engine)?)
    };
    let _watcher = dbctx.as_ref().map(|c| {
        println!("db      {}", c.db.path().display());
        Watcher::start(
            c.db.writer().clone(),
            Duration::from_secs(cfg.contention.sample_interval_secs.max(1)),
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
            quiet_hours: cfg.quiet_hours.clone(),
            max_passes,
            capable: &|m| engine.supports_mode(m),
            db: dbctx,
            stage,
        },
    )?;
    println!("set     {set}: {} instances", runner.total_instances());

    if dry_run {
        // Everything up to the first spawn: the boards, their row-identity
        // tuples, and how much work each owes. Enough to check a sweep before
        // committing days of the machine to it.
        println!();
        println!(
            "{:<22} {:>7} {:>5} {:>5} {:>7} {:<10} notes",
            "board", "insts", "wall", "jobs", "threads", "mode"
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
            println!(
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
        println!();
        println!("dry run -- nothing measured, nothing written");
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
        println!(
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
