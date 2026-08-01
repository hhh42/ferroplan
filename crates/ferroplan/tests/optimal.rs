//! The admissible mode's fixture ladder (0.19 Phase 2): known optima
//! BEFORE any corpus entry. Every claim the mode can make is pinned —
//! the certified optimum on unit-cost and action-costs tasks (including
//! a cost trap where the satisficing ladder's plan is legal but
//! suboptimal), proof-or-nothing under a node cap, and a certified
//! UNSOLVABLE verdict the delete relaxation cannot see.

use ferroplan::{solve, Mode, Options};

fn opts() -> Options {
    Options {
        mode: Mode::Optimal,
        threads: 1,
        ..Default::default()
    }
}

/// Unit-cost chain: exactly 3 steps to the goal, no shorter plan exists.
#[test]
fn unit_cost_optimum_is_certified() {
    let dom = "(define (domain chain3)
      (:predicates (p0) (p1) (p2) (p3))
      (:action s1 :parameters () :precondition (p0) :effect (p1))
      (:action s2 :parameters () :precondition (p1) :effect (p2))
      (:action s3 :parameters () :precondition (p2) :effect (p3)))";
    let prb = "(define (problem c) (:domain chain3) (:init (p0)) (:goal (p3)))";
    let sol = solve(dom, prb, &opts()).unwrap();
    assert!(sol.solved);
    assert_eq!(sol.plan.unwrap().length, 3);
    assert!(
        sol.notes.iter().any(|n| n.contains("PROVEN OPTIMAL")),
        "notes: {:?}",
        sol.notes
    );
}

/// The cost trap: a 1-step plan costing 10 vs a 2-step plan costing 4.
/// Satisficing FF (shortest relaxed plan) walks into the expensive step;
/// the optimal mode must certify cost 4.
#[test]
fn action_costs_optimum_beats_the_short_expensive_plan() {
    let dom = "(define (domain trap)
      (:requirements :action-costs)
      (:predicates (start) (mid) (goal))
      (:functions (total-cost) - number)
      (:action jump :parameters ()
        :precondition (start) :effect (and (goal) (increase (total-cost) 10)))
      (:action walk1 :parameters ()
        :precondition (start) :effect (and (mid) (increase (total-cost) 2)))
      (:action walk2 :parameters ()
        :precondition (mid) :effect (and (goal) (increase (total-cost) 2))))";
    let prb = "(define (problem t) (:domain trap)
      (:init (start) (= (total-cost) 0))
      (:goal (goal))
      (:metric minimize (total-cost)))";
    let sol = solve(dom, prb, &opts()).unwrap();
    assert!(sol.solved);
    let p = sol.plan.unwrap();
    assert_eq!(p.length, 2, "takes the cheap 2-step route");
    assert_eq!(p.metric, Some(4.0), "certified cost 4, not the trap's 10");
}

/// Proof-or-nothing: a node cap far below the search's needs returns
/// INCONCLUSIVE with no plan — never a best-effort incumbent.
#[test]
fn cap_yields_inconclusive_not_a_plan() {
    let dom = ferroplan::parser::parse_domain(
        "(define (domain chain3)
      (:predicates (p0) (p1) (p2) (p3))
      (:action s1 :parameters () :precondition (p0) :effect (p1))
      (:action s2 :parameters () :precondition (p1) :effect (p2))
      (:action s3 :parameters () :precondition (p2) :effect (p3)))",
    )
    .unwrap();
    let prb = ferroplan::parser::parse_problem(
        "(define (problem c) (:domain chain3) (:init (p0)) (:goal (p3)))",
    )
    .unwrap();
    let task = ferroplan::ground::ground_task(&dom, &prb, 1).unwrap();
    let o = ferroplan::optimal::solve(&task, None, 2); // room for init + one node
    assert!(!o.proven);
    assert!(o.ops.is_none(), "no uncertified plan is ever reported");
}

