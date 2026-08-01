//! The "just parse" public API (`ferroplan::parse`): syntax-validate + summarize a
//! PDDL domain or problem without grounding or solving.

use ferroplan::parse;

const DOM: &str = "(define (domain gripper)
  (:requirements :strips :typing)
  (:types room ball gripper)
  (:predicates (at-robby ?r - room) (at ?b - ball ?r - room) (free ?g - gripper))
  (:functions (cost))
  (:action move :parameters (?from ?to - room)
    :precondition (at-robby ?from) :effect (and (not (at-robby ?from)) (at-robby ?to)))
  (:action pick :parameters (?b - ball ?r - room ?g - gripper)
    :precondition (and (at ?b ?r) (at-robby ?r)) :effect (not (at ?b ?r))))";

const PROB: &str = "(define (problem gripper-1) (:domain gripper)
  (:objects rooma roomb - room b1 b2 - ball left right - gripper)
  (:init (at-robby rooma) (at b1 rooma) (= (cost) 0))
  (:goal (at b1 roomb))
  (:metric minimize (cost)))";

#[test]
fn parses_domain_summary() {
    let r = parse(DOM);
    assert!(r.ok, "domain parses: {:?}", r.error);
    assert_eq!(r.kind.as_deref(), Some("domain"));
    assert_eq!(r.name.as_deref(), Some("gripper"));
    assert!(r.requirements.contains(&":strips".to_string()));
    assert!(r.requirements.contains(&":typing".to_string()));
    let d = r.domain.expect("domain summary");
    assert_eq!(d.actions, ["move", "pick"]);
    assert!(d.durative_actions.is_empty());
    assert_eq!(d.types, ["room", "ball", "gripper"]);
    // a predicate signature renders as `name argtypes…`
    assert!(
        d.predicates.iter().any(|p| p == "at-robby room"),
        "got {:?}",
        d.predicates
    );
    assert!(d.functions.iter().any(|f| f == "cost"));
    assert!(r.problem.is_none());
}

#[test]
fn parses_problem_summary() {
    let r = parse(PROB);
    assert!(r.ok, "problem parses: {:?}", r.error);
    assert_eq!(r.kind.as_deref(), Some("problem"));
    assert_eq!(r.name.as_deref(), Some("gripper-1"));
    let p = r.problem.expect("problem summary");
    assert_eq!(p.domain, "gripper");
    assert_eq!(p.objects, 6);
    assert_eq!(p.init_facts, 2); // at-robby + at  (the = is a fluent)
    assert_eq!(p.init_fluents, 1);
    assert_eq!(p.timed_initial_literals, 0);
    assert!(p.has_goal);
    assert!(p.has_metric);
    assert!(r.domain.is_none());
}

#[test]
fn reports_syntax_errors_without_panicking() {
    let r = parse("(define (domain broken) (:predicates (p");
    assert!(!r.ok);
    assert_eq!(r.kind.as_deref(), Some("domain"));
    assert!(r.error.is_some(), "an error message is reported");
    assert!(r.domain.is_none());
}

#[test]
fn counts_timed_initial_literals() {
    let prob = "(define (problem g) (:domain d)
      (:init (door) (at 5 (open)) (at 9 (not (door))))
      (:goal (through)))";
    let r = parse(prob);
    assert!(r.ok);
    let p = r.problem.unwrap();
    assert_eq!(p.timed_initial_literals, 2);
    assert_eq!(p.init_facts, 1); // (door); the two `(at <t> …)` are TILs, not init facts
}

/// Negative number literals (0.19 Phase 1): `(= (d p0) -370)` in `:init`
/// killed sailing-numeric and fo-sailing whole (40/40 rejects on the
/// 2023 numeric board) — the lexer emitted `Dash` for `-370`. The solve
/// half pins the SIGN, not just parse success: with `(x) = -3`, one
/// +1 step is needed to reach `(>= (x) -2)`; a sign flip would make the
/// goal true at init and the plan empty.
#[test]
fn negative_init_literals_parse_and_read_signed() {
    let dom = "(define (domain negnum)
      (:requirements :typing :numeric-fluents)
      (:functions (x))
      (:action bump :parameters ()
        :precondition (<= (x) 10) :effect (increase (x) 1)))";
    let prob = "(define (problem negnum-1) (:domain negnum)
      (:init (= (x) -3))
      (:goal (>= (x) -2)))";
    let r = parse(prob);
    assert!(r.ok, "negative init literal parses: {:?}", r.error);
    let sol = ferroplan::solve(dom, prob, &ferroplan::Options::default()).unwrap();
    assert!(sol.solved);
    assert_eq!(
        sol.plan.unwrap().length,
        1,
        "init must read -3 (a sign flip would satisfy the goal at init)"
    );
}

/// Uninitialized `(total-cost)` (0.19 Phase 1): IPC-2018's agricola,
/// flashfill, and settlers omit `(= (total-cost) 0)` from init (the
/// PDDL 3.1 `:action-costs` convention makes 0 implicit — Fast Downward
/// defaults it). The undefined fluent killed every cost-bearing action
/// in reachability and the whole domains (60 instances) returned
/// unsolvable with zero facts. The cost fluent defaults to 0; the metric
/// still counts (the plan's metric is 1 after one unit-cost step).
#[test]
fn uninitialized_total_cost_defaults_to_zero() {
    let dom = "(define (domain nocost)
      (:requirements :typing :action-costs)
      (:predicates (done))
      (:functions (total-cost) - number)
      (:action finish :parameters ()
        :precondition (and) :effect (and (done) (increase (total-cost) 1))))";
    let prob = "(define (problem nocost-1) (:domain nocost)
      (:init)
      (:goal (done))
      (:metric minimize (total-cost)))";
    let sol = ferroplan::solve(dom, prob, &ferroplan::Options::default()).unwrap();
    assert!(sol.solved, "notes: {:?}", sol.notes);
    let p = sol.plan.unwrap();
    assert_eq!(p.length, 1);
    assert_eq!(p.metric, Some(1.0), "cost counts from the implicit 0");
}
