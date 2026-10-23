//! Pattern databases (0.31 Lane C.1, docs/roadmap-0.31.md; `pdb.rs`,
//! `vars.rs`).
//!
//! Three pins. (1) ADMISSIBILITY on a task small enough to solve exactly:
//! the agent ring from `tests/partition_rung.rs` -- every reachable state
//! enumerated, its true cost-to-goal by a backward search over the
//! explicit graph, and the goal-pattern database never above it, zero on
//! the goal. (2) THE CERTIFICATE is unchanged: optimal A* with the
//! database installed proves the same cost as without, on the ring and
//! on a two-counter task with action costs. (3) CONDITIONAL EFFECTS on
//! the pattern are nondeterministic, not dropped: a task whose only route
//! to the goal is a `when` effect still gets a finite, admissible bound.

use std::collections::{HashMap, VecDeque};
use std::sync::Arc;

use ferroplan::packed::{PackedTask, State};
use ferroplan::pdb::{self, Limits, Pdb};
use ferroplan::vars::Variables;

fn ground(dom: &str, prb: &str) -> (ferroplan::types::Domain, PackedTask) {
    let d = ferroplan::parser::parse_domain(dom).unwrap();
    let p = ferroplan::parser::parse_problem(prb).unwrap();
    let t = ferroplan::ground::ground_task(&d, &p, 1).unwrap();
    (d, t)
}

fn ring(m: usize, k: usize) -> (String, String) {
    let dom = "(define (domain agentring)
      (:requirements :strips)
      (:predicates (at ?a ?c) (free ?c) (adj ?c1 ?c2))
      (:action move
        :parameters (?a ?from ?to)
        :precondition (and (at ?a ?from) (adj ?from ?to) (free ?to))
        :effect (and (not (at ?a ?from)) (at ?a ?to) (free ?from) (not (free ?to)))))"
        .to_string();
    let mut objs = String::new();
    let mut init = String::new();
    let mut goal = String::new();
    for c in 0..m {
        objs.push_str(&format!(" c{c}"));
        let n = (c + 1) % m;
        init.push_str(&format!(" (adj c{c} c{n}) (adj c{n} c{c})"));
        if c >= k {
            init.push_str(&format!(" (free c{c})"));
        }
    }
    for a in 0..k {
        objs.push_str(&format!(" a{a}"));
        init.push_str(&format!(" (at a{a} c{a})"));
        // Rotate everyone one cell: solvable, unlike the reversal.
        goal.push_str(&format!(" (at a{a} c{})", (a + 1) % m));
    }
    (
        dom,
        format!(
            "(define (problem ring) (:domain agentring) (:objects{objs}) \
             (:init{init}) (:goal (and{goal})))"
        ),
    )
}

/// Every reachable state's exact unit-cost distance to a goal state.
fn exact_distances(task: &PackedTask) -> Vec<(State, f64)> {
    let key = |s: &State| task.state_key(s);
    let mut idx: HashMap<ferroplan::packed::StateKey, usize> = HashMap::new();
    let mut states: Vec<State> = vec![task.initial()];
    idx.insert(key(&states[0]), 0);
    let mut succ: Vec<Vec<usize>> = vec![Vec::new()];
    let mut q = VecDeque::from([0usize]);
    let mut cands = Vec::new();
    while let Some(i) = q.pop_front() {
        cands.clear();
        task.applicable_ops(&states[i], &mut cands);
        for &oi in &cands {
            let ns = task.apply(oi as usize, &states[i]);
            let k = key(&ns);
            let j = match idx.get(&k) {
                Some(&j) => j,
                None => {
                    let j = states.len();
                    idx.insert(k, j);
                    states.push(ns);
                    succ.push(Vec::new());
                    q.push_back(j);
                    j
                }
            };
            succ[i].push(j);
        }
    }
    let n = states.len();
    let mut pred: Vec<Vec<usize>> = vec![Vec::new(); n];
    for (i, ss) in succ.iter().enumerate() {
        for &j in ss {
            pred[j].push(i);
        }
    }
    let mut dist = vec![f64::INFINITY; n];
    let mut q = VecDeque::new();
    for (i, s) in states.iter().enumerate() {
        if task.goal_met_with(s, &task.goal_pos, &task.goal_num) {
            dist[i] = 0.0;
            q.push_back(i);
        }
    }
    while let Some(j) = q.pop_front() {
        for &i in &pred[j] {
            if dist[i].is_infinite() {
                dist[i] = dist[j] + 1.0;
                q.push_back(i);
            }
        }
    }
    states.into_iter().zip(dist).collect()
}

