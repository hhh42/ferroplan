//! The ladder's time table (0.31 Lane A, docs/roadmap-0.31.md; `ladder.rs`).
//!
//! `FF_LADDER_TABLE=<file>` gives each rung its SECONDS at a reference
//! wall, scaled to the armed wall; the precedence is the explicit knob,
//! then the table, then the hand-set default fraction. Three pins, each a
//! child process (the
//! wall is a process-wide `OnceLock`, the table too), on the openstacks
//! shape from `tests/ladder_wall.rs` (`laddertax`): EHC needs seconds it is
//! not given, hands down, and novelty-light finishes in milliseconds.
//!
//! - `table`: `ehc = 3` at `wall = 60` under a 6 s wall is 0.3 s; EHC's
//!   slice trips at ~0.3 s (the narrated seconds), the task solves.
//! - `explicit-wins`: the same table with `FF_EHC_WALL_FRAC=0.5` set --
//!   the knob wins, no table line is narrated, the slice is ~3 s.
//! - `hatched`: `FF_LADDER_TABLE=0.30` -- byte-identical to no table.
//! - `bad-table`: a malformed file is narrated once and the ladder runs
//!   on its defaults; the task still solves.

use std::process::Command;

fn laddertax(l: usize) -> (String, String) {
    let mut preds = String::from(" (ga) (gb) (one)");
    for i in 0..=l {
        preds.push_str(&format!(" (p{i})"));
    }
    let mut acts = String::from(
        "(:action make-ga :parameters () :precondition (one) :effect (and (ga) (not (one))))\n\
         (:action make-gb :parameters () :precondition (one) :effect (and (gb) (not (one))))\n",
    );
    acts.push_str(&format!(
        "(:action refill :parameters () :precondition (p{l}) :effect (and (one) (p0) (not (p{l}))))\n"
    ));
    for i in 0..l {
        acts.push_str(&format!(
            "(:action step{i} :parameters () :precondition (p{i}) :effect (and (p{}) (not (p{i}))))\n",
            i + 1
        ));
    }
    (
        format!("(define (domain laddertax) (:predicates{preds}) {acts})"),
        "(define (problem lt) (:domain laddertax) (:init (one) (p0)) (:goal (and (ga) (gb))))"
            .into(),
    )
}

const WALL_SECS: f64 = 6.0;

fn table_file(name: &str, body: &str) -> std::path::PathBuf {
    let dir = std::env::temp_dir().join(format!("ferroplan-ladder-table-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let p = dir.join(name);
    std::fs::write(&p, body).unwrap();
    p
}

fn run_child(scenario: &str) -> (String, String) {
    let exe = std::env::current_exe().unwrap();
    let mut cmd = Command::new(&exe);
    cmd.args(["--exact", "the_table_sets_a_rungs_slice", "--nocapture"])
        .env("LADDER_TABLE_CHILD", scenario)
        .env("FF_WALL_DEBUG", "1")
        .env("FF_TIME_LIMIT", WALL_SECS.to_string());
    cmd.env_remove("FF_PARTITION_RUNG");
    cmd.env_remove("FF_EHC_WALL_FRAC");
    cmd.env_remove("FF_LADDER_TABLE");
    let good = table_file(
        "good.tbl",
        "# a learned table\nwall = 60\nehc = 3\nlama = 18\n",
    );
    let bad = table_file("bad.tbl", "wall = 60\nehc three\n");
    match scenario {
        "table" => {
            cmd.env("FF_LADDER_TABLE", &good);
        }
        "explicit-wins" => {
            cmd.env("FF_LADDER_TABLE", &good)
                .env("FF_EHC_WALL_FRAC", "0.5");
        }
        "hatched" => {
            cmd.env("FF_LADDER_TABLE", "0.30");
        }
        "bad-table" => {
            cmd.env("FF_LADDER_TABLE", &bad);
        }
        _ => unreachable!(),
    }
    let out = cmd.output().unwrap();
    assert!(out.status.success(), "child {scenario} failed");
    (
        String::from_utf8_lossy(&out.stdout).into_owned(),
        String::from_utf8_lossy(&out.stderr).into_owned(),
    )
}

fn field<'a>(stdout: &'a str, key: &str) -> &'a str {
    stdout
        .lines()
        .find_map(|l| l.trim().strip_prefix(key))
        .unwrap_or_else(|| panic!("child printed no {key}:\n{stdout}"))
}

