//! The TEMPORAL grounding wall (0.23 Phase 6): the 0.22 checkpoint
//! (ladder_wall.rs's ground-wall legs) armed ONLY on the plain classical
//! solve entry, and the temporal solve path kept grinding unwalled — the
//! 0.22 sitting's receipt is sokoban-t 2008 i21 spending >10 minutes in
//! snap grounding against a 30 s wall. The solve-side stratified entry
//! (`ground_stratified_walled`: temporal.rs's solve_inner, tresolve's
//! decomposer) now pays the same honest budget exit — WallExhausted, no
//! partial task, no verdict — while VALIDATION re-grounding stays exempt
//! (a plan already found must still be groundable for verification after
//! the wall) and the session entry keeps its own budget discipline.
//!
//! One sequential test on purpose: the wall clock is a process-global
//! OnceLock and FF_* are process-global env knobs, so each scenario runs
//! in a CHILD process (the tests/ladder_wall.rs convention).

use std::process::Command;
use std::time::Instant;

/// ladder_wall.rs's 2048 grounding shape, made durative: one 5-parameter
/// action whose only gating literal names the FIRST and LAST parameters,
/// so the snap-compiled HEAVY-START walks the full n^4 prefix product and
/// checks n^5 candidates at the leaf — zero survivors (`trip` has no init
/// atoms), so HEAVY-END's producer-known stratum joins against nothing.
/// The task itself is solved by the trivial EASY action AFTER grounding,
/// which is what lets the hatched leg prove the overrun shape: grounding
/// grinds far past the armed wall and then "solves".
fn tbindstorm(n: usize) -> (String, String) {
    let objs: String = (0..n).map(|i| format!(" o{i}")).collect();
    (
        "(define (domain tbindstorm)
          (:requirements :typing :durative-actions)
          (:types obj)
          (:predicates (trip ?a - obj ?e - obj) (go) (gw))
          (:durative-action heavy
            :parameters (?a - obj ?b - obj ?c - obj ?d - obj ?e - obj)
            :duration (= ?duration 1)
            :condition (at start (trip ?a ?e))
            :effect (at end (gw)))
          (:durative-action easy
            :parameters ()
            :duration (= ?duration 1)
            :condition (at start (go))
            :effect (at end (gw))))"
            .into(),
        format!(
            "(define (problem tbs) (:domain tbindstorm) (:objects{objs} - obj) \
             (:init (go)) (:goal (gw)))"
        ),
    )
}

fn run_child(scenario: &str) -> (String, String, f64) {
    let exe = std::env::current_exe().unwrap();
    let mut cmd = Command::new(&exe);
    cmd.args(["--exact", "temporal_grounding_pays_the_wall", "--nocapture"])
        .env("TGROUND_WALL_CHILD", scenario)
        .env("FF_WALL_DEBUG", "1")
        // MCV's join order collapses the bindstorm shape outright on this
        // path too (it is entry-independent — the sokoban-t 2008 i8 +1
        // receipt); every leg hatches it off so the wall check stays the
        // pinned backstop for shapes MCV cannot collapse.
        .env("FF_NO_MCV_JOIN", "1")
        // The SAT wing (0.24) spends the same armed wall these legs
        // measure in — its scan/promotion cost eats the exempt leg's
        // 1 s pre-expiry margin. Same rule as MCV above: this battery
        // pins the grounding-wall/validate contract, and orthogonal
        // machinery that merely SPENDS the wall hatches off. The wing's
        // own promotion cost is a named sweep-watch item in its record.
        .env("FF_NO_SAT", "1");
    match scenario {
        "twall" => {
            // THE PIN: the temporal solve entry must exit HONESTLY at a
            // tiny armed wall — no partial task, no zombie enumeration.
            cmd.env("FF_TIME_LIMIT", "1");
        }
        "twall-hatched" => {
            // The permanent RED record (the ladder_wall.rs pattern):
            // unchecked snap enumeration grinds far past the wall, then
            // "solves" — the sokoban-t i21 receipt shape in miniature.
            cmd.env("FF_TIME_LIMIT", "1").env("FF_NO_RUNG_WALLCAP", "1");
        }
        "tcontrol" => {
            // A small storm under a LONG armed wall grounds and solves
            // exactly as before — the wall only ever refuses work that
            // cannot finish inside the budget.
            let wall = if cfg!(debug_assertions) { "300" } else { "30" };
            cmd.env("FF_TIME_LIMIT", wall);
        }
        "exempt" => {
            // Solve-side vs validation-side, pinned in one process: after
            // the wall FULLY expires, the solve entry refuses (honest
            // budget exit) while `temporal::validate` still re-grounds
            // the same pair and accepts the plan found before expiry.
            let wall = if cfg!(debug_assertions) { "8" } else { "1" };
            cmd.env("FF_TIME_LIMIT", wall);
        }
        other => panic!("unknown scenario {other}"),
    }
    let t0 = Instant::now();
    let out = cmd.output().unwrap();
    let secs = t0.elapsed().as_secs_f64();
    assert!(out.status.success(), "child {scenario} failed");
    (
        String::from_utf8_lossy(&out.stdout).into_owned(),
        String::from_utf8_lossy(&out.stderr).into_owned(),
        secs,
    )
}

