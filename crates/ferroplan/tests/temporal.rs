//! Temporal (PDDL2.1 durative-action) parsing + snap-action compilation
//! (EPIC-Temporal T1/T2).

use ferroplan::ground::{ground, Outcome};
use ferroplan::parser::{parse_domain, parse_problem};
use ferroplan::temporal;
use ferroplan::types::{Expr, Formula, TimeSpec};

const DOM: &str = "
(define (domain temporal-test)
  (:requirements :strips :typing :durative-actions :numeric-fluents)
  (:types location)
  (:predicates (at ?l - location) (connected ?a ?b - location) (free))
  (:functions (dist ?a ?b - location))
  (:durative-action move
    :parameters (?from ?to - location)
    :duration (= ?duration (dist ?from ?to))
    :condition (and (at start (at ?from))
                    (at start (connected ?from ?to))
                    (over all (free)))
    :effect (and (at start (not (at ?from)))
                 (at end (at ?to)))))
";

#[test]
fn parses_durative_action() {
    let d = parse_domain(DOM).expect("durative domain should parse");
    assert_eq!(d.durative_actions.len(), 1, "one durative action");
    let a = &d.durative_actions[0];
    assert_eq!(a.name, "MOVE");
    assert_eq!(a.params.len(), 2);

    // duration = (dist ?from ?to)
    assert!(
        matches!(&a.duration, Expr::Fluent(f, _) if f == "DIST"),
        "duration is the dist fluent, got {:?}",
        a.duration
    );

    // conditions: 2 at-start + 1 over-all
    let starts = a
        .conditions
        .iter()
        .filter(|(t, _)| *t == TimeSpec::Start)
        .count();
    let alls = a
        .conditions
        .iter()
        .filter(|(t, _)| *t == TimeSpec::All)
        .count();
    assert_eq!(starts, 2, "two at-start conditions");
    assert_eq!(alls, 1, "one over-all invariant");

    // effects: at-start + at-end
    assert_eq!(a.effects.len(), 2);
    assert!(a.effects.iter().any(|(t, _)| *t == TimeSpec::Start));
    assert!(a.effects.iter().any(|(t, _)| *t == TimeSpec::End));
}

#[test]
fn fixed_numeric_duration_and_single_clauses() {
    let dom = "(define (domain d) (:requirements :durative-actions)
        (:predicates (p) (q))
        (:durative-action a :parameters ()
            :duration (= ?duration 5)
            :condition (at start (p))
            :effect (at end (q))))";
    let d = parse_domain(dom).expect("parse");
    let a = &d.durative_actions[0];
    assert!(matches!(a.duration, Expr::Num(n) if (n - 5.0).abs() < 1e-9));
    assert_eq!(a.conditions.len(), 1);
    assert_eq!(a.effects.len(), 1);
    assert_eq!(a.conditions[0].0, TimeSpec::Start);
    assert_eq!(a.effects[0].0, TimeSpec::End);
}

#[test]
fn classic_action_domains_still_parse() {
    // adding the durative machinery must not break non-temporal domains
    let dom = "(define (domain d) (:requirements :strips)
        (:predicates (p) (q))
        (:action a :parameters () :precondition (p) :effect (q)))";
    let d = parse_domain(dom).expect("parse");
    assert_eq!(d.actions.len(), 1);
    assert_eq!(d.durative_actions.len(), 0);
}

const DUR_DOM: &str = "
(define (domain t)
  (:requirements :strips :durative-actions :numeric-fluents)
  (:predicates (at) (goal) (light))
  (:durative-action act
    :parameters ()
    :duration (= ?duration 3)
    :condition (and (at start (at)) (over all (light)))
    :effect (and (at start (not (at))) (at end (goal)))))";
const DUR_PROB: &str = "(define (problem p) (:domain t) (:init (at) (light)) (:goal (goal)))";

#[test]
fn compiles_durative_to_snaps_and_grounds() {
    let dom = parse_domain(DUR_DOM).expect("domain");
    let prob = parse_problem(DUR_PROB).expect("problem");
    let c = temporal::compile(&dom, &prob);

    assert_eq!(c.snaps.len(), 1);
    let s = &c.snaps[0];
    assert_eq!(s.start_action, "ACT-START");
    assert_eq!(s.end_action, "ACT-END");
    assert!(matches!(s.duration, Expr::Num(n) if (n - 3.0).abs() < 1e-9));
    // the over-all invariant is captured (the (light) atom, not True)
    assert!(matches!(&s.invariant, Formula::Atom(p, _) if p == "LIGHT"));
    assert!(
        c.domain.durative_actions.is_empty(),
        "durative compiled away"
    );

    let names: Vec<&str> = c.domain.actions.iter().map(|a| a.name.as_str()).collect();
    assert!(names.contains(&"ACT-START") && names.contains(&"ACT-END"));

    // and the compiled classical domain grounds, with both snaps reachable
    match ground(&c.domain, &c.problem, 1) {
        Outcome::Task(t) => {
            let ops: Vec<&str> = t.op_display.iter().map(|s| s.as_str()).collect();
            assert!(
                ops.iter().any(|o| o.starts_with("ACT-START")),
                "start reachable"
            );
            assert!(
                ops.iter().any(|o| o.starts_with("ACT-END")),
                "end reachable"
            );
        }
        _ => panic!("compiled temporal domain should ground to a task"),
    }
}

