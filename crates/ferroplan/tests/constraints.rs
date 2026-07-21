//! PDDL3 trajectory constraints — ENFORCED since 0.7 (docs/roadmap-0.7.md).
//!
//! 0.4.1 pinned that `(:constraints ...)` was cleanly rejected; 0.7 narrows
//! that fence: the six untimed modal operators compile into monitor automata
//! and are ENFORCED on the classical path — hard constraints as goal
//! conjuncts (Phase 1), soft `(preference name ...)` constraints priced
//! through the PDDL3 metric machinery (Phase 2) — while the timed operators,
//! the temporal path (Phase 3), and `Session` keep NAMED rejections. Each
//! operator gets a bite/no-bite pair on a hand-checkable switch domain, and
//! every solved plan is cross-checked through the independent verifier
//! (`verify::verify` folds the ORIGINAL constraint semantics over its
//! replay — never the compiled monitors).

use std::sync::Mutex;

use ferroplan::{solve, Options, SolveError};

/// One test in this file mutates a process-global env hatch
/// (`FF_CONSTRAINTS_REJECT`); every test takes this lock so the default
/// parallel test runner cannot race it (the suite runs in milliseconds).
static ENV_LOCK: Mutex<()> = Mutex::new(());

const DOM: &str = "(define (domain sw)
  (:requirements :strips :constraints)
  (:predicates (on) (off) (lamp) (used))
  (:action flip-on :precondition (off) :effect (and (not (off)) (on)))
  (:action flip-off :precondition (on) :effect (and (not (on)) (off)))
  (:action light :precondition (on) :effect (and (lamp) (used))))";

fn prob(init: &str, goal: &str, constraints: &str) -> String {
    format!(
        "(define (problem sw-1) (:domain sw) (:init {init}) (:goal {goal})
         (:constraints {constraints}))"
    )
}

fn steps(plan: &ferroplan::Plan) -> Vec<(String, Vec<String>)> {
    plan.steps
        .iter()
        .map(|s| (s.action.clone(), s.args.clone()))
        .collect()
}

/// Solve at 1 and 8 threads, assert identical plans, verify constraints via
/// the independent oracle, and return the t1 plan.
fn solve_ok(d: &str, p: &str) -> ferroplan::Plan {
    let t1 = solve(
        d,
        p,
        &Options {
            threads: 1,
            ..Options::default()
        },
    )
    .expect("solve t1");
    let t8 = solve(
        d,
        p,
        &Options {
            threads: 8,
            ..Options::default()
        },
    )
    .expect("solve t8");
    let plan1 = t1.plan.expect("plan t1");
    let plan8 = t8.plan.expect("plan t8");
    assert_eq!(steps(&plan1), steps(&plan8), "plan differs across threads");
    let v = ferroplan::verify::verify(d, p, &steps(&plan1)).expect("verify");
    assert!(v.hard_goal_met, "verifier: hard goal");
    assert!(
        v.constraints_met,
        "verifier: trajectory constraints violated: {:?}",
        v.constraint_failures
    );
    plan1
}

fn unsolvable(d: &str, p: &str) {
    let sol = solve(d, p, &Options::default()).expect("solve runs");
    assert!(
        sol.plan.is_none(),
        "expected unsolvable, got a plan: {:?}",
        sol.plan.map(|pl| steps(&pl))
    );
}

// ---- always -------------------------------------------------------------

#[test]
fn always_blocks_the_forbidden_route() {
    let _g = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    // goal needs (lamp); lighting needs (on); but (always (off)) forbids ever
    // leaving (off) — unsolvable.
    let p = prob("(off)", "(lamp)", "(always (off))");
    unsolvable(DOM, &p);
}

#[test]
fn always_no_bite_when_route_complies() {
    let _g = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    // (always (or (on) (off))) is an invariant every state satisfies.
    let p = prob("(off)", "(lamp)", "(always (or (on) (off)))");
    let plan = solve_ok(DOM, &p);
    assert!(!plan.steps.is_empty());
}

// ---- sometime -----------------------------------------------------------

