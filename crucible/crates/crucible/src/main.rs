//! `crucible` -- the resident benchmark sweep harness for ferroplan.
//!
//! Replaces a pile of shell drivers, a Python runner, a contention watcher and
//! a standings generator with one supervised process that owns the sweep end to
//! end. The design record is `crucible-spec.md`; the corrections to it, and the
//! decisions this cycle made, are in `docs/roadmap-0.26.md`.
//!
//! The organising rule for the whole program: **contention may cost a timing
//! number; it must never cost hours of computation.** Everything measured is
//! kept, marked dirty when the box was not quiet, and a board is only banked
//! and only publishable once every one of its rows is clean.

mod backfill;
mod config;
mod out;
mod repo;
mod sweep;
mod tui;

use anyhow::Context;
use clap::{Parser, Subcommand};
use crucible_publish::manifest::Manifest;
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "crucible", version, about = "the ferroplan sweep harness")]
struct Cli {
    /// The ferroplan working tree to sweep and publish into.
    #[arg(long, global = true)]
    repo: Option<PathBuf>,
    /// Operator settings. The INSTRUMENT lives in the repo's manifest.toml,
    /// deliberately not here.
    #[arg(long, global = true)]
    config: Option<PathBuf>,
    #[command(subcommand)]
    cmd: Cmd,
}

#[derive(Subcommand)]
enum Cmd {
    /// Check the box, the binary, the corpus, the validator and the manifest,
    /// and say what is missing. Run this before trusting anything else.
    Doctor,
    /// Enumerate a track's variants, the way `ipc67.py --track T --list` does.
    List {
        #[arg(long)]
        track: String,
    },
    /// Show the manifest's boards, sets and any validation complaints.
    Boards {
        /// Only boards in this set (cut25, entries25, ...).
        #[arg(long)]
        set: Option<String>,
    },
    /// Identify the planner binary that would be swept.
    Engine,
    /// Sweep a set of boards. This is what the shell drivers did.
    ///
    /// Nothing measured is ever discarded: a row taken while the box was busy
    /// is written and kept, it simply does not bank its instance. A board is
    /// banked only when every instance it holds has a clean row.
    Sweep {
        /// A `[[set]]` from the manifest: cut25, entries25, ...
        #[arg(long)]
        set: String,
        /// Refuse unless the binary reports this version. The gate every sweep
        /// driver opens with -- measure the CANDIDATE, not whatever is built.
        #[arg(long)]
        require_version: Option<String>,
        /// Measure only while the throttle says FULL. Off by default because a
        /// dirty row is still worth having; on, the sweep waits instead.
        #[arg(long)]
        quiet_only: bool,
        /// Print the log instead of hosting the dashboard. The default when
        /// stdout is not a terminal.
        #[arg(long)]
        headless: bool,
        /// Enumerate the boards and their run parameters, then stop. Nothing is
        /// spawned and nothing is written.
        #[arg(long)]
        dry_run: bool,
        /// Stop after this many passes. Unbounded by default: a board that
        /// cannot bank because the box is never quiet is waiting, not failing,
        /// and a resident harness should go on waiting.
        #[arg(long)]
        max_passes: Option<u32>,
        /// Run without the database: rows held in memory, cleanliness judged
        /// from a before/after sample pair, no engine stamp on the rows. The
        /// pre-DB behaviour, bit for bit, kept as the restore hatch. Env twin:
        /// CRUCIBLE_NO_DB=1.
        #[arg(long)]
        no_db: bool,
    },
    /// Sweep a TAG with the working tree's instrument: build the tag's planner
    /// in a detached worktree under crucible's own prefix, skip the version
    /// gate, stage under benchmarks/air-<ver>/, and skip (feature-absent) any
    /// board the old engine cannot run. The instrument never varies with the
    /// engine, or the delta means nothing.
    Backfill {
        /// The git tag to build and measure, e.g. v0.18.0.
        #[arg(long)]
        tag: String,
        /// A `[[set]]` from the manifest: cut25, entries25, ...
        #[arg(long)]
        set: String,
        /// Staging dir override. Default: benchmarks/air-<ver>/.
        #[arg(long)]
        stage: Option<PathBuf>,
        /// Print the board plan (with the capability skips) and stop.
        #[arg(long)]
        dry_run: bool,
        #[arg(long)]
        max_passes: Option<u32>,
        /// The pre-database path, as for `sweep`. Env twin: CRUCIBLE_NO_DB=1.
        #[arg(long)]
        no_db: bool,
    },
    /// Regenerate (or check) the standings documents.
    Standings {
        /// detail | summary | all
        #[arg(long, default_value = "all")]
        doc: String,
        /// Render in memory and diff against disk; write nothing. Exits
        /// non-zero when a document is out of date.
        #[arg(long)]
        check: bool,
        /// Write the documents. Without this, they go to stdout -- a bare run
        /// must never overwrite a table on a box holding only some of the raws.
        #[arg(long)]
        write: bool,
    },
    /// Compare two staging directories or promoted sets, board by board.
    Diff {
        a: String,
        b: String,
        #[arg(long, default_value = "text")]
        mode: String,
    },
    /// Open the dashboard. With --demo it runs against a synthetic sweep, which
    /// is how the layout gets looked at without burning three days of CPU.
    Tui {
        /// Which view to dump: grid | board | instance | timeline.
        #[arg(long, default_value = "grid")]
        view: String,
        #[arg(long)]
        demo: bool,
        /// Render ONE frame to stdout and exit, instead of taking the terminal.
        /// Useful for reviewing the layout, for documentation, and for a CI
        /// check that the dashboard still renders at a given size.
        #[arg(long)]
        dump: bool,
        #[arg(long, default_value_t = 118)]
        width: u16,
        #[arg(long, default_value_t = 30)]
        height: u16,
    },
}

