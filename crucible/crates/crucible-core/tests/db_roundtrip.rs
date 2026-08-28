//! The database is a CACHE. This is the test that says so.
//!
//! The repo's committed board raws are the durable record; the database is a
//! fast, queryable copy of them and a work queue. That claim is only worth
//! anything if a board can go in and come back out unchanged, so this file
//! takes `benchmarks/ipc2014-opt.jsonl` -- 256 real rows, git-tracked, so the
//! test is hermetic -- loads it, exports it, and compares BYTES with
//! `crucible_publish::write_row`.
//!
//! # Why there are two laps
//!
//! Lap 1 (file -> database -> bytes) can pass by luck. A lossy numeric column
//! survives one trip if the value it mangles happens to re-print the same way
//! -- a `time` stored as a float and re-formatted, a `metric` rounded to two
//! places that was already two places. Lap 2 feeds lap 1's output back in and
//! exports again: anything lossy has now been through the lossy step twice and
//! the two laps disagree. It is the cheapest available proof that the storage
//! layer is not quietly normalising measured values.
//!
//! The other properties tested here each defend a specific way this layer could
//! be wrong without anything failing:
//!
//! * a second crucible against a live database is two schedulers on one queue;
//! * a rebuilt run claiming `clean` timing manufactures the exact claim the
//!   contention watcher exists to refuse;
//! * an import that is not idempotent turns a restart into `attempt = 2` rows
//!   and doubles every board;
//! * `val = 0` and `val IS NULL` are different questions, and confusing them is
//!   three published-number incidents;
//! * the resume gate is now a SQL range query, and it has to fail CLOSED in
//!   every direction the Python fails closed in -- a gate that answers "clean"
//!   where the Python answered "re-run" stitches a contended row into a board
//!   and says nothing about it;
//! * a `live_child` row has to carry enough to prove identity, because a
//!   `killpg` on a recycled pgid kills a stranger.

use crucible_core::db::{export, rebuild_from_artifacts, Db, DbError, LockError};
use crucible_publish::manifest::Manifest;
use std::path::{Path, PathBuf};

/// The board this file is built around, and the only one copied into the
/// scratch artifact directory -- every other board in the manifest is then
/// legitimately missing, which also exercises the "a missing raw is skipped,
/// not an error" rule.
const BOARD_RAW: &str = "ipc2014-opt.jsonl";
const BOARD_ID: &str = "ipc2014-opt";

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../..")
        .canonicalize()
        .expect("locating the repo root")
}

/// A temp directory that removes itself, so a failing test does not leave a
/// database behind that the next run's lock would then refuse.
struct Scratch(PathBuf);

