//! A SUBSET of a set, end to end (`src/select.rs`): the same runner, referee
//! and owed-row cascade over fewer cells -- and nothing said about a board.
//!
//! The toy set is two boards of three instances each. Everything real is
//! exercised except the planner (`fakeff`, which solves what it is given, so
//! every measured row BANKS whatever the test box is doing: a solve is a
//! solve).

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

[track.alpha]
ipcs = ["ipc-test"]
include = "-alpha$"

[track.beta]
ipcs = ["ipc-test"]
include = "-beta$"

[[board]]
id = "toy-a"
raw = "toy-a.jsonl"
md = "toy-a.md"
label = "toy a"
competition = "test"
budget_secs = 5
track = "alpha"
timeout_secs = 5
rebaselined_on = ["m5-air"]

[[board]]
id = "toy-b"
raw = "toy-b.jsonl"
md = "toy-b.md"
label = "toy b"
competition = "test"
budget_secs = 5
track = "beta"
timeout_secs = 5
rebaselined_on = ["m5-air"]

[[set]]
name = "toyset"
stage = "benchmarks/airtest"
requires_version = "0.0.0-fake"
boards = ["toy-a", "toy-b"]
"#;

/// The PROMOTED raw for toy-a: the predecessor solved 1, missed 2, and never
/// saw 3 -- the three things `--prior` has to tell apart.
const PROMOTED_A: &str = concat!(
    r#"{"ipc": "ipc-test", "variant": "toy-alpha", "instance": 1, "solved": true, "time": 0.1, "metric": null, "length": 1, "val": null, "notes": null, "budget": 5, "ver": "ff 0.0.0-old", "mode": "auto", "jobs": 2, "threads": "1"}"#,
    "\n",
    r#"{"ipc": "ipc-test", "variant": "toy-alpha", "instance": 2, "solved": false, "time": 5, "metric": null, "length": null, "val": null, "notes": null, "budget": 5, "ver": "ff 0.0.0-old", "mode": "auto", "jobs": 2, "threads": "1"}"#,
    "\n",
);

fn fakeff() -> PathBuf {
    // fakeff is a bin of crucible-core, so CARGO_BIN_EXE_ is not defined for
    // this crate's tests; it sits beside this test binary either way.
    let mut src = std::env::current_exe().unwrap();
    src.pop();
    src.pop();
    src.push("fakeff");
    src
}

fn make_repo(dir: &Path) {
    for variant in ["toy-alpha", "toy-beta"] {
        let v = dir
            .join("benchmarks/.ipc-corpus/ipc-test/domains")
            .join(variant);
        std::fs::create_dir_all(v.join("instances")).unwrap();
        std::fs::write(v.join("domain.pddl"), "(define (domain toy))").unwrap();
        for n in 1u32..=3 {
            std::fs::write(
                v.join("instances").join(format!("instance-{n}.pddl")),
                format!("(define (problem p{n}))"),
            )
            .unwrap();
        }
    }
    std::fs::write(dir.join("benchmarks/manifest.toml"), MANIFEST).unwrap();
    std::fs::write(dir.join("benchmarks/toy-a.jsonl"), PROMOTED_A).unwrap();
    // The database lives INSIDE the scratch repo (see sweep_end_to_end.rs).
    std::fs::write(
        dir.join("config.toml"),
        format!("[db]\ndir = {:?}\n", dir.join("db").display().to_string()),
    )
    .unwrap();
    let bin = dir.join("target/release/ff");
    std::fs::create_dir_all(bin.parent().unwrap()).unwrap();
    std::fs::copy(fakeff(), &bin).unwrap();
}

fn tmp(name: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!("crucible-subset-{name}"));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    d
}

fn crucible(repo: &Path, args: &[&str]) -> (bool, String, String) {
    let out = std::process::Command::new(env!("CARGO_BIN_EXE_crucible"))
        .arg("--repo")
        .arg(repo)
        .args(args)
        .env("CRUCIBLE_CONFIG", repo.join("config.toml"))
        .env_remove("CRUCIBLE_NO_DB")
        .output()
        .expect("crucible runs");
    (
        out.status.success(),
        String::from_utf8_lossy(&out.stdout).into_owned(),
        String::from_utf8_lossy(&out.stderr).into_owned(),
    )
}

