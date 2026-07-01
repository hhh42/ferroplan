//! Built-in plan validator — check a plan against ferroplan's OWN semantics.
//!
//! The external VAL validator enforces strict textbook PDDL2.1 (e.g. it forbids
//! two durative actions touching the same numeric fluent at once, and demands
//! ε-separation at shared timestamps). ferroplan's temporal engine deliberately
//! uses a sequential decision-epoch model, so VAL rejects perfectly good
//! ferroplan plans. This module replays a plan the SAME way the planner applies
//! actions — reusing [`verify::verify`] (classical) and [`temporal::validate`]
//! (temporal) — so "valid here" means "valid under the semantics that produced it".
//!
//! It also supplies the one missing piece: a parser for the two plan text formats
//! `ff` emits — classical `step N: NAME ARGS` and temporal IPC `t: (name args) [d]`.

use crate::parser;
use crate::temporal::{self, TimedPlan, TimedStep};
use crate::verify;

/// Outcome of validating a plan.
#[derive(Debug, Clone, PartialEq)]
pub enum Validity {
    Valid,
    Invalid(String),
}

/// Parse a classical `ff` text plan into `(NAME, [ARGS])` pairs (uppercased to
/// match grounded-operator display names). Accepts the `step N:` first line and
/// the bare `N:` continuation lines, with or without parens around the action,
/// and ignores banner / timing / `REACH-GOAL` lines.
pub fn parse_classical(src: &str) -> Vec<(String, Vec<String>)> {
    let mut out = Vec::new();
    for line in src.lines() {
        let body = match step_body(line.trim()) {
            Some(b) => b,
            None => continue,
        };
        let body = body
            .trim()
            .trim_start_matches('(')
            .trim_end_matches(')')
            .trim();
        let mut toks = body.split_whitespace().map(|t| t.to_uppercase());
        let name = match toks.next() {
            Some(n) => n,
            None => continue,
        };
        if name == "REACH-GOAL" {
            continue;
        }
        out.push((name, toks.collect()));
    }
    out
}

/// If `line` is `[step ]<digits>: <rest>`, return `<rest>`. Else None.
fn step_body(line: &str) -> Option<&str> {
    let line = line
        .strip_prefix("step")
        .map(str::trim_start)
        .unwrap_or(line);
    let (head, rest) = line.split_once(':')?;
    if !head.trim().is_empty() && head.trim().chars().all(|c| c.is_ascii_digit()) {
        Some(rest)
    } else {
        None
    }
}

/// Parse a temporal IPC plan (`t: (name args) [dur]`) into a [`TimedPlan`].
/// Action strings are uppercased to match grounded-operator names; banner and
/// `plan makespan:` lines are ignored.
pub fn parse_timed(src: &str) -> Result<TimedPlan, String> {
    let mut steps = Vec::new();
    let mut makespan = 0.0_f64;
    for line in src.lines() {
        let line = line.trim();
        let (time_str, rest) = match line.split_once(':') {
            Some(x) => x,
            None => continue,
        };
        let time: f64 = match time_str.trim().parse() {
            Ok(t) => t,
            Err(_) => continue, // not a `<float>: ...` plan line
        };
        let rest = rest.trim();
        if !rest.starts_with('(') {
            continue;
        }
        let close = match rest.find(')') {
            Some(i) => i,
            None => continue,
        };
        let action = rest[1..close]
            .split_whitespace()
            .map(|t| t.to_uppercase())
            .collect::<Vec<_>>()
            .join(" ");
        if action.is_empty() || action.starts_with("REACH-GOAL") {
            continue;
        }
        // duration in the trailing [ ... ]
        let after = &rest[close + 1..];
        let duration = after
            .find('[')
            .and_then(|i| {
                after[i + 1..]
                    .find(']')
                    .map(|j| after[i + 1..i + 1 + j].trim())
            })
            .and_then(|d| d.parse::<f64>().ok());
        makespan = makespan.max(time + duration.unwrap_or(0.0));
        steps.push(TimedStep {
            time,
            action,
            duration,
        });
    }
    if steps.is_empty() {
        return Err("no plan steps found in plan file".into());
    }
    Ok(TimedPlan { steps, makespan })
}

