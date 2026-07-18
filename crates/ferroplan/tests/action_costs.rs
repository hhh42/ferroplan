//! IPC6 `:action-costs` (0.9 roadmap Phase 2): the classical path must report
//! the cost metric's real (replayed) value and the anytime sweep must find
//! cheaper-than-shortest plans. Heavy IPC-2008 fixtures are `#[ignore]`d like
//! the other IPC guards (CI runs them in release).

use ferroplan::{solve, Options};
use std::fs;

/// Two roads from a to c: a 1-step road costing 10, and a 2-step road costing
/// 2. Shortest-plan search takes the expensive hop; the cost sweep must trade
/// length for cost.
const ROADS_DOMAIN: &str = "
(define (domain roads)
  (:requirements :strips :typing :action-costs)
  (:types loc)
  (:constants a b c - loc)
  (:predicates (at ?l - loc))
  (:functions (total-cost) - number)
  (:action hop
    :parameters ()
    :precondition (at a)
    :effect (and (not (at a)) (at c) (increase (total-cost) 10)))
  (:action step1
    :parameters ()
    :precondition (at a)
    :effect (and (not (at a)) (at b) (increase (total-cost) 1)))
  (:action step2
    :parameters ()
    :precondition (at b)
    :effect (and (not (at b)) (at c) (increase (total-cost) 1))))
";

const ROADS_PROBLEM: &str = "
(define (problem roads-1) (:domain roads)
  (:init (at a) (= (total-cost) 0))
  (:goal (at c))
  (:metric minimize (total-cost)))
";

#[test]
fn cost_sweep_trades_length_for_cost() {
    let sol = solve(ROADS_DOMAIN, ROADS_PROBLEM, &Options::default()).unwrap();
    assert!(sol.solved);
    let plan = sol.plan.unwrap();
    assert_eq!(
        plan.metric,
        Some(2.0),
        "cheap 2-step road (cost 2) must beat the 1-step hop (cost 10); notes: {:?}",
        sol.notes
    );
    assert_eq!(plan.length, 2);
}

#[test]
fn satisfice_reports_cost_without_sweeping() {
    let opts = Options {
        optimize: false,
        ..Options::default()
    };
    let sol = solve(ROADS_DOMAIN, ROADS_PROBLEM, &opts).unwrap();
    let plan = sol.plan.unwrap();
    // The shortest plan (the expensive hop) is kept, but its true cost is
    // still reported — never silently unreported, never silently optimized.
    assert_eq!(plan.metric, Some(10.0));
    assert_eq!(plan.length, 1);
    assert!(
        sol.notes.iter().any(|n| n.contains("not optimized")),
        "notes must say the metric was not optimized: {:?}",
        sol.notes
    );
}

#[test]
fn zero_cost_plan_is_proven_optimal() {
    let domain = "
(define (domain freebie)
  (:requirements :strips :action-costs)
  (:predicates (start) (done))
  (:functions (total-cost) - number)
  (:action finish
    :parameters ()
    :precondition (start)
    :effect (and (done) (increase (total-cost) 0))))
";
    let problem = "
(define (problem freebie-1) (:domain freebie)
  (:init (start) (= (total-cost) 0))
  (:goal (done))
  (:metric minimize (total-cost)))
";
    let sol = solve(domain, problem, &Options::default()).unwrap();
    let plan = sol.plan.unwrap();
    assert_eq!(plan.metric, Some(0.0));
    assert!(
        sol.notes.iter().any(|n| n.contains("proven optimal")),
        "a zero-cost plan cannot be beaten: {:?}",
        sol.notes
    );
}

#[test]
fn maximize_metric_is_not_silently_claimed() {
    // maximize is outside the supported classical-cost class: the plan solves
    // but no metric value is claimed for it.
    let problem = "
(define (problem roads-max) (:domain roads)
  (:init (at a) (= (total-cost) 0))
  (:goal (at c))
  (:metric maximize (total-cost)))
";
    let sol = solve(ROADS_DOMAIN, problem, &Options::default()).unwrap();
    assert!(sol.solved);
    assert_eq!(sol.plan.unwrap().metric, None);
}

// ---- IPC-2008 guards (vendored subset; heavy, release-CI) ----

fn ipc_costs(domain_dir: &str, problem: &str) -> (f64, usize) {
    let base = concat!(env!("CARGO_MANIFEST_DIR"), "/../../benchmarks/ipc/costs");
    let d = fs::read_to_string(format!("{base}/{domain_dir}/domain.pddl")).unwrap();
    let p = fs::read_to_string(format!("{base}/{domain_dir}/{problem}.pddl")).unwrap();
    let sol = solve(&d, &p, &Options::default()).unwrap();
    assert!(sol.solved, "{domain_dir}/{problem} should solve");
    let plan = sol.plan.unwrap();
    let m = plan.metric.expect("cost metric must be reported");
    (m, plan.length)
}

#[test]
#[ignore = "heavy IPC cost solve; opt-in via --include-ignored (CI runs these in release)"]
fn elevators08_p01_beats_cost_blind_baseline() {
    // The cost-blind shortest plan replays to cost 100 (recorded 0.8.0
    // behavior); the sweep's recorded result is 54. Guard the wall, not the
    // exact value, so heuristic improvements can only move it down.
    let (m, _) = ipc_costs("elevators08", "p01");
    assert!(m < 100.0, "elevators08/p01 cost {m} must beat the 100 wall");
}

#[test]
#[ignore = "heavy IPC cost solve; opt-in via --include-ignored (CI runs these in release)"]
fn woodworking08_p01_reports_validated_cost() {
    // Recorded wall: 110 at 0.9 Phase 2 (VAL-validated).
    let (m, _) = ipc_costs("woodworking08", "p01");
    assert!(m <= 110.0, "woodworking08/p01 cost {m} regressed past 110");
}

#[test]
#[ignore = "heavy IPC cost solve; opt-in via --include-ignored (CI runs these in release)"]
fn transport08_p01_reports_validated_cost() {
    // Recorded wall: 54 at 0.9 Phase 2 (VAL-validated).
    let (m, _) = ipc_costs("transport08", "p01");
    assert!(m <= 54.0, "transport08/p01 cost {m} regressed past 54");
}