/// The one probe stage under `benchmarks/probes/<name>/` (its directory is
/// named for the engine, which the test does not know the hash of).
fn probe_stage(repo: &Path, name: &str) -> PathBuf {
    let root = repo.join("benchmarks/probes").join(name);
    let mut dirs: Vec<PathBuf> = std::fs::read_dir(&root)
        .unwrap_or_else(|e| panic!("{}: {e}", root.display()))
        .map(|e| e.unwrap().path())
        .collect();
    assert_eq!(dirs.len(), 1, "one engine, one directory: {dirs:?}");
    dirs.pop().unwrap()
}

fn instances_in(raw: &Path) -> Vec<String> {
    std::fs::read_to_string(raw)
        .unwrap_or_default()
        .lines()
        .map(|l| {
            let i = l.find(r#""instance": "#).expect("a row") + r#""instance": "#.len();
            l[i..].split(',').next().unwrap().trim().to_string()
        })
        .collect()
}

#[test]
fn a_subset_measures_its_cells_and_says_nothing_about_a_board() {
    let repo = tmp("cells");
    make_repo(&repo);
    let (ok, stdout, stderr) = crucible(
        &repo,
        &[
            "sweep",
            "--set",
            "toyset",
            "--max-passes",
            "1",
            "--board",
            "toy-a",
            "--only",
            "/(2|3)$",
            "--name",
            "two-cells",
        ],
    );
    assert!(ok, "stdout:\n{stdout}\nstderr:\n{stderr}");
    assert!(stdout.contains("subset  boards toy-a; only /"), "{stdout}");
    assert!(stdout.contains("2 instance(s) of set toyset"), "{stdout}");

    // Its cells, and only its cells, under probes/<name>/<engine>/.
    let stage = probe_stage(&repo, "two-cells");
    assert_eq!(instances_in(&stage.join("toy-a.jsonl")), ["2", "3"]);
    assert!(
        !stage.join("toy-b.jsonl").exists(),
        "toy-b was not selected"
    );

    // NOTHING where a set stages: a board raw with two of its three rows,
    // written there, is exactly the artifact `promote` would read -- and a
    // `.done` there is what `resident` reads as "the set is complete".
    assert!(
        !repo.join("benchmarks/airtest").exists(),
        "a subset must not touch the set's stage"
    );

    // And no board pass: the rows are on record, the board is not.
    let db = rusqlite::Connection::open(repo.join("db/crucible.db")).unwrap();
    let runs: i64 = db
        .query_row("SELECT count(*) FROM run", [], |r| r.get(0))
        .unwrap();
    let passes: i64 = db
        .query_row("SELECT count(*) FROM board_pass", [], |r| r.get(0))
        .unwrap();
    assert_eq!(runs, 2, "two cells measured, two receipts");
    assert_eq!(passes, 0, "a subset records no pass for any board");
}

/// A probe is never wasted work: its rows are keyed exactly as a full sweep
/// keys them, so the sweep that follows reads them back and owes that many
/// cells fewer.
#[test]
fn the_sweep_that_follows_owes_only_what_the_subset_did_not_measure() {
    let repo = tmp("reuse");
    make_repo(&repo);
    let (ok, stdout, stderr) = crucible(
        &repo,
        &[
            "sweep",
            "--set",
            "toyset",
            "--max-passes",
            "1",
            "--board",
            "toy-a",
            "--only",
            "/2$",
        ],
    );
    assert!(ok, "stdout:\n{stdout}\nstderr:\n{stderr}");

    let (ok, stdout, stderr) = crucible(&repo, &["sweep", "--set", "toyset", "--max-passes", "1"]);
    assert!(ok, "stdout:\n{stdout}\nstderr:\n{stderr}");
    assert!(
        stdout.contains("1 row(s) read back, 1 banked -- 2 still owed"),
        "the full sweep must resume from the probe's row:\n{stdout}"
    );
    // The full sweep DOES stage where the set stages, with every row -- the
    // probe's among them -- and it is the one that may say the board is done.
    let raw = repo.join("benchmarks/airtest/toy-a.jsonl");
    assert_eq!(instances_in(&raw), ["1", "2", "3"]);
    let db = rusqlite::Connection::open(repo.join("db/crucible.db")).unwrap();
    let runs: i64 = db
        .query_row("SELECT count(*) FROM run", [], |r| r.get(0))
        .unwrap();
    assert_eq!(
        runs, 6,
        "six cells, six receipts: instance 2 was not re-measured"
    );
}

/// `--prior` reads the PROMOTED raw: `unsolved` is what there is to gain (and
/// a cell the predecessor never recorded is a gain waiting to be measured),
/// `solved` is what there is to lose.
#[test]
fn prior_selects_by_what_the_promoted_raw_says() {
    let repo = tmp("prior");
    make_repo(&repo);
    let (ok, stdout, stderr) = crucible(
        &repo,
        &[
            "sweep",
            "--set",
            "toyset",
            "--max-passes",
            "1",
            "--board",
            "toy-a",
            "--prior",
            "unsolved",
            "--name",
            "gain",
        ],
    );
    assert!(ok, "stdout:\n{stdout}\nstderr:\n{stderr}");
    assert_eq!(
        instances_in(&probe_stage(&repo, "gain").join("toy-a.jsonl")),
        ["2", "3"]
    );
    let (ok, stdout, stderr) = crucible(
        &repo,
        &[
            "sweep",
            "--set",
            "toyset",
            "--max-passes",
            "1",
            "--board",
            "toy-a",
            "--prior",
            "solved",
            "--name",
            "lose",
        ],
    );
    assert!(ok, "stdout:\n{stdout}\nstderr:\n{stderr}");
    assert_eq!(
        instances_in(&probe_stage(&repo, "lose").join("toy-a.jsonl")),
        ["1"]
    );
}

#[test]
fn a_typo_stops_the_run_and_so_does_a_subset_of_nothing() {
    let repo = tmp("refuse");
    make_repo(&repo);
    let (ok, _, stderr) = crucible(&repo, &["sweep", "--set", "toyset", "--board", "toy-z"]);
    assert!(!ok);
    assert!(stderr.contains("--board toy-z: not in set"), "{stderr}");
    assert!(
        stderr.contains("toy-a, toy-b"),
        "it names what the set holds: {stderr}"
    );

    let (ok, _, stderr) = crucible(
        &repo,
        &[
            "sweep",
            "--set",
            "toyset",
            "--board",
            "toy-a",
            "--only",
            "^no-such-variant/",
        ],
    );
    assert!(!ok);
    assert!(stderr.contains("the subset selects nothing"), "{stderr}");
    assert!(
        !repo.join("benchmarks/probes").exists()
            || !repo.join("db/crucible.db").exists()
            || rusqlite::Connection::open(repo.join("db/crucible.db"))
                .unwrap()
                .query_row("SELECT count(*) FROM run", [], |r| r.get::<_, i64>(0))
                .unwrap()
                == 0,
        "a refused run measures nothing"
    );
}

/// Two engines measured over the same subset are COMPARED over that subset.
/// Without the selection every board of a probe reads `--`: both sides "owe"
/// the cells nobody asked them to measure.
#[test]
fn compare_reads_two_engines_over_the_cells_they_were_both_given() {
    let repo = tmp("compare");
    make_repo(&repo);
    let subset = ["--board", "toy-a", "--only", "/(1|2)$"];
    let mut args = vec!["sweep", "--set", "toyset", "--max-passes", "1"];
    args.extend(subset);
    let (ok, stdout, stderr) = crucible(&repo, &args);
    assert!(ok, "stdout:\n{stdout}\nstderr:\n{stderr}");

    // A second ENGINE: identity is the binary's content hash, so a wrapper
    // around the same planner is a different engine to every table.
    let bin = repo.join("target/release/ff");
    let inner = repo.join("target/release/ff-inner");
    std::fs::rename(&bin, &inner).unwrap();
    std::fs::write(
        &bin,
        format!("#!/bin/sh\nexec {:?} \"$@\"\n", inner.display().to_string()),
    )
    .unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&bin, std::fs::Permissions::from_mode(0o755)).unwrap();
    }
    let (ok, stdout, stderr) = crucible(&repo, &args);
    assert!(ok, "stdout:\n{stdout}\nstderr:\n{stderr}");

    let db = rusqlite::Connection::open(repo.join("db/crucible.db")).unwrap();
    let mut q = db.prepare("SELECT blake3 FROM engine ORDER BY id").unwrap();
    let hashes: Vec<String> = q
        .query_map([], |r| r.get::<_, String>(0))
        .unwrap()
        .map(|h| h.unwrap())
        .collect();
    assert_eq!(hashes.len(), 2, "two binaries, two engines");
    let (a, b) = (&hashes[0][..12], &hashes[1][..12]);

    // Unselected: toy-a owes instance 3 on both sides, toy-b owes everything.
    let (ok, whole, stderr) = crucible(&repo, &["compare", "--set", "toyset", "--a", a, "--b", b]);
    assert!(ok, "{whole}\n{stderr}");
    assert!(whole.contains("net +0 over the 0 board(s)"), "{whole}");

    // Selected: exactly the two cells, neither side owes, the board decides.
    let lost = repo.join("lost.rows");
    let mut cmp = vec!["compare", "--set", "toyset", "--a", a, "--b", b];
    cmp.extend(subset);
    cmp.extend(["--lost", lost.to_str().unwrap()]);
    let (ok, picked, stderr) = crucible(&repo, &cmp);
    assert!(ok, "{picked}\n{stderr}");
    assert!(picked.contains("over boards toy-a; only /"), "{picked}");
    assert!(picked.contains("net +0 over the 1 board(s)"), "{picked}");
    assert!(
        picked.contains("cells BOTH engines banked: 0 gained by B, 0 lost by B"),
        "{picked}"
    );
    let written = std::fs::read_to_string(&lost).unwrap();
    assert!(written.starts_with("# cells "), "{written}");
}

