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

// ---- what is still rejected, by name ---------------------------------------

#[test]
fn timed_operators_stay_rejected_by_name() {
    let p = atend_prob("(within 5 (flag))");
    match solve(ATEND_DOM, &p, &Options::default()) {
        Err(SolveError::Unsupported(msg)) => {
            assert!(msg.contains("within"), "must name the operator: {msg}");
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