impl Scratch {
    fn new(tag: &str) -> Scratch {
        let d = std::env::temp_dir().join(format!(
            "crucible-roundtrip-{}-{}-{tag}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_nanos())
                .unwrap_or(0)
        ));
        std::fs::create_dir_all(&d).expect("creating scratch");
        Scratch(d)
    }
    fn join(&self, p: &str) -> PathBuf {
        let d = self.0.join(p);
        std::fs::create_dir_all(&d).expect("creating scratch subdir");
        d
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

fn manifest() -> Manifest {
    Manifest::load(&repo_root().join("benchmarks/manifest.toml")).expect("loading the manifest")
}

/// One board's raw, and its sibling conditions file, in a directory of their
/// own. The conditions file is the rescued rollup-only fixture -- the shape
/// 72 of the repo's 76 conditions files have, verdict `clean`, no timeline.
///
/// A rescued fixture rather than `benchmarks/air24/`'s file: `air*/` is
/// gitignored, so on a fresh checkout that file does not exist, the pass
/// verdict degrades to `unknown`, and this test fails for a reason that has
/// nothing to do with the code under test. Hermetic means git-tracked.
fn artifacts(scratch: &Scratch, name: &str, raw: &str) -> PathBuf {
    let dir = scratch.join(name);
    std::fs::write(dir.join(BOARD_RAW), raw).expect("writing the board raw");
    let cond = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/fixtures/conditions/rollup-only.json");
    std::fs::copy(&cond, dir.join("ipc2014-opt.conditions.json"))
        .unwrap_or_else(|e| panic!("{}: {e}", cond.display()));
    dir
}

fn committed_raw() -> String {
    std::fs::read_to_string(repo_root().join("benchmarks").join(BOARD_RAW))
        .expect("reading the committed board raw")
}

/// Load one artifact directory into a fresh database and hand back the export.
fn load_and_export(scratch: &Scratch, tag: &str, art: &Path) -> (String, usize) {
    let db = Db::open(&scratch.join(tag)).expect("opening the database");
    let built = rebuild_from_artifacts(db.writer(), &manifest(), art, None).expect("rebuilding");
    assert_eq!(
        built.len(),
        1,
        "one raw in the directory should produce exactly one board pass"
    );
    let b = &built[0];
    assert_eq!(b.board_name, BOARD_ID);
    let reader = db.reader().expect("reader");
    let text = export(&reader, b.board_id, b.engine_id).expect("exporting");
    (text, b.rows)
}

/// The claim the whole module rests on: a committed board goes in and the same
/// bytes come out, in the canonical order, through `write_row`.
///
/// Not "the same rows" and not "the same numbers" -- the same BYTES. Key order,
/// which optional keys are present, whether `time` is an int or a float, and
/// `\uXXXX` escaping of the em-dashes in the engine's notes are all part of it,
/// and every one of them has a way of going wrong that a field-by-field
/// comparison would wave through.
#[test]
fn a_committed_board_round_trips_byte_for_byte() {
    let scratch = Scratch::new("lap1");
    let original = committed_raw();
    let art = artifacts(&scratch, "artifacts", &original);
    let (lap1, rows) = load_and_export(&scratch, "db1", &art);
    assert_eq!(rows, 256, "the fixture is 256 rows");
    assert_eq!(
        lap1, original,
        "exported board differs from the committed raw"
    );
}

/// Lap 2. Lap 1 can pass by luck; a value that a lossy column mangles into
/// itself survives one trip. Feeding lap 1's output back through the whole
/// pipeline and exporting again puts every value through the lossy step twice,
/// which is where the disagreement shows up.
#[test]
fn a_second_lap_agrees_with_the_first() {
    let scratch = Scratch::new("lap2");
    let original = committed_raw();
    let art1 = artifacts(&scratch, "artifacts1", &original);
    let (lap1, _) = load_and_export(&scratch, "db1", &art1);

    let art2 = artifacts(&scratch, "artifacts2", &lap1);
    let (lap2, _) = load_and_export(&scratch, "db2", &art2);

    assert_eq!(lap2, lap1, "lap 2 differs from lap 1: a column is lossy");
}

/// Two crucibles against one database are two schedulers each believing they
/// own the queue: both dequeue the same board, and a box carefully limited to
/// two jobs quietly runs four. The hour's timings are worthless and nothing in
/// the output says so.
#[test]
fn the_lock_refuses_a_second_opener() {
    let scratch = Scratch::new("lock");
    let dir = scratch.join("db");
    let _first = Db::open(&dir).expect("first open");
    match Db::open(&dir) {
        Err(DbError::Lock(LockError::Busy { .. })) => {}
        Err(other) => panic!("expected a busy lock, got {other:?}"),
        // `Db` is deliberately not `Debug` -- it owns a live connection and a
        // thread -- so the success case is named rather than formatted.
        Ok(_) => panic!("a second crucible opened a database another one holds"),
    }
}

/// A rebuilt run's timing quality is `unknown`, and it is NEVER `clean`.
///
/// Only 4 of the 76 conditions files in this repo carry a per-sample timeline;
/// this board's carries a whole-board rollup and the single word "clean". That
/// word is the BOARD's verdict, and promoting it to a per-run one -- or taking
/// the spec's `DEFAULT 'clean'` -- would stamp 256 rows with a cleanliness
/// nobody measured, in one statement, silently.
#[test]
fn a_rebuilt_run_is_unknown_never_clean() {
    let scratch = Scratch::new("timing");
    let art = artifacts(&scratch, "artifacts", &committed_raw());
    let db = Db::open(&scratch.join("db")).expect("open");
    let built = rebuild_from_artifacts(db.writer(), &manifest(), &art, None).expect("rebuild");
    let b = &built[0];
    let reader = db.reader().expect("reader");

    let census = reader
        .timing_census(b.board_id, b.engine_id)
        .expect("census");
    assert_eq!(
        census,
        vec![("unknown".to_string(), 256)],
        "a rebuilt run claimed a timing verdict it cannot have"
    );

    // The board-level verdict IS kept -- it is real evidence, just not a
    // per-run one.
    let (verdict, ran, reused) = reader
        .pass_verdict(b.board_id, b.engine_id)
        .expect("pass lookup")
        .expect("a pass was recorded");
    assert_eq!(verdict, crucible_core::db::PassVerdict::Clean);
    assert_eq!((ran, reused), (256, 0));
}

/// Re-importing the same artifacts must update the rows, not add a second
/// attempt beside them.
///
/// `rebuild` is what a restart runs, so a non-idempotent import does not fail
/// -- it doubles the board, and `attempt = 2` rows then win the "highest
/// attempt" rule in the exporter. The board would still export 256 lines and
/// still look right.
#[test]
fn re_importing_does_not_create_a_second_attempt() {
    let scratch = Scratch::new("idempotent");
    let art = artifacts(&scratch, "artifacts", &committed_raw());
    let db = Db::open(&scratch.join("db")).expect("open");
    let m = manifest();

    let first = rebuild_from_artifacts(db.writer(), &m, &art, None).expect("first rebuild");
    let second = rebuild_from_artifacts(db.writer(), &m, &art, None).expect("second rebuild");
    assert_eq!(
        first[0].board_id, second[0].board_id,
        "board identity moved"
    );
    assert_eq!(
        first[0].engine_id, second[0].engine_id,
        "engine identity moved"
    );
    assert_eq!(
        first[0].pass_id, second[0].pass_id,
        "a second pass row appeared"
    );

    let reader = db.reader().expect("reader");
    let (rows, max_attempt) = reader
        .run_census(first[0].board_id, first[0].engine_id)
        .expect("census");
    assert_eq!(rows, 256, "re-importing duplicated rows");
    assert_eq!(max_attempt, 1, "re-importing created attempt=2 rows");

    // ...and the export is still the committed bytes.
    let text = export(&reader, first[0].board_id, first[0].engine_id).expect("export");
    assert_eq!(text, committed_raw());
}

/// `val` is a tristate, and the two ways a row can fail to be "valid" are
/// different questions with different answers.
///
/// On this board 77 rows solved and every one of those plans was accepted by
/// VAL; the other 179 never produced a plan, so there was nothing to submit and
/// validation is UNAVAILABLE for them. Zero plans were rejected.
///
/// A `validated INTEGER` column of the kind `crucible-spec.md` §8 sketches
/// cannot hold that. Flattened, those 179 nulls become 179 zeros, and the next
/// query that reaches for `= 0` to mean "not valid" reports 179 rejected plans
/// on a board that had none -- which is the shape of the 0.20, 0.21 and 0.23
/// incidents, where domains VAL could not INGEST were counted as domains VAL
/// had REJECTED and a published table came out fifteen instances light.
#[test]
fn unavailable_validation_is_not_a_rejection() {
    let scratch = Scratch::new("val");
    let art = artifacts(&scratch, "artifacts", &committed_raw());
    let db = Db::open(&scratch.join("db")).expect("open");
    let built = rebuild_from_artifacts(db.writer(), &manifest(), &art, None).expect("rebuild");
    let b = &built[0];
    let reader = db.reader().expect("reader");

    assert_eq!(
        reader.val_ok(b.board_id, b.engine_id).unwrap(),
        77,
        "every solved plan on this board was accepted"
    );
    assert_eq!(
        reader.val_unavailable(b.board_id, b.engine_id).unwrap(),
        179,
        "the unsolved rows have validation unavailable, not failed"
    );
    assert_eq!(
        reader.val_rejected(b.board_id, b.engine_id).unwrap(),
        0,
        "`val = 0` must never pick up a NULL: this board has no rejected plans"
    );
    // The three answers partition the board. If they ever stop adding up, one
    // of the queries has started reading NULL as a verdict.
    let (rows, _) = reader.run_census(b.board_id, b.engine_id).unwrap();
    assert_eq!(rows, 77 + 179);
}

/// The four boards with a real timeline are the only ones whose samples can be
/// imported at all, and importing one twice must not double it -- a doubled
/// timeline does not change the resume gate's verdict, it just makes every
/// future span query slower and every sample count a lie.
#[test]
fn a_re_imported_timeline_is_not_doubled() {
    let scratch = Scratch::new("timeline");
    let art = artifacts(&scratch, "artifacts", &committed_raw());
    // Swap in the one conditions file shape that carries a timeline. It belongs
    // to a different board; only its timeline is being exercised here.
    let with_timeline =
        repo_root().join("crucible/tests/fixtures/conditions/timeline-complex-pref.json");
    if !with_timeline.exists() {
        return; // fixture not vendored on this checkout; nothing to prove
    }
    std::fs::copy(&with_timeline, art.join("ipc2014-opt.conditions.json")).expect("copy");

    let db = Db::open(&scratch.join("db")).expect("open");
    let m = manifest();
    let first = rebuild_from_artifacts(db.writer(), &m, &art, None).expect("first");
    let reader = db.reader().expect("reader");
    let once = reader.sample_count(Some(first[0].pass_id)).expect("count");
    assert!(once > 0, "the fixture carries a timeline");

    let again = rebuild_from_artifacts(db.writer(), &m, &art, None).expect("second");
    let reader = db.reader().expect("reader");
    assert_eq!(
        reader.sample_count(Some(again[0].pass_id)).expect("count"),
        once,
        "re-importing a conditions file doubled its timeline"
    );
}

/// The resume gate, in the shape it takes now that the timeline lives in SQL
/// rather than in one pass's conditions file.
///
/// `ipc67.py:load_resume` fails CLOSED in four separate ways and every one of
/// them has to survive the translation, because the failure direction is not
/// symmetric: answering "re-run" costs a few minutes of compute, and answering
/// "clean" stitches a contended row into a published board with nothing in the
/// output saying so.
///
/// The thing SQL buys is the span. The Python could only ever consult ONE prior
/// pass's timeline, so any window outside it re-ran regardless of what the box
/// was actually doing; a box-wide table has no such limit.
#[test]
fn the_resume_gate_fails_closed_in_every_direction() {
    use crucible_core::db::{Cleanliness, SampleRec};

    let scratch = Scratch::new("gate");
    let db = Db::open(&scratch.join("db")).expect("open");
    let w = db.writer();

    // A quiet stretch at t=1000..1100, one twenty-second interval apart, with
    // one spike and one unattributable sample deliberately placed in it.
    let quiet = [
        (1000.0, Some(4.0)),
        (1020.0, Some(9.0)),
        (1040.0, Some(60.0)), // a real competitor: over the 25% line
        (1060.0, None),       // the sampler could not attribute at all
        (1080.0, Some(3.0)),
        (1100.0, Some(5.0)),
    ];
    for (at, total) in quiet {
        w.sample(SampleRec {
            at,
            competitors_total: total,
            ..SampleRec::default()
        });
    }
    w.flush().expect("flush");
    let r = db.reader().expect("reader");

    // Clean: the window sits between the two quiet samples at either end, and
    // the padding reaches only samples under the line.
    assert_eq!(
        r.window_gate(1082.0, 1098.0, 20.0, None).unwrap(),
        Cleanliness::Clean
    );

    // Dirty: the padded window reaches the 60% spike. Per-sample, not a median
    // -- a clean average across a dirty stretch is the exact lie the whole-board
    // retry existed to prevent.
    assert_eq!(
        r.window_gate(1045.0, 1050.0, 20.0, None).unwrap(),
        Cleanliness::Dirty
    );

    // Dirty: a sample the watcher could not attribute is NOT evidence of
    // quiet. `load_resume` reads `t[2] is None` as dirty and so must this.
    assert_eq!(
        r.window_gate(1061.0, 1062.0, 5.0, None).unwrap(),
        Cleanliness::Dirty
    );

    // Uncovered: the window ends after the last sample plus one interval, so
    // nobody was watching for part of it. Not clean, not dirty -- unknown, and
    // the caller re-runs.
    assert_eq!(
        r.window_gate(1090.0, 5000.0, 20.0, None).unwrap(),
        Cleanliness::Uncovered
    );
    // ...and the same at the front edge.
    assert_eq!(
        r.window_gate(500.0, 1010.0, 20.0, None).unwrap(),
        Cleanliness::Uncovered
    );

    // Uncovered: inside the span but with no sample within an interval of the
    // window. An empty overlap is not a clean one.
    assert_eq!(
        r.window_gate(1005.0, 1006.0, 1.0, None).unwrap(),
        Cleanliness::Uncovered
    );
}

/// A `live_child` row exists so a startup reap can PROVE the pid it is about to
/// signal is still the process it recorded.
///
/// Pids recycle and process groups recycle with them, so a `killpg` on a number
/// read off disk can kill a stranger's work silently. The row therefore carries
/// (binary_path, proc_start_tvsec) as well as the pid -- and because pids
/// recycle, re-registering one must REPLACE the stale row rather than leave two
/// rows claiming the same number.
#[test]
fn a_live_child_carries_its_identity_and_a_recycled_pid_replaces_it() {
    use crucible_core::db::LiveChild;
    use crucible_core::platform::ProcIdentity;

    let scratch = Scratch::new("children");
    let db = Db::open(&scratch.join("db")).expect("open");
    let w = db.writer();

    let ghost = LiveChild {
        pid: 4242,
        pgid: 4242,
        run_id: None,
        binary_path: "/old/ff".into(),
        proc_start_tvsec: 1_700_000_000,
        spawned_at: 1.0,
        stopped: false,
    };
    w.child_spawned(ghost.clone()).expect("register");

    let fresh = LiveChild {
        binary_path: "/new/ff".into(),
        proc_start_tvsec: 1_800_000_000,
        spawned_at: 2.0,
        ..ghost.clone()
    };
    w.child_spawned(fresh.clone()).expect("re-register");

    let r = db.reader().expect("reader");
    let live = r.live_children().expect("read");
    assert_eq!(live.len(), 1, "a recycled pid left a second row behind");
    assert_eq!(live[0].identity(), fresh.identity());
    assert_ne!(
        live[0].identity(),
        ProcIdentity {
            path: ghost.binary_path.clone(),
            start_tvsec: ghost.proc_start_tvsec,
        },
        "the stale identity survived and would have been signalled"
    );

    w.child_gone(4242).expect("reap");
    let r = db.reader().expect("reader");
    assert!(r.live_children().expect("read").is_empty());
}

/// The per-process breakdown is the half of the timeline that says WHAT was
/// competing, and it has to survive the trip through the batch.
///
/// A conditions file's rollup can only ever answer "was the board clean". When
/// the answer is no, the next question is always "what was it" -- and if the
/// breakdown were dropped at write time, nothing would notice until somebody
/// went looking months later at a board that had already been published.
#[test]
fn the_per_process_breakdown_survives_the_batch() {
    use crucible_core::db::SampleRec;
    use crucible_core::monitor::Sample;

    let scratch = Scratch::new("competitors");
    let db = Db::open(&scratch.join("db")).expect("open");
    let w = db.writer();

    // Built from `monitor::Sample` rather than by hand, so the mapping under
    // test is the one the runner will actually use.
    let mut first = Sample {
        at: 100.0,
        idle_pct: Some(70.0),
        competitors_total: 31.0,
        ..Sample::default()
    };
    first
        .competitors
        .insert("Brave Browser Helper (Renderer)".into(), 22.0);
    first.competitors.insert("corespotlightd".into(), 9.0);
    w.sample(SampleRec::of(&first));

    let mut second = Sample {
        at: 120.0,
        competitors_total: 12.0,
        ..Sample::default()
    };
    second.competitors.insert("corespotlightd".into(), 12.0);
    w.sample(SampleRec::of(&second));
    w.flush().expect("flush");

    let r = db.reader().expect("reader");
    // Summed ACROSS samples and worst first: spotlight totals 21 over two
    // samples, the renderer 22 in one, so the renderer still leads.
    assert_eq!(
        r.competitors_between(90.0, 130.0).expect("competitors"),
        vec![
            ("Brave Browser Helper (Renderer)".to_string(), 22.0),
            ("corespotlightd".to_string(), 21.0),
        ]
    );

    // The window is a window: a sample outside it contributes nothing.
    assert_eq!(
        r.competitors_between(110.0, 130.0).expect("competitors"),
        vec![("corespotlightd".to_string(), 12.0)]
    );
}