#[test]
fn sometime_forces_a_detour() {
    let _g = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    // goal already true at init, but (sometime (on)) forces flipping on —
    // the empty plan no longer qualifies.
    let p = prob("(off)", "(off)", "(sometime (on))");
    let plan = solve_ok(DOM, &p);
    assert!(
        plan.steps.len() >= 2,
        "must flip on and back off, got {:?}",
        steps(&plan)
    );
}

#[test]
fn sometime_no_bite_when_already_on_route() {
    let _g = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let p = prob("(off)", "(lamp)", "(sometime (on))");
    solve_ok(DOM, &p); // lighting requires (on) anyway
}

// ---- at-most-once -------------------------------------------------------

#[test]
fn at_most_once_blocks_a_second_episode() {
    let _g = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    // Reach (used) then return to (off), under at-most-once (on) — fine:
    // one episode. But demanding (on) at the end AFTER an off-interlude
    // (forced via sometime (lamp) needing on, then off, then on again for
    // the end) is a second episode — unsolvable.
    let good = prob("(off)", "(and (used) (off))", "(at-most-once (on))");
    solve_ok(DOM, &good);

    // (used) forces an (on) episode; ending (off) closes it; a SECOND
    // (sometime (on)) after... encode directly: end state (on) after the
    // episode closed — the goal needs (used) and (on), and (sometime (off))
    // demands an off-state in between whenever used happened while on.
    let bad = prob(
        "(off)",
        "(and (used) (on))",
        "(and (at-most-once (on)) (sometime (and (used) (off))))",
    );
    unsolvable(DOM, &bad);
}

// ---- sometime-after -----------------------------------------------------

#[test]
fn sometime_after_forces_the_response() {
    let _g = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    // Any (lamp) must eventually be followed by (off): the goal (and (lamp)
    // (on)) violates it (lamp set, never off after), while ending (off)
    // satisfies it.
    let good = prob(
        "(off)",
        "(and (lamp) (off))",
        "(sometime-after (lamp) (off))",
    );
    solve_ok(DOM, &good);
    let bad = prob(
        "(off)",
        "(and (lamp) (on))",
        "(sometime-after (lamp) (off))",
    );
    unsolvable(DOM, &bad);
}

// ---- sometime-before ----------------------------------------------------

#[test]
fn sometime_before_orders_the_trajectory() {
    let _g = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    // (lamp) may only appear if (on) held STRICTLY earlier — always true
    // here (light requires on in the source state), so no bite:
    let good = prob("(off)", "(lamp)", "(sometime-before (lamp) (on))");
    solve_ok(DOM, &good);
    // but demanding (used) strictly-before (on): the first on-state comes
    // before any used-state can exist (light needs on) — unsolvable.
    let bad = prob("(off)", "(lamp)", "(sometime-before (on) (used))");
    unsolvable(DOM, &bad);
}

// ---- at end ---------------------------------------------------------------

#[test]
fn at_end_is_a_goal_conjunct() {
    let _g = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let p = prob("(off)", "(lamp)", "(at end (off))");
    let plan = solve_ok(DOM, &p);
    // must light (needs on) and then return to off
    assert!(plan.steps.len() >= 3, "got {:?}", steps(&plan));
}

// ---- init-state (S_0) coverage -------------------------------------------

#[test]
fn s0_counts_for_the_trajectory() {
    let _g = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    // (sometime (off)) is satisfied by S_0 alone — the empty plan works.
    let p = prob("(off)", "(off)", "(sometime (off))");
    let plan = solve_ok(DOM, &p);
    assert!(
        plan.steps.is_empty(),
        "S_0 satisfies it: {:?}",
        steps(&plan)
    );
    // (sometime-before (off) (on)): φ=(off) true at S_0 with nothing
    // strictly earlier — violated immediately, unsolvable.
    let bad = prob("(off)", "(off)", "(sometime-before (off) (on))");
    unsolvable(DOM, &bad);
}

// ---- static simplification (fast pins; the heavy sentinel lives in --------
// ---- tests/ipc5_qual_metric.rs::storage_p03_survives_the_quadratic_forall)

/// Switch domain plus a STATIC predicate (`linked` — no effect touches it),
/// so `peval_static` has something to decide.
const DOMS: &str = "(define (domain sws)
  (:requirements :strips :constraints)
  (:predicates (on) (off) (lamp) (used) (linked))
  (:action flip-on :precondition (off) :effect (and (not (off)) (on)))
  (:action flip-off :precondition (on) :effect (and (not (on)) (off)))
  (:action light :precondition (on) :effect (and (lamp) (used))))";

