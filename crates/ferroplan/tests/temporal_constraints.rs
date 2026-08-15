//! PDDL3 trajectory constraints on DURATIVE domains — enforced since 0.23
//! Phase 2 (docs/roadmap-0.23.md).
//!
//! RED, recorded before the change (0.22.0 + cd645f1, this worktree): every
//! problem below — including the EMPTY `(and)` block — died at the one gate
//! (constraints.rs `gate()`) with the blanket
//! "trajectory constraints on durative-action (temporal) domains are not
//! yet enforced" rejection, exit 1; so did storage-time-constraints i1,
//! tpp-metric-time-constraints i1/i2, and trucks-time-constraints i1 on the
//! baseline binary. GREEN: the untimed operators now compile onto the
//! snap-compiled task (monitor `When`s riding every happening; the at-end
//! fold as a transition-free `TRAJ-END` latch), while the timed operators
//! and soft (preference) constraints keep NAMED rejections.
//!
//! The oracle everywhere is `temporal::validate`, which since this phase
//! folds the ORIGINAL constraint semantics over its replay — independent of
//! the compiled monitors, the verify.rs convention. On the boards the
//! referee is external VAL (constraints enforced natively).

use ferroplan::temporal::{TimedPlan, TimedStep};
use ferroplan::{solve, Options, SolveError};

fn solve1(d: &str, p: &str) -> ferroplan::Solution {
    solve(
        d,
        p,
        &Options {
            threads: 1,
            ..Options::default()
        },
    )
    .expect("solve")
}

fn steps(plan: &ferroplan::Plan) -> Vec<String> {
    plan.steps
        .iter()
        .map(|s| {
            if s.args.is_empty() {
                s.action.clone()
            } else {
                format!("{} {}", s.action, s.args.join(" "))
            }
        })
        .collect()
}

fn timed_plan_of(plan: &ferroplan::Plan) -> TimedPlan {
    TimedPlan {
        steps: plan
            .steps
            .iter()
            .map(|s| TimedStep {
                time: s.time.expect("temporal step has a time"),
                action: if s.args.is_empty() {
                    s.action.clone()
                } else {
                    format!("{} {}", s.action, s.args.join(" "))
                },
                duration: s.duration,
            })
            .collect(),
        makespan: plan.makespan.unwrap_or(0.0),
    }
}

/// Solve, assert a plan, and referee it through `temporal::validate` — which
/// folds the ORIGINAL constraints over the replayed trajectory.
fn solve_green(d: &str, p: &str) -> ferroplan::Plan {
    let sol = solve1(d, p);
    let plan = sol.plan.expect("expected a plan");
    assert!(
        !steps(&plan).iter().any(|s| s == "TRAJ-END"),
        "synthetic TRAJ-END step leaked into the reported plan: {:?}",
        steps(&plan)
    );
    let dom = ferroplan::parser::parse_domain(d).expect("domain");
    let prb = ferroplan::parser::parse_problem(p).expect("problem");
    if let Err(e) = ferroplan::temporal::validate(&dom, &prb, &timed_plan_of(&plan)) {
        panic!(
            "validate (constraint fold included) rejected the plan: {e}\nsteps: {:?}",
            steps(&plan)
        );
    }
    plan
}

fn unsolvable(d: &str, p: &str) {
    let sol = solve1(d, p);
    assert!(
        sol.plan.is_none(),
        "expected unsolvable, got {:?}",
        sol.plan.map(|pl| steps(&pl))
    );
}

// ---- stage a: the at-end fold ---------------------------------------------

const ATEND_DOM: &str = "(define (domain tconstr)
  (:requirements :strips :durative-actions :constraints)
  (:predicates (home) (done) (flag))
  (:durative-action work
    :parameters ()
    :duration (= ?duration 2)
    :condition (at start (home))
    :effect (at end (done)))
  (:durative-action raise
    :parameters ()
    :duration (= ?duration 1)
    :condition (at start (home))
    :effect (at end (flag))))";

fn atend_prob(constraints: &str) -> String {
    format!(
        "(define (problem tc) (:domain tconstr)
           (:init (home)) (:goal (done)) (:constraints {constraints}))"
    )
}

