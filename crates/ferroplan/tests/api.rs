//! Public-API + JSON round-trip tests for the smart `solve` surface.

use ferroplan::{solve, Mode, Options, Solution};

const GRID: &str = "(define (domain g)
 (:requirements :strips :typing)
 (:types loc)
 (:predicates (at ?l - loc) (link ?a - loc ?b - loc))
 (:action move :parameters (?a ?b - loc)
   :precondition (and (at ?a) (link ?a ?b)) :effect (and (at ?b) (not (at ?a)))))";

fn prob(goal: &str) -> String {
    format!(
        "(define (problem p) (:domain g) (:objects x y z - loc)
         (:init (at x) (link x y) (link y z))
         (:goal {goal}))"
    )
}

#[test]
fn solve_returns_structured_plan() {
    let sol = solve(GRID, &prob("(at z)"), &Options::default()).unwrap();
    assert!(sol.solved);
    assert_eq!(sol.mode, Mode::Ff); // no preferences -> classic FF
    let plan = sol.plan.unwrap();
    assert_eq!(plan.length, 2);
    assert_eq!(plan.steps[0].action, "MOVE");
    assert_eq!(plan.steps[0].args, vec!["X", "Y"]);
    assert!(sol.statistics.grounded_actions > 0);
}

#[test]
fn solution_json_round_trips() {
    let sol = solve(GRID, &prob("(at z)"), &Options::default()).unwrap();
    let json = serde_json::to_string(&sol).unwrap();
    let back: Solution = serde_json::from_str(&json).unwrap();
    assert_eq!(back.solved, sol.solved);
    assert_eq!(back.plan.unwrap().length, 2);
    // the serialized form carries structured steps
    assert!(json.contains("\"action\":\"MOVE\""));
}

#[test]
fn unsolvable_goal_reports_unsolved() {
    // z has no link back; goal (at x) after forcing? use an unreachable fact
    let p = "(define (problem p) (:domain g) (:objects x y - loc)
             (:init (at x)) (:goal (at y)))"; // no link x->y
    let sol = solve(GRID, p, &Options::default()).unwrap();
    assert!(!sol.solved);
    assert!(sol.plan.is_none());
}

const TRANSPORT: &str = "(define (domain t)
 (:requirements :strips :typing)
 (:types loc pkg)
 (:predicates (truck-at ?l - loc) (pkg-at ?p - pkg ?l - loc) (in ?p - pkg) (road ?a ?b - loc))
 (:action drive :parameters (?a ?b - loc)
   :precondition (and (truck-at ?a) (road ?a ?b)) :effect (and (truck-at ?b) (not (truck-at ?a))))
 (:action load :parameters (?p - pkg ?l - loc)
   :precondition (and (pkg-at ?p ?l) (truck-at ?l)) :effect (and (in ?p) (not (pkg-at ?p ?l))))
 (:action unload :parameters (?p - pkg ?l - loc)
   :precondition (and (in ?p) (truck-at ?l)) :effect (and (pkg-at ?p ?l) (not (in ?p)))))";

#[test]
fn partition_solves_multi_goal_transport() {
    // two packages share one truck — multiple interacting subgoals exercise the
    // partition resolver (interaction-seeded groups + sibling protection).
    let p = "(define (problem p) (:domain t)
        (:objects a b c - loc p1 p2 - pkg)
        (:init (truck-at a) (pkg-at p1 a) (pkg-at p2 b)
               (road a b) (road b a) (road b c) (road c b) (road a c) (road c a))
        (:goal (and (pkg-at p1 c) (pkg-at p2 c))))";
    let sol = solve(
        TRANSPORT,
        p,
        &Options {
            mode: Mode::Partition,
            threads: 1,
            ..Default::default()
        },
    )
    .unwrap();
    assert!(sol.solved, "partition should deliver both packages");
    assert_eq!(sol.mode, Mode::Partition);
    assert!(sol.plan.unwrap().length >= 4); // at least load/drive/unload each pkg
}

#[test]
fn explicit_modes_run() {
    for m in [Mode::Ff, Mode::Partition] {
        let sol = solve(
            GRID,
            &prob("(at z)"),
            &Options {
                mode: m,
                threads: 1,
                ..Default::default()
            },
        )
        .unwrap();
        assert!(sol.solved, "mode {:?} should solve", m);
        assert_eq!(sol.mode, m);
    }
}

#[test]
fn parse_error_is_typed() {
    let err = solve("(define (domain", "(define (problem", &Options::default()).unwrap_err();
    // it's a typed error, not a panic
    assert!(matches!(err, ferroplan::SolveError::DomainParse(_)));
}

#[test]
fn parse_error_reports_line() {
    // bad requirement on line 2 -> ParseError carries line 2
    let dom = "(define (domain d)\n (:requirements :strips :bogus)\n (:predicates (x)))";
    let prob = "(define (problem p) (:domain d) (:init) (:goal (x)))";
    match solve(dom, prob, &Options::default()).unwrap_err() {
        ferroplan::SolveError::DomainParse(pe) => assert_eq!(pe.line, 2, "{}", pe),
        e => panic!("expected DomainParse, got {e:?}"),
    }
}
