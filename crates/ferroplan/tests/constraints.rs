//! PDDL3 trajectory constraints (`(:constraints ...)` — `always`, `sometime`, …)
//! are parsed but not yet enforced by any solving path. Rather than silently accept
//! and drop a user's hard constraint, every public entrypoint rejects an input that
//! carries one. These tests pin that contract.

use ferroplan::{solve, Options, SolveError};

// A trivially-solvable STRIPS problem, but with a trajectory constraint attached.
const DOM: &str = "(define (domain sw)
  (:requirements :strips :constraints)
  (:predicates (on) (off))
  (:action flip-on :precondition (off) :effect (and (not (off)) (on)))
  (:action flip-off :precondition (on) :effect (and (not (on)) (off))))";

const PROB_WITH_CONSTRAINT: &str = "(define (problem sw-1) (:domain sw)
  (:init (off))
  (:goal (on))
  (:constraints (always (or (on) (off)))))";

const PROB_PLAIN: &str = "(define (problem sw-2) (:domain sw)
  (:init (off))
  (:goal (on)))";

#[test]
fn solve_rejects_trajectory_constraints_instead_of_ignoring_them() {
    let err = solve(DOM, PROB_WITH_CONSTRAINT, &Options::default())
        .expect_err("a (:constraints ...) block must be rejected, not silently dropped");
    match err {
        SolveError::Unsupported(msg) => {
            assert!(
                msg.contains(":constraints"),
                "message should name the unsupported feature: {msg}"
            );
        }
        other => panic!("expected SolveError::Unsupported, got {other:?}"),
    }
}

#[test]
fn solve_still_works_without_constraints() {
    let sol = solve(DOM, PROB_PLAIN, &Options::default()).expect("plain problem solves");
    let plan = sol.plan.expect("a plan is found");
    assert_eq!(plan.steps.len(), 1, "flip-on solves it in one step");
}

#[test]
fn cli_text_path_rejects_trajectory_constraints() {
    // The `ff` text path (run_planner) parses independently of solve(); it must
    // reject too, with a non-zero exit code.
    let (out, code) = ferroplan::run_planner(DOM, PROB_WITH_CONSTRAINT, &Options::default(), false);
    assert_eq!(code, 1, "constraint domain must exit non-zero:\n{out}");
    assert!(
        out.contains(":constraints"),
        "output should explain the rejection:\n{out}"
    );
}
