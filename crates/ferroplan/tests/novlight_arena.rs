//! THE NOVELTY-LIGHT ARENA (0.30 cut, `novelty::search_light`).
//!
//! The light rung kept a `Node { state: State }` per generated state -- three
//! small heap vectors a node plus a bucket vector per hash. On woodworking
//! i15 it held 12.5M of them when the memory wall tripped, and the allocator
//! could not hand ~50M tiny freed blocks back on a fragmented small zone:
//! LAMA, the next rung, entered with 3.96 GiB of dead nodes resident, tripped
//! the wall at 15,869 evals, and a 12 s solve (4.27 GiB on its own) became a
//! 60 s loss on five board-cells of the cut30 sweep.
//!
//! States now live in three flat arrays and the dedup buckets in one chain
//! array: about a third of the bytes per node, so the same pops leave a
//! third of the heap behind for the next rung (woodworking i15's LAMA now
//! enters at 2.17 GiB and solves in 16 s). The allocator still keeps most of
//! what a rung freed resident -- measured here, 71 % at 1.1M nodes -- so the
//! receipt is the per-node cost, not a hand-back. In a child process, so the
//! kernel's peak reading is the child's own, on a task the rung cannot
//! finish (the agent ring of tests/lazy_frontier.rs: the goal reverses a
//! cyclic order, impossible and invisible to the relaxation): under 150
//! bytes a held node (measured 88), against the old layout's 229 on this
//! ring (~340 on woodworking's larger states).

use std::process::Command;

/// Bytes per held node the arena may cost: the ring's state is a handful of
/// words, plus father/op, the chain link and a hash entry.
const BYTES_PER_NODE_MAX: f64 = 150.0;

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
    let prb = format!(
        "(define (problem ring) (:domain agentring) (:objects{objs}) \
         (:init{init}) (:goal (and{goal})))"
    );
    (dom, prb)
}

fn field<'a>(text: &'a str, key: &str) -> &'a str {
    text.lines()
        .find_map(|l| l.trim().strip_prefix(key))
        .unwrap_or_else(|| panic!("child printed no {key}:\n{text}"))
}

#[test]
#[cfg(any(target_os = "linux", target_os = "macos"))]
fn a_light_node_costs_a_third_of_a_boxed_state() {
    if std::env::var("NOVLIGHT_ARENA_CHILD").is_ok() {
        let (m, k, max_eval) = if cfg!(debug_assertions) {
            (24, 6, 4_000)
        } else {
            (40, 8, 100_000)
        };
        let (dom, prb) = ring(m, k);
        let d = ferroplan::parser::parse_domain(&dom).unwrap();
        let p = ferroplan::parser::parse_problem(&prb).unwrap();
        let task = ferroplan::ground::ground_task(&d, &p, 1).unwrap();
        let before = ferroplan::mem::resident_bytes().expect("readable here");
        let r = ferroplan::novelty::search_light(&task, max_eval, &[]);
        let peak = ferroplan::mem::peak_resident_bytes().expect("readable here");
        println!("CHILD-SOLVED:{}", r.is_some());
        println!("CHILD-GROWTH:{}", peak.saturating_sub(before));
        return;
    }
    let exe = std::env::current_exe().unwrap();
    let out = Command::new(&exe)
        .args([
            "--exact",
            "a_light_node_costs_a_third_of_a_boxed_state",
            "--nocapture",
        ])
        .env("NOVLIGHT_ARENA_CHILD", "1")
        .env("FF_RES_DEBUG", "1")
        .env_remove("FF_MEM_BUDGET_GB")
        .output()
        .unwrap();
    assert!(out.status.success(), "child failed");
    let stdout = String::from_utf8_lossy(&out.stdout);
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert_eq!(
        field(&stdout, "CHILD-SOLVED:"),
        "false",
        "the reversal is impossible on a ring:\n{stdout}\n{stderr}"
    );
    // "[novelty-light] capped: E evals (max M), N nodes"
    let capped = stderr
        .lines()
        .find(|l| l.contains("[novelty-light] capped:"))
        .unwrap_or_else(|| panic!("the light rung must cap on the ring:\n{stderr}"));
    let nodes: f64 = capped
        .split("), ")
        .nth(1)
        .and_then(|s| s.split(' ').next())
        .and_then(|s| s.parse().ok())
        .unwrap_or_else(|| panic!("unparsed cap line: {capped}"));
    let growth: f64 = field(&stdout, "CHILD-GROWTH:").parse().unwrap();
    let floor = if cfg!(debug_assertions) {
        10_000.0
    } else {
        300_000.0
    };
    assert!(
        nodes > floor,
        "the ring must hold a real frontier: {nodes} nodes\n{capped}"
    );
    let per_node = growth / nodes;
    assert!(
        per_node < BYTES_PER_NODE_MAX,
        "{per_node:.0} bytes per held node ({growth} over {nodes}); the old layout read 229 here\n{capped}"
    );
}