fn main() {
    if let Err(e) = real_main() {
        // A chain, because the useful part is usually the innermost cause.
        eprintln!("crucible: {e:#}");
        std::process::exit(1);
    }
}

fn real_main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    let cfg_path = cli.config.unwrap_or_else(config::Config::path);
    let cfg = config::Config::load(&cfg_path)?;
    let repo_root = cli.repo.unwrap_or_else(|| cfg.repo.local.clone());

    match cli.cmd {
        Cmd::Doctor => doctor(&repo_root, &cfg, &cfg_path),
        Cmd::List { track } => list(&repo_root, &track),
        Cmd::Boards { set } => boards(&repo_root, set.as_deref()),
        Cmd::Engine => engine(&repo_root),
        Cmd::Sweep {
            set,
            require_version,
            quiet_only,
            headless,
            dry_run,
            max_passes,
            no_db,
        } => sweep::run(
            &repo_root,
            &cfg,
            sweep::Opts {
                set: &set,
                require_version: require_version.as_deref(),
                headless,
                quiet_only,
                dry_run,
                max_passes,
                no_db: no_db || std::env::var_os("CRUCIBLE_NO_DB").is_some_and(|v| v == "1"),
            },
        ),
        Cmd::Backfill {
            tag,
            set,
            stage,
            dry_run,
            max_passes,
            no_db,
        } => backfill::run(
            &repo_root,
            &cfg,
            backfill::Opts {
                tag: &tag,
                set: &set,
                stage,
                dry_run,
                max_passes,
                no_db: no_db || std::env::var_os("CRUCIBLE_NO_DB").is_some_and(|v| v == "1"),
            },
        ),
        Cmd::Standings { doc, check, write } => standings(&repo_root, &cfg, &doc, check, write),
        Cmd::Diff { a, b, mode } => diff(&repo_root, &a, &b, &mode),
        Cmd::Tui {
            view,
            demo,
            dump,
            width,
            height,
        } => tui_cmd(&cfg, demo, dump, width, height, &view),
    }
}