#[test]
fn statically_accepted_constraints_drop_and_solve() {
    let _g = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    // Every drop rule fires once here (always/sometime/at-most-once/at-end
    // body → true; sometime-after ψ → true; sometime-before φ → false): the
    // whole block compiles away and the plain 2-step plan stands, with the
    // verifier's UNSIMPLIFIED folds agreeing everything held.
    let p = "(define (problem s) (:domain sws) (:init (off) (linked)) (:goal (lamp))
        (:constraints (and (always (linked))
                           (sometime (linked))
                           (at-most-once (linked))
                           (sometime-after (used) (linked))
                           (sometime-before (not (linked)) (used))
                           (at end (linked)))))";
    let plan = solve_ok(DOMS, p);
    assert_eq!(plan.steps.len(), 2);
}

#[test]
fn statically_violated_constraints_still_bite() {
    let _g = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    // The must-KEEP side of the drop rules. sometime-before with φ
    // static-TRUE is violated at S_0 (nothing is strictly earlier):
    let p1 = "(define (problem s) (:domain sws) (:init (off) (linked)) (:goal (lamp))
        (:constraints (sometime-before (linked) (used))))";
    unsolvable(DOMS, p1);
    // always with a statically-FALSE body:
    let p2 = "(define (problem s) (:domain sws) (:init (off)) (:goal (lamp))
        (:constraints (always (linked))))";
    unsolvable(DOMS, p2);
}

#[test]
fn statically_satisfied_soft_prefs_still_count() {
    let _g = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    // pa's members all drop → it lowers to `(preference pa true)`: never
    // violated, still a COUNTED instance (the CLI footer reports both).
    let p = "(define (problem s) (:domain sws) (:init (off) (linked)) (:goal (off))
        (:constraints (and (preference pa (always (linked)))
                           (preference pb (sometime (on))))))";
    let (out, _) = ferroplan::run_planner(DOMS, p, &Options::default(), false);
    assert!(
        out.contains("2 preferences"),
        "both instances counted:\n{out}"
    );
    assert!(
        out.contains("metric value 0"),
        "pb satisfied en route:\n{out}"
    );
}

#[test]
fn reserved_monitor_names_reject_by_name() {
    let _g = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    // A user predicate inside the generated monitor namespace could silently
    // clear a hard violation — rejected by name instead.
    let dom_bad = "(define (domain sw9)
      (:requirements :strips :constraints)
      (:predicates (off) (lamp) (traj0-viol))
      (:action nop :precondition (off) :effect (lamp)))";
    let p = "(define (problem s) (:domain sw9) (:init (off)) (:goal (lamp))
        (:constraints (always (off))))";
    match solve(dom_bad, p, &Options::default()).expect_err("must reject") {
        SolveError::Unsupported(msg) => {
            assert!(msg.contains("TRAJ0-VIOL"), "names the predicate: {msg}")
        }
        other => panic!("expected Unsupported, got {other:?}"),
    }
    // A user preference named inside the TRAJPREF{n} namespace aliases
    // anonymous constraint-preference pricing — same rejection.
    let p2 = prob("(off)", "(lamp)", "(preference trajpref0 (sometime (on)))");
    match solve(DOM, &p2, &Options::default()).expect_err("must reject") {
        SolveError::Unsupported(msg) => {
            assert!(msg.contains("TRAJPREF0"), "names the preference: {msg}")
        }
        other => panic!("expected Unsupported, got {other:?}"),
    }
}

// ---- named rejections (the fence that remains) ----------------------------

#[test]
fn timed_operators_reject_by_name() {
    let _g = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let p = prob("(off)", "(on)", "(within 5 (on))");
    let err = solve(DOM, &p, &Options::default()).expect_err("must reject");
    match err {
        SolveError::Unsupported(msg) => {
            assert!(msg.contains("within"), "message names the operator: {msg}")
        }
        other => panic!("expected Unsupported, got {other:?}"),
    }
}

// ---- soft constraint-preferences (Phase 2) --------------------------------

