//! The text path arms the wall (0.31 Phase 0.4, docs/roadmap-0.31.md).
//!
//! `run_planner` (the `ff` default without `--json`) grounds and tries
//! `resolve::solve` BEFORE anything arms `FF_TIME_LIMIT`: `crate::solve`
//! arms it, and the text path never passes through `crate::solve`. Found
//! 2026-10-17 calibrating the Lane 6 fixture: a ring(40,12) under
//! `FF_TIME_LIMIT=12` narrated `remaining None` and ran past 30 s. The
//! boards and every probe go through `--json`, so no measurement was
//! touched; the drop-in's contract was.
//!
//! The pin: the agent ring from `tests/partition_rung.rs` -- k agents on a
//! ring whose goal reverses their cyclic order, impossible and invisible to
//! the relaxation -- under a short armed wall returns inside ~2x the wall
//! on the TEXT path. One child process per leg (the wall is a process-wide
//! `OnceLock`).

use std::process::Command;

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

const WALL_SECS: f64 = 4.0;

#[test]
fn the_text_path_returns_inside_its_armed_wall() {
    if std::env::var("TEXT_WALL_CHILD").is_ok() {
        // The debug build's grounding and cascade are slower; a smaller
        // ring keeps the wall the thing measured, not the grounding.
        let (dom, prb) = if cfg!(debug_assertions) {
            ring(24, 8)
        } else {
            ring(40, 12)
        };
        let opts = ferroplan::Options {
            threads: 1,
            ..Default::default()
        };
        let t0 = std::time::Instant::now();
        let (out, code) = ferroplan::run_planner(&dom, &prb, &opts, false);
        println!("CHILD-SECS:{:.3}", t0.elapsed().as_secs_f64());
        println!("CHILD-CODE:{code}");
        println!("CHILD-LINES:{}", out.lines().count());
        return;
    }
    let exe = std::env::current_exe().unwrap();
    let out = Command::new(&exe)
        .args([
            "--exact",
            "the_text_path_returns_inside_its_armed_wall",
            "--nocapture",
        ])
        .env("TEXT_WALL_CHILD", "1")
        .env("FF_WALL_DEBUG", "1")
        .env("FF_TIME_LIMIT", WALL_SECS.to_string())
        .output()
        .unwrap();
    let stdout = String::from_utf8_lossy(&out.stdout);
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(out.status.success(), "child failed:\n{stdout}\n{stderr}");
    let secs: f64 = stdout
        .lines()
        .find_map(|l| l.trim().strip_prefix("CHILD-SECS:"))
        .expect("child printed no CHILD-SECS")
        .parse()
        .unwrap();
    // Inside the wall plus the grounding and report overheads the budget
    // cannot cover; the unarmed text path ran this shape past 30 s.
    assert!(
        secs < WALL_SECS * 2.0,
        "text path ran {secs:.2}s against a {WALL_SECS}s wall:\n{stdout}\n{stderr}"
    );
    assert!(
        !stderr.contains("remaining None"),
        "the wall was not armed on the text path:\n{stderr}"
    );
}