#[test]
fn at_end_forces_the_extra_action() {
    // goal needs only WORK; (at end (flag)) forces RAISE too.
    let plan = solve_green(ATEND_DOM, &atend_prob("(at end (flag))"));
    let s = steps(&plan);
    assert!(
        s.iter().any(|x| x == "RAISE"),
        "constraint must bite: {s:?}"
    );
    assert!(
        s.iter().any(|x| x == "WORK"),
        "goal still needs WORK: {s:?}"
    );
}

#[test]
fn at_end_no_bite_when_goal_already_implies_it() {
    let plan = solve_green(ATEND_DOM, &atend_prob("(at end (done))"));
    assert_eq!(steps(&plan), vec!["WORK"], "no extra machinery in the plan");
}

#[test]
fn empty_and_block_is_consumed_not_rejected() {
    // tpp-metric-time-constraints i1 ships literally `(:constraints (and))`.
    let plan = solve_green(ATEND_DOM, &atend_prob("(and)"));
    assert_eq!(steps(&plan), vec!["WORK"]);
}

#[test]
fn empty_goal_with_at_end_constraint_is_the_whole_objective() {
    // The storage-time-constraints shape: `(:goal (and))` — the at-end
    // block IS the objective, and the validator must not short-circuit on
    // the trivially-true goal.
    let p = "(define (problem tc0) (:domain tconstr)
      (:init (home)) (:goal (and)) (:constraints (at end (flag))))";
    let plan = solve_green(ATEND_DOM, p);
    assert_eq!(steps(&plan), vec!["RAISE"]);
}

// ---- stage b: untimed monitors on the temporal path ------------------------

#[test]
fn always_blocks_the_violating_route() {
    let d = "(define (domain tsafe)
      (:requirements :strips :durative-actions :constraints)
      (:predicates (idle) (safe) (goal-fact))
      (:durative-action fast
        :parameters ()
        :duration (= ?duration 1)
        :condition (at start (idle))
        :effect (and (at start (not (safe))) (at end (safe)) (at end (goal-fact))))
      (:durative-action slow
        :parameters ()
        :duration (= ?duration 3)
        :condition (at start (idle))
        :effect (at end (goal-fact))))";
    let p = "(define (problem ps) (:domain tsafe)
      (:init (idle) (safe)) (:goal (goal-fact))
      (:constraints (always (safe))))";
    let plan = solve_green(d, p);
    let s = steps(&plan);
    assert!(
        s.iter().any(|x| x == "SLOW") && !s.iter().any(|x| x == "FAST"),
        "always (safe) must forbid FAST's mid-interval violation: {s:?}"
    );
    // no-bite twin: without the block, the problem stays solvable (either route).
    let p0 = "(define (problem ps0) (:domain tsafe)
      (:init (idle) (safe)) (:goal (goal-fact)))";
    assert!(solve1(d, p0).plan.is_some());
}

#[test]
fn sometime_forces_the_probe() {
    let d = "(define (domain tprobe)
      (:requirements :strips :durative-actions :constraints)
      (:predicates (home) (done) (probe-on))
      (:durative-action work
        :parameters ()
        :duration (= ?duration 2)
        :condition (at start (home))
        :effect (at end (done)))
      (:durative-action probe
        :parameters ()
        :duration (= ?duration 1)
        :condition (at start (home))
        :effect (and (at start (probe-on)) (at end (not (probe-on))))))";
    let p = "(define (problem pp) (:domain tprobe)
      (:init (home)) (:goal (done))
      (:constraints (sometime (probe-on))))";
    let plan = solve_green(d, p);
    let s = steps(&plan);
    assert!(s.iter().any(|x| x == "PROBE"), "sometime must bite: {s:?}");
}

