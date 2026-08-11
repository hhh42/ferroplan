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
    let o = ferroplan::optimal::solve(&task, None, 2, None); // room for init + one node
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

/// Rung 1's end-to-end note stamp (0.23 Phase 5): on the pumppair shape
/// (two interchangeable tanks behind numeric bands) the api detects the
/// orbit, the numeric arm rides along, and the PROVEN note names BOTH —
/// "+numRPG+orbits" — so the prover ledger reads the symmetry engine's
/// contribution without archaeology.
#[test]
fn proven_note_stamps_numrpg_and_orbits() {
    let dom = "(define (domain pumppair)
      (:requirements :typing :numeric-fluents)
      (:types tank)
      (:predicates (full ?t - tank))
      (:functions (level ?t - tank))
      (:action pump :parameters (?t - tank)
        :precondition (and)
        :effect (increase (level ?t) 1))
      (:action cap :parameters (?t - tank)
        :precondition (>= (level ?t) 3)
        :effect (full ?t)))";
    let prb = "(define (problem pp) (:domain pumppair)
      (:objects t1 t2 - tank)
      (:init (= (level t1) 0) (= (level t2) 0))
      (:goal (and (full t1) (full t2))))";
    let sol = solve(dom, prb, &opts()).unwrap();
    assert!(sol.solved);
    assert_eq!(sol.plan.unwrap().length, 8);
    assert!(
        sol.notes.iter().any(|n| n.contains("h^max+numRPG+orbits")),
        "the note must name every prover component: {:?}",
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

/// Numeric-optimal soundness (0.21 Phase 4, the ipc2026-opt entry's pin):
/// the EXACT numeric goal test. [bigpump fire] costs 5 but ends at
/// charge 5 < 6 — an engine that relaxes the numeric goal conjunct in
/// the goal test (not just in h, where relaxing is admissible) certifies
/// 5. The true optimum is 6.
#[test]
fn numeric_goal_optimum_is_certified_exactly() {
    let dom = include_str!("../../../benchmarks/bench/numopt-domain.pddl");
    let prb = include_str!("../../../benchmarks/bench/numopt-p01.pddl");
    let sol = solve(dom, prb, &opts()).unwrap();
    assert!(sol.solved, "notes: {:?}", sol.notes);
    let plan = sol.plan.unwrap();
    assert_eq!(plan.metric, Some(6.0), "bigpump + pump + fire");
    assert_eq!(plan.length, 3);
    assert!(sol.notes.iter().any(|n| n.contains("PROVEN OPTIMAL")));
}

/// Pure numeric goal, no metric: the certificate is a LENGTH optimum
/// (what the ipc2026-opt board reports). States along the pump chain
/// differ ONLY in the charge fluent — a StateKey that dropped fluents
/// would merge them into init and "prove" the task unsolvable.
#[test]
fn numeric_goal_without_metric_certifies_length_optimum() {
    let dom = include_str!("../../../benchmarks/bench/numopt-domain.pddl");
    let prb = include_str!("../../../benchmarks/bench/numopt-p02.pddl");
    let sol = solve(dom, prb, &opts()).unwrap();
    assert!(sol.solved, "notes: {:?}", sol.notes);
    let plan = sol.plan.unwrap();
    assert_eq!(plan.length, 2, "bigpump + pump reaches charge 6");
    assert_eq!(plan.metric, None, "no metric fluent: length optimum only");
    assert!(sol.notes.iter().any(|n| n.contains("PROVEN OPTIMAL")));
}

/// Numeric preconditions on the optimal path: fire alone costs 1, but
/// its charge >= 4 precondition forces 4 charge-units first. An engine
/// relaxing numeric preconditions in EXPANSION certifies 1; the true
/// optimum is 5.
#[test]
fn numeric_precondition_holds_in_exact_expansion() {
    let dom = include_str!("../../../benchmarks/bench/numopt-domain.pddl");
    let prb = include_str!("../../../benchmarks/bench/numopt-p03.pddl");
    let sol = solve(dom, prb, &opts()).unwrap();
    assert!(sol.solved, "notes: {:?}", sol.notes);
    let plan = sol.plan.unwrap();
    assert_eq!(plan.metric, Some(5.0), "bigpump then fire");
    assert!(sol.notes.iter().any(|n| n.contains("PROVEN OPTIMAL")));
}

/// The numeric arm's note pin (0.22 Phase 4 L1): p02 is length-currency,
/// numeric, audit-clean, root-informative — the PROVEN note must name
/// BOTH admissible components, "+numRPG" beside the propositional prover.
#[test]
fn numeric_arm_names_its_prover_in_the_note() {
    let dom = include_str!("../../../benchmarks/bench/numopt-domain.pddl");
    let prb = include_str!("../../../benchmarks/bench/numopt-p02.pddl");
    let sol = solve(dom, prb, &opts()).unwrap();
    assert!(sol.solved, "notes: {:?}", sol.notes);
    assert_eq!(sol.plan.unwrap().length, 2, "the certificate must not move");
    assert!(
        sol.notes.iter().any(|n| n.contains("+numRPG")),
        "the note must name the numeric component: {:?}",
        sol.notes
    );
}

/// The audit's TEETH (0.22 Phase 4 L0): a factor-0.25 scale-up SHRINKS
/// the true value while one-sided widening never lowers lb, so an
/// UNAUDITED arm would read the goal unreachable at every layer and
/// certify PROVEN UNSOLVABLE on this 1-step task. The audit disarms and
/// the mode certifies the truth.
#[test]
fn scale_effects_disarm_the_numeric_bound_and_stay_sound() {
    let dom = "(define (domain sc) (:requirements :numeric-fluents)
      (:functions (x))
      (:action shrink :parameters ()
        :precondition (and) :effect (scale-up (x) 0.25)))";
    let prb = "(define (problem p) (:domain sc)
      (:init (= (x) 1)) (:goal (<= (x) 0.5)))";
    let sol = solve(dom, prb, &opts()).unwrap();
    assert!(sol.solved, "notes: {:?}", sol.notes);
    let plan = sol.plan.unwrap();
    assert_eq!(plan.length, 1, "one shrink: 1 -> 0.25 <= 0.5");
    assert!(
        sol.notes.iter().any(|n| n.contains("PROVEN OPTIMAL")),
        "notes: {:?}",
        sol.notes
    );
    assert!(
        !sol.notes.iter().any(|n| n.contains("+numRPG")),
        "the audit must keep the arm down here: {:?}",
        sol.notes
    );
}

