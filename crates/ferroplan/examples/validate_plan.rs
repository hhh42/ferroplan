//! Independently check a plan — [`ferroplan::plan::validate_plan`].
//!
//! Run: cargo run --release -p ferroplan --example validate_plan
//!
//! `validate_plan` replays a plan file under ferroplan's OWN apply semantics
//! (auto-detecting classical vs. temporal), returning `Valid` or `Invalid(reason)`.
//! Here we solve a tiny classical problem, render its plan to the `step N: (…)` text
//! format, confirm it validates — then tamper with it and watch validation reject
//! the broken plan.
use ferroplan::plan::{validate_plan, Validity};

const DOMAIN: &str = "(define (domain gripper)
  (:requirements :strips :typing)
  (:types room ball gripper)
  (:predicates (at-robby ?r - room) (at ?b - ball ?r - room)
               (free ?g - gripper) (carry ?b - ball ?g - gripper))
  (:action move :parameters (?from ?to - room)
    :precondition (at-robby ?from)
    :effect (and (not (at-robby ?from)) (at-robby ?to)))
  (:action pick :parameters (?b - ball ?r - room ?g - gripper)
    :precondition (and (at ?b ?r) (at-robby ?r) (free ?g))
    :effect (and (carry ?b ?g) (not (at ?b ?r)) (not (free ?g))))
  (:action drop :parameters (?b - ball ?r - room ?g - gripper)
    :precondition (and (carry ?b ?g) (at-robby ?r))
    :effect (and (at ?b ?r) (free ?g) (not (carry ?b ?g)))))";

const PROBLEM: &str = "(define (problem gripper-1) (:domain gripper)
  (:objects rooma roomb - room b1 - ball left - gripper)
  (:init (at-robby rooma) (at b1 rooma) (free left))
  (:goal (at b1 roomb)))";

fn render(plan: &ferroplan::Plan) -> String {
    plan.steps
        .iter()
        .enumerate()
        .map(|(i, s)| format!("step {i}: ({} {})", s.action, s.args.join(" ")))
        .collect::<Vec<_>>()
        .join("\n")
}

fn main() -> Result<(), String> {
    let sol = ferroplan::solve(DOMAIN, PROBLEM, &ferroplan::Options::default())
        .map_err(|e| e.to_string())?;
    let plan = sol.plan.ok_or("expected a plan")?;
    let text = render(&plan);
    println!("solved plan:\n{text}\n");

    match validate_plan(DOMAIN, PROBLEM, &text)? {
        Validity::Valid => println!("=> the produced plan is Valid ✅"),
        Validity::Invalid(why) => println!("=> unexpectedly Invalid: {why}"),
    }

    // Drop the last step: the ball never reaches roomb, so the goal is unmet.
    let broken = text.rsplit_once('\n').map(|(head, _)| head).unwrap_or("");
    match validate_plan(DOMAIN, PROBLEM, broken)? {
        Validity::Valid => println!("=> truncated plan unexpectedly Valid"),
        Validity::Invalid(why) => println!("=> the truncated plan is Invalid ❌ — {why}"),
    }
    Ok(())
}
