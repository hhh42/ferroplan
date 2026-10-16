//! A TRANSFER IS NOT CONSUMPTION (0.30 Lane 2b).
//!
//! The consumption charge (0.29 Lane 2, `heuristic::consumption_charge`)
//! sums what the relaxed plan SPENDS of every fluent its preconditions
//! read from below and prices the deficit's best raiser. On rovers that is
//! the recharge the relaxation could not see. On the 2026 register machine
//! `ztalloc-sum` it is a plateau: the "minus one, divide by three" sequence
//! decreases `work-value` by 3 per step while `(>= (work-value) 3)` reads
//! it from below, and the step's own raiser, `copy-reset`, MOVES `value`
//! into `work-value` -- a transfer the charge bills as spending. On ztalloc
//! i12 the charged `best_h` falls 4264 -> 1 in 1,352 evaluations and sits
//! at 1 for 305,000 more; uncharged it reaches 2 and LAMA solves in 3 s
//! (`benchmarks/probes-0.30/losses29/summary.txt`: i9 and i12, settlers i5,
//! sugar i17 and rovers i28 all restored by `FF_NO_CONSUME=1`).
//!
//! The fixture is ztalloc i12's own shape, which is small: five registers
//! at 1, the goal a SUM of their values equal to 4269 with every register
//! back in its normal state (the instance is 17 init lines). A single
//! register with a point goal does NOT show the plateau (the first version
//! of this fixture, 730 -> 243, solved both ways in a blink): the sum goal
//! over several registers is what the charge prices wrongly. Both arms
//! must solve under the same evaluation cap. RED on the 0.29 charge for the
//! charged arm; the uncharged arm pins that the task itself is a 3 s solve.
//! One test in this binary: the hatch is a process-global knob.

fn machine() -> (String, String) {
    let dom = "(define (domain ztalloc-sum)
      (:requirements :typing :strips :numeric-fluents :action-costs)
      (:types register)
      (:predicates (free) (normal ?r - register)
                   (m1d3-after-dec ?r - register) (m1d3-dividing ?r - register))
      (:functions (value ?r - register) (work-value ?r - register) (total-cost))
      (:action double :parameters (?r - register)
        :precondition (and (free) (normal ?r))
        :effect (and (increase (value ?r) (value ?r)) (increase (total-cost) 1)))
      (:action m1d3-start :parameters (?r - register)
        :precondition (and (free) (normal ?r) (>= (value ?r) 1) (= (work-value ?r) 0))
        :effect (and (not (free)) (not (normal ?r)) (m1d3-after-dec ?r)
                     (decrease (value ?r) 1) (increase (total-cost) 1)))
      (:action m1d3-copy-reset :parameters (?r - register)
        :precondition (and (m1d3-after-dec ?r))
        :effect (and (not (m1d3-after-dec ?r)) (m1d3-dividing ?r)
                     (increase (work-value ?r) (value ?r)) (decrease (value ?r) (value ?r))))
      (:action m1d3-div-step :parameters (?r - register)
        :precondition (and (m1d3-dividing ?r) (>= (work-value ?r) 3))
        :effect (and (decrease (work-value ?r) 3) (increase (value ?r) 1)))
      (:action m1d3-finish :parameters (?r - register)
        :precondition (and (m1d3-dividing ?r) (= (work-value ?r) 0))
        :effect (and (not (m1d3-dividing ?r)) (free) (normal ?r))))"
        .to_string();
    let prb = "(define (problem ztalloc-12) (:domain ztalloc-sum)
      (:objects r1 r2 r3 r4 r5 - register)
      (:init (free) (normal r1) (normal r2) (normal r3) (normal r4) (normal r5)
             (= (value r1) 1) (= (work-value r1) 0)
             (= (value r2) 1) (= (work-value r2) 0)
             (= (value r3) 1) (= (work-value r3) 0)
             (= (value r4) 1) (= (work-value r4) 0)
             (= (value r5) 1) (= (work-value r5) 0)
             (= (total-cost) 0))
      (:goal (and
        (= (+ (+ (+ (+ (value r1) (value r2)) (value r3)) (value r4)) (value r5)) 4269)
        (free)
        (normal r1) (= (work-value r1) 0)
        (normal r2) (= (work-value r2) 0)
        (normal r3) (= (work-value r3) 0)
        (normal r4) (= (work-value r4) 0)
        (normal r5) (= (work-value r5) 0)))
      (:metric minimize (total-cost)))"
        .to_string();
    (dom, prb)
}

fn solve(charged: bool) -> (bool, usize) {
    let (dom, prb) = machine();
    let opts = ferroplan::Options {
        threads: 1,
        max_evaluated: Some(400_000),
        ..Default::default()
    };
    if charged {
        std::env::remove_var("FF_NO_CONSUME");
    } else {
        std::env::set_var("FF_NO_CONSUME", "1");
    }
    let sol = ferroplan::solve(&dom, &prb, &opts).unwrap();
    std::env::remove_var("FF_NO_CONSUME");
    (sol.solved, sol.statistics.evaluated_states)
}

/// RED on 2026-10-15 (400,086 evaluations charged, a 3 s solve uncharged)
/// until the linear achiever stopped pricing an equality goal with a mover
/// that overshoots it (0.30 Lane 2b, mechanism (c)).
#[test]
fn a_sum_the_goal_needs_is_not_billed_as_spending() {
    let (solved, evals) = solve(false);
    assert!(
        solved,
        "uncharged, ztalloc i12's shape is a 3 s solve: {evals} evaluations"
    );
    let (solved, evals) = solve(true);
    assert!(
        solved,
        "charged, the search must still reach the sum: the charge holds every \
         near-goal state at h = 1 -- {evals} evaluations without a plan"
    );
}
