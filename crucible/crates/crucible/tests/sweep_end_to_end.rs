//! A whole sweep, from the manifest to the artifacts, against a planner that
//! does exactly what it is told.
//!
//! This is the chain the shell drivers ran: read the board list, walk the
//! corpus, measure each instance, write the raw and the summary, and bank the
//! board only when nothing is still owed. Everything real is exercised except
//! the planner itself -- and the planner is exercised by the paired
//! differential sweep, which is the only place it belongs.

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

[[set]]
name = "toyset"
stage = "benchmarks/airtest"
requires_version = "0.0.0-fake"
boards = ["toy-board"]
"#;

/// A corpus of two instances under one variant, laid out exactly as
/// `get-ipc.sh` normalises the real one.
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

    // The candidate binary lives where a ferroplan checkout puts it.
    let bin = dir.join("target/release/ff");
    std::fs::create_dir_all(bin.parent().unwrap()).unwrap();
    // fakeff is a bin of crucible-core, so CARGO_BIN_EXE_ is not defined for
    // this crate's tests; it sits beside this test binary either way.
    let mut src = std::env::current_exe().unwrap();
    src.pop();
    src.pop();
    src.push("fakeff");
    std::fs::copy(&src, &bin).unwrap_or_else(|e| panic!("{}: {e}", src.display()));
    bin
}

fn tmp(name: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!("crucible-e2e-{name}"));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    d
}

fn crucible(repo: &Path, args: &[&str]) -> std::process::Output {
    std::process::Command::new(env!("CARGO_BIN_EXE_crucible"))
        .arg("--repo")
        .arg(repo)
        .args(args)
        // The sweep scrubs the child environment, so fakeff's instructions
        // reach it the way a real board's hatches do -- declared, and on the
        // record. Here there are none, so it takes its defaults: solved.
        .env("CRUCIBLE_CONFIG", repo.join("nonexistent.toml"))
        .output()
        .expect("crucible runs")
}

/// A test box is a BUSY box -- cargo is compiling three crates beside it -- so
/// the sweep may honestly refuse to bank anything. That is the policy working,
/// not a failure: the rows are still measured and still written. What this
/// asserts is the chain, not the verdict.
#[test]
fn a_sweep_measures_every_instance_and_writes_its_artifacts() {
    let repo = tmp("bank");
    make_repo(&repo);

    let out = crucible(&repo, &["sweep", "--set", "toyset", "--max-passes", "1"]);
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        out.status.success(),
        "stdout:\n{stdout}\nstderr:\n{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let stage = repo.join("benchmarks/airtest");

    // The raw, in the corpus's canonical order.
    let raw = std::fs::read_to_string(stage.join("toy-board.jsonl")).expect("a raw is written");
    let lines: Vec<&str> = raw.lines().collect();
    assert_eq!(lines.len(), 2, "one row per instance: {raw}");
    assert!(lines[0].contains(r#""instance": 1"#), "{}", lines[0]);
    assert!(lines[1].contains(r#""instance": 2"#), "{}", lines[1]);
    // Every row carries the tuple the resume gate compares.
    for l in &lines {
        for stamp in [
            r#""budget": 5"#,
            r#""mode": "auto""#,
            r#""jobs": 2"#,
            r#""threads": "1""#,
        ] {
            assert!(l.contains(stamp), "missing {stamp} in {l}");
        }
    }
    // A solved row carries makespan last; an unsolved one omits it entirely.
    assert!(lines[0].ends_with(r#""makespan": null}"#), "{}", lines[0]);

    // The summary the drivers `tail -1` for their log line.
    let md = std::fs::read_to_string(stage.join("toy-board.md")).expect("a summary is written");
    assert!(md.contains("total coverage: **2/2**"), "{md}");
    assert!(md.contains("VAL not available"), "no validator here: {md}");

    // The zero-byte marker is the whole board-level checkpoint -- and it is
    // written if and only if NOTHING is still owed. On a quiet box that is
    // after one pass; on a busy one it is not, and the rows above are still
    // there either way. Refuse-not-bank, at row granularity.
    let done = stage.join("toy-board.done");
    let banked = stdout.contains("SWEEP COMPLETE");
    assert_eq!(
        done.exists(),
        banked,
        "the marker and the verdict must agree: {stdout}"
    );
    if banked {
        assert_eq!(
            std::fs::metadata(&done).unwrap().len(),
            0,
            "zero-byte marker"
        );
    }
}

/// The version gate every sweep driver opens with. Measuring whatever happens
/// to be built, rather than the cut candidate, is how a board ends up
/// attributed to the wrong engine.
#[test]
fn a_sweep_refuses_a_binary_that_is_not_the_candidate() {
    let repo = tmp("version");
    make_repo(&repo);
    let out = crucible(
        &repo,
        &["sweep", "--set", "toyset", "--require-version", "9.9"],
    );
    assert!(!out.status.success());
    let err = String::from_utf8_lossy(&out.stderr);
    assert!(err.contains("build the cut candidate first"), "{err}");
    assert!(
        !repo.join("benchmarks/airtest/toy-board.jsonl").exists(),
        "a refused sweep measures nothing"
    );
}

/// A dry run reports the plan and touches nothing. It is how a sweep gets
/// checked before days of the machine are committed to it.
#[test]
fn a_dry_run_writes_nothing() {
    let repo = tmp("dry");
    make_repo(&repo);
    let out = crucible(&repo, &["sweep", "--set", "toyset", "--dry-run"]);
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(out.status.success(), "{stdout}");
    assert!(stdout.contains("toy-board"));
    assert!(stdout.contains("nothing measured, nothing written"));
    assert!(!repo.join("benchmarks/airtest").exists());
}

/// Re-running a banked board re-measures nothing: every instance already has a
/// clean row, so there is no work to do. This is the property that makes a
/// killed sweep cheap to resume.
#[test]
fn a_second_sweep_of_a_banked_board_has_nothing_to_do() {
    let repo = tmp("resume");
    make_repo(&repo);
    assert!(
        crucible(&repo, &["sweep", "--set", "toyset", "--max-passes", "1"])
            .status
            .success()
    );
    let first = std::fs::read_to_string(repo.join("benchmarks/airtest/toy-board.jsonl")).unwrap();

    let out = crucible(&repo, &["sweep", "--set", "toyset", "--max-passes", "1"]);
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(out.status.success(), "{stdout}");
    let second = std::fs::read_to_string(repo.join("benchmarks/airtest/toy-board.jsonl")).unwrap();
    assert_eq!(
        first.lines().count(),
        second.lines().count(),
        "the board still holds one row per instance"
    );
}