#[test]
fn at_most_once_blocks_the_second_episode() {
    // Producing (a) and (b) needs two holding episodes: USE consumes (fresh),
    // SHARPEN needs the tool returned — so a-then-b forces take/return/take.
    let d = "(define (domain ttool)
      (:requirements :strips :durative-actions :constraints)
      (:predicates (free) (holding) (fresh) (a) (b))
      (:durative-action take
        :parameters ()
        :duration (= ?duration 1)
        :condition (at start (free))
        :effect (and (at start (not (free))) (at start (holding))))
      (:durative-action return
        :parameters ()
        :duration (= ?duration 1)
        :condition (at start (holding))
        :effect (and (at start (not (holding))) (at start (free))))
      (:durative-action sharpen
        :parameters ()
        :duration (= ?duration 1)
        :condition (and (at start (free)) (over all (free)))
        :effect (at end (fresh)))
      (:durative-action use-a
        :parameters ()
        :duration (= ?duration 1)
        :condition (at start (and (holding) (fresh)))
        :effect (and (at start (not (fresh))) (at end (a))))
      (:durative-action use-b
        :parameters ()
        :duration (= ?duration 1)
        :condition (at start (and (holding) (fresh)))
        :effect (and (at start (not (fresh))) (at end (b)))))";
    let goal_both = "(define (problem pt) (:domain ttool)
      (:init (free) (fresh)) (:goal (and (a) (b)))
      (:constraints (at-most-once (holding))))";
    unsolvable(d, goal_both);
    // no-bite: one episode suffices for (a) alone.
    let goal_one = "(define (problem pt1) (:domain ttool)
      (:init (free) (fresh)) (:goal (a))
      (:constraints (at-most-once (holding))))";
    let plan = solve_green(d, goal_one);
    assert!(steps(&plan).iter().any(|x| x == "USE-A"));
}

#[test]
fn sometime_before_orders_the_plan_and_rejects_s0() {
    let d = "(define (domain tord)
      (:requirements :strips :durative-actions :constraints)
      (:predicates (idle) (tested) (deployed))
      (:durative-action test
        :parameters ()
        :duration (= ?duration 1)
        :condition (at start (idle))
        :effect (at end (tested)))
      (:durative-action deploy
        :parameters ()
        :duration (= ?duration 1)
        :condition (at start (idle))
        :effect (at end (deployed))))";
    let p = "(define (problem po) (:domain tord)
      (:init (idle)) (:goal (deployed))
      (:constraints (sometime-before (deployed) (tested))))";
    let plan = solve_green(d, p);
    assert!(
        steps(&plan).iter().any(|x| x == "TEST"),
        "sometime-before must force TEST: {:?}",
        steps(&plan)
    );
    // φ already true at S_0: nothing can be strictly earlier — unsolvable.
    let p0 = "(define (problem po0) (:domain tord)
      (:init (idle) (deployed)) (:goal (tested))
      (:constraints (sometime-before (deployed) (tested))))";
    unsolvable(d, p0);
}

#[test]
fn til_happenings_are_monitored_and_green() {
    // A TIL opens the window GO needs; the constraint is satisfied along the
    // way. Pins that TIL appliers ride the monitor block (and the TRAJ
    // phase-fact precondition) without breaking the search or the replay.
    let d = "(define (domain ttil)
      (:requirements :strips :durative-actions :timed-initial-literals :constraints)
      (:predicates (window) (done))
      (:durative-action go
        :parameters ()
        :duration (= ?duration 1)
        :condition (at start (window))
        :effect (at end (done))))";
    let p = "(define (problem ptil) (:domain ttil)
      (:init (at 1 (window))) (:goal (done))
      (:constraints (sometime (window))))";
    let plan = solve_green(d, p);
    assert!(steps(&plan).iter().any(|x| x == "GO"));
}

// ---- the ε-interplay: emitted order vs search order ------------------------

