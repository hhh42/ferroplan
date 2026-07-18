//! Costs × preferences composition (0.9 roadmap Phase 5): one SHARED metric
//! evaluation — action costs and weighted `is-violated` terms trade off in a
//! single objective with no double-counting, and the satisfy-vs-forgo
//! decision flips exactly where the numbers say it should. Trajectory
//! constraints stay enforced while the combined metric is optimized.

use ferroplan::{solve, Options};

/// One deliverable behind a costly action, one soft goal on top of it:
/// `deliver` (hard) costs 2; `polish` (soft, satisfying `shiny`) costs
/// `POLISH_COST`; the preference weighs `PREF_W`.
fn domain(polish_cost: u32) -> String {
    format!(
        "
(define (domain shop)
  (:requirements :strips :action-costs :preferences)
  (:predicates (raw) (delivered) (shiny))
  (:functions (total-cost) - number)
  (:action deliver
    :parameters ()
    :precondition (raw)
    :effect (and (delivered) (increase (total-cost) 2)))
  (:action polish
    :parameters ()
    :precondition (raw)
    :effect (and (shiny) (increase (total-cost) {polish_cost}))))
"
    )
}

fn problem(pref_w: u32) -> String {
    format!(
        "
(define (problem shop-1) (:domain shop)
  (:init (raw) (= (total-cost) 0))
  (:goal (and (delivered) (preference nice (shiny))))
  (:metric minimize (+ (total-cost) (* {pref_w} (is-violated nice)))))
"
    )
}

#[test]
fn satisfies_preference_when_cheaper_than_its_weight() {
    // polish costs 1, forgoing weighs 5: polish. Metric = 2 (deliver) + 1.
    let sol = solve(&domain(1), &problem(5), &Options::default()).unwrap();
    let plan = sol.plan.unwrap();
    assert_eq!(plan.metric, Some(3.0), "notes: {:?}", sol.notes);
    assert_eq!(plan.length, 2, "deliver + polish");
}

#[test]
fn forgoes_preference_when_dearer_than_its_weight() {
    // polish costs 10, forgoing weighs 5: skip it. Metric = 2 (deliver) + 5
    // (violated) — counted ONCE, in one shared objective.
    let sol = solve(&domain(10), &problem(5), &Options::default()).unwrap();
    let plan = sol.plan.unwrap();
    assert_eq!(plan.metric, Some(7.0), "notes: {:?}", sol.notes);
    assert_eq!(plan.length, 1, "deliver only");
}

#[test]
fn trajectory_constraint_enforced_while_combined_metric_optimized() {
    // A hard `always` monitor rides along: the plan must still optimize the
    // combined costs+preference metric while never violating the constraint.
    // `rush` would be the cheap route to `delivered` but breaks `(calm)`.
    let domain = "
(define (domain guarded)
  (:requirements :strips :action-costs :preferences :constraints :negative-preconditions)
  (:predicates (raw) (delivered) (shiny) (calm))
  (:functions (total-cost) - number)
  (:action rush
    :parameters ()
    :precondition (raw)
    :effect (and (delivered) (not (calm)) (increase (total-cost) 1)))
  (:action deliver
    :parameters ()
    :precondition (raw)
    :effect (and (delivered) (increase (total-cost) 4)))
  (:action polish
    :parameters ()
    :precondition (raw)
    :effect (and (shiny) (increase (total-cost) 1))))
";
    let problem = "
(define (problem guarded-1) (:domain guarded)
  (:init (raw) (calm) (= (total-cost) 0))
  (:goal (and (delivered) (preference nice (shiny))))
  (:constraints (always (calm)))
  (:metric minimize (+ (total-cost) (* 3 (is-violated nice)))))
";
    let sol = solve(domain, problem, &Options::default()).unwrap();
    assert!(sol.solved);
    let plan = sol.plan.unwrap();
    // rush (cost 1) is forbidden by the monitor; deliver (4) + polish (1) = 5
    // beats deliver + forgo (4 + 3 = 7).
    assert_eq!(plan.metric, Some(5.0), "notes: {:?}", sol.notes);
    assert!(
        !plan.steps.iter().any(|s| s.action == "RUSH"),
        "the always-monitor must forbid RUSH: {:?}",
        plan.steps
    );
}
