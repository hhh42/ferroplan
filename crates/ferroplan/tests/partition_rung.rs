//! THE PARTITION RUNG (0.30 Lane 6, `api.rs::solve_classical`).
//!
//! In auto, before the ladder, one bounded partition-and-resolve attempt
//! under a slice of the wall that is left (`FF_PARTITION_RUNG_FRAC`, 0.5),
//! the ladder after it with the rest. Under `--mode partition` the route
//! solved six pathways cells the ladder never reaches on any budget
//! (`benchmarks/probes-0.30/lane6/partition.tsv`); no key on the task
//! separates pathways from trucks (`groups.tsv`: every variant decomposes),
//! so the attempt is a rung with a slice, as LAMA is.
//!
//! Two pins, each in a CHILD process with its own armed wall. (1) A task
//! that decomposes (two independent counters to raise) solves through the
//! rung and says so in its notes. (2) A task the partition cannot finish
//! -- k agents on a ring whose goal reverses their cyclic order, impossible
//! and invisible to the relaxation -- is handed down inside the slice and
//! the whole solve still returns inside the wall: the rung never spends
//! the ladder's half. `FF_NO_PARTITION_RUNG=1` is the hatch, pinned on (1).

use std::process::Command;

fn counters() -> (String, String) {
    (
        "(define (domain twocount)
          (:requirements :strips)
          (:predicates (a0) (a1) (a2) (b0) (b1) (b2))
          (:action a01 :precondition (a0) :effect (and (not (a0)) (a1)))
          (:action a12 :precondition (a1) :effect (and (not (a1)) (a2)))
          (:action b01 :precondition (b0) :effect (and (not (b0)) (b1)))
          (:action b12 :precondition (b1) :effect (and (not (b1)) (b2))))"
            .into(),
        "(define (problem tc) (:domain twocount) (:init (a0) (b0)) (:goal (and (a2) (b2))))".into(),
    )
}

fn ring(m: usize, k: usize) -> (String, String) {
    let dom = "(define (domain agentring)
      (:requirements :strips)
      (:predicates (at ?a ?c) (free ?c) (adj ?c1 ?c2))
      (:action move
        :parameters (?a ?from ?to)
        :precondition (and (at ?a ?from) (adj ?from ?to) (free ?to))
        :effect (and (not (at ?a ?from)) (at ?a ?to) (free ?from) (not (free ?to)))))"
        .to_string();
    let mut objs = String::new();
    let mut init = String::new();
    let mut goal = String::new();
    for c in 0..m {
        objs.push_str(&format!(" c{c}"));
        let n = (c + 1) % m;
        init.push_str(&format!(" (adj c{c} c{n}) (adj c{n} c{c})"));
        if c >= k {
            init.push_str(&format!(" (free c{c})"));
        }
    }
    for a in 0..k {
        objs.push_str(&format!(" a{a}"));
        init.push_str(&format!(" (at a{a} c{a})"));
        goal.push_str(&format!(" (at a{a} c{})", k - 1 - a));
    }
    (
        dom,
        format!(
            "(define (problem ring) (:domain agentring) (:objects{objs}) \
             (:init{init}) (:goal (and{goal})))"
        ),
    )
}

/// Over the rung's ten-second floor (a shorter wall is the ladder's alone).
const WALL_SECS: f64 = 12.0;

fn child(which: &str, hatched: bool) -> (String, String) {
    let exe = std::env::current_exe().unwrap();
    let mut cmd = Command::new(&exe);
    cmd.args([
        "--exact",
        "the_partition_rung_runs_first_and_hands_down",
        "--nocapture",
    ])
    .env("PARTITION_RUNG_CHILD", which)
    .env("FF_WALL_DEBUG", "1")
    .env("FF_TIME_LIMIT", WALL_SECS.to_string());
    if hatched {
        cmd.env("FF_NO_PARTITION_RUNG", "1");
    }
    let out = cmd.output().unwrap();
    assert!(
        out.status.success(),
        "child {which} failed (hatched={hatched})"
    );
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

#[test]
fn the_partition_rung_runs_first_and_hands_down() {
    if let Ok(which) = std::env::var("PARTITION_RUNG_CHILD") {
        let (dom, prb) = if which == "counters" {
            counters()
        } else {
            ring(16, 5)
        };
        let opts = ferroplan::Options {
            threads: 1,
            ..Default::default()
        };
        let t0 = std::time::Instant::now();
        let sol = ferroplan::solve(&dom, &prb, &opts).unwrap();
        println!("CHILD-SECS:{:.3}", t0.elapsed().as_secs_f64());
        println!("CHILD-SOLVED:{}", sol.solved);
        println!("CHILD-NOTES:{}", sol.notes.join(" | "));
        return;
    }

    // (1) The decomposable task goes through the rung, and says so.
    let (stdout, stderr) = child("counters", false);
    assert_eq!(
        field(&stdout, "CHILD-SOLVED:"),
        "true",
        "{stdout}\n{stderr}"
    );
    assert!(
        field(&stdout, "CHILD-NOTES:").contains("partition rung"),
        "the rung must have solved it:\n{stdout}\n{stderr}"
    );
    // The hatch: the same task through the ladder, no rung note.
    let (stdout, stderr) = child("counters", true);
    assert_eq!(
        field(&stdout, "CHILD-SOLVED:"),
        "true",
        "{stdout}\n{stderr}"
    );
    assert!(
        !field(&stdout, "CHILD-NOTES:").contains("partition rung"),
        "the hatch must disarm the rung:\n{stdout}\n{stderr}"
    );

    // (2) The impossible ring: the rung hands down inside its slice and
    // the solve returns inside the wall.
    let (stdout, stderr) = child("ring", false);
    assert_eq!(
        field(&stdout, "CHILD-SOLVED:"),
        "false",
        "{stdout}\n{stderr}"
    );
    assert!(
        stderr.contains("wall: partition rung handed down"),
        "the rung must have handed down:\n{stderr}"
    );
    let secs: f64 = field(&stdout, "CHILD-SECS:").parse().unwrap();
    assert!(
        secs < WALL_SECS * 1.5,
        "the solve must return near its wall, rung and ladder together: {secs:.1} s of {WALL_SECS}"
    );
}
