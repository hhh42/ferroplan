//! The deterministic search node cap (0.8 Phase 3, docs/roadmap-0.8.md).
//!
//! One sequential test on purpose: FF_SEARCH_NODE_CAP is a process-global
//! env knob, and a separate test binary running a single test can never race
//! the other suites (the tests/espc.rs convention).

use ferroplan::{solve, Options, Search};

#[test]
fn node_cap_bounds_the_search_and_zero_disables() {
    // A chain domain the best-first search needs a few dozen insertions to
    // solve. Default (env unset): solves. Cap 1: the pass trips immediately
    // with no incumbent -> unsolved, deterministically at any thread count.
    // Cap 0: explicit disable -> solves again.
    let dom = "(define (domain chain)
      (:predicates (p0) (p1) (p2) (p3) (p4) (p5))
      (:action s1 :precondition (p0) :effect (p1))
      (:action s2 :precondition (p1) :effect (p2))
      (:action s3 :precondition (p2) :effect (p3))
      (:action s4 :precondition (p3) :effect (p4))
      (:action s5 :precondition (p4) :effect (p5)))";
    let prob = "(define (problem c) (:domain chain) (:init (p0)) (:goal (p5)))";
    let opts = |threads| Options {
        search: Search::BestFirst,
        threads,
        ..Options::default()
    };

    let sol = solve(dom, prob, &opts(1)).unwrap();
    assert!(sol.solved, "default cap must be inert on a tiny task");

    std::env::set_var("FF_SEARCH_NODE_CAP", "1");
    let capped1 = solve(dom, prob, &opts(1)).unwrap();
    let capped8 = solve(dom, prob, &opts(8)).unwrap();
    std::env::set_var("FF_SEARCH_NODE_CAP", "0");
    let disabled = solve(dom, prob, &opts(1)).unwrap();
    std::env::remove_var("FF_SEARCH_NODE_CAP");

    assert!(!capped1.solved, "cap=1 must trip before the goal");
    assert_eq!(
        capped1.solved, capped8.solved,
        "the cap is thread-count independent"
    );
    assert!(disabled.solved, "cap=0 disables the bound");
}
