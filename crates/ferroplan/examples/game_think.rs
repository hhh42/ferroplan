//! The budgeted think loop (0.11 Phase 4) — the game-embedding shape from
//! STATUS.md's design answers: real-time play with EPISODIC stop-and-think.
//! An agent grounds its world once, then each "think" is a BOUNDED call —
//! an eval budget (the deterministic unit, never wall clock) and a memory
//! target — that either returns a plan to follow or an honest
//! budget-exhausted verdict the game can react to (think again later,
//! escalate the budget, or pick a fallback behavior).
//!
//! Run: cargo run --release -p ferroplan --example game_think
use ferroplan::{Options, Session};

const DOM: &str = "
(define (domain homestead) (:requirements :strips :typing :numeric-fluents)
  (:types agent place)
  (:predicates (at ?a - agent ?p - place) (road ?x ?y - place) (fertile ?p - place))
  (:functions (grain))
  (:action walk :parameters (?a - agent ?from ?to - place)
    :precondition (and (at ?a ?from) (road ?from ?to))
    :effect (and (not (at ?a ?from)) (at ?a ?to)))
  (:action harvest :parameters (?a - agent ?p - place)
    :precondition (and (at ?a ?p) (fertile ?p))
    :effect (increase (grain) 1)))";

const PRB: &str = "
(define (problem morning) (:domain homestead)
  (:objects vera - agent hut field barn - place)
  (:init (at vera hut) (road hut field) (road field hut)
         (road field barn) (road barn field) (fertile field) (= (grain) 0))
  (:goal (>= (grain) 3)))";

fn main() -> Result<(), String> {
    let mut world = Session::new(DOM, PRB, &Options::default())?;

    // Think 1: a generous budget — plan the morning's work.
    let think = world.replan_budgeted(100_000, Some(256));
    assert!(think.solved);
    let plan = think.plan.as_ref().unwrap();
    println!(
        "think 1: {} steps within {} evals",
        plan.length, think.statistics.evaluated_states
    );

    // The agent FOLLOWS the plan in real time; the game mirrors the first
    // two steps into the session (walk to the field, harvest once)...
    world.set_fact("(at vera hut)", false)?;
    world.set_fact("(at vera field)", true)?;
    world.set_fluent("(grain)", 1.0)?;

    // ...then the world drifts mid-plan (a bird ate a grain): stop, rethink.
    world.set_fluent("(grain)", 0.0)?;
    let rethink = world.replan_budgeted(100_000, Some(256));
    assert!(rethink.solved);
    println!(
        "rethink after drift: {} steps from the CURRENT state",
        rethink.plan.as_ref().unwrap().length
    );

    // A think can be too small to finish — the verdict is honest, the game
    // decides what to do with the tick (here: nothing crashed, no plan yet).
    let tiny = world.replan_budgeted(1, Some(1));
    println!(
        "tiny think: solved={} after {} evals (bounded, deterministic)",
        tiny.solved, tiny.statistics.evaluated_states
    );
    Ok(())
}