/// Solve a preference problem at 1 and 8 threads, assert the REPORTED metric
/// matches `want` at both, and that the independent verifier's trajectory
/// replay computes the same metric (reported == verified, the 0.7 contract).
fn soft_ok(d: &str, p: &str, want: f64) -> ferroplan::Plan {
    let t1 = solve(
        d,
        p,
        &Options {
            threads: 1,
            ..Options::default()
        },
    )
    .expect("solve t1");
    let t8 = solve(
        d,
        p,
        &Options {
            threads: 8,
            ..Options::default()
        },
    )
    .expect("solve t8");
    let plan1 = t1.plan.expect("plan t1");
    let plan8 = t8.plan.expect("plan t8");
    assert_eq!(plan1.metric, Some(want), "t1 reported metric");
    assert_eq!(plan8.metric, Some(want), "t8 reported metric");
    let v = ferroplan::verify::verify(d, p, &steps(&plan1)).expect("verify");
    assert!(v.hard_goal_met, "verifier: hard goal");
    assert!(
        v.constraints_met,
        "verifier: hard constraints violated: {:?}",
        v.constraint_failures
    );
    assert_eq!(v.metric, want, "verified metric");
    plan1
}

#[test]
fn soft_sometime_default_weight_forces_the_detour() {
    let _g = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    // No :metric → every preference weighs 1. Satisfying (sometime (on))
    // costs two flips but saves the violation — optimal metric 0.
    let p = prob("(off)", "(off)", "(preference pv (sometime (on)))");
    let plan = soft_ok(DOM, &p, 0.0);
    assert!(
        plan.steps.len() >= 2,
        "must flip on and back: {:?}",
        steps(&plan)
    );
}

#[test]
fn soft_always_pays_when_the_goal_demands_violation() {
    let _g = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    // (lamp) requires leaving (off): the preference is unavoidably violated
    // and the metric prices it via (is-violated pv).
    let p = "(define (problem sw-1) (:domain sw) (:init (off)) (:goal (lamp))
         (:constraints (preference pv (always (off))))
         (:metric minimize (* 3 (is-violated pv))))";
    soft_ok(DOM, p, 3.0);
}

#[test]
fn soft_metric_unreferenced_weighs_zero() {
    let _g = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    // A metric that never mentions pv → violating pv is free (weight 0),
    // matching the goal-preference default semantics.
    let p = "(define (problem sw-1) (:domain sw) (:init (off)) (:goal (lamp))
         (:constraints (preference pv (always (off))))
         (:metric minimize (is-violated ghost)))";
    soft_ok(DOM, p, 0.0);
}

#[test]
fn anonymous_constraint_pref_defaults_like_named() {
    let _g = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let p = prob("(off)", "(lamp)", "(preference (always (off)))");
    soft_ok(DOM, &p, 1.0);
}

#[test]
fn mixed_hard_and_soft_split_correctly() {
    let _g = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    // One (:constraints ...) block, both kinds: hard (sometime (used))
    // forces lighting; the soft (always (off)) is thereby violated — the
    // hard one is ENFORCED (verifier constraints_met), the soft one PRICED.
    let p = prob(
        "(off)",
        "(off)",
        "(and (sometime (used)) (preference pv (always (off))))",
    );
    let plan = soft_ok(DOM, &p, 1.0);
    assert!(plan.steps.len() >= 3, "on, light, off: {:?}", steps(&plan));
}

#[test]
fn pref_with_and_body_counts_once() {
    let _g = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    // `(preference pv (and C1 C2))` is ONE preference (the PDDL3 instance
    // boundary): both members fail on every goal-reaching trajectory, but
    // (is-violated pv) contributes its weight AT MOST ONCE — metric 3, not 6.
    let p = "(define (problem sw-1) (:domain sw) (:init (off)) (:goal (lamp))
         (:constraints (preference pv (and (always (off))
                                           (sometime-before (lamp) (used)))))
         (:metric minimize (* 3 (is-violated pv))))";
    soft_ok(DOM, p, 3.0);
}

#[test]
fn pref_and_body_violated_by_a_single_member() {
    let _g = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    // Only the always-member is violated (the sometime-member is satisfied
    // en route); any violated member violates the ONE instance exactly once.
    let p = "(define (problem sw-1) (:domain sw) (:init (off)) (:goal (lamp))
         (:constraints (preference pv (and (always (off)) (sometime (on)))))
         (:metric minimize (* 5 (is-violated pv))))";
    soft_ok(DOM, p, 5.0);
}