/// `--engine`: the instrument and the promoted raws come from `--repo`, the
/// binary from wherever it was built. A subset only -- the set's own stage
/// belongs to the working tree's candidate.
#[test]
fn an_arbitrary_binary_is_probed_and_never_swept() {
    let repo = tmp("engine");
    make_repo(&repo);
    // A second binary, somewhere else entirely; the repo's own candidate is
    // REMOVED, so a run that measures anything measured the named one.
    let elsewhere = repo.join("elsewhere/ff");
    std::fs::create_dir_all(elsewhere.parent().unwrap()).unwrap();
    std::fs::rename(repo.join("target/release/ff"), &elsewhere).unwrap();
    let engine = elsewhere.to_str().unwrap();

    let (ok, _, stderr) = crucible(&repo, &["sweep", "--set", "toyset", "--engine", engine]);
    assert!(
        !ok,
        "a whole set must not be swept with an arbitrary binary"
    );
    assert!(stderr.contains("only a SUBSET may"), "{stderr}");

    let (ok, stdout, stderr) = crucible(
        &repo,
        &[
            "sweep",
            "--set",
            "toyset",
            "--max-passes",
            "1",
            "--board",
            "toy-b",
            "--engine",
            engine,
            "--name",
            "elsewhere",
        ],
    );
    assert!(ok, "stdout:\n{stdout}\nstderr:\n{stderr}");
    assert_eq!(
        instances_in(&probe_stage(&repo, "elsewhere").join("toy-b.jsonl")),
        ["1", "2", "3"]
    );
    let db = rusqlite::Connection::open(repo.join("db/crucible.db")).unwrap();
    let path: String = db
        .query_row("SELECT binary_path FROM engine", [], |r| r.get(0))
        .unwrap();
    assert_eq!(
        path, engine,
        "the receipt names the binary that was measured"
    );
}
