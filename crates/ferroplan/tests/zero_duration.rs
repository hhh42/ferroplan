//! Zero-duration durative actions (0.25 Phase 4 — the pathways decode's
//! find): IPC-2006 pathways-metric-time gates EVERYTHING behind
//! `choose`/`initialize` with `(= ?duration 0)`, and `eval_duration`'s
//! positivity guard silently SKIPPED such actions — the search then
//! exhausted an empty reachable space in milliseconds and the board
//! booked thirty false instant failures as "early exits". RED first:
//! before the fix, the trivial task below reported unsolved.
//!
//! The semantics: a dur-0 interval's END shares its START's epoch (the
//! decision-epoch order still fires it after the start's effects); the
//! plan states duration 0 verbatim, which both the internal validator
//! (`[min,max]` check) and VAL (tolerance ε/2) accept against the
//! domain's `= 0` constraint.

const DOMAIN: &str = "
(define (domain z0)
  (:requirements :strips :durative-actions)
  (:predicates (a) (g) (h2))
  (:durative-action zap
    :parameters ()
    :duration (= ?duration 0)
    :condition (at start (a))
    :effect (at start (g)))
  (:durative-action chain
    :parameters ()
    :duration (= ?duration 2)
    :condition (at start (g))
    :effect (at end (h2))))
";

#[test]
fn zero_duration_action_solves() {
    let sol = ferroplan::solve(
        DOMAIN,
        "(define (problem z1) (:domain z0) (:init (a)) (:goal (g)))",
        &ferroplan::Options::default(),
    )
    .expect("solve runs");
    assert!(
        sol.solved,
        "a zero-duration action must be schedulable, not silently skipped: {:?}",
        sol.notes
    );
    let plan = sol.plan.expect("plan");
    let zap = plan
        .steps
        .iter()
        .find(|s| s.action.contains("ZAP") || s.action.contains("zap"))
        .expect("the zap step is in the plan");
    assert_eq!(
        zap.duration,
        Some(0.0),
        "the plan states the domain's own duration, verbatim"
    );
}

#[test]
fn zero_duration_chains_into_real_durations() {
    // The pathways shape in miniature: a dur-0 gate enabling a real
    // interval — the whole board died on exactly this composition.
    let sol = ferroplan::solve(
        DOMAIN,
        "(define (problem z2) (:domain z0) (:init (a)) (:goal (h2)))",
        &ferroplan::Options::default(),
    )
    .expect("solve runs");
    assert!(
        sol.solved,
        "dur-0 gate then dur-2 interval must chain: {:?}",
        sol.notes
    );
    let plan = sol.plan.expect("plan");
    assert!(
        plan.makespan.unwrap_or(0.0) >= 2.0 - 1e-6,
        "the chain's real duration is paid: {:?}",
        plan.makespan
    );
}
