//! THE DASHBOARD, BUILT FROM THE DATABASE (0.28).
//!
//! Until now the dashboard existed only INSIDE a running `crucible sweep`:
//! the TUI hosted the sweep, so the one way to watch was to be the process
//! doing the work. `crucible tui` without `--demo` said so in as many words --
//! "no sweep is running to attach to" -- and that was true even while a sweep
//! WAS running, if it was running headless. Which is now the normal case, and
//! since 0.28 the normal case is also a launchd agent nobody is attached to.
//!
//! WHY NOT THE SOCKET. crucible-spec §Phase 7 sketches a daemon/attach split
//! over a Unix socket, so "a Zellij restart can't kill a three-day sweep".
//! That solves a control problem this does not have. Everything a dashboard
//! shows is already in the database -- it is the record the sweep itself
//! resumes from -- so a reader needs no protocol, no handshake, and no
//! cooperation from the writer. What falls out of that is worth more than the
//! socket would be:
//!
//!   * it works when NOTHING is running, showing where the set stands;
//!   * any number of terminals can watch at once;
//!   * a viewer cannot perturb the measurement, because it only ever reads;
//!   * and closing it does nothing to the sweep, which was the socket's point.
//!
//! The refresh is deliberately slow (`REFRESH`). The sweep commits a row per
//! transaction and this reads whole boards, so polling at frame rate would put
//! a reader in front of the writer for no gain a human could perceive.

use crate::tui::app::*;
use anyhow::Context;
use crucible_core::db::Reader;
use crucible_publish::manifest::{Manifest, SetSpec};
use crucible_publish::RawRow;
use std::collections::{HashMap, HashSet};
use std::path::Path;
use std::time::{Duration, Instant};

/// How often the database is re-read. Frames still draw at `ui.fps` -- the
/// clock and the sparkline keep moving between reads.
const REFRESH: Duration = Duration::from_secs(2);

/// A row's state, reduced to the one thing the grid paints.
fn cell_for(r: &RawRow, banked: bool) -> Cell {
    if r.val == Some(false) {
        return Cell::Error;
    }
    match (r.solved, banked) {
        (true, _) => Cell::SolvedClean,
        (false, true) => Cell::TimeoutBanked,
        (false, false) => Cell::Owed,
    }
}

