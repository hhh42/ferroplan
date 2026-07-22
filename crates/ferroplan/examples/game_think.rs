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

    // ...then the world drifts mid-plan (a bird ate a grain). FOLLOW BEFORE
    // YOU RETHINK: the suffix replay is free — only a broken suffix spends a
    // think.
    world.set_fluent("(grain)", 0.0)?;
    let broken = !world.plan_still_valid(plan, 2);
    println!("drift broke the plan: {broken} (suffix replay, zero search)");
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

    // Act 2 (0.12): CONCURRENT work. The same bounded-think surface on a
    // TEMPORAL world — two smiths forge in parallel, the plan comes back
    // timed, and the game follows the overlapping intervals in real time.
    let forge_dom = "
    (define (domain forge) (:requirements :strips :typing :durative-actions)
      (:types smith)
      (:predicates (ready ?s - smith) (blade ?s - smith))
      (:durative-action forge
        :parameters (?s - smith)
        :duration (= ?duration 8)
        :condition (at start (ready ?s))
        :effect (and (at start (not (ready ?s))) (at end (blade ?s)))))";
    let forge_prb = "
    (define (problem two-blades) (:domain forge)
      (:objects anvil-a anvil-b - smith)
      (:init (ready anvil-a) (ready anvil-b))
      (:goal (and (blade anvil-a) (blade anvil-b))))";
    let mut forge = Session::new(forge_dom, forge_prb, &Options::default())?;
    let think = forge.replan_budgeted(50_000, Some(128));
    assert!(think.solved);
    let plan = think.plan.as_ref().unwrap();
    println!(
        "temporal think: {} steps, makespan {:.3} (concurrent — sequential would be 16)",
        plan.length,
        plan.makespan.unwrap()
    );
    for s in &plan.steps {
        println!(
            "  {:>6.3}: {} {} [{}]",
            s.time.unwrap(),
            s.action,
            s.args.join(" "),
            s.duration.unwrap()
        );
    }
    // anvil-a's blade finishes out-of-band; the rethink covers only anvil-b.
    forge.set_fact("(blade anvil-a)", true)?;
    forge.set_fact("(ready anvil-a)", false)?;
    let rethink = forge.replan_budgeted(50_000, Some(128));
    assert!(rethink.solved);
    println!(
        "temporal rethink after drift: {} step(s)",
        rethink.plan.as_ref().unwrap().length
    );

    // Act 3 (0.13): the BARTER CHAIN — a forked mind with a retargetable
    // desire in the vendored bazaar. Vendors release goods only for the
    // item they want, so wanting the depth-3 item means planning a 3-hop
    // trade-up chain; drift can make a desire IMPOSSIBLE, and the honest
    // unsolved verdict is what lets the NPC settle for less.
    let market = Session::new(
        include_str!("../../../benchmarks/bench/bazaar-chain-domain.pddl"),
        include_str!("../../../benchmarks/bench/bazaar-chain.pddl"),
        &Options::default(),
    )?;
    let mut trader = market.fork(); // one mind of a possible population
    trader.set_goal("(has a0 item3)")?;
    let think = trader.replan_budgeted(10_000, Some(64));
    assert!(think.solved);
    let plan = think.plan.as_ref().unwrap();
    println!("barter think: {}-hop trade chain", plan.length);
    for s in &plan.steps {
        println!("  {} {}", s.action, s.args.join(" "));
    }
    // Drift: v2 already bartered item2 away to v3 (an off-screen trade —
    // and the only legal destination: grounding knows items move along
    // want-edges, so `(has v5 item2)` isn't even in the fact space).
    trader.set_fact("(has v2 item2)", false)?;
    trader.set_fact("(has v3 item2)", true)?;
    let broken = !trader.plan_still_valid(plan, 0);
    let rethink = trader.replan_budgeted(10_000, Some(64));
    println!(
        "drift broke the chain: {broken}; rethink solved={} — the desire {}",
        rethink.solved,
        if rethink.solved {
            "survives by another route"
        } else {
            "is IMPOSSIBLE now (honest verdict, no wasted wandering)"
        }
    );
    // The NPC settles for the reachable rung instead — one world, changing
    // desires, zero regrounding.
    trader.set_goal("(has a0 item1)")?;
    let settle = trader.replan_budgeted(10_000, Some(64));
    assert!(settle.solved);
    println!(
        "retargeted desire: {} step(s) to the reachable rung",
        settle.plan.as_ref().unwrap().length
    );

    // Act 4 (0.14): the SCHEDULED world. The kiln district's fuel line
    // closes at a known hour — `set_timed_fact` schedules the clock-relative
    // event, and thinks plan AROUND the window: fire early, or wait through
    // a scheduled outage for the line's return.
    let kiln_dom = "
    (define (domain kilnyard) (:requirements :strips :typing :durative-actions)
      (:types pot)
      (:predicates (raw ?p - pot) (glazed ?p - pot) (fired ?p - pot) (fuel))
      (:durative-action glaze :parameters (?p - pot) :duration (= ?duration 4)
        :condition (at start (raw ?p))
        :effect (and (at start (not (raw ?p))) (at end (glazed ?p))))
      (:durative-action fire :parameters (?p - pot) :duration (= ?duration 6)
        :condition (and (at start (glazed ?p)) (at start (fuel)))
        :effect (at end (fired ?p)))
      (:durative-action fuel-line :parameters () :duration (= ?duration 1)
        :condition (at start (fuel))
        :effect (and (at start (not (fuel))) (at end (fuel)))))";
    let kiln_prb = "
    (define (problem morning-batch) (:domain kilnyard)
      (:objects urn - pot)
      (:init (raw urn) (fuel))
      (:goal (fired urn)))";
    let mut yard = Session::new(kiln_dom, kiln_prb, &Options::default())?;
    // The fuel line shuts at t=12: glaze (4) then fire (6) fits if the firing
    // starts inside the window.
    yard.set_timed_fact(12.0, "(fuel)", false)?;
    let think = yard.replan_budgeted(50_000, Some(128));
    assert!(think.solved);
    let fire = think
        .plan
        .as_ref()
        .unwrap()
        .steps
        .iter()
        .find(|s| s.action == "FIRE")
        .unwrap();
    println!(
        "scheduled world: fuel dies at 12; the think fires at t={:.1} — inside the window",
        fire.time.unwrap()
    );
    // A maintenance outage with a KNOWN end: fuel out at 2, back at 9. The
    // think waits through it (the agenda carries the repair).
    let mut yard = Session::new(kiln_dom, kiln_prb, &Options::default())?;
    yard.set_timed_fact(2.0, "(fuel)", false)?;
    yard.set_timed_fact(9.0, "(fuel)", true)?;
    let think = yard.replan_budgeted(50_000, Some(128));
    assert!(think.solved);
    let fire = think
        .plan
        .as_ref()
        .unwrap()
        .steps
        .iter()
        .find(|s| s.action == "FIRE")
        .unwrap();
    println!(
        "scheduled outage [2, 9): the think WAITS and fires at t={:.1}",
        fire.time.unwrap()
    );
    Ok(())
}
