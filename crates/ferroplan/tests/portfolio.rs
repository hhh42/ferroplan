//! The sequential portfolio scheduler (ferroplan-roadmap.md Phase 6).

use ferroplan::{solve, Mode, Options};

const DOM: &str = "(define (domain chain)
  (:predicates (p0) (p1) (p2) (p3))
  (:action s1 :precondition (p0) :effect (p1))
  (:action s2 :precondition (p1) :effect (p2))
  (:action s3 :precondition (p2) :effect (p3)))";

#[test]
fn portfolio_solves_and_reports_the_winner() {
    let p = "(define (problem c) (:domain chain) (:init (p0)) (:goal (p3)))";
    let opts = |threads| Options {
        mode: Mode::Portfolio,
        threads,
        ..Options::default()
    };
    let t1 = solve(DOM, p, &opts(1)).unwrap();
    let t8 = solve(DOM, p, &opts(8)).unwrap();
    assert!(t1.solved);
    let plan1 = t1.plan.unwrap();
    let plan8 = t8.plan.unwrap();
    assert_eq!(plan1.length, 3);
    let steps = |pl: &ferroplan::Plan| {
        pl.steps
            .iter()
            .map(|s| s.action.clone())
            .collect::<Vec<_>>()
    };
    assert_eq!(
        steps(&plan1),
        steps(&plan8),
        "portfolio is thread-count independent"
    );
    assert!(
        t1.notes.iter().any(|n| n.contains("portfolio: solved by")),
        "winner reported: {:?}",
        t1.notes
    );
}

#[test]
fn portfolio_proves_unsolvable_early() {
    // (p3) is unreachable without (p0): a complete member's exhaustion
    // settles the whole portfolio — no schedule runs forever.
    let p = "(define (problem c) (:domain chain) (:init (p1)) (:goal (and (p3) (p0))))";
    let sol = solve(
        DOM,
        p,
        &Options {
            mode: Mode::Portfolio,
            threads: 1,
            ..Options::default()
        },
    )
    .unwrap();
    assert!(sol.plan.is_none(), "unsolvable settles: {:?}", sol.plan);
}

#[test]
fn portfolio_falls_back_for_preference_problems() {
    // A PDDL3 problem under --mode portfolio keeps the metric machinery
    // (the fallback routing) — the metric must still be reported.
    let d = "(define (domain sw) (:requirements :strips :preferences)
      (:predicates (on) (off))
      (:action flip :precondition (off) :effect (and (not (off)) (on))))";
    let p = "(define (problem s) (:domain sw) (:init (off))
      (:goal (and (off) (preference pv (on))))
      (:metric minimize (is-violated pv)))";
    let sol = solve(
        d,
        p,
        &Options {
            mode: Mode::Portfolio,
            threads: 1,
            ..Options::default()
        },
    )
    .unwrap();
    assert!(sol.solved);
    assert!(
        sol.plan.unwrap().metric.is_some(),
        "preference problems keep the metric machinery"
    );
}