/// The dashboard. Without a live sweep to attach to there is nothing to draw,
/// so `--demo` animates a synthetic one -- which is also how the layout gets
/// reviewed without spending three days of CPU to see it.
fn tui_cmd(
    cfg: &config::Config,
    demo: bool,
    dump: bool,
    width: u16,
    height: u16,
    view: &str,
) -> anyhow::Result<()> {
    use std::time::Instant;

    if dump {
        return dump_frame(cfg, width, height, view);
    }
    if !demo {
        anyhow::bail!(
            "no sweep is running to attach to; start one (`crucible sweep` hosts \
             the dashboard), or pass --demo to look at the layout"
        );
    }

    let start = Instant::now();
    tui::run::run(
        cfg.ui.fps,
        &cfg.ui.banner_text,
        |_prev| Some(tui::demo::snapshot(start.elapsed().as_secs_f64())),
        |_, _| {},
    )?;
    Ok(())
}

pub(crate) fn manifest_path(repo: &std::path::Path) -> PathBuf {
    repo.join("benchmarks/manifest.toml")
}

pub(crate) fn load_manifest(repo: &std::path::Path) -> anyhow::Result<Manifest> {
    let p = manifest_path(repo);
    Manifest::load(&p).with_context(|| format!("reading {}", p.display()))
}

/// Everything a sweep depends on, checked before anything is measured.
///
/// The shell drivers checked exactly one of these -- the binary's version --
/// and discovered the rest by failing hours in. A corpus that is absent, a
/// validator that is not built, a manifest that disagrees with itself: each of
/// those produces a board that looks measured and is not.
fn doctor(
    repo: &std::path::Path,
    cfg: &config::Config,
    cfg_path: &std::path::Path,
) -> anyhow::Result<()> {
    use crucible_core::platform::{self, Platform};
    let mut problems = 0;
    let mut say = |ok: bool, label: &str, detail: String| {
        if !ok {
            problems += 1;
        }
        println!(
            "  {} {label:<22} {detail}",
            if ok { "ok  " } else { "MISS" }
        );
    };

    // A missing config is not a problem: every default here is either the
    // value the shell drivers actually used or one the record justifies.
    println!("config");
    say(
        true,
        "config.toml",
        format!(
            "{} {}",
            cfg_path.display(),
            if cfg_path.exists() {
                ""
            } else {
                "(absent -- using defaults)"
            }
        ),
    );

    println!("box");
    let plat = platform::host();
    let t = plat.topology();
    say(
        t.p_cores > 0,
        "topology",
        format!(
            "{} P + {} E cores, {} logical, {:.0} GiB",
            t.p_cores,
            t.e_cores,
            t.logical,
            t.mem_bytes as f64 / (1u64 << 30) as f64
        ),
    );
    let cap = plat.probe_mem_cap(1 << 30);
    say(
        true,
        "memory cap",
        format!(
            "{} (Darwin rejects every setrlimit on RLIMIT_AS, so the watchdog \
             measures RESIDENT bytes)",
            cap.instrument()
        ),
    );
    let th = tui::theme::Theme::forge();
    say(
        true,
        "terminal",
        format!(
            "{:?} colour, {} glyphs -- the dashboard degrades rather than fails",
            th.depth,
            if th.unicode { "unicode" } else { "ascii" }
        ),
    );
    say(
        plat.keep_awake().is_some(),
        "keep-awake",
        "caffeinate -- a three-day sweep that sleeps at hour four is not a sweep".into(),
    );

    println!("repo");
    say(
        repo.join("Cargo.toml").exists(),
        "working tree",
        repo.display().to_string(),
    );
    let bin = repo::candidate_path(repo);
    match repo::Engine::probe(&bin) {
        Ok(e) => say(true, "planner", format!("{} [{}]", e.ver, e.short_hash())),
        Err(e) => say(false, "planner", e.to_string()),
    }
    let corpus = repo.join("benchmarks/.ipc-corpus");
    let variants: usize = std::fs::read_dir(&corpus)
        .map(|rd| {
            rd.flatten()
                .filter_map(|e| std::fs::read_dir(e.path().join("domains")).ok())
                .map(|d| d.count())
                .sum()
        })
        .unwrap_or(0);
    say(
        variants > 0,
        "corpus",
        format!(
            "{variants} variant dirs in {} (gitignored; benchmarks/get-ipc.sh)",
            corpus.display()
        ),
    );
    let val = cfg
        .sweep
        .validator
        .clone()
        .or_else(|| std::env::var_os("FERROPLAN_VAL").map(PathBuf::from))
        .unwrap_or_else(|| repo.join("benchmarks/.val/VAL/build/bin/Validate"));
    say(
        val.exists(),
        "validator",
        format!(
            "{} {}",
            val.display(),
            if val.exists() {
                ""
            } else {
                "(benchmarks/get-val.sh; boards render VAL-unavailable without it)"
            }
        ),
    );

    println!("instrument");
    match load_manifest(repo) {
        Ok(m) => {
            say(
                true,
                "manifest",
                format!(
                    "{} boards, {} tracks, {} sets",
                    m.boards.len(),
                    m.tracks.len(),
                    m.sets.len()
                ),
            );
            let errs = m.errors();
            let warns = m.warnings();
            say(
                errs.is_empty(),
                "manifest valid",
                if errs.is_empty() {
                    "no errors".into()
                } else {
                    format!("{} error(s)", errs.len())
                },
            );
            for e in &errs {
                println!("       error: {e}");
            }
            for w in &warns {
                println!("       note:  {w}");
            }
        }
        Err(e) => say(false, "manifest", e.to_string()),
    }

    println!();
    if problems == 0 {
        println!("crucible doctor: ready");
        Ok(())
    } else {
        anyhow::bail!("{problems} thing(s) missing -- see above")
    }
}

