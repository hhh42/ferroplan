//! THE LAZY FRONTIER (0.30 Lane 5, `novelty::search_driver`).
//!
//! The eager shape stored every generated successor's `State` and
//! accepted-R vector at generation; on city-car i6 the driver held
//! 3,880,241 of them at the memory wall after 29,952 pops -- ~130 stored
//! states per evaluated one, ~4.5 GB for a frontier the search never looked
//! at (`benchmarks/probes-0.30/lane5/measure3.sh`). An open entry is now
//! (key, father, op) on the heap plus a hash-set key, and the state is
//! regenerated from its father's when it is popped.
//!
//! The fixture reads the bytes the process grows by per HELD node (open +
//! closed) on a task the driver cannot finish and the grounder cannot
//! refute: k agents on a ring of m cells, one agent per cell, whose goal
//! reverses their cyclic order -- impossible on a ring (rotation preserves
//! the order), invisible to relaxed reachability (every `at` is reachable
//! on its own), and C(m, k) states wide. In a child process, so the
//! kernel's peak reading is the child's own: under 160 bytes a held node,
//! against the eager shape's ~350 and up (three heap blocks for the
//! `State`, the R vector, the node, the dedup bucket). The driver's
//! verdicts are pinned by tests/novdriver.rs; this is the byte receipt.

use std::process::Command;

/// Bytes per held node the lazy frontier may cost. The eager shape read
/// 350+; the heap tuple is 24, the key entry ~16, the closed states ~300
/// each but a small share of what is held.
const BYTES_PER_NODE_MAX: f64 = 160.0;

/// `k` agents on a ring of `m` cells, goal: their cyclic order reversed.
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
fn an_open_node_costs_a_tuple_not_a_state() {
    if std::env::var("LAZY_FRONTIER_CHILD").is_ok() {
        let (m, k, max_eval) = if cfg!(debug_assertions) {
            (24, 6, 6_000)
        } else {
            (40, 8, 40_000)
        };
        let (dom, prb) = ring(m, k);
        let d = ferroplan::parser::parse_domain(&dom).unwrap();
        let p = ferroplan::parser::parse_problem(&prb).unwrap();
        let task = ferroplan::ground::ground_task(&d, &p, 1).unwrap();
        let before = ferroplan::mem::resident_bytes().expect("readable here");
        let r = ferroplan::novelty::search_driver(
            &task,
            max_eval,
            &[],
            None,
            &ferroplan::novelty::DriverCfg::default(),
        );
        let peak = ferroplan::mem::peak_resident_bytes().expect("readable here");
        println!("CHILD-SOLVED:{}", r.is_some());
        println!("CHILD-GROWTH:{}", peak.saturating_sub(before));
        return;
    }
    let exe = std::env::current_exe().unwrap();
    let out = Command::new(&exe)
        .args([
            "--exact",
            "an_open_node_costs_a_tuple_not_a_state",
            "--nocapture",
        ])
        .env("LAZY_FRONTIER_CHILD", "1")
        .env("FF_RES_DEBUG", "1")
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
    // "[novelty-driver] capped: P pops (max M), N nodes (C closed)"
    let capped = stderr
        .lines()
        .find(|l| l.contains("[novelty-driver] capped:"))
        .unwrap_or_else(|| panic!("the driver must cap on the ring:\n{stderr}"));
    let nodes: f64 = capped
        .split("), ")
        .nth(1)
        .and_then(|s| s.split(' ').next())
        .and_then(|s| s.parse().ok())
        .unwrap_or_else(|| panic!("unparsed cap line: {capped}"));
    let growth: f64 = field(&stdout, "CHILD-GROWTH:").parse().unwrap();
    let per_node = growth / nodes;
    assert!(
        nodes > 50_000.0,
        "the ring must hold a real frontier: {nodes} nodes\n{capped}"
    );
    assert!(
        per_node < BYTES_PER_NODE_MAX,
        "{per_node:.0} bytes per held node ({growth} over {nodes}); the eager shape read 350+\n{capped}"
    );
}
