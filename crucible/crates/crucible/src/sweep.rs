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

use anyhow::Context;
use crucible_core::corpus;
use crucible_core::exec::Ctl;
use crucible_core::monitor::{self, Level, Sample, Throttle};
use crucible_core::platform::{self, Platform};
use crucible_core::sched::{self, Attempt, BoardState, Event, LoopConfig, Next, Runner};
use crucible_core::sweep::{BoardCfg, Engine as SweepEngine};
use crucible_publish::manifest::{BoardSpec, Manifest};
use std::path::{Path, PathBuf};
use std::sync::mpsc;
use std::time::{Duration, Instant};

/// One board's work: its spec, its instances, and which of them still owe a
/// clean row.
pub(crate) struct Board {
    spec: BoardSpec,
    position: usize,
    instances: Vec<(String, String, corpus::Instance)>,
    /// Instance keys that have banked a CLEAN row. A dirty row is written and
    /// kept, but does not remove the instance from the owed set.
    clean: std::collections::BTreeSet<String>,
    /// Every row measured, keyed so a later clean measurement SUPERSEDES an
    /// earlier dirty one. Nothing is ever dropped -- a dirty row is the record
    /// that the instance was attempted and what the box was doing.
    rows: std::collections::BTreeMap<String, crucible_publish::RawRow>,
}

impl Board {
    fn remaining(&self) -> usize {
        self.instances.len() - self.clean.len()
    }
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
        } = setup;
        let spec = manifest
            .set(set)
            .with_context(|| format!("no set {set:?} in the manifest"))?;
        let corpus_dir = std::env::var_os("FERROPLAN_IPC_CORPUS")
            .map(PathBuf::from)
            .unwrap_or_else(|| repo.join("benchmarks/.ipc-corpus"));

        let mut boards = Vec::new();
        let mut warnings = Vec::new();
        for (position, id) in spec.boards.iter().enumerate() {
            let Some(b) = manifest.board(id) else {
                continue;
            };
            // A board this engine cannot run is SKIPPED, with ZERO rows
            // written -- never a board of zeroes. "The feature does not exist,
            // and recording a zero would be a lie the standings would then
            // average." Old tags predate Mode::Optimal, and a stale binary can
            // predate a whole track.
            if let Some(mode) = &b.mode {
                if !capable(mode) {
                    println!(
                        "SKIP {id}: this engine has no --mode {mode} -- \
                         feature-absent, not zero coverage"
                    );
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
            boards.push(Board {
                spec: b.clone(),
                position,
                instances,
                clean: Default::default(),
                rows: Default::default(),
            });
        }
        for w in &warnings {
            eprintln!("WARN {w}");
        }

        Ok(SweepRunner {
            stage: repo.join(&spec.stage),
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
        })
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
            let key = format!("{ipc}\u{1}{variant}\u{1}{}", i.label);
            if let Some(r) = b.rows.get(&key) {
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
                reused_total: 0,
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

    /// Sample the box once. The verdict is named-competitor load, never idle:
    /// a `--threads 8` board burns most of this machine by design.
    fn sample(&self) -> Sample {
        let ps = std::process::Command::new("ps")
            .args(["-Ao", "pcpu,comm", "-r"])
            .output()
            .map(|o| String::from_utf8_lossy(&o.stdout).into_owned())
            .unwrap_or_default();
        let mine = self.plat.descendants(std::process::id() as i32);
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
            at: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| (d.as_secs_f64() * 10.0).round() / 10.0)
                .unwrap_or(0.0),
            idle_pct: None,
            competitors,
            competitors_total: total,
            loadavg1: None,
            swap_mb: self.plat.swap_used_mb(),
            cpu_speed_limit: self.plat.cpu_speed_limit(),
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

        let (_tx, rx) = mpsc::channel::<Ctl>();
        let mut banked = 0usize;
        let mut all_dirty = true;
        let todo: Vec<usize> = (0..self.boards[idx].instances.len())
            .filter(|i| {
                let key = &self.boards[idx].instances[*i].2.label;
                !self.boards[idx].clean.contains(key)
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
            );

            // The row is KEPT either way -- nothing is discarded for
            // contention. It just does not count toward banking.
            let after = self.sample();
            let clean = !dirty_now && after.is_clean() && m.clock_jump.is_zero();
            if clean {
                self.boards[idx].clean.insert(inst.label.clone());
                banked += 1;
                all_dirty = false;
            }
            // Written either way. The board is not banked, but the work is
            // not lost -- and a later clean row supersedes this one under the
            // same key.
            let key = format!("{ipc}\u{1}{variant}\u{1}{}", inst.label);
            self.boards[idx].rows.insert(key, m.row);
        }

        if let Err(e) = self.write_artifacts(idx) {
            eprintln!("!! could not write {}: {e}", self.boards[idx].spec.id);
        }

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
    /// with: measure the CANDIDATE, not whatever happens to be built.
    pub require_version: Option<&'a str>,
    pub quiet_only: bool,
    pub dry_run: bool,
    /// `None` is the resident behaviour: a board that cannot bank because the
    /// box is never quiet is waiting, not failing.
    pub max_passes: Option<u32>,
}

pub fn run(repo: &Path, cfg: &crate::config::Config, o: Opts<'_>) -> anyhow::Result<()> {
    let Opts {
        set,
        require_version,
        quiet_only,
        dry_run,
        max_passes,
    } = o;
    let manifest = crate::load_manifest(repo)?;
    let bin = crate::repo::candidate_path(repo);
    let engine = crate::repo::Engine::probe(&bin)?;
    if let Some(want) = require_version {
        // The gate every sweep driver opens with: measure the CANDIDATE, not
        // whatever happens to be built.
        engine.require_version(want)?;
    }
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
    let mut runner = SweepRunner::new(
        &manifest,
        Setup {
            repo,
            set,
            engine: SweepEngine {
                path: engine.path.clone(),
                ver: engine.ver.clone(),
            },
            val,
            quiet_only,
            quiet_hours: cfg.quiet_hours.clone(),
            max_passes,
            capable: &|m| engine.supports_mode(m),
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
            let c = board_cfg(&manifest, &b.spec);
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