#[test]
fn temporal_grounding_pays_the_wall() {
    if let Ok(scenario) = std::env::var("TGROUND_WALL_CHILD") {
        let opts = ferroplan::Options {
            threads: 1,
            ..Default::default()
        };
        match scenario.as_str() {
            "twall" | "twall-hatched" => {
                // n=30 is ladder_wall.rs's calibration: the n^5 walk
                // reliably overruns a 1 s wall on both build profiles.
                let (dom, prb) = tbindstorm(30);
                let sol = ferroplan::solve(&dom, &prb, &opts).unwrap();
                println!("CHILD-{scenario}-SOLVED:{}", sol.solved);
            }
            "tcontrol" => {
                let (dom, prb) = tbindstorm(8);
                let sol = ferroplan::solve(&dom, &prb, &opts).unwrap();
                println!("CHILD-{scenario}-SOLVED:{}", sol.solved);
            }
            "exempt" => {
                // Small enough to solve well inside the wall, big enough
                // that the walled entry's 8192-node check cadence fires
                // once the wall expires (n^4 prefix nodes alone >> 8192).
                let n = if cfg!(debug_assertions) { 10 } else { 16 };
                let (dom, prb) = tbindstorm(n);
                let d = ferroplan::parser::parse_domain(&dom).unwrap();
                let p = ferroplan::parser::parse_problem(&prb).unwrap();
                // Arms the process wall clock (api::solve) and solves
                // pre-expiry — the plan the validator leg replays.
                let sol = ferroplan::solve(&dom, &prb, &opts).unwrap();
                println!("CHILD-exempt-SOLVED1:{}", sol.solved);
                let plan = ferroplan::temporal::solve(&d, &p, 1).expect("pre-expiry solve");
                let wall = if cfg!(debug_assertions) { 8.0 } else { 1.0 };
                std::thread::sleep(std::time::Duration::from_secs_f64(wall + 0.4));
                let refused = ferroplan::temporal::solve(&d, &p, 1).is_none();
                println!("CHILD-exempt-SOLVE2-REFUSED:{refused}");
                let validated = ferroplan::temporal::validate(&d, &p, &plan).is_ok();
                println!("CHILD-exempt-VALIDATE-OK:{validated}");
            }
            other => panic!("unknown scenario {other}"),
        }
        return;
    }

    // THE PIN: a 1 s armed wall on the durative binding storm must end
    // HONESTLY — solved:false, the checkpoint narration, and a prompt
    // exit — instead of enumerating n^5 snap candidates to the end (RED
    // today: the sokoban-t 2008 i21 shape, >10 minutes of grounding
    // against a 30 s wall).
    let (stdout, stderr, secs) = run_child("twall");
    assert!(
        stdout.contains("CHILD-twall-SOLVED:false"),
        "the temporal solve entry must refuse at the wall, not grind past it:\n{stdout}"
    );
    assert!(
        stderr.contains("wall: grounding checkpoint expired"),
        "grounding trip narration missing:\n{stderr}"
    );
    if !cfg!(debug_assertions) {
        assert!(
            secs < 5.0,
            "the temporal wall stop must be prompt: {secs:.1} s"
        );
    }

    // The hatched leg keeps the zombie shape on the record: unchecked
    // snap enumeration grinds past the wall, then solves via EASY.
    let (stdout, stderr, secs) = run_child("twall-hatched");
    assert!(
        stdout.contains("CHILD-twall-hatched-SOLVED:true"),
        "{stdout}"
    );
    assert!(
        !stderr.contains("wall: grounding checkpoint expired"),
        "hatch must disarm the temporal grounding check:\n{stderr}"
    );
    if !cfg!(debug_assertions) {
        assert!(
            secs > 1.0,
            "unchecked snap grounding was expected to blow the 1 s wall (the RED shape): {secs:.1} s"
        );
    }

    // Negative control: a small storm under a long armed wall solves
    // exactly as before.
    let (stdout, _, _) = run_child("tcontrol");
    assert!(stdout.contains("CHILD-tcontrol-SOLVED:true"), "{stdout}");

    // The entry split, both halves in one process: post-expiry the SOLVE
    // entry refuses, and the VALIDATION re-ground still accepts the plan
    // found before expiry (a plan found is a plan, never discarded).
    let (stdout, _, _) = run_child("exempt");
    assert!(stdout.contains("CHILD-exempt-SOLVED1:true"), "{stdout}");
    assert!(
        stdout.contains("CHILD-exempt-SOLVE2-REFUSED:true"),
        "the solve entry must refuse once the wall has expired:\n{stdout}"
    );
    assert!(
        stdout.contains("CHILD-exempt-VALIDATE-OK:true"),
        "validation re-grounding must stay exempt from the wall:\n{stdout}"
    );
}
