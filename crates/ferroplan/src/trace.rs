//! Plan *trace*: replay a plan and capture the world state before the first
//! action and after every action — the instrumentation a UI needs to animate a
//! plan. Mirrors [`crate::verify`]'s replay loop but snapshots each intermediate
//! state. Classic/numeric/PDDL3 plans (a sequential op list) only; temporal
//! plans (overlapping durative actions) are not replayed this way.

use serde::{Deserialize, Serialize};

use crate::ground::ground_task;
use crate::packed::State;
use crate::parser::{parse_domain, parse_problem};

/// The set of true facts and defined fluents at one point in a plan, as
/// display strings (`(AT TRUCK1 LOC2)`, `(FUEL TRUCK1) = 30`).
#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct StateSnapshot {
    pub facts: Vec<String>,
    pub fluents: Vec<(String, f64)>,
}

/// Replay `plan` (a sequence of `(action, args)`, e.g. from a [`crate::Solution`])
/// over a freshly grounded task, returning the initial state followed by the
/// state after each action — so the result has `plan.len() + 1` snapshots.
/// Errors if grounding fails, an action isn't a grounded op, or it isn't
/// applicable in the reached state.
pub fn trace(
    domain_src: &str,
    problem_src: &str,
    plan: &[(String, Vec<String>)],
) -> Result<Vec<StateSnapshot>, String> {
    let domain = parse_domain(domain_src).map_err(|e| format!("domain: {e}"))?;
    let problem = parse_problem(problem_src).map_err(|e| format!("problem: {e}"))?;
    // Compile `:derived` axioms away, like the solve that produced the plan —
    // replaying against the raw problem would miss the derived init facts.
    let (domain, problem) = crate::derived::compile(&domain, &problem)?;
    let task = ground_task(&domain, &problem, 1)
        .ok_or_else(|| "grounding failed (empty type)".to_string())?;

    let snap = |s: &State| -> StateSnapshot {
        let facts = (0..task.n_facts)
            .filter(|&i| (s.bits[i / 64] >> (i % 64)) & 1 == 1)
            .map(|i| task.fact_names[i].clone())
            .collect();
        let fluents = (0..task.fluent_names.len())
            .filter(|&i| s.fdef[i])
            .map(|i| (task.fluent_names[i].clone(), s.fv[i]))
            .collect();
        StateSnapshot { facts, fluents }
    };

    let mut s = task.initial();
    let mut out = vec![snap(&s)];
    for (name, args) in plan {
        let want: Vec<&str> = args.iter().map(|x| x.as_str()).collect();
        let oi = (0..task.n_ops)
            .find(|&oi| {
                let mut it = task.op_display[oi].split_whitespace();
                it.next() == Some(name.as_str()) && it.eq(want.iter().copied())
            })
            .ok_or_else(|| {
                format!(
                    "plan action `{} {}` is not a grounded op",
                    name,
                    args.join(" ")
                )
            })?;
        if !task.op_applicable(oi, &s) {
            return Err(format!(
                "plan action `{} {}` is not applicable in the reached state",
                name,
                args.join(" ")
            ));
        }
        s = task.apply(oi, &s);
        out.push(snap(&s));
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    const DOM: &str = "
    (define (domain logi) (:requirements :typing)
      (:types location truck)
      (:predicates (at ?t - truck ?l - location) (road ?a ?b - location))
      (:action drive :parameters (?t - truck ?from ?to - location)
        :precondition (and (at ?t ?from) (road ?from ?to))
        :effect (and (not (at ?t ?from)) (at ?t ?to))))";
    const PRB: &str = "
    (define (problem p) (:domain logi)
      (:objects a b - location  t1 - truck)
      (:init (at t1 a) (road a b))
      (:goal (at t1 b)))";

    #[test]
    fn trace_captures_each_step() {
        let plan = vec![(
            "DRIVE".to_string(),
            vec!["T1".into(), "A".into(), "B".into()],
        )];
        let snaps = trace(DOM, PRB, &plan).expect("trace");
        assert_eq!(snaps.len(), 2, "initial + after the one action");
        assert!(snaps[0].facts.iter().any(|f| f == "(AT T1 A)"));
        assert!(!snaps[0].facts.iter().any(|f| f == "(AT T1 B)"));
        assert!(snaps[1].facts.iter().any(|f| f == "(AT T1 B)"));
        assert!(!snaps[1].facts.iter().any(|f| f == "(AT T1 A)"));
    }

    #[test]
    fn trace_rejects_inapplicable() {
        // driving from B (truck is at A) is not applicable
        let plan = vec![(
            "DRIVE".to_string(),
            vec!["T1".into(), "B".into(), "A".into()],
        )];
        assert!(trace(DOM, PRB, &plan).is_err());
    }
}