#[test]
fn forall_inside_pref_is_one_instance() {
    let _g = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    // Contrast with `forall_pref_instances_share_one_name`: there the forall
    // is OUTSIDE the preference (→ one instance per binding); here it is
    // INSIDE the body (→ ONE instance whose members are the bindings). With
    // no switch flippable, BOTH members are unsatisfiable — a per-member
    // split would price 2 × 2 = 4; the correct single instance prices 2.
    let dom = "(define (domain sw2)
      (:requirements :strips :typing :constraints)
      (:types sw)
      (:predicates (on ?s - sw) (flippable ?s - sw) (base))
      (:action flip :parameters (?s - sw)
        :precondition (flippable ?s) :effect (on ?s)))";
    let p = "(define (problem sw2-1) (:domain sw2)
      (:objects s1 s2 - sw)
      (:init (base))
      (:goal (base))
      (:constraints (preference pv (forall (?s - sw) (sometime (on ?s)))))
      (:metric minimize (* 2 (is-violated pv))))";
    soft_ok(dom, p, 2.0);
}

#[test]
fn forall_pref_instances_share_one_name() {
    let _g = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    // (forall ...) around a named preference expands to one INSTANCE per
    // binding, all sharing the name — (is-violated pv) counts violated
    // instances. Only s1 is flippable, so exactly one instance is violated:
    // metric = 1 instance × weight 2.
    let dom = "(define (domain sw2)
      (:requirements :strips :typing :constraints)
      (:types sw)
      (:predicates (on ?s - sw) (flippable ?s - sw))
      (:action flip :parameters (?s - sw)
        :precondition (flippable ?s) :effect (on ?s)))";
    let p = "(define (problem sw2-1) (:domain sw2)
      (:objects s1 s2 - sw)
      (:init (flippable s1))
      (:goal (flippable s1))
      (:constraints (forall (?s - sw) (preference pv (sometime (on ?s)))))
      (:metric minimize (* 2 (is-violated pv))))";
    soft_ok(dom, p, 2.0);
}

#[test]
fn hatch_restores_the_blanket_rejection() {
    let _g = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    std::env::set_var("FF_CONSTRAINTS_REJECT", "1");
    let p = prob("(off)", "(lamp)", "(always (or (on) (off)))");
    let r = solve(DOM, &p, &Options::default());
    let ps = prob("(off)", "(lamp)", "(preference pv (sometime (on)))");
    let rs = solve(DOM, &ps, &Options::default());
    std::env::remove_var("FF_CONSTRAINTS_REJECT");
    assert!(
        matches!(r, Err(SolveError::Unsupported(_))),
        "hatch must restore rejection (hard)"
    );
    assert!(
        matches!(rs, Err(SolveError::Unsupported(_))),
        "hatch must restore rejection (soft)"
    );
}

#[test]
fn cli_text_path_enforces_too() {
    let _g = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    // run_planner must enforce (same gate): the always-bite case reports
    // proven-unsolvable rather than emitting a violating plan. (Exit code is
    // 0 — Metric-FF convention: a *proof* of unsolvability is a successful
    // run, unlike a parse/reject error.)
    let p = prob("(off)", "(lamp)", "(always (off))");
    let (out, code) = ferroplan::run_planner(DOM, &p, &Options::default(), false);
    assert_eq!(code, 0, "proven-unsolvable is a clean exit:\n{out}");
    assert!(
        !out.contains("found legal plan"),
        "must not emit a violating plan:\n{out}"
    );
    assert!(
        out.contains("proven unsolvable"),
        "must report the proof:\n{out}"
    );
}

#[test]
fn decompose_gate_enforces_and_rejects_alike() {
    let _g = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    // Same gate as solve: an enforceable constraint set passes through the
    // decomposer entrypoint (non-temporal → falls back to one contract) …
    let ok = prob("(off)", "(lamp)", "(always (or (on) (off)))");
    let d = ferroplan::decompose(DOM, &ok, &Options::default()).expect("decompose runs");
    assert!(d.solved, "compliant route must still solve via decompose");
    // … and a timed operator is rejected by name, not dropped.
    let timed = prob("(off)", "(on)", "(within 5 (on))");
    match ferroplan::decompose(DOM, &timed, &Options::default()) {
        Err(SolveError::Unsupported(msg)) => {
            assert!(msg.contains("within"), "names the operator: {msg}")
        }
        other => panic!("expected Unsupported, got {other:?}"),
    }
}

