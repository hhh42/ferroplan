//! INCUMBENT ZERO (0.28 Lane I): the PDDL3 optimizer is handed a plan for
//! the HARD goals before it starts, so it can no longer end a run with
//! nothing to report.
//!
//! The defect this closes is a SCALE phenomenon and its RED receipts live on
//! the boards, not here: rovers-preferences-qualitative i9 spends 70k
//! evaluations and a whole 20 s wall in the compiled task without reaching
//! the hard goal once, while the classical ladder reaches it in 133
//! evaluations and 30 ms -- `metric_optimize` returned `None`, the row read
//! "unsolved", and 46 of `ipc5-qual-pref`'s 49 misses at 60 s were that
//! shape. At fixture scale the compiled search always finds SOMETHING, so
//! what is pinned here is the mechanism the fix rests on: the lift from the
//! original task's plan into the compiled task, through the two places the
//! op sets differ (precondition-preference VARIANTS, and the trajectory
//! gate's synthetic TRAJ-END), and the floor's price.

use ferroplan::pddl3;
use ferroplan::search::SearchCfg;

const CORRIDOR: &str = "(define (domain corridor)
 (:requirements :strips :typing :preferences)
 (:types cell)
 (:predicates (at ?c - cell) (adj ?a ?b - cell) (lit ?c - cell) (visited ?c - cell))
 (:action move :parameters (?a ?b - cell)
   :precondition (and (at ?a) (adj ?a ?b) (preference darkstep (lit ?a)))
   :effect (and (not (at ?a)) (at ?b) (visited ?b)))
 (:action light :parameters (?a - cell)
   :precondition (at ?a)
   :effect (lit ?a)))";

/// c0 - c1 - c2 - c3, with a side cell s1 hanging off c1. Hard goal: reach
/// c3. Soft: every step leaves a lit cell (1 each), and s1 gets visited (5).
const WALK: &str = "(define (problem walk) (:domain corridor)
 (:objects c0 c1 c2 c3 s1 - cell)
 (:init (at c0)
        (adj c0 c1) (adj c1 c2) (adj c2 c3) (adj c1 s1) (adj s1 c1))
 (:goal (and (at c3) (preference sidetrip (visited s1))))
 (:metric minimize (+ (is-violated darkstep) (* 5 (is-violated sidetrip)))))";

struct Compiled {
    task: ferroplan::packed::PackedTask,
    cost_fluent: usize,
    forgos: Vec<(usize, f64)>,
    groups: Vec<Vec<u32>>,
    folded: bool,
}

fn compile(d: &ferroplan::types::Domain, p: &ferroplan::types::Problem) -> Compiled {
    let c = pddl3::compile(d, p);
    assert!(c.unsupported.is_none(), "{:?}", c.unsupported);
    let task = ferroplan::ground::ground_task(&c.domain, &c.problem, 1).expect("grounds");
    let cost_fluent = task.fluent_id(pddl3::COST_DISP).expect("total-cost");
    let forgos = c
        .forgos
        .iter()
        .filter_map(|(name, w)| {
            task.op_display
                .iter()
                .position(|d| d == name)
                .map(|oi| (oi, *w))
        })
        .collect();
    let groups = ferroplan::invariants::synthesize(&c.domain, &task);
    Compiled {
        task,
        cost_fluent,
        forgos,
        groups,
        folded: c.folded_metric,
    }
}

