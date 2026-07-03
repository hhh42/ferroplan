//! PDDL3 preference compilation tests — validate that goal forall-preferences
//! and precondition preferences are counted EXACTLY (small enough that the
//! anytime branch-and-bound converges to the true optimum).

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

#[test]
fn forall_preference_counts_violated_instances() {
    // (forall (?x) (preference p (special ?x))) over a,b,c; only a,b CAN become
    // special -> exactly one instance (c) is unavoidably violated -> metric 1.
    let prob = "(define (problem p) (:domain mk)
        (:objects a b c - item)
        (:init (can a) (can b))
        (:goal (forall (?x - item) (preference p (special ?x))))
        (:metric minimize (is-violated p)))";
    let sol = pddl3(MARK, prob);
    assert!(sol.solved);
    assert_eq!(sol.mode, Mode::Pddl3);
    assert_eq!(
        sol.plan.unwrap().metric,
        Some(1.0),
        "one unsatisfiable instance -> 1"
    );
}

#[test]
fn forall_preference_all_satisfiable_is_zero() {
    let prob = "(define (problem p) (:domain mk)
        (:objects a b - item)
        (:init (can a) (can b))
        (:goal (forall (?x - item) (preference p (special ?x))))
        (:metric minimize (is-violated p)))";
    assert_eq!(pddl3(MARK, prob).plan.unwrap().metric, Some(0.0));
}

#[test]
fn weighted_forall_preferences() {
    // two preferences, weights 1 and 10; c cannot be special, none can be gold.
    // p (special): c violated -> 1*1=1 ; q (gold): a,b,c all violated -> 3*10=30 -> 31
    let dom = "(define (domain mk2)
        (:requirements :strips :typing :adl :fluents)
        (:types item)
        (:predicates (special ?x - item) (gold ?x - item) (can ?x - item))
        (:functions (total-cost))
        (:action make :parameters (?x - item) :precondition (can ?x) :effect (special ?x)))";
    let prob = "(define (problem p) (:domain mk2)
        (:objects a b c - item)
        (:init (can a) (can b))
        (:goal (and (forall (?x - item) (preference p (special ?x)))
                    (forall (?x - item) (preference q (gold ?x)))))
        (:metric minimize (+ (* 1 (is-violated p)) (* 10 (is-violated q)))))";
    assert_eq!(pddl3(dom, prob).plan.unwrap().metric, Some(31.0));
}

const SOFTPRE: &str = "(define (domain sp)
 (:requirements :strips :adl :fluents)
 (:predicates (ready) (done))
 (:functions (total-cost))
 (:action go :parameters ()
   :precondition (preference want (ready))
   :effect (done)))";

#[test]
fn precondition_preference_charged_once_when_violated() {
    // `go` must run with (ready) false -> violated variant -> exactly +1.
    let prob = "(define (problem p) (:domain sp)
        (:init) (:goal (done)) (:metric minimize (is-violated want)))";
    let sol = pddl3(SOFTPRE, prob);
    assert!(sol.solved);
    assert_eq!(sol.plan.unwrap().metric, Some(1.0));
}

#[test]
fn precondition_preference_free_when_satisfied() {
    let prob = "(define (problem p) (:domain sp)
        (:init (ready)) (:goal (done)) (:metric minimize (is-violated want)))";
    assert_eq!(pddl3(SOFTPRE, prob).plan.unwrap().metric, Some(0.0));
}

#[test]
fn static_simplification_drops_never_violable_instances_metric_exact() {
    // Storage-shaped quadratic forall-preference: `(imply (linked ?a ?b) (ok ?a ?b))`
    // where `linked` is STATIC (no action touches it). Only (x,y) is linked, so
    // 3 of the 4 instances are statically satisfied (antecedent false) and must
    // be dropped at compile time without changing the metric. The remaining
    // (x,y) instance is violable: `ok x y` is achievable, so the optimum
    // collects it -> metric 0. A second pref `q` on a statically-UNLINKABLE
    // pair keeps weight accounting honest when phi folds to True.
    let dom = "(define (domain st)
     (:requirements :strips :typing :adl :fluents)
     (:types spot)
     (:predicates (linked ?a - spot ?b - spot) (ok ?a - spot ?b - spot) (busy))
     (:functions (total-cost))
     (:action fix :parameters (?a - spot ?b - spot)
       :precondition (linked ?a ?b) :effect (ok ?a ?b)))";
    let prob = "(define (problem p) (:domain st)
        (:objects x y - spot)
        (:init (linked x y))
        (:goal (and
          (forall (?a - spot ?b - spot)
            (preference p (imply (linked ?a ?b) (ok ?a ?b))))
          (forall (?a - spot ?b - spot)
            (preference q (imply (linked ?b ?a) (ok ?b ?a))))))
        (:metric minimize (+ (* 2 (is-violated p)) (* 3 (is-violated q)))))";
    let sol = pddl3(dom, prob);
    assert!(sol.solved);
    // both live instances ((x,y) for p; (x,y) via (?b,?a)=(y,x)... for q) are
    // satisfied by one `fix x y` -> 0; the 6 statically-true instances add 0.
    assert_eq!(sol.plan.unwrap().metric, Some(0.0));
}

#[test]
fn static_simplification_keeps_violable_instances() {
    // The linked pair CANNOT be fixed (no achiever precondition holds), so its
    // preference instance must survive simplification and pay its weight.
    let dom = "(define (domain st2)
     (:requirements :strips :typing :adl :fluents)
     (:types spot)
     (:predicates (linked ?a - spot ?b - spot) (ok ?a - spot ?b - spot) (can))
     (:functions (total-cost))
     (:action fix :parameters (?a - spot ?b - spot)
       :precondition (and (can) (linked ?a ?b)) :effect (ok ?a ?b)))";
    let prob = "(define (problem p) (:domain st2)
        (:objects x y - spot)
        (:init (linked x y))
        (:goal (forall (?a - spot ?b - spot)
            (preference p (imply (linked ?a ?b) (ok ?a ?b)))))
        (:metric minimize (* 5 (is-violated p))))";
    // (can) never holds -> the one live instance is unavoidably violated -> 5.
    assert_eq!(pddl3(dom, prob).plan.unwrap().metric, Some(5.0));
}