#[test]
fn run_ff_gate_enforces_too() {
    let _g = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    // The plain-FF text path compiles the same monitors: the always-bite
    // case must prove unsolvable, never emit a violating plan.
    let p = prob("(off)", "(lamp)", "(always (off))");
    let (out, _code) = ferroplan::run_ff(DOM, &p, &Options::default());
    assert!(
        !out.contains("found legal plan"),
        "must not emit a violating plan:\n{out}"
    );
    // and the compliant case still plans
    let ok = prob("(off)", "(lamp)", "(always (or (on) (off)))");
    let (out, code) = ferroplan::run_ff(DOM, &ok, &Options::default());
    assert_eq!(code, 0, "compliant route solves:\n{out}");
    assert!(out.contains("found legal plan"), "plans:\n{out}");
}

#[test]
fn session_keeps_a_named_rejection() {
    let _g = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    // Session grounds once and replans from mutated states — a compiled
    // monitor's S_0 baking would go stale, so it rejects by name for now.
    let p = prob("(off)", "(lamp)", "(always (or (on) (off)))");
    let err = ferroplan::Session::new(DOM, &p, &Options::default())
        .err()
        .expect("Session must reject constraint problems");
    assert!(
        err.contains("trajectory constraints"),
        "message names the feature: {err}"
    );
}

#[test]
fn validate_plan_rejects_a_constraint_violating_plan() {
    let _g = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    // A plan that reaches the goal but breaks (always (off)) must be
    // Invalid — hard_goal_met alone no longer grants Valid.
    let p = prob("(off)", "(lamp)", "(always (off))");
    let plan_src = "0: (FLIP-ON)\n1: (LIGHT)\n";
    let v = ferroplan::plan::validate_plan(DOM, &p, plan_src).expect("validator runs");
    match v {
        ferroplan::plan::Validity::Invalid(why) => assert!(
            why.contains("always"),
            "reason names the violated operator: {why}"
        ),
        other => panic!("violating plan must be Invalid, got {other:?}"),
    }
}

#[test]
fn constraint_free_input_is_untouched() {
    let _g = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    // No (:constraints ...) block: the gate is a no-op and behavior is the
    // 0.6 one, bit for bit (pinned indirectly by the whole existing suite;
    // this is the direct smoke).
    let p = "(define (problem sw-1) (:domain sw) (:init (off)) (:goal (lamp)))";
    let plan = solve_ok(DOM, p);
    assert_eq!(plan.steps.len(), 2);
}

// ---- the 0.8 END construction (docs/roadmap-0.8.md Phase 1) -------------
//
// Hard-monitor S_n acceptance rides a forced-terminal synthetic TRAJ-END
// action (conditional ACC latches; literal-only goal) instead of goal-side
// disjunctions that DNF-multiply into REACH-GOAL ops. These tests pin the
// three observable contracts: the synthetic step never reaches a reported
// surface, the compiled-goal shape is linear (grounding-level), and the
// FF_NO_TRAJ_END hatch restores the 0.7 goal-side shape byte-for-byte.

/// Gate + ground a (domain, problem) source pair, returning
/// (op displays, REACH-GOAL op count). Uses the same public pipeline the
/// entrypoints use, so the shape it sees is the shape search sees.
fn gate_and_ground(d: &str, p: &str) -> (Vec<String>, usize) {
    let dom = ferroplan::parser::parse_domain(d).expect("domain");
    let prob = ferroplan::parser::parse_problem(p).expect("problem");
    let (dom, prob) = ferroplan::derived::compile(&dom, &prob).expect("derived");
    let (dom, prob) = ferroplan::constraints::gate(&dom, &prob)
        .expect("gate")
        .expect("constraints present");
    let task = ferroplan::ground::ground_task(&dom, &prob, 1).expect("ground");
    let reach = task
        .op_display
        .iter()
        .filter(|d| d.starts_with("REACH-GOAL"))
        .count();
    (task.op_display.to_vec(), reach)
}