/// The monitor-vs-emission pin (docs/roadmap-0.23.md Phase 2: "where a
/// monitor's violation flip lands relative to ε-chains"). C's end deletes
/// (p) on the same ε-slot where B's start adds (q); the search certifies
/// `always (or p q)` on ITS order (B-START before C-END), the plain
/// ends-first emission inverts it, and no footprint guard in the ε-repair
/// models a conditional-effect condition read. The monitor audit replays
/// the EMITTED schedule and refuses the red one; the search keeps going and
/// lands the compliant schedule (start C late enough that its end falls
/// after q exists). The unit half of this pin (audit red on the handcrafted
/// inverted plan) lives in temporal.rs's test module.
#[test]
fn emitted_order_monitor_flip_is_audited_never_shipped() {
    let d = "(define (domain epsmon)
      (:requirements :strips :durative-actions :constraints)
      (:predicates (p) (q) (g1) (g2) (g3))
      (:durative-action acta
        :parameters ()
        :duration (= ?duration 2)
        :condition (at start (p))
        :effect (at end (g1)))
      (:durative-action actb
        :parameters ()
        :duration (= ?duration 1)
        :condition (at start (g1))
        :effect (and (at start (q)) (at end (g2))))
      (:durative-action actc
        :parameters ()
        :duration (= ?duration 2)
        :condition (at start (p))
        :effect (and (at end (not (p))) (at end (g3)))))";
    let p = "(define (problem pe) (:domain epsmon)
      (:init (p)) (:goal (and (g1) (g2) (g3)))
      (:constraints (always (or (p) (q)))))";
    let plan = solve_green(d, p);
    // Whatever schedule shipped, the fold above proved it holds the
    // constraint in EMITTED order — the audit's whole contract.
    assert_eq!(plan.steps.len(), 3, "three actions: {:?}", steps(&plan));
}

// ---- stage c: the timed operators (0.24 Phase 4) ---------------------------
//
// RED, recorded before the change (this worktree at adcb673): every `within`
// / `always-within` problem below died at the gate with the 0.23 blanket
// "time-bounded and not yet enforced" rejection, exit path
// SolveError::Unsupported — the named rejection stage b left in place. GREEN:
// a search-maintained clock fluent (`TRAJ-CLOCK`, stamped at every decision
// epoch) lowers both operators to ordinary monitor transitions with numeric
// conditions on the stage a+b machinery. `hold-during` / `hold-after` keep
// the named rejection (grepped absent from the whole 2006 corpus), and the
// classical path keeps rejecting all four by name.

/// Both flag achievers exist; only the quick one lands inside the deadline.
const WITHIN_DOM: &str = "(define (domain twin)
  (:requirements :strips :durative-actions :constraints)
  (:predicates (home) (done) (flag))
  (:durative-action work
    :parameters ()
    :duration (= ?duration 2)
    :condition (at start (home))
    :effect (at end (done)))
  (:durative-action quick-flag
    :parameters ()
    :duration (= ?duration 1)
    :condition (at start (home))
    :effect (at end (flag)))
  (:durative-action slow-flag
    :parameters ()
    :duration (= ?duration 6)
    :condition (at start (home))
    :effect (at end (flag))))";

#[test]
fn within_bites_and_forces_the_quick_achiever() {
    // goal needs only WORK; (within 2 (flag)) forces a flag by t=2 — only
    // QUICK-FLAG (end t=1) can land it. SLOW-FLAG's t=6 flag is too late.
    let p = "(define (problem pw) (:domain twin)
      (:init (home)) (:goal (done))
      (:constraints (within 2 (flag))))";
    let plan = solve_green(WITHIN_DOM, p);
    let s = steps(&plan);
    assert!(
        s.iter().any(|x| x == "QUICK-FLAG"),
        "within must bite — the quick achiever is forced: {s:?}"
    );
}

/// φ only reachable late: the deadline has passed before any achiever can
/// land, so the VIOL branch is every branch — honest unsolvable. Both ops
/// consume a one-shot token: an unsolvable verdict must come from a FINITE
/// exhaustion, not a node-cap march — on monitored tasks the identical-
/// interval reduction is off (the arrival-order rule), so a freely
/// restartable interval stacks agenda copies all the way to the cap.
#[test]
fn within_red_when_phi_only_arrives_late() {
    let d = "(define (domain twlate)
      (:requirements :strips :durative-actions :constraints)
      (:predicates (idle) (fresh) (done) (flag))
      (:durative-action work
        :parameters ()
        :duration (= ?duration 2)
        :condition (at start (idle))
        :effect (and (at start (not (idle))) (at end (done))))
      (:durative-action slow-flag
        :parameters ()
        :duration (= ?duration 6)
        :condition (at start (and (done) (fresh)))
        :effect (and (at start (not (fresh))) (at end (flag)))))";
    // flag's earliest state is t=8 (work 0->2, slow-flag 2->8): within 3 is
    // unmeetable; within 9 is the no-bite twin on the SAME shape.
    let red = "(define (problem pwl) (:domain twlate)
      (:init (idle) (fresh)) (:goal (and (done) (flag)))
      (:constraints (within 3 (flag))))";
    unsolvable(d, red);
    let green = "(define (problem pwg) (:domain twlate)
      (:init (idle) (fresh)) (:goal (and (done) (flag)))
      (:constraints (within 9 (flag))))";
    let plan = solve_green(d, green);
    assert!(steps(&plan).iter().any(|x| x == "SLOW-FLAG"));
}