/// Certified UNSOLVABLE where the delete relaxation says solvable: the
/// goal needs (a) AND (b), but each achiever consumes the shared (s) —
/// relaxed-reachable both ways, exactly-unreachable together. The
/// grounder keeps the task; A* exhausts and PROVES it.
#[test]
fn exhaustion_proves_unsolvable_past_the_relaxation() {
    let dom = "(define (domain mutex)
      (:predicates (s) (a) (b))
      (:action mk-a :parameters () :precondition (s) :effect (and (a) (not (s))))
      (:action mk-b :parameters () :precondition (s) :effect (and (b) (not (s)))))";
    let prb = "(define (problem m) (:domain mutex) (:init (s)) (:goal (and (a) (b))))";
    let sol = solve(dom, prb, &opts()).unwrap();
    assert!(!sol.solved);
    assert!(
        sol.notes.iter().any(|n| n.contains("PROVEN UNSOLVABLE")),
        "notes: {:?}",
        sol.notes
    );
}

/// Out-of-scope shapes are rejected with a named note, never mis-certified.
#[test]
fn temporal_problems_are_rejected_by_name() {
    let dom = include_str!("../../../benchmarks/bench/eps-cross-domain.pddl");
    let prb = include_str!("../../../benchmarks/bench/eps-cross-p01.pddl");
    let sol = solve(dom, prb, &opts()).unwrap();
    assert!(!sol.solved);
    assert!(
        sol.notes.iter().any(|n| n.contains("classical-only")),
        "notes: {:?}",
        sol.notes
    );
}

/// The 0.20 admissibility repair, end to end: the cheap route to the goal
/// runs through a CONDITIONAL add (setp 10, then a 1), the unconditional
/// route costs 100. 0.19's h^max ignored conditional achievers,
/// overestimated, and CERTIFIED the cost-100 plan; with the achiever
/// model both heuristics see the truth and the mode certifies 11.
#[test]
fn conditional_effect_optimum_is_certified() {
    let dom = "(define (domain co)
      (:requirements :conditional-effects :action-costs)
      (:predicates (p) (g))
      (:functions (total-cost))
      (:action setp :parameters () :precondition (and)
        :effect (and (p) (increase (total-cost) 10)))
      (:action a :parameters () :precondition (and)
        :effect (and (when (p) (g)) (increase (total-cost) 1)))
      (:action direct :parameters () :precondition (and)
        :effect (and (g) (increase (total-cost) 100))))";
    let prb = "(define (problem p) (:domain co)
      (:init (= (total-cost) 0)) (:goal (g))
      (:metric minimize (total-cost)))";
    let sol = solve(dom, prb, &opts()).unwrap();
    assert!(sol.solved);
    let plan = sol.plan.unwrap();
    assert_eq!(plan.metric, Some(11.0));
    let actions: Vec<&str> = plan.steps.iter().map(|s| s.action.as_str()).collect();
    assert_eq!(actions, ["SETP", "A"]);
}

/// A conditional effect ON THE COST FLUENT is a state-dependent cost in
/// disguise — outside the certified scope, rejected by name.
#[test]
fn conditional_cost_effect_rejects_by_name() {
    let dom = "(define (domain cc)
      (:requirements :conditional-effects :action-costs)
      (:predicates (p) (g))
      (:functions (total-cost))
      (:action delp :parameters () :precondition (p)
        :effect (not (p)))
      (:action a :parameters () :precondition (and)
        :effect (and (g) (when (p) (increase (total-cost) 5)))))";
    let prb = "(define (problem p) (:domain cc)
      (:init (p) (= (total-cost) 0)) (:goal (g))
      (:metric minimize (total-cost)))";
    let sol = solve(dom, prb, &opts()).unwrap();
    assert!(!sol.solved);
    assert!(
        sol.notes
            .iter()
            .any(|n| n.contains("conditional cost effect")),
        "notes: {:?}",
        sol.notes
    );
}
