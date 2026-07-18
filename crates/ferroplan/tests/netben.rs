//! IPC6 net-benefit (0.9 roadmap Phase 4): `maximize (- C (+ (total-cost)
//! (* (is-violated p) w)...))` normalizes onto the PDDL3 minimize B&B, soft
//! goals stay soft (the empty plan is a legal candidate), and the reported
//! metric is the ORIGINAL (maximized) net benefit. Vendored IPC-2008 guards
//! are `#[ignore]`d like the other heavy IPC tests.

use ferroplan::{solve, Options};
use std::fs;

/// One collectable reward (utility 10) behind a cost-3 action, one
/// unreachable reward (utility 5): net benefit = 10 - 3 = 7 with the
/// unreachable preference forgone.
const NB_DOMAIN: &str = "
(define (domain nb)
  (:requirements :strips :action-costs)
  (:predicates (have-a) (have-b) (blocked))
  (:functions (total-cost) - number)
  (:action get-a
    :parameters ()
    :precondition ()
    :effect (and (have-a) (increase (total-cost) 3)))
  (:action get-b
    :parameters ()
    :precondition (blocked)
    :effect (and (have-b) (increase (total-cost) 1))))
";

const NB_PROBLEM: &str = "
(define (problem nb-1) (:domain nb)
  (:init (= (total-cost) 0))
  (:goal (and (preference pa (have-a)) (preference pb (have-b))))
  (:metric maximize (- 15 (+ (total-cost)
                             (* (is-violated pa) 10)
                             (* (is-violated pb) 5)))))
";

#[test]
fn net_benefit_collects_worthwhile_goal_and_forgoes_unreachable() {
    let sol = solve(NB_DOMAIN, NB_PROBLEM, &Options::default()).unwrap();
    assert!(sol.solved);
    let plan = sol.plan.unwrap();
    // 15 - (cost 3 + violated pb 5) = 7: collect pa, forgo unreachable pb.
    assert_eq!(plan.metric, Some(7.0), "notes: {:?}", sol.notes);
    assert_eq!(plan.length, 1);
}

#[test]
fn net_benefit_takes_empty_plan_when_utilities_do_not_pay() {
    // pa costs 30 to collect but is only worth 10: doing NOTHING is optimal
    // (net benefit 15 - 10 - 5 = 0 from forgoing everything).
    let domain = NB_DOMAIN.replace("(increase (total-cost) 3)", "(increase (total-cost) 30)");
    let sol = solve(&domain, NB_PROBLEM, &Options::default()).unwrap();
    assert!(sol.solved);
    let plan = sol.plan.unwrap();
    assert_eq!(plan.length, 0, "collecting pa at cost 30 > utility 10 must not pay");
    assert_eq!(plan.metric, Some(0.0), "notes: {:?}", sol.notes);
}

// ---- IPC-2008 net-benefit guards (vendored subset; heavy, release-CI) ----

fn netben(domain_dir: &str, problem: &str) -> f64 {
    let base = concat!(env!("CARGO_MANIFEST_DIR"), "/../../benchmarks/ipc/netben");
    let d = fs::read_to_string(format!("{base}/{domain_dir}/domain.pddl")).unwrap();
    let p = fs::read_to_string(format!("{base}/{domain_dir}/{problem}.pddl")).unwrap();
    let sol = solve(&d, &p, &Options::default()).unwrap();
    assert!(sol.solved, "{domain_dir}/{problem} should solve");
    sol.plan
        .unwrap()
        .metric
        .unwrap_or_else(|| panic!("{domain_dir}/{problem} must report net benefit"))
}

#[test]
#[ignore = "heavy IPC net-benefit solve; opt-in via --include-ignored (CI runs these in release)"]
fn elevators08_netben_p01_beats_all_forgo() {
    // All-forgo nets 0 (70 - 32 - 36 - 2); recorded 33 at 0.9 Phase 4.
    let nb = netben("elevators08", "p01");
    assert!(nb >= 33.0, "elevators08 netben p01 = {nb}, recorded wall 33");
}

#[test]
#[ignore = "heavy IPC net-benefit solve; opt-in via --include-ignored (CI runs these in release)"]
fn pegsol08_netben_p02_reports_value() {
    // Recorded 36 at 0.9 Phase 4.
    let nb = netben("pegsol08", "p02");
    assert!(nb >= 36.0, "pegsol08 netben p02 = {nb}, recorded wall 36");
}

#[test]
#[ignore = "heavy IPC net-benefit solve; opt-in via --include-ignored (CI runs these in release)"]
fn openstacks08_netben_p01_reports_value() {
    // Recorded 8 at 0.9 Phase 4.
    let nb = netben("openstacks08", "p01");
    assert!(nb >= 8.0, "openstacks08 netben p01 = {nb}, recorded wall 8");
}

#[test]
#[ignore = "heavy IPC net-benefit solve; opt-in via --include-ignored (CI runs these in release)"]
fn crew08_netben_p01_reports_value() {
    let nb = netben("crew08", "p01");
    assert!(nb > 0.0, "crew08 netben p01 = {nb} must beat the all-forgo floor");
}
