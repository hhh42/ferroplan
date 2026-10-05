//! THE SCORER AT PLAN SIZE (0.29 Lane 1, mechanism 3). `temporal::score_soft`
//! grounded the whole snap-compiled task to price one plan -- 4.4 GB on
//! pipesworld-complex, 12.7 s on pathways-complex i20, and four complex cells
//! banked "NOT scored" for want of room. It now specialises the domain to the
//! plan's own steps and grounds that. The score must be the full grounding's,
//! exactly: metric, violated set, satisfied count.

use ferroplan::parser::{parse_domain, parse_problem};
use ferroplan::temporal::{self, SoftScorer, TimedPlan, TimedStep};

const DOM: &str = "(define (domain tanks)
  (:requirements :typing :durative-actions :fluents :preferences :constraints)
  (:types tank)
  (:predicates (open ?t - tank) (full ?t - tank) (sealed ?t - tank) (spare ?t - tank))
  (:functions (level ?t - tank))
  (:durative-action fill :parameters (?t - tank)
    :duration (= ?duration 2)
    :condition (and (at start (open ?t)))
    :effect (and (at end (increase (level ?t) 4)) (at end (full ?t))))
  (:durative-action seal :parameters (?t - tank)
    :duration (= ?duration 1)
    :condition (and (at start (full ?t)))
    :effect (and (at start (not (open ?t))) (at end (sealed ?t)))))";

const PRB: &str = "(define (problem tk) (:domain tanks)
  (:objects t1 t2 t3 - tank)
  (:init (open t1) (open t2) (open t3) (spare t3) (= (level t1) 2) (= (level t2) 0) (= (level t3) 0))
  (:goal (and (full t1)
              (preference p1 (sealed t1))
              (preference p2 (full t2))
              (preference p3 (>= (level t2) 8))
              (preference p4 (spare t3))
              (preference p5 (sealed t3))))
  (:constraints (and (preference c1 (sometime (full t2)))
                     (preference c2 (at-most-once (open t1)))))
  (:metric minimize (+ (* 3 (is-violated p1)) (* 2 (is-violated p2)) (* 5 (is-violated p3))
                       (* 1 (is-violated p4)) (* 4 (is-violated p5))
                       (* 2 (is-violated c1)) (* 1 (is-violated c2)) (* 0.1 (total-time)))))";

fn step(time: f64, action: &str, duration: f64) -> TimedStep {
    TimedStep {
        time,
        action: action.into(),
        duration: Some(duration),
    }
}

fn same(d: &ferroplan::types::Domain, p: &ferroplan::types::Problem, plan: &TimedPlan) {
    let full = SoftScorer::prepare_full(d, p)
        .expect("preferences present")
        .score(plan)
        .expect("the full grounding scores it");
    let sized = temporal::score_soft(d, p, plan).expect("the plan-sized grounding scores it");
    assert_eq!(sized.violated, full.violated, "violated set");
    assert_eq!(sized.satisfied, full.satisfied, "satisfied count");
    match (sized.metric, full.metric) {
        (Some(a), Some(b)) => assert!((a - b).abs() < 1e-9, "metric {a} vs {b}"),
        (a, b) => assert_eq!(a, b, "metric presence"),
    }
}

/// The solver's own plan, and three hand-built ones that touch different
/// subsets of the preferences: a static fact the plan never touches (p4,
/// `spare t3`, true from the init and compiled away in the specialised
/// grounding), a trajectory preference the plan satisfies mid-way (c1), one
/// it breaks (c2 by re-opening... which no action does, so it holds), and
/// numeric terms read from the final state.
#[test]
fn the_plan_sized_scorer_prices_exactly_as_the_full_grounding() {
    let d = parse_domain(DOM).expect("domain");
    let p = parse_problem(PRB).expect("problem");
    let solved = temporal::solve(&d, &p, 1).expect("a plan");
    same(&d, &p, &solved);
    let plans = [
        TimedPlan {
            steps: vec![step(0.0, "FILL T1", 2.0)],
            makespan: 2.0,
        },
        TimedPlan {
            steps: vec![
                step(0.0, "FILL T1", 2.0),
                step(0.0, "FILL T2", 2.0),
                step(2.001, "FILL T2", 2.0),
                step(2.001, "SEAL T1", 1.0),
            ],
            makespan: 4.001,
        },
        TimedPlan {
            steps: vec![
                step(0.0, "FILL T3", 2.0),
                step(0.0, "FILL T1", 2.0),
                step(2.001, "SEAL T3", 1.0),
                step(2.001, "SEAL T1", 1.0),
            ],
            makespan: 3.001,
        },
    ];
    for plan in &plans {
        temporal::validate(&d, &p, plan).expect("the hand-built plan validates");
        same(&d, &p, plan);
    }
    // And the numbers themselves, so the pin is not two wrongs agreeing:
    // plan 2 seals t1 (p1), fills t2 twice (p2, p3 at level 8, c1), keeps
    // t3 spare (p4) and unsealed (p5 violated); c2 holds (t1 opened once).
    let s = temporal::score_soft(&d, &p, &plans[1]).unwrap();
    assert_eq!(s.violated, vec!["P5".to_string()]);
    assert_eq!(s.satisfied, 6);
    assert!(
        (s.metric.unwrap() - (4.0 + 0.1 * 4.001)).abs() < 1e-9,
        "{:?}",
        s.metric
    );
}
