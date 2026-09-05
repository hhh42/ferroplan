//! Surviving `kill -9` with zero lost work -- the premise, tested.
//!
//! `sweep_end_to_end.rs` proves the chain writes what it measured. This file
//! proves the other half: that a SECOND process, opened over the same stage
//! and database, owes exactly the instances that never banked a clean row --
//! and re-spawns nothing for the ones that did. The killed-process case is
//! not simulated: a crucible is started, `SIGKILL`ed mid-instance, and the
//! restart is asked what it found.

use std::path::{Path, PathBuf};

const MANIFEST: &str = r#"
schema = 1

[corpus]
root = ".ipc-corpus"
domain_shared = "domain.pddl"
domain_per_instance = "domains/domain-{first}.pddl"

[defaults]
timeout_secs = 60
jobs = 2
threads = 1
mode = "auto"
mem_gb = 0.0

[track.toy]
ipcs = ["ipc-test"]
include = "-satisficing$"

[[board]]
id = "toy-board"
raw = "toy-board.jsonl"
md = "toy-board.md"
label = "toy"
competition = "test"
budget_secs = 5
track = "toy"
timeout_secs = 5
rebaselined_on = ["m5-air"]

[[board]]
id = "slow-board"
raw = "slow-board.jsonl"
md = "slow-board.md"
label = "slow"
competition = "test"
budget_secs = 60
track = "toy"
timeout_secs = 60
rebaselined_on = ["m5-air"]
env = { FAKEFF_SLEEP_MS = "30000" }

[[set]]
name = "toyset"
stage = "benchmarks/airtest"
requires_version = "0.0.0-fake"
boards = ["toy-board"]

[[set]]
name = "slowset"
stage = "benchmarks/airslow"
boards = ["slow-board"]
"#;

/// A corpus of two instances under one variant, laid out exactly as
/// `get-ipc.sh` normalises the real one; the fake planner where a checkout
/// puts its candidate; and an operator config pointing the database INSIDE
/// the scratch repo, never at `~/.crucible`.
fn make_repo(dir: &Path) -> PathBuf {
    let v = dir.join("benchmarks/.ipc-corpus/ipc-test/domains/toy-satisficing");
    std::fs::create_dir_all(v.join("instances")).unwrap();
    std::fs::write(v.join("domain.pddl"), "(define (domain toy))").unwrap();
    for n in [1u32, 2] {
        std::fs::write(
            v.join("instances").join(format!("instance-{n}.pddl")),
            format!("(define (problem p{n}))"),
        )
        .unwrap();
    }
    std::fs::create_dir_all(dir.join("benchmarks")).unwrap();
    std::fs::write(dir.join("benchmarks/manifest.toml"), MANIFEST).unwrap();
    std::fs::write(
        dir.join("config.toml"),
        format!("[db]\ndir = {:?}\n", dir.join("db").display().to_string()),
    )
    .unwrap();

    let bin = dir.join("target/release/ff");
    std::fs::create_dir_all(bin.parent().unwrap()).unwrap();
    let mut src = std::env::current_exe().unwrap();
    src.pop();
    src.pop();
    src.push("fakeff");
    std::fs::copy(&src, &bin).unwrap_or_else(|e| panic!("{}: {e}", src.display()));
    bin
}

fn tmp(name: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!("crucible-kill9-{name}"));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    d
}

fn command(repo: &Path, args: &[&str]) -> std::process::Command {
    let mut c = std::process::Command::new(env!("CARGO_BIN_EXE_crucible"));
    c.arg("--repo")
        .arg(repo)
        .args(args)
        .env("CRUCIBLE_CONFIG", repo.join("config.toml"))
        .env_remove("CRUCIBLE_NO_DB");
    c
}