#[test]
fn end_action_never_reaches_reported_plans() {
    let _g = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    // A bite case that forces a real detour: the compiled task plans
    // through TRAJ-END, but no reported surface may show it. solve_ok
    // doubles as the leak oracle — the verifier replays against the
    // ORIGINAL problem, where TRAJ-END is not a grounded op, so any leak
    // is an immediate verify error.
    let p = prob("(off)", "(off)", "(sometime (on))");
    let plan = solve_ok(DOM, &p);
    assert!(
        plan.steps.iter().all(|s| s.action != "TRAJ-END"),
        "synthetic TRAJ-END step leaked into the API plan: {:?}",
        steps(&plan)
    );
    // The grounded task DOES carry it (that is the construction)…
    let (ops, reach) = gate_and_ground(DOM, &p);
    assert!(
        ops.iter().any(|d| d == "TRAJ-END"),
        "compiled task must carry the END action"
    );
    // …with a literal goal: zero REACH-GOAL disjunct ops.
    assert_eq!(
        reach, 0,
        "goal must be literal-only under the END construction"
    );
    // CLI text path: no TRAJ-END line either.
    let (out, code) = ferroplan::run_planner(DOM, &p, &Options::default(), false);
    assert_eq!(code, 0, "cli solves:\n{out}");
    assert!(
        out.contains("found legal plan"),
        "cli emits the plan:\n{out}"
    );
    assert!(
        !out.contains("TRAJ-END"),
        "synthetic TRAJ-END step leaked into CLI text:\n{out}"
    );
}

#[test]
fn init_true_goal_with_constraints_reports_the_empty_plan() {
    let _g = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    // Goal already true at S_0 and the constraint holds on the empty
    // trajectory: the COMPILED task needs exactly one step (TRAJ-END), the
    // REPORTED plan is empty — and the verifier accepts the empty replay.
    let p = prob("(off)", "(off)", "(always (off))");
    let plan = solve_ok(DOM, &p);
    assert_eq!(
        plan.steps.len(),
        0,
        "reported plan must be empty once TRAJ-END is stripped: {:?}",
        steps(&plan)
    );
}

#[test]
fn end_hatch_restores_goal_side_acceptance() {
    let _g = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    // FF_NO_TRAJ_END=1 restores the 0.7 shape: acceptance conjoined into
    // the goal, whose disjunctive members (sometime: SEEN ∨ φ) DNF-expand
    // into REACH-GOAL ops — and no TRAJ-END op exists.
    let p = prob("(off)", "(off)", "(sometime (on))");
    std::env::set_var("FF_NO_TRAJ_END", "1");
    let (ops, reach) = gate_and_ground(DOM, &p);
    let hatch_solve = solve_ok(DOM, &p);
    std::env::remove_var("FF_NO_TRAJ_END");
    assert!(
        ops.iter().all(|d| d != "TRAJ-END"),
        "hatch must not emit the END action"
    );
    assert!(
        reach > 0,
        "hatch restores the goal-side disjunction (REACH-GOAL ops)"
    );
    // Same semantics either way: the detour is still forced and verified.
    let default_solve = solve_ok(DOM, &p);
    assert_eq!(
        steps(&hatch_solve),
        steps(&default_solve),
        "hatch and default must agree on this plan"
    );
}

#[test]
fn end_action_name_is_fenced_when_constraints_present() {
    let _g = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    // A user action named TRAJ-END would be filtered from reported plans by
    // the strip — rejected by name, like the monitor-fact namespace. (Only
    // when a (:constraints ...) block exists: the constraint-free path
    // never strips, so plain domains keep any name.)
    let d = "(define (domain sw2)
      (:requirements :strips :constraints)
      (:predicates (on) (off))
      (:action TRAJ-END :precondition (off) :effect (on)))";
    let p = "(define (problem sw2-1) (:domain sw2) (:init (off)) (:goal (on))
             (:constraints (always (or (on) (off)))))";
    match solve(d, p, &Options::default()) {
        Err(SolveError::Unsupported(msg)) => {
            assert!(msg.contains("TRAJ-END"), "names the collision: {msg}")
        }
        other => panic!("expected Unsupported, got {other:?}"),
    }
}
