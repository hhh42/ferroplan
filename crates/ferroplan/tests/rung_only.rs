//! The matrix hatch (0.31 Phase 0.5, docs/roadmap-0.31.md; `search.rs`
//! `FF_RUNG_ONLY`): one rung of the ladder, the whole wall, ITS verdict.
//!
//! The per-rung solve-time matrix the ladder table is learned from is built
//! by running every cell of a sample once per rung this way, so the hatch
//! must (a) run the named rung and nothing after it -- a rung that fails
//! must NOT fall through to the next, or the matrix would attribute the
//! ladder's solve to the rung; (b) say which rung solved in the JSON notes,
//! so the crucible reads it without stderr. Each leg is a child process on
//! the openstacks shape from `tests/ladder_wall.rs` (`laddertax`): EHC
//! needs ~4 s solo, novelty-light finishes in milliseconds.
//!
//! - `novlight`: solves, note `rung: solved by novelty-light`.
//! - `ehc` under a 1 s wall: UNSOLVED (EHC alone cannot), and the child
//!   returns inside ~2x the wall -- it did not fall through to the light
//!   rung, which would have solved it instantly.
//! - `lama`, `bfs`: solve, with their notes.
//! - `partition`: the route, mapped in `api.rs`, solves a decomposable
//!   task and says so.
//! - an unknown name: narrated, the ladder runs as usual and solves.

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

/// Two independent counters to raise (tests/partition_rung.rs's shape):
/// the partition route solves it as two subgoals.
fn counters() -> (String, String) {
    (
        "(define (domain twocount)
          (:requirements :strips)
          (:predicates (a0) (a1) (a2) (a3) (b0) (b1) (b2) (b3))
          (:action a01 :precondition (a0) :effect (and (a1) (not (a0))))
          (:action a12 :precondition (a1) :effect (and (a2) (not (a1))))
          (:action a23 :precondition (a2) :effect (and (a3) (not (a2))))
          (:action b01 :precondition (b0) :effect (and (b1) (not (b0))))
          (:action b12 :precondition (b1) :effect (and (b2) (not (b1))))
          (:action b23 :precondition (b2) :effect (and (b3) (not (b2)))))"
            .into(),
        "(define (problem tc) (:domain twocount) (:init (a0) (b0)) (:goal (and (a3) (b3))))".into(),
    )
}

fn run_child(scenario: &str, rung: &str, wall: &str) -> (String, String, f64) {
    let exe = std::env::current_exe().unwrap();
    let mut cmd = Command::new(&exe);
    cmd.args([
        "--exact",
        "one_rung_the_whole_wall_its_verdict",
        "--nocapture",
    ])
    .env("RUNG_ONLY_CHILD", scenario)
    .env("FF_RUNG_ONLY", rung)
    .env("FF_WALL_DEBUG", "1")
    .env("FF_TIME_LIMIT", wall);
    cmd.env_remove("FF_PARTITION_RUNG");
    let t0 = std::time::Instant::now();
    let out = cmd.output().unwrap();
    let secs = t0.elapsed().as_secs_f64();
    assert!(out.status.success(), "child {scenario}/{rung} failed");
    (
        String::from_utf8_lossy(&out.stdout).into_owned(),
        String::from_utf8_lossy(&out.stderr).into_owned(),
        secs,
    )
}

fn field<'a>(stdout: &'a str, key: &str) -> &'a str {
    stdout
        .lines()
        .find_map(|l| l.trim().strip_prefix(key))
        .unwrap_or_else(|| panic!("child printed no {key}:\n{stdout}"))
}

#[test]
fn one_rung_the_whole_wall_its_verdict() {
    if let Ok(scenario) = std::env::var("RUNG_ONLY_CHILD") {
        let (dom, prb) = match scenario.as_str() {
            "counters" => counters(),
            "tax-big" => laddertax(if cfg!(debug_assertions) { 900 } else { 1700 }),
            _ => laddertax(60),
        };
        let opts = ferroplan::Options {
            threads: 1,
            ..Default::default()
        };
        let sol = ferroplan::solve(&dom, &prb, &opts).unwrap();
        println!("CHILD-SOLVED:{}", sol.solved);
        println!("CHILD-NOTES:{}", sol.notes.join(" | "));
        return;
    }

    // (a) The light rung alone solves, and the note says so.
    let (stdout, stderr, _) = run_child("tax", "novlight", "6");
    assert_eq!(
        field(&stdout, "CHILD-SOLVED:"),
        "true",
        "{stdout}\n{stderr}"
    );
    assert!(
        field(&stdout, "CHILD-NOTES:").contains("rung: solved by novelty-light"),
        "{stdout}"
    );

    // (b) EHC alone, on a task it cannot finish inside the wall: unsolved,
    // and NO fall-through -- the light rung would have solved it in
    // milliseconds, so a solve here is the ladder, not the rung.
    let (stdout, stderr, secs) = run_child("tax-big", "ehc", "1");
    assert_eq!(
        field(&stdout, "CHILD-SOLVED:"),
        "false",
        "EHC alone must not solve this inside 1 s -- did the hatch fall through?\n{stdout}\n{stderr}"
    );
    assert!(
        !field(&stdout, "CHILD-NOTES:").contains("rung: solved by"),
        "{stdout}"
    );
    assert!(
        secs < 4.0,
        "the rung ran {secs:.2}s against a 1 s wall:\n{stderr}"
    );

    // (c) LAMA alone and the fallback alone each solve the small task.
    let (stdout, _, _) = run_child("tax", "lama", "6");
    assert_eq!(field(&stdout, "CHILD-SOLVED:"), "true", "{stdout}");
    assert!(
        field(&stdout, "CHILD-NOTES:").contains("rung: solved by LAMA"),
        "{stdout}"
    );
    let (stdout, _, _) = run_child("tax", "bfs", "6");
    assert_eq!(field(&stdout, "CHILD-SOLVED:"), "true", "{stdout}");
    assert!(
        field(&stdout, "CHILD-NOTES:").contains("rung: solved by best-first fallback"),
        "{stdout}"
    );

    // (d) The partition route, mapped to its mode.
    let (stdout, _, _) = run_child("counters", "partition", "6");
    assert_eq!(field(&stdout, "CHILD-SOLVED:"), "true", "{stdout}");
    assert!(
        field(&stdout, "CHILD-NOTES:").contains("partition"),
        "the route must say so:\n{stdout}"
    );

    // (e) An unknown name is narrated; the ladder runs as usual.
    let (stdout, stderr, _) = run_child("tax", "rocket", "6");
    assert_eq!(
        field(&stdout, "CHILD-SOLVED:"),
        "true",
        "{stdout}\n{stderr}"
    );
    assert!(
        stderr.contains("FF_RUNG_ONLY=rocket: not a rung"),
        "{stderr}"
    );
}
