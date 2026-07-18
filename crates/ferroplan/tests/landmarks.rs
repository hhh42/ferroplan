//! Landmark extraction (0.9 Phase 3): first-achiever backchaining must find
//! the necessary facts of a chain, claim nothing across independent
//! alternatives (soundness), and the LAMA rung must solve with them.

use ferroplan::ground::{ground, Outcome};
use ferroplan::{lama, landmarks, parser};

fn task_of(domain: &str, problem: &str) -> ferroplan::packed::PackedTask {
    let d = parser::parse_domain(domain).unwrap();
    let p = parser::parse_problem(problem).unwrap();
    match ground(&d, &p, 1) {
        Outcome::Task(t) => t,
        other => panic!("expected task, got {:?}", std::mem::discriminant(&other)),
    }
}

const CHAIN: &str = "
(define (domain chain)
  (:requirements :strips)
  (:predicates (a) (b) (c))
  (:action ab :parameters () :precondition (a) :effect (b))
  (:action bc :parameters () :precondition (b) :effect (c)))
";

#[test]
fn chain_landmarks_are_the_chain() {
    let task = task_of(
        CHAIN,
        "(define (problem p) (:domain chain) (:init (a)) (:goal (c)))",
    );
    let lms = landmarks::goal_landmarks(&task);
    // b and c are landmarks; a is excluded (already true in init).
    let names: Vec<&str> = lms
        .iter()
        .map(|&f| task.fact_names[f as usize].as_str())
        .collect();
    assert_eq!(names, vec!["(B)", "(C)"], "chain landmarks");
}

#[test]
fn independent_alternatives_yield_no_false_landmark() {
    // c is reachable via b1 OR b2 with disjoint preconditions: neither b1 nor
    // b2 is a landmark — claiming either would be unsound.
    let domain = "
(define (domain alt)
  (:requirements :strips)
  (:predicates (a1) (a2) (b1) (b2) (c))
  (:action m1 :parameters () :precondition (a1) :effect (b1))
  (:action m2 :parameters () :precondition (a2) :effect (b2))
  (:action f1 :parameters () :precondition (b1) :effect (c))
  (:action f2 :parameters () :precondition (b2) :effect (c)))
";
    let task = task_of(
        domain,
        "(define (problem p) (:domain alt) (:init (a1) (a2)) (:goal (c)))",
    );
    let lms = landmarks::goal_landmarks(&task);
    let names: Vec<&str> = lms
        .iter()
        .map(|&f| task.fact_names[f as usize].as_str())
        .collect();
    assert_eq!(names, vec!["(C)"], "only the goal itself is necessary");
}

#[test]
fn lama_rung_solves_the_chain() {
    let task = task_of(
        CHAIN,
        "(define (problem p) (:domain chain) (:init (a)) (:goal (c)))",
    );
    let (ops, _evaluated) = lama::search(&task, 1, 10_000, &[]).expect("chain is solvable");
    assert_eq!(ops.len(), 2);
}

#[test]
fn lama_rung_reports_unsolvable_within_budget() {
    // DYNAMICALLY unreachable goal (relaxed-reachable, so grounding keeps
    // it): making b consumes a, but the goal wants both. The rung must
    // exhaust its two states and return None, not spin.
    let domain = "
(define (domain consume)
  (:requirements :strips :negative-preconditions)
  (:predicates (a) (b))
  (:action ab :parameters () :precondition (a) :effect (and (b) (not (a)))))
";
    let task = task_of(
        domain,
        "(define (problem p) (:domain consume) (:init (a)) (:goal (and (a) (b))))",
    );
    assert!(lama::search(&task, 1, 10_000, &[]).is_none());
}
