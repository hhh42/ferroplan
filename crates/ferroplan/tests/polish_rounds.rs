//! THE ANYTIME POLISH (0.30 Lane 1b, `pddl3::polish_rounds`).
//!
//! One pass of the preference polish tries each held preference once, in
//! order, each candidate under a slice of what is left; 0.29 ran the pass
//! once on the seed's task and once after the optimizer. Now the pass runs
//! again from the plan it improved while the share has wall left and the
//! last pass gained something (`FF_PREF_POLISH_ROUNDS`, 4): a candidate
//! capped by its slice in the first pass has a longer one in the second,
//! and the preferences accepted earlier ride along as held goals.
//!
//! The pin is the loop's shape, not its points (those are the board's). The
//! task is tests/pref_polish.rs's corridor with the optimizer starved
//! (`FF_PREF_EVAL_BUDGET=1`, no escalation, restarts or ESPC) so the polish
//! is what moves the metric (11 -> 3, the two sides won): the first round
//! gains, the second starts and reports nothing more -- `polish` is
//! monotone -- and with `FF_PREF_POLISH_ROUNDS=1` no second round runs.
//! Child processes, so each has its own armed wall and narration.

use std::process::Command;

const CORRIDOR: &str = "(define (domain corridor-plain)
 (:requirements :strips :typing :preferences)
 (:types cell)
 (:predicates (at ?c - cell) (adj ?a ?b - cell) (visited ?c - cell))
 (:action move :parameters (?a ?b - cell)
   :precondition (and (at ?a) (adj ?a ?b))
   :effect (and (not (at ?a)) (at ?b) (visited ?b))))";

const WALK: &str = "(define (problem walk) (:domain corridor-plain)
 (:objects c0 c1 c2 c3 s1 s2 - cell)
 (:init (at c0) (adj c0 c1) (adj c1 c0) (adj c1 c2) (adj c2 c1) (adj c2 c3) (adj c3 c2)
        (adj c1 s1) (adj s1 c1) (adj c2 s2) (adj s2 c2))
 (:goal (and (at c3)
             (preference side1 (visited s1))
             (preference side2 (visited s2))
             (preference home (at c0))))
 (:metric minimize (+ 1 (* 5 (is-violated side1)) (* 3 (is-violated side2)) (* 2 (is-violated home)))))";

fn child(rounds: Option<&str>) -> (String, String) {
    let exe = std::env::current_exe().unwrap();
    let mut cmd = Command::new(&exe);
    cmd.args([
        "--exact",
        "the_polish_runs_again_from_its_own_improvement",
        "--nocapture",
    ])
    .env("POLISH_ROUNDS_CHILD", "1")
    .env("FF_WALL_DEBUG", "1")
    .env("FF_TIME_LIMIT", "20")
    .env("FF_PREF_EVAL_BUDGET", "1")
    .env("FF_PREF_NO_ESCALATE", "1")
    .env("FF_PREF_NO_RESTARTS", "1")
    .env("FF_NO_ESPC", "1");
    if let Some(r) = rounds {
        cmd.env("FF_PREF_POLISH_ROUNDS", r);
    }
    let out = cmd.output().unwrap();
    assert!(out.status.success(), "child failed (rounds={rounds:?})");
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
fn the_polish_runs_again_from_its_own_improvement() {
    if std::env::var("POLISH_ROUNDS_CHILD").is_ok() {
        let opts = ferroplan::Options {
            threads: 1,
            ..Default::default()
        };
        let sol = ferroplan::solve(CORRIDOR, WALK, &opts).unwrap();
        println!("CHILD-SOLVED:{}", sol.solved);
        println!(
            "CHILD-METRIC:{:?}",
            sol.plan.as_ref().and_then(|p| p.metric)
        );
        println!("CHILD-NOTES:{}", sol.notes.join(" | "));
        return;
    }

    // Default: a first round that gains, a second that starts and reports
    // nothing more, and the metric the single pass reaches (3).
    let (stdout, stderr) = child(None);
    assert_eq!(
        field(&stdout, "CHILD-SOLVED:"),
        "true",
        "{stdout}\n{stderr}"
    );
    assert!(
        stderr.contains("[polish] round 1:") && stderr.contains("gained, metric"),
        "the first round must gain:\n{stderr}"
    );
    assert!(
        stderr.contains("[polish] round 2: nothing more gained"),
        "the second round must run and stop:\n{stderr}"
    );
    assert!(
        field(&stdout, "CHILD-METRIC:").starts_with("Some(3"),
        "both sides won, home still owed: {stdout}"
    );

    // One round only: no second round anywhere.
    let (stdout, stderr) = child(Some("1"));
    assert_eq!(
        field(&stdout, "CHILD-SOLVED:"),
        "true",
        "{stdout}\n{stderr}"
    );
    assert!(
        !stderr.contains("[polish] round 2"),
        "FF_PREF_POLISH_ROUNDS=1 must keep the single pass:\n{stderr}"
    );
}