#[test]
fn t3_decision_epoch_solves_simple_durative() {
    let dom = parse_domain(DUR_DOM).expect("domain");
    let prob = parse_problem(DUR_PROB).expect("problem");
    let plan = temporal::solve(&dom, &prob, 1).expect("temporal plan");
    // one durative step: ACT at time 0 with duration 3; makespan 3
    assert_eq!(plan.steps.len(), 1, "one durative action, end implied");
    assert_eq!(plan.steps[0].action, "ACT");
    assert!((plan.steps[0].time - 0.0).abs() < 1e-9);
    assert_eq!(plan.steps[0].duration, Some(3.0));
    assert!(
        (plan.makespan - 3.0).abs() < 1e-9,
        "makespan {}",
        plan.makespan
    );
}

#[test]
fn t3_required_concurrency_match_fuse() {
    // classic required-concurrency: mend-fuse must run *while* the match is lit;
    // the match (duration 5) provides (light) over its interval, mend (duration 2)
    // needs (light) over all. Sequential ordering can't work; concurrency must.
    let dom = "
    (define (domain mf)
      (:requirements :strips :durative-actions :numeric-fluents)
      (:predicates (light) (mended) (unused))
      (:durative-action light-match
        :parameters ()
        :duration (= ?duration 5)
        :condition (at start (unused))
        :effect (and (at start (and (light) (not (unused)))) (at end (not (light)))))
      (:durative-action mend-fuse
        :parameters ()
        :duration (= ?duration 2)
        :condition (over all (light))
        :effect (at end (mended))))";
    let prob = "(define (problem p) (:domain mf) (:init (unused)) (:goal (mended)))";
    let d = parse_domain(dom).expect("domain");
    let p = parse_problem(prob).expect("problem");
    let plan = temporal::solve(&d, &p, 1).expect("must find a concurrent plan");
    // both durative actions appear; mend-fuse runs within light-match's interval
    assert!(plan.steps.iter().any(|s| s.action == "LIGHT-MATCH"));
    assert!(plan.steps.iter().any(|s| s.action == "MEND-FUSE"));
    temporal::validate(&d, &p, &plan).expect("required-concurrency plan must validate");
}

#[test]
fn t4_public_api_routes_durative_to_temporal() {
    use ferroplan::{solve, Mode, Options};
    let plan = solve(DUR_DOM, DUR_PROB, &Options::default()).expect("solve");
    let p = plan.plan.expect("plan");
    assert_eq!(plan.mode, Mode::Temporal);
    assert_eq!(p.makespan, Some(3.0));
    assert_eq!(p.steps.len(), 1);
    assert_eq!(p.steps[0].action, "ACT");
    assert_eq!(p.steps[0].time, Some(0.0));
    assert_eq!(p.steps[0].duration, Some(3.0));
}

#[test]
fn t3_parameter_dependent_duration() {
    // duration = (dist ?from ?to) — a per-grounding value read from the init.
    // Two hops with different distances prove the duration is evaluated per
    // grounded action, not as a single constant.
    let dom = "
    (define (domain fly)
      (:requirements :typing :durative-actions :numeric-fluents)
      (:types loc)
      (:predicates (at ?l - loc))
      (:functions (dist ?a ?b - loc))
      (:durative-action fly
        :parameters (?from ?to - loc)
        :duration (= ?duration (dist ?from ?to))
        :condition (at start (at ?from))
        :effect (and (at start (not (at ?from))) (at end (at ?to)))))";
    let prob = "
    (define (problem p) (:domain fly)
      (:objects a b c - loc)
      (:init (at a) (= (dist a b) 7) (= (dist b c) 4))
      (:goal (at c)))";
    let d = parse_domain(dom).expect("domain");
    let p = parse_problem(prob).expect("problem");
    let plan = temporal::solve(&d, &p, 1).expect("temporal plan with param durations");
    let fly_ab = plan
        .steps
        .iter()
        .find(|s| s.action == "FLY A B")
        .expect("fly a b");
    let fly_bc = plan
        .steps
        .iter()
        .find(|s| s.action == "FLY B C")
        .expect("fly b c");
    assert_eq!(fly_ab.duration, Some(7.0), "dist a b = 7");
    assert_eq!(fly_bc.duration, Some(4.0), "dist b c = 4");
    // a then b then c, sequential: 7 + 4 = 11, plus one ε gap (fly b->c starts
    // just after fly a->b's at-end effect lands, for PDDL2.1 separation).
    assert!(
        plan.makespan >= 11.0 && plan.makespan < 11.0 + 0.01,
        "makespan {}",
        plan.makespan
    );
    temporal::validate(&d, &p, &plan).expect("param-duration plan must validate");
}

#[test]
fn validate_accepts_and_rejects_plans() {
    let d = parse_domain(DUR_DOM).expect("domain");
    let p = parse_problem(DUR_PROB).expect("problem");
    let plan = temporal::solve(&d, &p, 1).expect("plan");

    // positive: the solver's own plan executes and reaches the goal
    temporal::validate(&d, &p, &plan).expect("solved plan must validate");

    // negative: a tampered duration is caught by the domain cross-check
    let mut bad_dur = plan.clone();
    let s = bad_dur
        .steps
        .iter_mut()
        .find(|s| s.duration.is_some())
        .expect("a durative step");
    s.duration = Some(s.duration.unwrap() + 1.0);
    assert!(
        temporal::validate(&d, &p, &bad_dur).is_err(),
        "a wrong duration must be rejected"
    );

    // negative: the empty plan cannot achieve a non-trivial goal
    let empty = temporal::TimedPlan {
        steps: vec![],
        makespan: 0.0,
    };
    assert!(
        temporal::validate(&d, &p, &empty).is_err(),
        "empty plan must not validate against a real goal"
    );
}
