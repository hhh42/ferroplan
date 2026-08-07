//! The goal-DNF grounding budget (0.22 Phase 1): a goal whose disjunctive
//! items DNF-multiply (block-grouping's pairwise coordinate-Eq network —
//! i3 is 4^21 conjuncts) used to balloon inside `and_merge` with no check
//! at all: 76 s past a 60 s wall, upstream of every narrating driver, and
//! i13/i20 died as external RSS-watchdog kills. Under an armed
//! `FF_TIME_LIMIT` or a declared `FF_MEM_BUDGET_GB` the expansion now
//! stops honestly — solved:false with a named budget note, never the word
//! "unsolvable". With neither declared, behavior is untouched.
//!
//! One sequential test on purpose: the wall clock is a process-global
//! OnceLock and FF_* are process-global env knobs, so each scenario runs
//! in a CHILD process (the tests/ladder_wall.rs convention).

use std::process::Command;
use std::time::Instant;

/// K disjunctive goal items over disjoint fluent pairs: each
/// `(or (not (= (xi) (yi))) (not (= (yi) (xi))))` contributes 4 singleton
/// DNF conjuncts, so the goal DNF is 4^K — hopeless for K=60 on any box.
/// Every pair reads UNEQUAL at init so the task itself is trivially
/// solvable (the control's K=2 proves it end to end).
fn orbomb(k: usize) -> (String, String) {
    let mut fns = String::new();
    let mut init = String::new();
    let mut goal = String::new();
    for i in 0..k {
        fns.push_str(&format!(" (x{i}) (y{i})"));
        init.push_str(&format!(" (= (x{i}) 0) (= (y{i}) 1)"));
        goal.push_str(&format!(
            " (or (not (= (x{i}) (y{i}))) (not (= (y{i}) (x{i}))))"
        ));
    }
    (
        format!(
            "(define (domain orbomb) (:requirements :numeric-fluents)
               (:functions{fns})
               (:action bump :parameters () :effect (increase (x0) 1)))"
        ),
        format!("(define (problem ob) (:domain orbomb) (:init{init}) (:goal (and{goal})))"),
    )
}

fn run_child(scenario: &str) -> (String, f64) {
    let exe = std::env::current_exe().unwrap();
    let mut cmd = Command::new(&exe);
    cmd.args([
        "--exact",
        "goal_dnf_respects_the_declared_budget",
        "--nocapture",
    ])
    .env("GROUND_WALL_CHILD", scenario);
    match scenario {
        "wall" => {
            cmd.env("FF_TIME_LIMIT", "2");
        }
        "mem" => {
            cmd.env("FF_MEM_BUDGET_GB", "0.2");
        }
        "control" => {
            cmd.env("FF_TIME_LIMIT", "30");
        }
        other => panic!("unknown scenario {other}"),
    }
    let t0 = Instant::now();
    let out = cmd.output().unwrap();
    let secs = t0.elapsed().as_secs_f64();
    assert!(out.status.success(), "child {scenario} failed");
    (String::from_utf8_lossy(&out.stdout).into_owned(), secs)
}

#[test]
fn goal_dnf_respects_the_declared_budget() {
    if let Ok(scenario) = std::env::var("GROUND_WALL_CHILD") {
        let (dom, prb) = match scenario.as_str() {
            "wall" | "mem" => orbomb(60),
            "control" => orbomb(2),
            other => panic!("unknown scenario {other}"),
        };
        let opts = ferroplan::Options {
            threads: 1,
            ..Default::default()
        };
        let sol = ferroplan::solve(&dom, &prb, &opts).unwrap();
        println!("CHILD-{scenario}-SOLVED:{}", sol.solved);
        println!("CHILD-NOTE:{}", sol.notes.join(" | "));
        return;
    }

    // THE PIN: an armed 2 s wall stops the 4^60 expansion honestly —
    // named budget note, no "unsolvable" anywhere, and the exit is
    // prompt instead of i3's 76 s balloon (generous bound: the child
    // also pays process spawn and parse).
    let (stdout, secs) = run_child("wall");
    assert!(stdout.contains("CHILD-wall-SOLVED:false"), "{stdout}");
    assert!(
        stdout.contains("goal DNF expansion") && stdout.contains("declared budget"),
        "budget note must name the mechanism:\n{stdout}"
    );
    assert!(
        !stdout.to_lowercase().contains("unsolvable"),
        "a budget stop is never a verdict:\n{stdout}"
    );
    if !cfg!(debug_assertions) {
        assert!(secs < 20.0, "the wall stop must be prompt: {secs:.1} s");
    }

    // The byte-model arm: a declared FF_MEM_BUDGET_GB alone (no wall)
    // stops the same expansion before it balloons — the i13/i20 shape,
    // which used to die as an external RSS-watchdog kill.
    let (stdout, secs) = run_child("mem");
    assert!(stdout.contains("CHILD-mem-SOLVED:false"), "{stdout}");
    assert!(
        stdout.contains("declared budget"),
        "byte-arm note must land too:\n{stdout}"
    );
    if !cfg!(debug_assertions) {
        assert!(secs < 20.0, "the byte stop must be prompt: {secs:.1} s");
    }

    // Negative control: a SMALL disjunctive goal under an armed wall
    // grounds and solves exactly as before (the check only ever fires
    // past the stride, and only on a hopeless estimate).
    let (stdout, _) = run_child("control");
    assert!(stdout.contains("CHILD-control-SOLVED:true"), "{stdout}");
}