/// Everything the dashboard shows, read from the database in one pass.
#[allow(clippy::too_many_arguments)]
fn snapshot(
    repo: &Path,
    reader: &Reader,
    set: &SetSpec,
    manifest: &Manifest,
    engine: Option<&crate::repo::Engine>,
    engine_id: Option<i64>,
    census: &HashMap<String, Vec<(String, String)>>,
    started: Instant,
) -> anyhow::Result<Snapshot> {
    let mut boards = Vec::new();
    let (mut done, mut total, mut solved, mut owed) = (0usize, 0usize, 0usize, 0usize);

    for id in &set.boards {
        let Ok(boards_named) = reader.boards_named(id) else {
            continue;
        };
        let Some(&board_id) = boards_named.first() else {
            continue;
        };
        // ONE engine for the whole set, resolved by hash from the candidate
        // binary -- never the newest engine PER BOARD. See
        // `Reader::engine_by_hash`: per-board recency makes a board the
        // candidate has not reached yet answer with its PREDECESSOR's rows,
        // which are complete, so the set reads as 98% done when it is at 76%.
        // The first cut of this file did exactly that and the dashboard lied
        // on its first frame.
        let rows = engine_id
            .map(|e| reader.export_rows(board_id, e).unwrap_or_default())
            .unwrap_or_default();
        // (ipc, variant, label) -- keyed on the pair a row is identified by.
        let banked: HashSet<(String, String)> = engine_id
            .map(|e| reader.banked_instances(board_id, e).unwrap_or_default())
            .unwrap_or_default()
            .into_iter()
            .map(|(_ipc, variant, label)| (variant, label))
            .collect();

        let spec = manifest.board(id);
        // Indexed by the pair that identifies an instance, so the corpus --
        // not the row set -- drives what appears.
        let verdicts = engine_id
            .map(|e| reader.verdicts_for(board_id, e).unwrap_or_default())
            .unwrap_or_default();
        let by_key: HashMap<(String, String), &RawRow> = rows
            .iter()
            .map(|r| ((r.variant.clone(), r.instance.to_string()), r))
            .collect();

        // The PREDECESSOR, from the promoted raw the standings publish --
        // the same source the sweep's own dashboard uses. This is the number
        // every ad-hoc comparison got wrong: it belongs in the instrument.
        let prior = spec
            .map(|b| crate::sweep::prior_rows(repo, &b.raw))
            .unwrap_or_default();

        let declared = census.get(id.as_str()).cloned().unwrap_or_default();
        let mut cells = Vec::with_capacity(declared.len());
        for (variant, label) in declared {
            let key = (variant.clone(), label.clone());
            let is_banked = banked.contains(&key);
            let row = by_key.get(&key);
            if is_banked {
                done += 1;
                if row.is_some_and(|r| r.solved) {
                    solved += 1;
                }
            } else {
                owed += 1;
            }
            let prev = prior.get(&format!("{variant}/{label}"));
            cells.push(InstanceCell {
                variant,
                label,
                // No row at all is QUEUED, which is the state the database
                // cannot represent and the operator most needs to see.
                cell: row.map_or(Cell::Queued, |r| cell_for(r, is_banked)),
                prev_solved: prev.map(|(s, _)| *s),
                prev_secs: prev.and_then(|(_, t)| *t),
                this_solved: row.map(|r| r.solved),
                this_secs: row.and_then(|r| r.time.as_ref().and_then(|t| t.as_f64())),
                rho: None,
                verdict: verdicts.get(&key).cloned(),
                attempt: u32::from(row.is_some()),
            });
        }
        total += cells.len();
        boards.push(BoardRow {
            id: id.to_string(),
            label: spec.map_or_else(|| id.to_string(), |b| b.label.clone()),
            budget_secs: spec.map_or(60, |b| b.budget_secs as u64),
            threads: 1,
            cells,
        });
    }

    // WHAT IS RUNNING RIGHT NOW. The live_child table is the sweep's own
    // register of spawned planners, kept so a crashed run's orphans can be
    // reaped by identity. It doubles as the only cross-process view of the
    // slots, which is exactly what a detached dashboard needs.
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0.0, |d| d.as_secs_f64());
    let slots: Vec<Slot> = reader
        .live_children()
        .unwrap_or_default()
        .into_iter()
        .enumerate()
        .map(|(index, c)| Slot {
            index,
            what: Some(SlotRun {
                // The register records the CHILD, not the instance it was
                // given: the sweep knows that mapping in memory and has no
                // reason to persist it. A detached viewer therefore sees that
                // a planner is running and for how long, but not on what.
                // Honest about the gap rather than inventing a label.
                board: String::new(),
                variant: String::new(),
                instance: format!("pid {}", c.pid),
                effective: Duration::from_secs_f64((now - c.spawned_at).max(0.0)),
                suspended: c.stopped,
                budget: Duration::from_secs(60),
                rho: None,
                rss_mb: None,
                last_stderr: None,
            }),
        })
        .collect();

    // The box, from the samples the watcher is already writing.
    let recent = reader.samples_between(now - 600.0, now).unwrap_or_default();
    let canary = recent.iter().rev().find_map(|s| s.canary);
    let competitors: Vec<(String, f64)> = recent
        .last()
        .and_then(|s| s.foreign)
        .filter(|f| *f > 1.0)
        .map(|f| vec![("foreign load".to_string(), f)])
        .unwrap_or_default();

    Ok(Snapshot {
        detached: true,
        engine_ver: engine.map(|e| e.ver.clone()).unwrap_or_default(),
        engine_hash: engine
            .map(|e| e.blake3.chars().take(12).collect())
            .unwrap_or_default(),
        uptime: started.elapsed(),
        sweep: SweepProgress {
            done,
            total,
            solved,
            owed,
            ..Default::default()
        },
        boards,
        slots,
        canary,
        competitors,
        ..Default::default()
    })
}