/// The seconds EHC's hand-down line narrates: `wall: EHC slice exhausted
/// (N evals in S.SSs), handing down the ladder`.
fn ehc_exhausted_secs(stderr: &str) -> f64 {
    let line = stderr
        .lines()
        .find(|l| l.contains("EHC slice exhausted"))
        .unwrap_or_else(|| panic!("EHC never handed down:\n{stderr}"));
    let after = line.split(" in ").nth(1).unwrap();
    after.split('s').next().unwrap().parse().unwrap()
}

#[test]
fn the_table_sets_a_rungs_slice() {
    if std::env::var("LADDER_TABLE_CHILD").is_ok() {
        // EHC needs ~4 s solo on this (calibrated in ladder_wall.rs);
        // under a quarter of a 6 s wall it hands down at 1.5 s, under the
        // table at 0.3 s. L=1700 release; the debug build is slower per
        // eval and a shorter ladder keeps the slice the thing measured.
        let (dom, prb) = laddertax(if cfg!(debug_assertions) { 900 } else { 1700 });
        let opts = ferroplan::Options {
            threads: 1,
            ..Default::default()
        };
        let sol = ferroplan::solve(&dom, &prb, &opts).unwrap();
        println!("CHILD-SOLVED:{}", sol.solved);
        println!("CHILD-NOTES:{}", sol.notes.join(" | "));
        return;
    }

    // (1) The table's seconds, scaled to the wall, are EHC's slice.
    let (stdout, stderr) = run_child("table");
    assert_eq!(
        field(&stdout, "CHILD-SOLVED:"),
        "true",
        "{stdout}\n{stderr}"
    );
    assert!(
        stderr.contains("wall: table gives ehc 0.30s"),
        "the table's share was not narrated:\n{stderr}"
    );
    let secs = ehc_exhausted_secs(&stderr);
    assert!(
        (0.25..0.9).contains(&secs),
        "EHC handed down at {secs:.2}s, expected ~0.3s under the table:\n{stderr}"
    );
    assert!(
        field(&stdout, "CHILD-NOTES:").contains("rung: solved by novelty-light"),
        "the light rung must have solved it:\n{stdout}"
    );

    // (2) An explicit knob wins over the table.
    let (stdout, stderr) = run_child("explicit-wins");
    assert_eq!(
        field(&stdout, "CHILD-SOLVED:"),
        "true",
        "{stdout}\n{stderr}"
    );
    assert!(
        !stderr.contains("wall: table gives ehc"),
        "the knob was set; the table must not be consulted:\n{stderr}"
    );
    let secs = ehc_exhausted_secs(&stderr);
    assert!(
        secs > 1.5,
        "EHC handed down at {secs:.2}s under FF_EHC_WALL_FRAC=0.5 of 6s:\n{stderr}"
    );

    // (3) `0.30` is no table at all.
    let (stdout, stderr) = run_child("hatched");
    assert_eq!(
        field(&stdout, "CHILD-SOLVED:"),
        "true",
        "{stdout}\n{stderr}"
    );
    assert!(!stderr.contains("wall: table gives"), "{stderr}");
    let secs = ehc_exhausted_secs(&stderr);
    assert!(
        secs > 0.9,
        "EHC handed down at {secs:.2}s; the default quarter of 6s is 1.5s:\n{stderr}"
    );

    // (4) A malformed table is narrated and the defaults run.
    let (stdout, stderr) = run_child("bad-table");
    assert_eq!(
        field(&stdout, "CHILD-SOLVED:"),
        "true",
        "{stdout}\n{stderr}"
    );
    assert!(
        stderr.contains("the ladder runs on its defaults") && stderr.contains("line 2"),
        "a malformed table must be narrated with its line:\n{stderr}"
    );
    assert!(!stderr.contains("wall: table gives"), "{stderr}");
}