/// trader-cycle stays the NEGATIVE control (0.22 Phase 4) — and writing
/// this pin taught the honest surprise: the deduped lap state space is
/// TINY (5 markets × ~2k cash values × hold), so Mode::Optimal PROVES
/// the 2,376-step optimum in ~7k expansions even BLIND. The control is
/// therefore on soundness, not coverage: the layer bound prices the lap
/// at a handful of layers (cycle-blind, heuristic.rs pins GoalAt ≤ 20),
/// and an INADMISSIBLE bound — the repetition-count shortcut the design
/// bans — would prune the optimal path and certify a LONGER cost. The
/// certificate must read exactly 2376 with the arm in play.
#[test]
fn trader_cycle_negative_control_certificate_unbent() {
    let dom = ferroplan::parser::parse_domain(include_str!(
        "../../../benchmarks/bench/trader-cycle-domain.pddl"
    ))
    .unwrap();
    let prb = ferroplan::parser::parse_problem(include_str!(
        "../../../benchmarks/bench/trader-cycle-i1.pddl"
    ))
    .unwrap();
    let task = ferroplan::ground::ground_task(&dom, &prb, 1).unwrap();
    let o = ferroplan::optimal::solve(&task, None, 50_000, None);
    assert!(o.proven, "the lap certificate must not be lost");
    assert_eq!(
        o.ops.expect("proven plan").len(),
        2376,
        "the weak-but-admissible bound must not bend the optimum"
    );
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