#[test]
fn within_no_bite_when_phi_lands_inside_the_deadline() {
    // The goal route itself satisfies the constraint: no extra machinery.
    let p = "(define (problem pwn) (:domain twin)
      (:init (home)) (:goal (done))
      (:constraints (within 5 (done))))";
    let plan = solve_green(WITHIN_DOM, p);
    assert_eq!(steps(&plan), vec!["WORK"], "no bite: {:?}", steps(&plan));
}

/// The response-deadline shape: whenever φ (the alarm) appears, ψ (the
/// handler) must appear within t.
const RESPOND_DOM: &str = "(define (domain tresp)
  (:requirements :strips :durative-actions :constraints)
  (:predicates (idle) (alarm) (handled) (done))
  (:durative-action work
    :parameters ()
    :duration (= ?duration 2)
    :condition (at start (idle))
    :effect (and (at start (not (idle))) (at start (alarm)) (at end (done))))
  (:durative-action quiet-work
    :parameters ()
    :duration (= ?duration 2)
    :condition (at start (idle))
    :effect (and (at start (not (idle))) (at end (done))))
  (:durative-action respond
    :parameters ()
    :duration (= ?duration 1)
    :condition (at start (alarm))
    :effect (at end (handled))))";

#[test]
fn always_within_forces_the_response() {
    // The only route to the goal trips the alarm at t=0, so RESPOND
    // (handled at t=1 <= 0+2) is forced into the plan by the constraint —
    // without it the plan is WORK alone.
    let d = "(define (domain trbite)
      (:requirements :strips :durative-actions :constraints)
      (:predicates (idle) (alarm) (handled) (done))
      (:durative-action work
        :parameters ()
        :duration (= ?duration 2)
        :condition (at start (idle))
        :effect (and (at start (not (idle))) (at start (alarm)) (at end (done))))
      (:durative-action respond
        :parameters ()
        :duration (= ?duration 1)
        :condition (at start (alarm))
        :effect (at end (handled))))";
    let p = "(define (problem pr) (:domain trbite)
      (:init (idle)) (:goal (done))
      (:constraints (always-within 2 (alarm) (handled))))";
    let plan = solve_green(d, p);
    let s = steps(&plan);
    assert!(
        s.iter().any(|x| x == "RESPOND"),
        "always-within must bite — the responder is forced: {s:?}"
    );
    // no-constraint twin: the responder is NOT in the natural plan.
    let p0 = "(define (problem pr0) (:domain trbite)
      (:init (idle)) (:goal (done)))";
    let sol = solve1(d, p0);
    let s0 = steps(&sol.plan.expect("plain twin solves"));
    assert!(!s0.iter().any(|x| x == "RESPOND"), "twin: {s0:?}");
}

#[test]
fn always_within_red_when_the_response_cannot_land() {
    // The only responder takes 5 > the 2-unit window: any plan through the
    // alarm is doomed, and the alarm is the only route to the goal. The
    // responder consumes a one-shot token (same finite-exhaustion rule as
    // the within RED fixture above).
    let d = "(define (domain trlate)
      (:requirements :strips :durative-actions :constraints)
      (:predicates (idle) (ready) (alarm) (handled) (done))
      (:durative-action work
        :parameters ()
        :duration (= ?duration 2)
        :condition (at start (idle))
        :effect (and (at start (not (idle))) (at start (alarm)) (at end (done))))
      (:durative-action respond
        :parameters ()
        :duration (= ?duration 5)
        :condition (at start (and (alarm) (ready)))
        :effect (and (at start (not (ready))) (at end (handled)))))";
    let p = "(define (problem prl) (:domain trlate)
      (:init (idle) (ready)) (:goal (done))
      (:constraints (always-within 2 (alarm) (handled))))";
    unsolvable(d, p);
}