#[test]
fn the_seed_lifts_through_precondition_preference_variants() {
    let d = ferroplan::parser::parse_domain(CORRIDOR).unwrap();
    let p = ferroplan::parser::parse_problem(WALK).unwrap();
    let c = compile(&d, &p);

    // The original task reads both preferences as true, so its plan is the
    // bare walk: three moves, no lights, no side trip.
    let seed = pddl3::hard_goal_seed(&d, &p, &c.task, 1, SearchCfg::default())
        .expect("the hard goal is three steps away");
    let names: Vec<&str> = seed
        .iter()
        .map(|&oi| c.task.op_display[oi].as_str())
        .collect();
    assert_eq!(names, ["MOVE C0 C1", "MOVE C1 C2", "MOVE C2 C3"]);

    // In the compiled task every MOVE exists twice under one name -- lit
    // (free) and dark (pays darkstep). Each cell is dark, so the lift must
    // have chosen the dark variant three times: 3 x 1, plus the forgone side
    // trip's 5. A lift that picked the wrong variant would not replay at all.
    let (plan, cost) = pddl3::close_seed(&c.task, c.cost_fluent, &c.forgos, &seed).expect("closes");
    assert_eq!(cost, 8.0, "three dark steps and no side trip");
    assert!(plan.len() > seed.len(), "the phase tail closes the plan");

    // The floor is a FLOOR: the optimizer is free to beat it, and here it
    // can (light each cell before leaving it, and take the side trip).
    let r = pddl3::metric_optimize_seeded(
        &c.task,
        c.cost_fluent,
        &c.forgos,
        &c.groups,
        c.folded,
        1,
        Some(&seed),
    )
    .expect("seeded runs always report");
    assert!(!r.from_seed, "the optimum is reachable at this scale");
    assert_eq!(r.result.cost, 0.0);
}

#[test]
fn a_seed_that_does_not_replay_is_refused_not_trusted() {
    let d = ferroplan::parser::parse_domain(CORRIDOR).unwrap();
    let p = ferroplan::parser::parse_problem(WALK).unwrap();
    let c = compile(&d, &p);
    let seed = pddl3::hard_goal_seed(&d, &p, &c.task, 1, SearchCfg::default()).unwrap();

    // Short of the hard goal: replays, but must not become an incumbent.
    assert!(pddl3::close_seed(&c.task, c.cost_fluent, &c.forgos, &seed[..2]).is_none());
    // Out of order: does not replay.
    let mut scrambled = seed.clone();
    scrambled.swap(0, 2);
    assert!(pddl3::close_seed(&c.task, c.cost_fluent, &c.forgos, &scrambled).is_none());
}

/// The qualitative-track shape: soft TRAJECTORY constraints. The gate
/// compiles them to monitors and a synthetic TRAJ-END step that is a real
/// op to both tasks, so the lift has to carry it across by name too.
#[test]
fn the_seed_lifts_through_the_trajectory_gate() {
    let dom = "(define (domain corridor-q)
     (:requirements :strips :typing :preferences :constraints)
     (:types cell)
     (:predicates (at ?c - cell) (adj ?a ?b - cell) (visited ?c - cell))
     (:action move :parameters (?a ?b - cell)
       :precondition (and (at ?a) (adj ?a ?b))
       :effect (and (not (at ?a)) (at ?b) (visited ?b))))";
    let prob = "(define (problem walk-q) (:domain corridor-q)
     (:objects c0 c1 c2 c3 s1 - cell)
     (:init (at c0)
            (adj c0 c1) (adj c1 c2) (adj c2 c3) (adj c1 s1) (adj s1 c1))
     (:goal (at c3))
     (:constraints (and
        (preference detour (sometime (at s1)))
        (preference never-c2 (always (not (at c2))))))
     (:metric minimize (+ (* 5 (is-violated detour)) (* 2 (is-violated never-c2)))))";
    let sol = ferroplan::solve(
        dom,
        prob,
        &ferroplan::Options {
            threads: 1,
            ..Default::default()
        },
    )
    .unwrap();
    assert!(sol.solved);
    let plan = sol.plan.unwrap();
    // c2 is the only way through, so never-c2 (2) is unavoidable; the detour
    // is free to take. No reported step is synthetic.
    assert_eq!(plan.metric, Some(2.0));
    assert!(plan.steps.iter().all(|s| !s.action.contains("TRAJ")));

    let d = ferroplan::parser::parse_domain(dom).unwrap();
    let p = ferroplan::parser::parse_problem(prob).unwrap();
    let (gd, gp) = ferroplan::constraints::gate(&d, &p)
        .expect("supported")
        .expect("the gate compiles soft trajectory constraints");
    let c = compile(&gd, &gp);
    let seed = pddl3::hard_goal_seed(&gd, &gp, &c.task, 1, SearchCfg::default())
        .expect("hard goal reachable");
    let (_, cost) = pddl3::close_seed(&c.task, c.cost_fluent, &c.forgos, &seed).expect("closes");
    assert_eq!(
        cost, 7.0,
        "the bare walk skips the detour (5) and crosses c2 (2)"
    );
}