/// EVERY INSTANCE THE SET DECLARES, per board, enumerated from the corpus.
///
/// The denominator cannot come from the database. A row exists only once an
/// instance has been ATTEMPTED, so counting rows counts work done and calls
/// it work total: the second lie this file told on its first run, reporting
/// 6,392/6,476 (99%) for a set that is 6,392/8,444 (76%). An instance nobody
/// has reached yet is the most important thing on this screen.
///
/// Enumerated once and cached: it walks the filesystem, and the corpus does
/// not change under a running sweep.
fn census(
    repo: &Path,
    manifest: &Manifest,
    set: &SetSpec,
) -> HashMap<String, Vec<(String, String)>> {
    let corpus_dir = std::env::var_os("FERROPLAN_IPC_CORPUS")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| repo.join("benchmarks/.ipc-corpus"));
    let mut out = HashMap::new();
    for id in &set.boards {
        let Some(spec) = manifest.board(id) else {
            continue;
        };
        let Some(track) = manifest.track(&spec.track) else {
            continue;
        };
        let Ok(sel) = track.selector() else {
            continue;
        };
        let walk = crucible_core::corpus::variants(&corpus_dir, &track.ipcs, &|v| sel.is_match(v));
        let mut warnings = Vec::new();
        let mut instances = Vec::new();
        for v in &walk.variants {
            for i in crucible_core::corpus::instances(v, 0, &mut warnings) {
                instances.push((v.name.clone(), i.label));
            }
        }
        out.insert(id.clone(), instances);
    }
    out
}

/// The engine the dashboard is reporting on: the CANDIDATE binary, by hash.
///
/// `None` for either half is a legitimate state and shows as an empty set
/// rather than an error: the binary may not be built yet, or it may be built
/// and never swept. Both are things an operator opens this to find out.
fn resolve_engine(repo: &Path, reader: &Reader) -> (Option<crate::repo::Engine>, Option<i64>) {
    let engine = crate::repo::Engine::probe(&crate::repo::candidate_path(repo)).ok();
    let id = engine
        .as_ref()
        .and_then(|e| reader.engine_by_hash(&e.blake3).ok().flatten());
    (engine, id)
}

/// ONE snapshot, for `--dump`: where the set stands, without taking a
/// terminal. A headless box can report itself into a log or a transcript.
pub fn frame(repo: &Path, cfg: &crate::config::Config, set_name: &str) -> anyhow::Result<Snapshot> {
    let manifest = crate::load_manifest(repo)?;
    let set = manifest
        .set(set_name)
        .with_context(|| format!("no set {set_name:?} in the manifest"))?
        .clone();
    let db = cfg.db.dir.join("crucible.db");
    let reader = Reader::open(&db).with_context(|| format!("opening {} to read", db.display()))?;
    let (engine, engine_id) = resolve_engine(repo, &reader);
    let census = census(repo, &manifest, &set);
    snapshot(
        repo,
        &reader,
        &set,
        &manifest,
        engine.as_ref(),
        engine_id,
        &census,
        Instant::now(),
    )
}

