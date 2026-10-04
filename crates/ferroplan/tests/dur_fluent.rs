//! An end-side `?duration` whose duration reads a dynamic fluent (0.29
//! Lane 2): rovers' `recharge` lasts `(/ (- 80 (energy ?x)) (rate ?x))` and
//! pays `(* ?duration (rate ?x))` at its end. Until 0.29 the snap compile
//! SKIPPED every such action ("never compiled wrong"), so the temporal search
//! could never recharge and the compression rung's plans, which do, failed
//! the validator by the name of an action it never had. PDDL2.1 fixes
//! `?duration` at the START: the compile now assigns it to a hidden per-action
//! fluent there and reads that fluent at the end.

use ferroplan::parser::{parse_domain, parse_problem};
use ferroplan::temporal;

const DOM: &str = "(define (domain fill)
  (:requirements :typing :durative-actions :fluents)
  (:types tank)
  (:predicates (open ?t - tank))
  (:functions (level ?t - tank) (rate ?t - tank))
  (:durative-action fill :parameters (?t - tank)
    :duration (= ?duration (/ (- 10 (level ?t)) (rate ?t)))
    :condition (and (at start (open ?t)) (at start (<= (level ?t) 10)))
    :effect (and (at end (increase (level ?t) (* ?duration (rate ?t))))))
  (:durative-action drain :parameters (?t - tank)
    :duration (= ?duration 1)
    :condition (at start (>= (level ?t) 2))
    :effect (at start (decrease (level ?t) 2))))";

const PRB: &str = "(define (problem f1) (:domain fill)
  (:objects t1 t2 - tank)
  (:init (open t1) (open t2) (= (level t1) 3) (= (rate t1) 2)
         (= (level t2) 8) (= (rate t2) 1))
  (:goal (and (>= (level t1) 10) (>= (level t2) 10))))";

/// The compile keeps the action, the search uses it, the plan's durations
/// are the ones fixed at each start (3.5 s for t1 at level 3 and rate 2;
/// 2 s for t2), and the validator -- which replays the same compile --
/// accepts the plan.
#[test]
fn an_end_side_duration_over_a_dynamic_read_compiles_solves_and_validates() {
    let d = parse_domain(DOM).expect("domain");
    let p = parse_problem(PRB).expect("problem");
    let c = temporal::compile(&d, &p);
    assert!(
        c.snaps.iter().any(|s| s.start_action == "FILL-START"),
        "fill must compile to a snap pair, not be skipped"
    );
    assert!(
        c.domain.functions.iter().any(|(f, _)| f == "DUR-FILL"),
        "the hidden duration fluent is declared"
    );
    let plan = temporal::solve(&d, &p, 1).expect("a plan");
    let fill_t1 = plan
        .steps
        .iter()
        .find(|s| s.action == "FILL T1")
        .expect("fill t1 is in the plan");
    let fill_t2 = plan
        .steps
        .iter()
        .find(|s| s.action == "FILL T2")
        .expect("fill t2 is in the plan");
    assert_eq!(fill_t1.duration, Some(3.5), "(10 - 3) / 2");
    assert_eq!(fill_t2.duration, Some(2.0), "(10 - 8) / 1");
    temporal::validate(&d, &p, &plan).expect("the plan validates");
}

/// The value is fixed at START: a drain between a fill's start and end does
/// not change what the fill pays. Hand-built plan, through the validator,
/// which must accept it exactly as PDDL2.1 reads it (t1: 3 + 7 - 2 = 8 at
/// the end, so the goal is NOT reached -- the validator says so for the
/// goal, not for the duration).
#[test]
fn the_duration_is_fixed_at_start_not_re_read_at_end() {
    let d = parse_domain(DOM).expect("domain");
    let p = parse_problem(PRB).expect("problem");
    let plan = temporal::TimedPlan {
        steps: vec![
            temporal::TimedStep {
                time: 0.0,
                action: "FILL T1".into(),
                duration: Some(3.5),
            },
            temporal::TimedStep {
                time: 1.0,
                action: "DRAIN T1".into(),
                duration: Some(1.0),
            },
            temporal::TimedStep {
                time: 0.0,
                action: "FILL T2".into(),
                duration: Some(2.0),
            },
        ],
        makespan: 3.5,
    };
    let err = temporal::validate(&d, &p, &plan).expect_err("the goal is short");
    assert!(
        err.to_lowercase().contains("goal"),
        "a goal verdict, not a duration one: {err}"
    );
    // The same plan with a 4.5 s fill is refused on its DURATION: the value
    // fixed at start was 3.5, whatever the level reads at the end.
    let mut tampered = plan.clone();
    tampered.steps[0].duration = Some(4.5);
    let err = temporal::validate(&d, &p, &tampered).expect_err("the duration is wrong");
    assert!(
        err.to_lowercase().contains("duration"),
        "a duration verdict: {err}"
    );
}
