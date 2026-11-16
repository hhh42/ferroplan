//! An action that both deletes and adds a fact leaves it TRUE (PDDL: deletes
//! first, then adds), and a negative precondition on that fact must read it
//! so. The negative-precondition compilation keeps a complementary fact
//! `(NOT (p))`; until the 0.31 cut it mirrored Metric-FF's blind toggles, so
//! such an action left `p` AND `(NOT (p))` true and a plan could walk through
//! the contradiction -- tidybot's `base-cart-*` on `base-obstacle`, where
//! 0.31's novelty rungs returned a 58-step plan VAL rejects (cut31,
//! ipc7-mco t4 / t8, instance 12).
//!
//! The fixture: `mark` deletes and adds `p`; `fin` needs `(not (p))`. The
//! only valid plan is mark, clear, fin. Four shapes of `mark`: both effects
//! unconditional, and the three ways a conditional effect can be one side.

use ferroplan::ground::ground_task;
use ferroplan::packed::PackedTask;

fn domain(mark_effect: &str) -> String {
    format!(
        "(define (domain addel)
 (:requirements :adl :negative-preconditions)
 (:predicates (p) (q) (done) (cleared) (g))
 (:action mark :parameters () :precondition (not (p)) :effect {mark_effect})
 (:action clear :parameters () :precondition (p) :effect (and (not (p)) (cleared)))
 (:action fin :parameters () :precondition (and (done) (not (p))) :effect (g)))"
    )
}

const UNCOND: &str = "(and (not (p)) (p) (done))";
const DEL_THEN_WHEN_ADD: &str = "(and (not (p)) (when (q) (p)) (done))";
const ADD_THEN_WHEN_DEL: &str = "(and (p) (when (q) (not (p))) (done))";
const WHEN_BOTH: &str = "(and (when (q) (not (p))) (when (q) (p)) (done))";

fn problem(q: bool) -> String {
    format!(
        "(define (problem a) (:domain addel) (:init {}) (:goal (g)))",
        if q { "(q)" } else { "" }
    )
}

fn task(dom: &str, prb: &str) -> PackedTask {
    let d = ferroplan::parser::parse_domain(dom).unwrap();
    let p = ferroplan::parser::parse_problem(prb).unwrap();
    ground_task(&d, &p, 1).unwrap()
}

fn holds(t: &PackedTask, s: &ferroplan::packed::State, name: &str) -> bool {
    // A fact the grounder pruned as unreachable is false in every state.
    t.fact_id(name)
        .is_some_and(|id| s.bits[id / 64] >> (id % 64) & 1 == 1)
}

fn op(t: &PackedTask, name: &str) -> usize {
    (0..t.n_ops)
        .find(|&oi| t.op_display[oi] == name)
        .unwrap_or_else(|| panic!("no op {name}"))
}

/// After `mark`: is `p` true, and does the complement say the same thing?
fn after_mark(effect: &str, q: bool) -> bool {
    let t = task(&domain(effect), &problem(q));
    let s = t.apply(op(&t, "MARK"), &t.initial());
    let p = holds(&t, &s, "(P)");
    let not_p = holds(&t, &s, "(NOT (P))");
    assert_ne!(
        p, not_p,
        "{effect} (q={q}): p={p} and (NOT (P))={not_p} after MARK -- the complement disagrees"
    );
    assert_eq!(
        t.op_applicable(op(&t, "FIN"), &s),
        !p,
        "{effect} (q={q}): FIN needs (not (p)) and p is {p}"
    );
    p
}

fn plan_uses_clear(effect: &str, q: bool) -> bool {
    let (out, code) = ferroplan::run_ff(
        &domain(effect),
        &problem(q),
        &ferroplan::Options {
            mode: ferroplan::Mode::Ff,
            threads: 1,
            ..Default::default()
        },
    );
    assert_eq!(code, 0, "should solve\n{out}");
    assert!(out.contains("found legal plan as follows"), "{out}");
    out.contains("CLEAR")
}

#[test]
fn an_unconditional_delete_and_add_leaves_the_fact_true() {
    assert!(after_mark(UNCOND, false));
    assert!(
        plan_uses_clear(UNCOND, false),
        "mark, fin is not a plan: p is true after mark"
    );
}

#[test]
fn a_firing_conditional_add_wins_over_an_unconditional_delete() {
    assert!(after_mark(DEL_THEN_WHEN_ADD, true));
    assert!(plan_uses_clear(DEL_THEN_WHEN_ADD, true));
    // The condition false: only the delete fires, p stays false, and the
    // two-step plan is the plan.
    assert!(!after_mark(DEL_THEN_WHEN_ADD, false));
    assert!(!plan_uses_clear(DEL_THEN_WHEN_ADD, false));
}

#[test]
fn an_unconditional_add_wins_over_a_firing_conditional_delete() {
    assert!(after_mark(ADD_THEN_WHEN_DEL, true));
    assert!(plan_uses_clear(ADD_THEN_WHEN_DEL, true));
    assert!(after_mark(ADD_THEN_WHEN_DEL, false));
    assert!(plan_uses_clear(ADD_THEN_WHEN_DEL, false));
}

#[test]
fn two_firing_conditional_effects_resolve_add_wins() {
    assert!(after_mark(WHEN_BOTH, true));
    assert!(plan_uses_clear(WHEN_BOTH, true));
    // Neither fires: p untouched, false.
    assert!(!after_mark(WHEN_BOTH, false));
    assert!(!plan_uses_clear(WHEN_BOTH, false));
}