/// The `ipc67.py --track T --list` equivalent, and the first thing to diff
/// against the Python when the selector or the labelling is in doubt.
fn list(repo: &std::path::Path, track: &str) -> anyhow::Result<()> {
    use crucible_core::corpus;
    let m = load_manifest(repo)?;
    let spec = m
        .track(track)
        .with_context(|| format!("no track {track:?} in the manifest"))?;
    let sel = spec.selector().map_err(|e| anyhow::anyhow!("{e}"))?;
    let corpus_dir = std::env::var_os("FERROPLAN_IPC_CORPUS")
        .map(PathBuf::from)
        .unwrap_or_else(|| repo.join("benchmarks/.ipc-corpus"));

    let walk = corpus::variants(&corpus_dir, &spec.ipcs, &|v| sel.is_match(v));
    let mut warnings = walk.warnings;
    let mut total = 0usize;
    for v in &walk.variants {
        let insts = corpus::instances(v, 0, &mut warnings);
        total += insts.len();
        println!("{}  {}  ({} instances)", v.ipc, v.name, insts.len());
    }
    // Un-numbered files and unpaired domains are named, never dropped in
    // silence: a missing instance must read as a corpus bug, not as a smaller
    // corpus.
    for w in &warnings {
        eprintln!("WARN {w}");
    }
    eprintln!(
        "{track}: {total} instances across {} variants",
        walk.variants.len()
    );
    Ok(())
}

fn boards(repo: &std::path::Path, set: Option<&str>) -> anyhow::Result<()> {
    let m = load_manifest(repo)?;
    let filter: Option<&crucible_publish::manifest::SetSpec> = match set {
        Some(s) => Some(m.set(s).with_context(|| format!("no set {s:?}"))?),
        None => None,
    };
    for b in &m.boards {
        if let Some(f) = filter {
            if !f.boards.iter().any(|x| x == &b.id) {
                continue;
            }
        }
        let mut notes = Vec::new();
        if b.proof_track {
            // Coverage on these IS proof rate: 45% here is a categorically
            // different claim from 45% on a satisficing board.
            notes.push("proof".to_string());
        }
        if let Some(t) = b.timeout_secs {
            if (t as f64 - b.budget_secs).abs() > f64::EPSILON {
                notes.push(format!(
                    "TIER MOVE IN FLIGHT: sweeps at {t}s, registry says {}s",
                    b.budget_secs
                ));
            }
        }
        if b.threads.unwrap_or(1) > 1 {
            notes.push(format!(
                "--threads {} (jobs forced to 1)",
                b.threads.unwrap_or(1)
            ));
        }
        println!(
            "{:<20} {:<18} {:>4}s  {:<32} {}",
            b.id,
            b.track,
            b.budget_secs,
            b.label,
            notes.join("; ")
        );
    }
    for w in m.warnings() {
        eprintln!("note: {w}");
    }
    Ok(())
}