pub fn run(repo: &Path, cfg: &crate::config::Config, set_name: &str) -> anyhow::Result<()> {
    let manifest = crate::load_manifest(repo)?;
    let set = manifest
        .set(set_name)
        .with_context(|| format!("no set {set_name:?} in the manifest"))?
        .clone();
    let db = cfg.db.dir.join("crucible.db");
    let reader = Reader::open(&db).with_context(|| format!("opening {} to read", db.display()))?;

    // Walked once: the corpus does not change under a running sweep, and this
    // touches the filesystem.
    let census = census(repo, &manifest, &set);

    let started = Instant::now();
    let mut last = Instant::now()
        .checked_sub(REFRESH)
        .unwrap_or_else(Instant::now);
    let mut cached: Option<Snapshot> = None;

    crate::tui::run::run(
        cfg.ui.fps,
        &cfg.ui.banner_text,
        |_prev| {
            if cached.is_none() || last.elapsed() >= REFRESH {
                last = Instant::now();
                // A read that fails is not a reason to tear the screen down:
                // the writer may be mid-checkpoint, and the previous frame is
                // two seconds old rather than wrong.
                // Re-resolved every refresh on purpose: a rebuild changes the
                // hash, and the dashboard must follow the engine actually
                // being measured rather than the one it opened with.
                let (engine, engine_id) = resolve_engine(repo, &reader);
                if let Ok(s) = snapshot(
                    repo,
                    &reader,
                    &set,
                    &manifest,
                    engine.as_ref(),
                    engine_id,
                    &census,
                    started,
                ) {
                    cached = Some(s);
                }
            }
            cached.clone()
        },
        |_, _| {},
    )?;
    Ok(())
}

