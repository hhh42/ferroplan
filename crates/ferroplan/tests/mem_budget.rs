//! `FF_MEM_BUDGET_GB` (0.21 Phase 6 lever 0, docs/roadmap-0.21.md): the
//! runner's byte budget plumbed into the engine by env, read BEFORE the
//! `/proc/self/limits` path — on Darwin `setrlimit(RLIMIT_AS)` cannot be
//! enforced, so without this the internal node cap never arms and the
//! runner's RSS watchdog kills externally with wall unspent.
//!
//! One sequential test on purpose: FF_MEM_BUDGET_GB is a process-global
//! env knob (the tests/node_cap.rs convention).

use ferroplan::{solve, Options, Search};

#[test]
fn mem_budget_env_arms_the_internal_cap_on_any_kernel() {
    let dom = "(define (domain chain)
      (:predicates (p0) (p1) (p2) (p3) (p4) (p5))
      (:action s1 :precondition (p0) :effect (p1))
      (:action s2 :precondition (p1) :effect (p2))
      (:action s3 :precondition (p2) :effect (p3))
      (:action s4 :precondition (p3) :effect (p4))
      (:action s5 :precondition (p4) :effect (p5)))";
    let prob = "(define (problem c) (:domain chain) (:init (p0)) (:goal (p5)))";
    let opts = Options {
        search: Search::BestFirst,
        threads: 1,
        ..Options::default()
    };

    // No budget declared: today's behavior, the tiny task solves.
    let base = solve(dom, prob, &opts).unwrap();
    assert!(base.solved, "no budget must be inert on a tiny task");

    // A fractional budget so small the per-node model admits ~1 insertion:
    // the cap must trip INTERNALLY and return capped (unsolved), exactly
    // the woodworking shape — killed by the model, not the watchdog.
    std::env::set_var("FF_MEM_BUDGET_GB", "0.0000005");
    let capped = solve(dom, prob, &opts).unwrap();
    assert!(
        !capped.solved,
        "a ~500-byte budget must trip the internal cap before the goal"
    );

    // Garbage parses to no override — behavior must fall back to default.
    std::env::set_var("FF_MEM_BUDGET_GB", "not-a-number");
    let garbage = solve(dom, prob, &opts).unwrap();
    assert!(garbage.solved, "an unparsable budget must not arm the cap");

    // A generous fractional budget (0.5 GiB) is plenty for the chain.
    std::env::set_var("FF_MEM_BUDGET_GB", "0.5");
    let roomy = solve(dom, prob, &opts).unwrap();
    std::env::remove_var("FF_MEM_BUDGET_GB");
    assert!(roomy.solved, "a 0.5 GiB budget must not bind a 6-fact task");
}