fn engine(repo: &std::path::Path) -> anyhow::Result<()> {
    let bin = repo::candidate_path(repo);
    let e = repo::Engine::probe(&bin)?;
    println!("path    {}", e.path.display());
    println!("version {}", e.ver);
    // The identity the resume gate actually compares -- the version string
    // cannot tell two builds of one cycle apart.
    println!("blake3  {}", e.blake3);
    println!(
        "modes   {}",
        if e.modes.is_empty() {
            "(unprobed)".into()
        } else {
            e.modes.join(", ")
        }
    );
    Ok(())
}

/// Render one frame off-screen and print it. No terminal is touched, so this
/// works in a pipe, in CI, and in a transcript.
fn dump_frame(cfg: &config::Config, width: u16, height: u16, view: &str) -> anyhow::Result<()> {
    use ratatui::backend::TestBackend;
    use ratatui::Terminal;

    let mut snap = tui::demo::snapshot(400.0);
    snap.sel_board = 3;
    snap.sel_inst = 5;
    snap.view = match view {
        "board" => tui::app::View::Board,
        "instance" => {
            snap.detail = Some(tui::demo::detail());
            tui::app::View::Instance
        }
        "timeline" => {
            snap.timeline = tui::demo::detail().timeline;
            tui::app::View::Timeline
        }
        _ => tui::app::View::Grid,
    };

    let mut term = Terminal::new(TestBackend::new(width, height))?;
    let theme = tui::theme::Theme::forge();
    term.draw(|f| tui::draw::draw(f, &snap, &theme, &cfg.ui.banner_text))?;
    let buf = term.backend().buffer();
    for y in 0..buf.area.height {
        let mut line = String::new();
        for x in 0..buf.area.width {
            line.push_str(buf[(x, y)].symbol());
        }
        println!("{}", line.trim_end());
    }
    Ok(())
}

/// Regenerate the standings documents.
///
/// `--check` renders in memory and DIFFS against what is on disk without
/// writing anything. That is the shape `RELEASING.md` already asks for, and it
/// turns "regenerate after every sweep" from a discipline into a gate.
///
/// Nothing is written unless asked. `standings.py`'s own comment calls a bare
/// in-place run "a destructive act" on a box holding only some of the raws;
/// crucible does not inherit that.
fn standings(
    repo: &std::path::Path,
    cfg: &config::Config,
    which: &str,
    check: bool,
    write: bool,
) -> anyhow::Result<()> {
    use crucible_publish::history::BoxId;
    use crucible_publish::render;

    let ctx = render::RenderCtx::load(repo, BoxId::new(&cfg.sweep.box_id))
        .with_context(|| format!("loading standings inputs from {}", repo.display()))?;

    let targets: Vec<(&str, std::path::PathBuf, String)> = match which {
        "detail" => vec![(
            "detail",
            repo.join("benchmarks/ipc-standings.md"),
            render::detail::render(&ctx),
        )],
        "summary" => vec![(
            "summary",
            repo.join("STANDINGS.md"),
            render::summary::render(&ctx),
        )],
        "all" => vec![
            (
                "detail",
                repo.join("benchmarks/ipc-standings.md"),
                render::detail::render(&ctx),
            ),
            (
                "summary",
                repo.join("STANDINGS.md"),
                render::summary::render(&ctx),
            ),
        ],
        other => anyhow::bail!("unknown document {other:?}; try detail, summary or all"),
    };

    let mut differs = 0;
    for (name, path, text) in &targets {
        if check {
            let have = std::fs::read_to_string(path).unwrap_or_default();
            if &have == text {
                println!("ok    {name:<8} {} matches", path.display());
            } else {
                differs += 1;
                println!("DIFFERS {name:<6} {}", path.display());
                // The first differing LINE, not a wall of two 70-line strings.
                for (i, (a, b)) in have.lines().zip(text.lines()).enumerate() {
                    if a != b {
                        println!("  line {}:\n    on disk: {a}\n    would be: {b}", i + 1);
                        break;
                    }
                }
                let (n, m) = (have.lines().count(), text.lines().count());
                if n != m {
                    println!("  line count {n} on disk vs {m} rendered");
                }
            }
        } else if write {
            std::fs::write(path, text)?;
            println!("wrote {}", path.display());
        } else {
            print!("{text}");
        }
    }
    if check && differs > 0 {
        anyhow::bail!("{differs} document(s) out of date -- re-run without --check to regenerate");
    }
    Ok(())
}