#[test]
fn always_within_no_bite_when_never_triggered() {
    // QUIET-WORK reaches the goal without ever tripping the alarm.
    let p = "(define (problem prn) (:domain tresp)
      (:init (idle)) (:goal (done))
      (:constraints (always-within 2 (alarm) (handled))))";
    let plan = solve_green(RESPOND_DOM, p);
    let s = steps(&plan);
    assert!(
        !s.iter().any(|x| x == "RESPOND"),
        "no trigger, no response owed: {s:?}"
    );
}

/// The ε-boundary pin (the 0.23 emission-audit idiom, now with a clock):
/// two intervals end on the SAME ε-slot as the deadline. ε-separation
/// chains same-slot happenings ε apart, so whichever achiever is emitted
/// second lands at t+ε — PAST a deadline of exactly t. The monitor audit
/// replays the EMITTED schedule with emitted-time clock stamps and refuses
/// the pushed ordering; the search's arrival-ordered agenda holds the
/// alternative as a distinct state, and the shipped plan holds (ga) at
/// exactly t=2. `validate`'s timed fold (emitted times) is the referee.
#[test]
fn within_deadline_on_an_epsilon_chain_boundary_is_pinned() {
    let d = "(define (domain tedge)
      (:requirements :strips :durative-actions :constraints)
      (:predicates (idle) (ga) (gb))
      (:durative-action worka
        :parameters ()
        :duration (= ?duration 2)
        :condition (at start (idle))
        :effect (at end (ga)))
      (:durative-action workb
        :parameters ()
        :duration (= ?duration 2)
        :condition (at start (idle))
        :effect (at end (gb))))";
    let p = "(define (problem pe2) (:domain tedge)
      (:init (idle)) (:goal (and (ga) (gb)))
      (:constraints (within 2 (ga))))";
    let plan = solve_green(d, p);
    assert_eq!(plan.steps.len(), 2, "both intervals: {:?}", steps(&plan));
}

// ---- the timed fold is the oracle's half (temporal::validate) --------------

#[test]
fn validate_rejects_a_late_within_plan() {
    // Hand-built plan: flag lands at t=8, deadline 3 — the fold must say no,
    // independent of the compiled monitors.
    let d = "(define (domain twlate)
      (:requirements :strips :durative-actions :constraints)
      (:predicates (idle) (done) (flag))
      (:durative-action work
        :parameters ()
        :duration (= ?duration 2)
        :condition (at start (idle))
        :effect (and (at start (not (idle))) (at end (done))))
      (:durative-action slow-flag
        :parameters ()
        :duration (= ?duration 6)
        :condition (at start (done))
        :effect (at end (flag))))";
    let p = "(define (problem pvl) (:domain twlate)
      (:init (idle)) (:goal (and (done) (flag)))
      (:constraints (within 3 (flag))))";
    let dom = ferroplan::parser::parse_domain(d).expect("domain");
    let prb = ferroplan::parser::parse_problem(p).expect("problem");
    let plan = TimedPlan {
        steps: vec![
            TimedStep {
                time: 0.0,
                action: "WORK".into(),
                duration: Some(2.0),
            },
            TimedStep {
                time: 2.0,
                action: "SLOW-FLAG".into(),
                duration: Some(6.0),
            },
        ],
        makespan: 8.0,
    };
    let e = ferroplan::temporal::validate(&dom, &prb, &plan).expect_err("late flag must fold red");
    // The FOLD verdict, not a rejection: the oracle judged the trajectory.
    assert!(
        e.contains("within") && e.contains("violated"),
        "must be the fold's verdict, naming the operator: {e}"
    );
}