#[test]
fn the_goal_pattern_is_admissible_on_the_ring() {
    let (dom, prb) = ring(6, 3);
    let (d, t) = ground(&dom, &prb);
    let vars = Arc::new(Variables::from_task(&d, &t));
    assert!(
        vars.multi_valued() >= 3,
        "the ring's `at` facts should form one group per agent: {:?}",
        vars.vars
    );
    let pattern = Pdb::goal_pattern(&vars, &t, 1 << 16);
    assert!(!pattern.is_empty());
    let costs = vec![1.0; t.n_ops];
    let pdb = Pdb::build(&t, vars, pattern, &costs, &Limits::default()).unwrap();
    let all = exact_distances(&t);
    assert!(all.len() > 50, "the ring should have a real state space");
    let mut informative = 0;
    for (s, exact) in &all {
        let h = pdb.h(&s.bits);
        assert!(
            h <= *exact + 1e-9,
            "h {h} over the exact {exact} on a reachable state"
        );
        if *exact == 0.0 {
            assert_eq!(h, 0.0);
        }
        if h > 0.0 {
            informative += 1;
        }
    }
    assert!(informative > 0, "the database never said anything");
}

#[test]
fn the_certificate_is_unchanged_with_the_database_installed() {
    let (dom, prb) = ring(6, 3);
    let (d, t) = ground(&dom, &prb);
    let plain = ferroplan::optimal::solve(&t, None, 1 << 20, None);
    assert!(plain.proven, "the ring is provable without the database");
    let vars = Arc::new(Variables::from_task(&d, &t));
    let pattern = Pdb::goal_pattern(&vars, &t, 1 << 16);
    let pdb = Pdb::build(&t, vars, pattern, &vec![1.0; t.n_ops], &Limits::default()).unwrap();
    let _g = pdb::install(Some(Arc::new(pdb)));
    let with = ferroplan::optimal::solve(&t, None, 1 << 20, None);
    assert!(with.proven);
    assert_eq!(with.cost, plain.cost);
    assert!(with.heuristic.ends_with("+PDB"), "{}", with.heuristic);
    assert!(
        with.expanded <= plain.expanded,
        "a tighter bound must not expand more: {} vs {}",
        with.expanded,
        plain.expanded
    );
}

#[test]
fn action_costs_are_honoured_in_the_abstract_distances() {
    // Two routes to the goal: a 1-step op of cost 5, a 3-step chain of
    // cost 1 each. The abstract space IS the concrete one here, so the
    // database equals the optimum, 3.
    let (d, t) = ground(
        "(define (domain tworoutes) (:requirements :action-costs)
           (:predicates (s) (a) (b) (g))
           (:functions (total-cost))
           (:action jump :precondition (s) :effect (and (g) (not (s)) (increase (total-cost) 5)))
           (:action x :precondition (s) :effect (and (a) (not (s)) (increase (total-cost) 1)))
           (:action y :precondition (a) :effect (and (b) (not (a)) (increase (total-cost) 1)))
           (:action z :precondition (b) :effect (and (g) (not (b)) (increase (total-cost) 1))))",
        "(define (problem tr) (:domain tworoutes) (:init (s) (= (total-cost) 0)) (:goal (g)) (:metric minimize (total-cost)))",
    );
    let cf = t.fluent_id("total-cost");
    assert!(cf.is_some());
    let vars = Arc::new(Variables::from_task(&d, &t));
    let pattern = Pdb::goal_pattern(&vars, &t, 1 << 16);
    let costs = ferroplan::optimal::op_costs(&t, cf).unwrap();
    let pdb = Pdb::build(&t, vars, pattern, &costs, &Limits::default()).unwrap();
    assert_eq!(pdb.h(&t.initial().bits), 3.0);
}

#[test]
fn a_conditional_effect_on_the_pattern_keeps_the_bound_finite() {
    // The goal (g) is reachable only through `when (k) (g)`: a database
    // that dropped the conditional effect would read the goal as
    // unreachable (∞) and prune the start -- unsound.
    let (d, t) = ground(
        "(define (domain condgoal) (:requirements :adl)
           (:predicates (k) (g) (s))
           (:action arm :precondition (s) :effect (k))
           (:action fire :precondition (s) :effect (when (k) (g))))",
        "(define (problem cg) (:domain condgoal) (:init (s)) (:goal (g)))",
    );
    let vars = Arc::new(Variables::from_task(&d, &t));
    let pattern = Pdb::goal_pattern(&vars, &t, 1 << 16);
    let pdb = Pdb::build(&t, vars, pattern, &vec![1.0; t.n_ops], &Limits::default()).unwrap();
    let h = pdb.h(&t.initial().bits);
    assert!(h.is_finite() && h <= 2.0, "h {h}");
}