/// Compare two sets of boards, naming what was gained and what was LOST.
///
/// A loss is the loud case: a problem solved on the previous release and not on
/// this one is a regression and gets named individually, with what it was and
/// what it became. Coverage is counted through the referee, not the raw
/// `solved` flag -- `ipc67-diff.py` uses the flag and therefore counts
/// VAL-rejected plans as coverage, which the standings layer does not.
fn diff(repo: &std::path::Path, a: &str, b: &str, mode: &str) -> anyhow::Result<()> {
    use crucible_publish::compare::{Conditions, Diff, Mode, RunRef};

    let mode = match mode {
        "text" => Mode::Text,
        "markdown" | "md" => Mode::Markdown,
        "json" => Mode::Json,
        other => anyhow::bail!("unknown --mode {other:?}; try text, markdown or json"),
    };
    let m = load_manifest(repo)?;
    let referee = crucible_publish::Referee::default();

    let find = |dir: &std::path::Path, board: &crucible_publish::manifest::BoardSpec| {
        let a = dir.join(&board.raw);
        let b = dir.join(format!("{}.jsonl", board.id));
        if a.exists() {
            Some(a)
        } else if b.exists() {
            Some(b)
        } else {
            None
        }
    };
    let (da, db) = (repo.join(a), repo.join(b));
    let mut compared = 0usize;

    for board in &m.boards {
        let (Some(pa), Some(pb)) = (find(&da, board), find(&db, board)) else {
            continue;
        };
        let la = RunRef::from_jsonl(
            a,
            board.budget_secs,
            &std::fs::read_to_string(&pa)?,
            &pa.to_string_lossy(),
        )
        .map_err(|e| anyhow::anyhow!(e))?;
        let lb = RunRef::from_jsonl(
            b,
            board.budget_secs,
            &std::fs::read_to_string(&pb)?,
            &pb.to_string_lossy(),
        )
        .map_err(|e| anyhow::anyhow!(e))?;
        for k in la.collapsed.iter().chain(lb.collapsed.iter()) {
            // Two rows under one key means the instance labelling collapsed --
            // the incident that put 320 rows under 288 keys. Never silent.
            eprintln!("warning: {} collapsed onto an existing key", k.qualified());
        }
        // Cleanliness comes from each board's own conditions record; a board
        // whose watcher predates the per-sample timeline is Unknown, and
        // Unknown is not clean.
        let ca = Conditions::load(&pa.with_extension("conditions.json"));
        let cb = Conditions::load(&pb.with_extension("conditions.json"));
        let d = Diff::new(&referee, &la.run, &lb.run, &ca, &cb);
        println!("## {}\n{}", board.label, d.render(mode));
        compared += 1;
    }
    if compared == 0 {
        anyhow::bail!("no boards in common between {a} and {b}");
    }
    Ok(())
}