/// THE STATS, AS TEXT (0.28) -- `crucible status`.
///
/// This exists because of how it was got wrong. Over one session, three
/// separate hand-rolled SQL queries were written against this database to
/// answer "how is the sweep doing", and all three disagreed with the
/// instrument: one counted rows instead of instances, one took the newest
/// engine per board so untouched boards answered with their predecessor's
/// complete rows, and one dropped `board_id` from a `MAX(attempt)` subquery
/// and reported 1,979 instances losing banked work they had not lost.
///
/// Every one of those numbers was plausible, and two of them were reported
/// before being checked. The rules here are not hard, but they are exact --
/// latest attempt, scoped by board; the declared corpus as the denominator;
/// one engine for the whole set, resolved by hash -- and anything that
/// reimplements them will get them wrong eventually.
///
/// So the instrument answers, and it shares the snapshot the dashboard draws:
/// one implementation of the rule, two renderings.
pub fn status(
    repo: &Path,
    cfg: &crate::config::Config,
    set_name: &str,
    json: bool,
) -> anyhow::Result<()> {
    let snap = frame(repo, cfg, set_name)?;

    let mut verdicts: std::collections::BTreeMap<&str, usize> = Default::default();
    for b in &snap.boards {
        for c in &b.cells {
            if !c.cell.banked() {
                *verdicts
                    .entry(c.verdict.as_deref().unwrap_or("not yet run"))
                    .or_default() += 1;
            }
        }
    }

    if json {
        let boards: Vec<_> = snap
            .boards
            .iter()
            .map(|b| {
                let prev = b
                    .cells
                    .iter()
                    .filter(|c| c.prev_solved == Some(true))
                    .count();
                serde_json::json!({
                    "board": b.id,
                    "total": b.total(),
                    "banked": b.banked(),
                    "owed": b.owed(),
                    "solved": b.solved(),
                    "previous_solved": prev,
                    "delta": b.solved() as i64 - prev as i64,
                    "complete": b.owed() == 0,
                })
            })
            .collect();
        println!(
            "{}",
            serde_json::to_string_pretty(&serde_json::json!({
                "engine": {"version": snap.engine_ver, "hash": snap.engine_hash},
                "banked": snap.sweep.done,
                "total": snap.sweep.total,
                "solved": snap.sweep.solved,
                "owed": snap.sweep.owed,
                "owed_by_verdict": verdicts,
                "boards": boards,
            }))?
        );
        return Ok(());
    }

    println!(
        "engine  {} [{}]",
        if snap.engine_ver.is_empty() {
            "(no candidate built)"
        } else {
            &snap.engine_ver
        },
        snap.engine_hash
    );
    println!(
        "set     {} banked of {} ({:.0}%) -- {} solved, {} owed",
        snap.sweep.done,
        snap.sweep.total,
        100.0 * snap.sweep.done as f64 / snap.sweep.total.max(1) as f64,
        snap.sweep.solved,
        snap.sweep.owed
    );
    println!();
    println!(
        "{:<24}{:>7}{:>7}{:>7}{:>9}{:>8}",
        "board", "total", "banked", "owed", "solved", "vs prev"
    );
    let (mut net, mut complete) = (0i64, 0usize);
    for b in &snap.boards {
        let prev = b
            .cells
            .iter()
            .filter(|c| c.prev_solved == Some(true))
            .count();
        let d = b.solved() as i64 - prev as i64;
        // A board with rows still owed cannot be compared yet: the owed rows
        // are exactly the ones that might change the answer.
        let delta = if b.owed() == 0 {
            net += d;
            complete += 1;
            format!("{d:+}")
        } else {
            "--".into()
        };
        println!(
            "{:<24}{:>7}{:>7}{:>7}{:>9}{:>8}",
            b.id,
            b.total(),
            b.banked(),
            b.owed(),
            b.solved(),
            delta
        );
    }
    println!();
    println!("net {net:+} over the {complete} board(s) with nothing owed");
    if complete < snap.boards.len() {
        println!(
            "({} board(s) still owe rows and are shown as `--`: an owed row is \
             one that could still change its board's number)",
            snap.boards.len() - complete
        );
    }
    if !verdicts.is_empty() {
        println!();
        println!("owed by verdict:");
        let mut v: Vec<_> = verdicts.into_iter().collect();
        v.sort_by_key(|(_, n)| std::cmp::Reverse(*n));
        for (k, n) in v {
            println!("  {k:<14}{n:>6}");
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// BOTH LIES THIS FILE TOLD ON ITS FIRST RUN, pinned as arithmetic.
    ///
    /// A dashboard that overstates progress is worse than no dashboard: it is
    /// the one screen an operator uses to decide whether a sweep needs help,
    /// and the first draft reported a set at 98% and then 99% while it was
    /// actually at 76%.
    ///
    /// The lies were independent, which is why there are two assertions:
    /// picking the newest engine PER BOARD let untouched boards answer with
    /// their predecessor's complete rows, and counting rows for the total made
    /// work-attempted masquerade as work-total.
    #[test]
    fn progress_is_measured_against_the_corpus_and_one_engine() {
        // A set of two boards, 100 instances each. The candidate has reached
        // only the first, banking 40 of it; the second was measured in full by
        // a PREVIOUS engine and the candidate has not touched it.
        let declared_per_board = 100usize;
        let boards = 2usize;
        let banked_by_candidate = 40usize;
        let rows_from_candidate = 60usize; // measured, 40 of them banked

        // The wrong denominator: rows that exist.
        let by_rows = banked_by_candidate as f64 / rows_from_candidate as f64;
        // The wrong engine: the untouched board answers "100/100 complete".
        let by_newest_engine = (banked_by_candidate + declared_per_board) as f64
            / (declared_per_board * boards) as f64;
        // The truth.
        let honest = banked_by_candidate as f64 / (declared_per_board * boards) as f64;

        assert_eq!((by_rows * 100.0).round() as i64, 67);
        assert_eq!((by_newest_engine * 100.0).round() as i64, 70);
        assert_eq!((honest * 100.0).round() as i64, 20);
        assert!(
            honest < by_rows && honest < by_newest_engine,
            "both mistakes flatter the sweep, which is the direction that \
             makes them dangerous"
        );
    }

    /// An instance with no row is QUEUED, not absent. The database cannot
    /// represent "not started" -- only the corpus can -- and it is the state
    /// the operator most needs to see.
    #[test]
    fn an_instance_with_no_row_is_queued() {
        let cell: Cell = None::<&RawRow>.map_or(Cell::Queued, |_| Cell::SolvedClean);
        assert_eq!(cell, Cell::Queued);
        assert!(!Cell::Queued.banked(), "queued is owed, never banked");
    }
}
