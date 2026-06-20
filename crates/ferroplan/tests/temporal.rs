//! Temporal (PDDL2.1 durative-action) parsing — EPIC-Temporal T1.

use ferroplan::parser::parse_domain;
use ferroplan::types::{Expr, TimeSpec};

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