fn crucible(repo: &Path, args: &[&str]) -> (bool, String) {
    let out = command(repo, args).output().expect("crucible runs");
    let text = format!(
        "{}\n{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    (out.status.success(), text)
}

fn db_path(repo: &Path) -> PathBuf {
    repo.join("db").join(crucible_core::db::DB_FILE_NAME)
}

fn run_count(repo: &Path) -> i64 {
    let r = crucible_core::db::Reader::open(&db_path(repo)).expect("reader");
    r.conn()
        .query_row("SELECT COUNT(*) FROM run", [], |row| row.get(0))
        .expect("count")
}

/// Every row a sweep measures carries the hash of the binary that measured
/// it, in the exported raw as well as the database. Without the stamp the
/// resume gate refuses the row (`Reject::EngineUnstamped`) -- crucible's own
/// output was un-resumable under crucible's own gate.
#[test]
fn a_measured_row_is_stamped_with_the_engine_hash() {
    let repo = tmp("stamp");
    let bin = make_repo(&repo);
    let hash = blake3::hash(&std::fs::read(&bin).unwrap())
        .to_hex()
        .to_string();

    let (ok, text) = crucible(&repo, &["sweep", "--set", "toyset", "--max-passes", "1"]);
    assert!(ok, "{text}");
    let raw = std::fs::read_to_string(repo.join("benchmarks/airtest/toy-board.jsonl")).unwrap();
    assert_eq!(raw.lines().count(), 2, "{raw}");
    for l in raw.lines() {
        assert!(
            l.contains(&format!(r#""engine": "{hash}""#)),
            "unstamped row: {l}"
        );
    }
    assert!(db_path(&repo).exists(), "the database is the record");
    assert_eq!(run_count(&repo), 2, "one receipt per instance, committed");
}

/// `--no-db` is the restore hatch: the pre-database path, bit for bit -- no
/// database, and no stamp the old binary would not have written.
#[test]
fn the_no_db_hatch_writes_the_old_shape() {
    let repo = tmp("nodb");
    make_repo(&repo);
    let (ok, text) = crucible(
        &repo,
        &["sweep", "--set", "toyset", "--max-passes", "1", "--no-db"],
    );
    assert!(ok, "{text}");
    let raw = std::fs::read_to_string(repo.join("benchmarks/airtest/toy-board.jsonl")).unwrap();
    assert_eq!(raw.lines().count(), 2, "{raw}");
    assert!(!raw.contains(r#""engine""#), "{raw}");
    assert!(!db_path(&repo).exists(), "no database under --no-db");
}

/// THE PREMISE. A fresh process over the same stage and database owes
/// nothing for an instance that banked a clean row, and re-spawns zero of
/// them.
///
/// The first sweep runs on whatever box the tests are on, which may be too
/// busy to bank; so the clean verdicts are then written the way a quiet box
/// would have written them -- a later attempt, timing `clean`, same row --
/// and the restart is asked what it owes. The board identity the rows are
/// written under is reconstructed here from the manifest, and the test then
/// checks that the database still holds ONE board of that name: a key that
/// did not match the driver's would have created a second.
#[test]
fn a_restart_owes_nothing_that_banked_clean() {
    use crucible_core::db::{
        self, BoardFacts, BoardKey, Db, EngineFacts, EngineKey, Measured, RunRecord, RunState,
        TimingQuality,
    };

    let repo = tmp("restart");
    let bin = make_repo(&repo);
    let hash = blake3::hash(&std::fs::read(&bin).unwrap())
        .to_hex()
        .to_string();

    let (ok, text) = crucible(&repo, &["sweep", "--set", "toyset", "--max-passes", "1"]);
    assert!(ok, "{text}");
    let before = run_count(&repo);
    assert_eq!(before, 2);

    // Bank both instances clean, as a quiet box would have.
    {
        let db = Db::open(&repo.join("db")).expect("open");
        let r = db.reader().expect("reader");
        let bids = r.boards_named("toy-board").expect("boards");
        assert_eq!(bids.len(), 1, "one board identity: {bids:?}");
        let eids = r.engines_for_board(bids[0]).expect("engines");
        assert_eq!(eids.len(), 1, "one engine: {eids:?}");
        let rows = r.export_rows(bids[0], eids[0]).expect("rows");
        assert_eq!(rows.len(), 2);
        let (_, max_attempt) = r.run_census(bids[0], eids[0]).expect("census");
        let key = BoardKey {
            name: "toy-board".into(),
            budget_secs: 5.0,
            mode: "auto".into(),
            jobs: 2,
            threads: "1".into(),
            env: "{}".into(),
            args: "[]".into(),
        };
        let facts = BoardFacts {
            label: Some("toy".into()),
            competition: Some("test".into()),
            proof_track: false,
            threads_json: "\"1\"".into(),
        };
        let engine = EngineKey {
            blake3: Some(hash.clone()),
            ver: Some("ff 0.0.0-fake".into()),
        };
        for row in rows {
            db.writer()
                .run(RunRecord {
                    board: key.clone(),
                    board_facts: facts.clone(),
                    engine: engine.clone(),
                    engine_facts: EngineFacts::default(),
                    attempt: max_attempt + 1,
                    state: RunState::Done,
                    // R2: the resume reads the referee's column, not the
                    // timing. These rows banked (a clean window under the
                    // R1 rule, which is what a v3 migration would say).
                    banked: true,
                    verdict: Some("window".into()),
                    timing: TimingQuality::Clean,
                    val_reason: None,
                    row,
                    measured: Measured::default(),
                })
                .expect("bank");
        }
        db.writer().flush().expect("flush");
        assert_eq!(
            r.boards_named("toy-board").expect("boards").len(),
            1,
            "the reconstructed key matched the driver's -- no second board row"
        );
        assert_eq!(r.clean_instances(bids[0], eids[0]).expect("clean").len(), 2);
        assert_eq!(
            r.banked_instances(bids[0], eids[0]).expect("banked").len(),
            2
        );
        let _ = db::Cleanliness::Clean;
    }
    let banked = run_count(&repo);

    // The restart.
    let (ok, text) = crucible(&repo, &["sweep", "--set", "toyset", "--max-passes", "1"]);
    assert!(ok, "{text}");
    assert!(
        text.contains("SWEEP COMPLETE -- 0 banked in 0 pass(es)"),
        "the restart had nothing to do:\n{text}"
    );
    assert!(
        text.contains("2 row(s) read back, 2 banked -- 0 still owed"),
        "{text}"
    );
    assert_eq!(
        run_count(&repo),
        banked,
        "a restart re-spawned an instance that had banked clean"
    );
    let stage = repo.join("benchmarks/airtest");
    assert!(stage.join("toy-board.done").exists(), "banked");
    let raw = std::fs::read_to_string(stage.join("toy-board.jsonl")).unwrap();
    assert_eq!(
        raw.lines().count(),
        2,
        "the export regenerated from the database:\n{raw}"
    );
    assert!(raw.contains(r#""engine""#));
}

/// The half that has no destructor: a supervisor `SIGKILL`ed mid-instance
/// leaves a planner running that nobody owns. The `live_child` row was written
/// at spawn, so the next crucible finds it, verifies the process is the one
/// recorded, and kills it -- and closes the row either way.
#[test]
fn a_killed_supervisor_leaves_a_child_the_restart_reaps() {
    let repo = tmp("reap");
    make_repo(&repo);

    // slow-board's planner sleeps thirty seconds per instance under a sixty
    // second wall, so the supervisor is mid-instance for as long as we like.
    let mut child = command(&repo, &["sweep", "--set", "slowset", "--max-passes", "1"])
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn()
        .expect("spawn");
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(20);
    let orphan = loop {
        if let Ok(r) = crucible_core::db::Reader::open(&db_path(&repo)) {
            if let Ok(live) = r.live_children() {
                if let Some(c) = live.first() {
                    break c.clone();
                }
            }
        }
        assert!(
            std::time::Instant::now() < deadline,
            "no live_child row appeared within 20 s"
        );
        std::thread::sleep(std::time::Duration::from_millis(100));
    };
    child.kill().expect("SIGKILL the supervisor");
    let _ = child.wait();
    assert!(
        alive(orphan.pid),
        "the planner outlived its supervisor, as orphans do"
    );

    // The restart reaps first. The set is the quick one so the test is not
    // waiting on the slow board's wall.
    let (ok, text) = crucible(&repo, &["sweep", "--set", "toyset", "--max-passes", "1"]);
    assert!(ok, "{text}");
    assert!(text.contains("reap    1 child(ren)"), "{text}");
    assert!(
        text.contains(&format!(
            "killed {} (group {}) -- verified ours",
            orphan.pid, orphan.pgid
        )),
        "{text}"
    );
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
    while alive(orphan.pid) {
        assert!(
            std::time::Instant::now() < deadline,
            "the orphan survived the reap"
        );
        std::thread::sleep(std::time::Duration::from_millis(50));
    }
    let r = crucible_core::db::Reader::open(&db_path(&repo)).expect("reader");
    assert!(
        r.live_children().expect("rows").is_empty(),
        "the row was closed"
    );
}

fn alive(pid: i32) -> bool {
    // `ps -p` rather than kill(0): the zombie of a reaped-but-unwaited child
    // still answers kill, and this test wants to know whether it is RUNNING.
    std::process::Command::new("ps")
        .args(["-o", "stat=", "-p", &pid.to_string()])
        .output()
        .map(|o| {
            let s = String::from_utf8_lossy(&o.stdout);
            !s.trim().is_empty() && !s.contains('Z')
        })
        .unwrap_or(false)
}
