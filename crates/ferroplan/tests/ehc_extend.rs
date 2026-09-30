//! THE PROGRESS-CONDITIONAL EHC SLICE (0.29 Lane W). Under a declared wall
//! EHC gets `FF_EHC_WALL_FRAC` (0.25) of it and hands down when that is
//! spent -- and on blocks(30), which solved unbudgeted in 0.19 s on the 0.28
//! box, a 0.5 s wall FAILED: EHC needed 0.19 s and was handed 0.12. Telling
//! the engine it had half a second was strictly worse than telling it
//! nothing (0.28's Lane W table, docs/roadmap-0.28.md).
//!
//! What separates blocks from the laddertax shape (EHC eats twice the wall
//! and finds nothing) is ARRIVAL: blocks improves its h every few
//! milliseconds. So the slice is now charged against the last improvement,
//! not against entry, under a ceiling of `FF_EHC_WALL_MAX_FRAC` (0.6) of the
//! wall. laddertax keeps its verdicts in tests/ladder_wall.rs -- that suite
//! is this lane's other half.
//!
//! Calibrated 2026-09-30 on the M5: blocks(30) now solves in 0.03 s, so the
//! shape is pinned on blocks(50) (EHC alone: 0.34 s, 21,767 evaluations) at
//! a 1 s wall with the fixed slice set to a tenth of it (`FF_EHC_WALL_FRAC`
//! 0.1 = 0.10 s, a 3.4x margin under EHC's need; the 0.6 s ceiling is 1.8x
//! over it). Fixed slice: 0/3 solved, "EHC slice exhausted", the ladder does
//! not rescue it inside the second. Arrival-charged: 3/3 at 0.34 s.

use std::process::Command;

fn blocks(n: usize) -> (String, String) {
    let objs: Vec<String> = (0..n).map(|i| format!("b{i}")).collect();
    let init: String = objs
        .iter()
        .map(|b| format!(" (clear {b}) (ontable {b})"))
        .collect();
    let goal: String = (0..n - 1)
        .map(|i| format!(" (on b{i} b{})", i + 1))
        .collect();
    (
        "(define (domain bw) (:requirements :strips)
           (:predicates (on ?x ?y) (clear ?x) (ontable ?x) (holding ?x))
           (:action pick :parameters (?x) :precondition (and (clear ?x) (ontable ?x))
             :effect (and (holding ?x) (not (clear ?x)) (not (ontable ?x))))
           (:action stack :parameters (?x ?y) :precondition (and (holding ?x) (clear ?y))
             :effect (and (on ?x ?y) (clear ?x) (not (holding ?x)) (not (clear ?y)))))"
            .into(),
        format!(
            "(define (problem bw{n}) (:domain bw) (:objects {})
               (:init{init}) (:goal (and{goal})))",
            objs.join(" ")
        ),
    )
}

#[test]
fn a_run_that_keeps_arriving_keeps_its_slice() {
    if std::env::var("EHC_EXTEND_CHILD").is_ok() {
        let (d, p) = blocks(50);
        let t0 = std::time::Instant::now();
        let sol = ferroplan::solve(
            &d,
            &p,
            &ferroplan::Options {
                threads: 1,
                wall_ms: Some(1000),
                ..Default::default()
            },
        )
        .unwrap();
        println!(
            "CHILD-SOLVED:{} CHILD-SECS:{:.3} CHILD-NOTES:{}",
            sol.solved,
            t0.elapsed().as_secs_f64(),
            sol.notes.join(" | ")
        );
        return;
    }
    if cfg!(debug_assertions) {
        // The slice arithmetic is the same; the seconds are not.
        return;
    }
    let run = |hatch: bool| {
        let mut cmd = Command::new(std::env::current_exe().unwrap());
        cmd.args([
            "--exact",
            "a_run_that_keeps_arriving_keeps_its_slice",
            "--nocapture",
        ])
        .env("EHC_EXTEND_CHILD", "1")
        .env("FF_WALL_DEBUG", "1")
        .env("FF_EHC_WALL_FRAC", "0.1")
        .env_remove("FF_TIME_LIMIT");
        if hatch {
            cmd.env("FF_NO_EHC_EXTEND", "1");
        }
        let out = cmd.output().unwrap();
        assert!(out.status.success());
        (
            String::from_utf8_lossy(&out.stdout).into_owned(),
            String::from_utf8_lossy(&out.stderr).into_owned(),
        )
    };
    // The 0.28 table's shape, pinned as the RED record: with the fixed slice
    // a 1 s wall on blocks(50) hands down at 0.10 s and the row is lost.
    let (stdout, stderr) = run(true);
    assert!(
        stdout.contains("CHILD-SOLVED:false"),
        "the fixed slice was expected to fail blocks(50) at 1 s:\n{stdout}\n{stderr}"
    );
    assert!(stderr.contains("EHC slice exhausted"), "{stderr}");
    // With the slice charged against arrival, EHC finishes what it was
    // finishing, inside the wall.
    let (stdout, stderr) = run(false);
    assert!(
        stdout.contains("CHILD-SOLVED:true"),
        "the extended slice must solve blocks(50) at 1 s:\n{stdout}\n{stderr}"
    );
    assert!(
        stderr.contains("solved by EHC") && !stderr.contains("EHC slice exhausted"),
        "and EHC itself must have finished it:\n{stderr}"
    );
    let secs: f64 = stdout
        .split("CHILD-SECS:")
        .nth(1)
        .and_then(|s| s.split_whitespace().next())
        .and_then(|s| s.parse().ok())
        .unwrap();
    assert!(secs < 1.0, "and inside it: {secs:.3} s");
}
