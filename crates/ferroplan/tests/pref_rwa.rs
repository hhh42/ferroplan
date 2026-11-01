//! RESTARTING WEIGHTED A* over the compiled preference task (0.31 Lane E's
//! probe, `pddl3.rs` `rwa_schedule`): under `FF_PREF_RWA` the branch-and-
//! bound's default sweep returns on its first improving plan and the next
//! sweep restarts from the root one step lower on a weight schedule.
//!
//! What this leg pins: that the schedule leaves every certificate the
//! plain loop gives on the small tasks of `tests/pddl3.rs` (same optima),
//! that a caller's schedule and a malformed one both run, and that the
//! hatch is a hatch (`0` is the plain loop). It does NOT pin that RWA*
//! beats the stall ladder anywhere — that is the probe's question,
//! pre-registered on the ten worst trucks/tpp cells in
//! docs/roadmap-0.31.md, and the box answers it, not a fixture.
//!
//! One test, phases in order: the hatch is a process-wide env var, and
//! parallel legs setting different schedules would pin nothing.

use ferroplan::{solve, Mode, Options};

fn pddl3(dom: &str, prob: &str) -> ferroplan::Solution {
    solve(
        dom,
        prob,
        &Options {
            mode: Mode::Pddl3,
            threads: 1,
            ..Default::default()
        },
    )
    .unwrap()
}

const MARK: &str = "(define (domain mk)
 (:requirements :strips :typing :adl :fluents)
 (:types item)
 (:predicates (special ?x - item) (can ?x - item))
 (:functions (total-cost))
 (:action make :parameters (?x - item) :precondition (can ?x) :effect (special ?x)))";

const WEIGHTED_DOM: &str = "(define (domain mk2)
 (:requirements :strips :typing :adl :fluents)
 (:types item)
 (:predicates (special ?x - item) (gold ?x - item) (can ?x - item))
 (:functions (total-cost))
 (:action make :parameters (?x - item) :precondition (can ?x) :effect (special ?x)))";

const WEIGHTED_PRB: &str = "(define (problem p) (:domain mk2)
 (:objects a b c - item)
 (:init (can a) (can b))
 (:goal (and (forall (?x - item) (preference p (special ?x)))
             (forall (?x - item) (preference q (gold ?x)))))
 (:metric minimize (+ (* 1 (is-violated p)) (* 10 (is-violated q)))))";

fn weighted_metric() -> Option<f64> {
    let sol = pddl3(WEIGHTED_DOM, WEIGHTED_PRB);
    assert!(sol.solved, "{:?}", sol.notes);
    sol.plan.unwrap().metric
}

#[test]
fn the_schedule_keeps_every_small_optimum_and_the_hatch_is_a_hatch() {
    // Phase 1 — the default schedule, the three `tests/pddl3.rs` optima.
    std::env::set_var("FF_PREF_RWA", "1");
    let one = "(define (problem p) (:domain mk)
        (:objects a b c - item)
        (:init (can a) (can b))
        (:goal (forall (?x - item) (preference p (special ?x))))
        (:metric minimize (is-violated p)))";
    let sol = pddl3(MARK, one);
    assert!(sol.solved, "{:?}", sol.notes);
    assert_eq!(sol.plan.unwrap().metric, Some(1.0));
    let zero = "(define (problem p) (:domain mk)
        (:objects a b - item)
        (:init (can a) (can b))
        (:goal (forall (?x - item) (preference p (special ?x))))
        (:metric minimize (is-violated p)))";
    assert_eq!(pddl3(MARK, zero).plan.unwrap().metric, Some(0.0));
    assert_eq!(weighted_metric(), Some(31.0));

    // Phase 2 — a caller's schedule, a one-step schedule, and malformed
    // ones (fall back to the default) all run to the same optimum.
    for sched in ["5,3,2,1.5,1", "2", "1", "banana", "0.5,,x,8", "on"] {
        std::env::set_var("FF_PREF_RWA", sched);
        assert_eq!(weighted_metric(), Some(31.0), "schedule {sched:?}");
    }

    // Phase 3 — `0` and empty are the hatch closed: the plain loop.
    for off in ["0", ""] {
        std::env::set_var("FF_PREF_RWA", off);
        assert_eq!(weighted_metric(), Some(31.0), "hatch {off:?}");
    }
}
