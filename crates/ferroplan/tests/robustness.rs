//! A published planner must never PANIC on malformed input — it must return a
//! typed error (or a clean unsolved/Ok result). This sweep feeds adversarial
//! domains/problems through the public `solve` and asserts no panic, reporting
//! which input crashed.

use ferroplan::{solve, Options};

/// Minimal well-formed domain/problem to mutate against.
const GOOD_DOM: &str = "(define (domain d) (:requirements :strips)
  (:predicates (a) (b))
  (:action go :parameters () :precondition (a) :effect (and (not (a)) (b))))";
const GOOD_PROB: &str = "(define (problem p) (:domain d) (:init (a)) (:goal (b)))";

fn cases() -> Vec<(&'static str, &'static str, &'static str)> {
    vec![
        ("empty both", "", ""),
        ("empty domain", "", GOOD_PROB),
        ("empty problem", GOOD_DOM, ""),
        ("unbalanced parens", "(define (domain d) (:predicates (a)", GOOD_PROB),
        ("just parens", "((((((", ")))))"),
        ("garbage", "this is not pddl at all", "neither is this %%%"),
        ("only whitespace", "   \n\t  ", "  \n  "),
        (
            "domain decl no body",
            "(define (domain d))",
            "(define (problem p) (:domain d) (:init) (:goal (a)))",
        ),
        (
            "goal unknown predicate",
            GOOD_DOM,
            "(define (problem p) (:domain d) (:init (a)) (:goal (zzz)))",
        ),
        (
            "init unknown predicate",
            GOOD_DOM,
            "(define (problem p) (:domain d) (:init (zzz)) (:goal (b)))",
        ),
        (
            "action references undefined predicate",
            "(define (domain d) (:predicates (a))
              (:action go :parameters () :precondition (q) :effect (a)))",
            "(define (problem p) (:domain d) (:init) (:goal (a)))",
        ),
        (
            "param of unknown type",
            "(define (domain d) (:requirements :typing) (:predicates (at ?x - loc))
              (:action go :parameters (?x - nonsuch) :precondition () :effect (at ?x)))",
            "(define (problem p) (:domain d) (:objects o - loc) (:init) (:goal (at o)))",
        ),
        (
            "numeric divide by zero in effect",
            "(define (domain d) (:requirements :numeric-fluents) (:functions (x))
              (:action go :parameters () :precondition () :effect (increase (x) (/ 1 0))))",
            "(define (problem p) (:domain d) (:init (= (x) 0)) (:goal (>= (x) 5)))",
        ),
        (
            "metric over missing fluent",
            "(define (domain d) (:requirements :numeric-fluents) (:predicates (g))
              (:action go :parameters () :precondition () :effect (g)))",
            "(define (problem p) (:domain d) (:init) (:goal (g)) (:metric minimize (total-cost)))",
        ),
        (
            "comparison with no args",
            "(define (domain d) (:requirements :numeric-fluents) (:functions (x))
              (:action go :parameters () :precondition (>) :effect (g)))",
            "(define (problem p) (:domain d) (:init) (:goal (g)))",
        ),
        (
            "durative no duration",
            "(define (domain d) (:requirements :durative-actions) (:predicates (g))
              (:durative-action act :parameters () :condition () :effect (at end (g))))",
            "(define (problem p) (:domain d) (:init) (:goal (g)))",
        ),
        (
            "durative duration over undefined fluent",
            "(define (domain d) (:requirements :durative-actions :numeric-fluents)
              (:predicates (g)) (:functions (dur))
              (:durative-action act :parameters () :duration (= ?duration (dur))
                :condition () :effect (at end (g))))",
            "(define (problem p) (:domain d) (:init) (:goal (g)))",
        ),
        (
            "preference over unknown predicate",
            GOOD_DOM,
            "(define (problem p) (:domain d) (:init (a))
              (:goal (preference pp (zzz))) (:metric minimize (is-violated pp)))",
        ),
        ("pathologically nested effect", {
            // deeply-nested (and ...) effect must error cleanly, not overflow.
            let mut s = String::from(
                "(define (domain d) (:predicates (a) (b))
                  (:action go :parameters () :precondition (a) :effect ",
            );
            for _ in 0..2000 {
                s.push_str("(and ");
            }
            s.push_str("(b)");
            for _ in 0..2000 {
                s.push(')');
            }
            s.push_str("))");
            &*Box::leak(s.into_boxed_str())
        }, GOOD_PROB),
        ("near-cap nested (parses + grounds)", GOOD_DOM, {
            // ~cap-depth nesting that PARSES, then exercises the downstream
            // formula-recursive passes (grounding/heuristic) — must not overflow.
            let mut s = String::from("(define (problem p) (:domain d) (:init (a)) (:goal ");
            for _ in 0..140 {
                s.push_str("(and ");
            }
            s.push_str("(b)");
            for _ in 0..140 {
                s.push(')');
            }
            s.push_str("))");
            &*Box::leak(s.into_boxed_str())
        }),
        ("pathologically nested and/or", GOOD_DOM, {
            // leak a deeply-nested goal string (test-only): must error cleanly via
            // the parser's depth cap, NOT overflow the stack.
            let mut s = String::from("(define (problem p) (:domain d) (:init (a)) (:goal ");
            for _ in 0..2000 {
                s.push_str("(and ");
            }
            s.push_str("(b)");
            for _ in 0..2000 {
                s.push(')');
            }
            s.push_str("))");
            &*Box::leak(s.into_boxed_str())
        }),
        (
            "objects but empty types decl",
            "(define (domain d) (:requirements :typing) (:types) (:predicates (at ?x - t)))",
            "(define (problem p) (:domain d) (:objects o - t) (:init) (:goal (at o)))",
        ),
        (
            "mismatched domain name",
            GOOD_DOM,
            "(define (problem p) (:domain OTHER) (:init (a)) (:goal (b)))",
        ),
        (
            "negative numbers / weird tokens",
            "(define (domain d) (:requirements :numeric-fluents) (:functions (x))
              (:action go :parameters () :precondition (>= (x) -5) :effect (decrease (x) 999999999999)))",
            "(define (problem p) (:domain d) (:init (= (x) -3)) (:goal (>= (x) -1)))",
        ),
    ]
}

#[test]
fn no_panic_on_malformed_input() {
    let opts = Options::default();
    let mut panicked = Vec::new();
    for (name, dom, prob) in cases() {
        let res = std::panic::catch_unwind(|| {
            // ignore the Result — we only care that it does not panic
            let _ = solve(dom, prob, &opts);
        });
        if res.is_err() {
            panicked.push(name);
        }
    }
    assert!(
        panicked.is_empty(),
        "solve() panicked on malformed inputs: {:?}",
        panicked
    );
}