/// Validate `plan_src` against the domain/problem under ferroplan's own semantics.
/// Auto-detects classical vs temporal from the domain (durative actions => temporal).
pub fn validate_plan(
    domain_src: &str,
    problem_src: &str,
    plan_src: &str,
) -> Result<Validity, String> {
    let domain = parser::parse_domain(domain_src).map_err(|e| format!("domain: {}", e))?;
    if temporal::is_temporal(&domain) {
        let problem = parser::parse_problem(problem_src).map_err(|e| format!("problem: {}", e))?;
        // Compile `:derived` axioms away first, exactly like every solve path —
        // validating the raw problem grounds the goal/ops without the derived
        // facts and wrongly rejects valid plans ("grounds to unsolvable").
        let (domain, problem) = crate::derived::compile(&domain, &problem)?;
        let plan = parse_timed(plan_src)?;
        Ok(match temporal::validate(&domain, &problem, &plan) {
            Ok(()) => Validity::Valid,
            Err(why) => Validity::Invalid(why),
        })
    } else {
        let plan = parse_classical(plan_src);
        if plan.is_empty() {
            return Err("no plan steps found in plan file".into());
        }
        Ok(match verify::verify(domain_src, problem_src, &plan) {
            Ok(v) if v.hard_goal_met => Validity::Valid,
            Ok(_) => Validity::Invalid("plan executes but does not achieve the goal".into()),
            Err(why) => Validity::Invalid(why),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // A tiny instantaneous numeric chain: r0 -> r1 -> r2.
    const CLASSICAL_DOMAIN: &str = "(define (domain c)\n\
        (:requirements :strips :numeric-fluents)\n\
        (:functions (a) (b) (c))\n\
        (:action ab :parameters () :precondition (>= (a) 1)\n\
          :effect (and (decrease (a) 1) (increase (b) 1)))\n\
        (:action bc :parameters () :precondition (>= (b) 1)\n\
          :effect (and (decrease (b) 1) (increase (c) 1))))";
    const CLASSICAL_PROBLEM: &str = "(define (problem p) (:domain c)\n\
        (:init (= (a) 1) (= (b) 0) (= (c) 0)) (:goal (>= (c) 1)))";

    // A tiny durative version of the same chain.
    const TEMPORAL_DOMAIN: &str = "(define (domain ct)\n\
        (:requirements :strips :durative-actions :numeric-fluents)\n\
        (:functions (a) (b) (c))\n\
        (:durative-action ab :parameters () :duration (= ?duration 2)\n\
          :condition (at start (>= (a) 1))\n\
          :effect (and (at start (decrease (a) 1)) (at end (increase (b) 1))))\n\
        (:durative-action bc :parameters () :duration (= ?duration 2)\n\
          :condition (at start (>= (b) 1))\n\
          :effect (and (at start (decrease (b) 1)) (at end (increase (c) 1)))))";

    #[test]
    fn classical_valid_and_invalid() {
        let good = "step    0: AB\n        1: BC\n";
        assert_eq!(
            validate_plan(CLASSICAL_DOMAIN, CLASSICAL_PROBLEM, good).unwrap(),
            Validity::Valid
        );
        // dropping the last action leaves the goal unmet
        let short = "step    0: AB\n";
        assert!(matches!(
            validate_plan(CLASSICAL_DOMAIN, CLASSICAL_PROBLEM, short).unwrap(),
            Validity::Invalid(_)
        ));
        // an inapplicable action (BC before any B exists) is rejected
        let bad = "step    0: BC\n";
        assert!(matches!(
            validate_plan(CLASSICAL_DOMAIN, CLASSICAL_PROBLEM, bad).unwrap(),
            Validity::Invalid(_)
        ));
    }

    #[test]
    fn temporal_valid_and_invalid() {
        let good = "0.000: (ab) [2.000]\n2.000: (bc) [2.000]\n";
        assert_eq!(
            validate_plan(TEMPORAL_DOMAIN, CLASSICAL_PROBLEM, good).unwrap(),
            Validity::Valid
        );
        // wrong stated duration is caught against the domain's duration expression
        let bad_dur = "0.000: (ab) [9.000]\n2.000: (bc) [2.000]\n";
        assert!(matches!(
            validate_plan(TEMPORAL_DOMAIN, CLASSICAL_PROBLEM, bad_dur).unwrap(),
            Validity::Invalid(_)
        ));
    }

    #[test]
    fn parsers_skip_banner_lines() {
        let txt = "ff: parsing domain file\nstep    0: AB\ntime spent: 0.00 seconds\n";
        assert_eq!(parse_classical(txt), vec![("AB".to_string(), vec![])]);
    }
}