#[test]
fn validate_rejects_a_late_response_plan() {
    // Alarm at t=0, handler at t=5, window 2: executable, goal-reaching,
    // and constraint-red — only the timed fold can catch it.
    let d = "(define (domain trlate)
      (:requirements :strips :durative-actions :constraints)
      (:predicates (idle) (alarm) (handled) (done))
      (:durative-action work
        :parameters ()
        :duration (= ?duration 2)
        :condition (at start (idle))
        :effect (and (at start (not (idle))) (at start (alarm)) (at end (done))))
      (:durative-action respond
        :parameters ()
        :duration (= ?duration 5)
        :condition (at start (alarm))
        :effect (at end (handled))))";
    let p = "(define (problem pvr) (:domain trlate)
      (:init (idle)) (:goal (done))
      (:constraints (always-within 2 (alarm) (handled))))";
    let dom = ferroplan::parser::parse_domain(d).expect("domain");
    let prb = ferroplan::parser::parse_problem(p).expect("problem");
    let plan = TimedPlan {
        steps: vec![
            TimedStep {
                time: 0.0,
                action: "WORK".into(),
                duration: Some(2.0),
            },
            TimedStep {
                time: 0.001,
                action: "RESPOND".into(),
                duration: Some(5.0),
            },
        ],
        makespan: 5.001,
    };
    let e =
        ferroplan::temporal::validate(&dom, &prb, &plan).expect_err("late response must fold red");
    // The FOLD verdict, not a rejection: the oracle judged the trajectory.
    assert!(
        e.contains("always-within") && e.contains("violated"),
        "must be the fold's verdict, naming the operator: {e}"
    );
}

// ---- what is still rejected, by name ---------------------------------------

#[test]
fn hold_operators_stay_rejected_by_name() {
    // hold-during / hold-after: grepped absent from the whole 2006 corpus
    // (docs/roadmap-0.23.md, the stage-c sizing memo) — named rejection at
    // zero board cost, exactly as `within` was before this phase.
    for (block, op) in [
        ("(hold-during 1 2 (flag))", "hold-during"),
        ("(hold-after 1 (flag))", "hold-after"),
    ] {
        let p = atend_prob(block);
        match solve(ATEND_DOM, &p, &Options::default()) {
            Err(SolveError::Unsupported(msg)) => {
                assert!(msg.contains(op), "must name the operator: {msg}");
            }
            Err(e) => panic!("expected an Unsupported rejection, got {e:?}"),
            Ok(_) => panic!("expected a named rejection, got a solution"),
        }
    }
}

#[test]
fn within_on_a_classical_domain_stays_rejected_by_name() {
    // The clock is the SEARCH's decision-epoch time — a sequential task has
    // no such clock, so the classical path keeps the named rejection.
    let d = "(define (domain cwin)
      (:requirements :strips :constraints)
      (:predicates (a) (b))
      (:action step :parameters () :precondition (a) :effect (b)))";
    let p = "(define (problem pc) (:domain cwin)
      (:init (a)) (:goal (b))
      (:constraints (within 5 (b))))";
    match solve(d, p, &Options::default()) {
        Err(SolveError::Unsupported(msg)) => {
            assert!(msg.contains("within"), "must name the operator: {msg}");
        }
        Err(e) => panic!("expected an Unsupported rejection, got {e:?}"),
        Ok(_) => panic!("expected a named rejection, got a solution"),
    }
}

#[test]
fn preference_bodied_timed_constraints_stay_rejected() {
    // The complex-preferences unlock is 0.25's: a soft `within` still gets
    // the temporal soft fence, by name — never a silently ignored wrapper.
    let p = atend_prob("(preference deadline (within 5 (flag)))");
    match solve(ATEND_DOM, &p, &Options::default()) {
        Err(SolveError::Unsupported(msg)) => {
            assert!(
                msg.contains("preference"),
                "must name the soft fence: {msg}"
            );
        }
        Err(e) => panic!("expected an Unsupported rejection, got {e:?}"),
        Ok(_) => panic!("expected a named rejection, got a solution"),
    }
}

#[test]
fn soft_constraints_stay_rejected_on_temporal() {
    let p = atend_prob("(preference cautious (sometime (flag)))");
    match solve(ATEND_DOM, &p, &Options::default()) {
        Err(SolveError::Unsupported(msg)) => {
            assert!(
                msg.contains("preference"),
                "must name the soft fence: {msg}"
            );
        }
        Err(e) => panic!("expected an Unsupported rejection, got {e:?}"),
        Ok(_) => panic!("expected a named rejection, got a solution"),
    }
}
