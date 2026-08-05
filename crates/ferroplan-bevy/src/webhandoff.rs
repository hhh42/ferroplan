//! Bounded browser handoff from the solver page into the Bevy visualizer.
//!
//! The handoff is candidate data, never authority. Domain, problem, and JSON
//! sizes are bounded before deserialization. A supplied plan is independently
//! validated against the supplied model before it may enter the animation
//! timeline.

use bevy::prelude::*;

use crate::anim::{load_result, result_from_solution, Plan};
use crate::scene::Scene;

const KEY: &str = "ferroplan.handoff";
const MAX_HANDOFF_BYTES: usize = 8 * 1024 * 1024;
const MAX_MODEL_BYTES: usize = 4 * 1024 * 1024;

pub(crate) fn try_load(scene: &mut Scene, plan: &mut Plan) -> bool {
    let Some(raw) = read_local_storage(KEY) else {
        return false;
    };
    if raw.len() > MAX_HANDOFF_BYTES {
        warn("handoff exceeds the 8 MiB input limit; refusing");
        return false;
    }
    let Ok(value) = serde_json::from_str::<serde_json::Value>(&raw) else {
        warn("invalid JSON; refusing");
        return false;
    };
    let (Some(domain), Some(problem)) = (
        value.get("domain").and_then(|item| item.as_str()),
        value.get("problem").and_then(|item| item.as_str()),
    ) else {
        warn("missing domain/problem; refusing");
        return false;
    };
    if domain.is_empty()
        || problem.is_empty()
        || domain.len() > MAX_MODEL_BYTES
        || problem.len() > MAX_MODEL_BYTES
    {
        warn("domain/problem is empty or exceeds the 4 MiB model limit; refusing");
        return false;
    }

    scene.load_src(domain);
    scene.load_src(problem);

    if let Some(solution_value) = value.get("solution") {
        match serde_json::from_value::<ferroplan::Solution>(solution_value.clone()) {
            Ok(solution) if candidate_solution_valid(domain, problem, &solution) => {
                let result = result_from_solution(domain, problem, solution);
                load_result(plan, result, true);
                plan.status.push_str(" · candidate-only · independently validated");
            }
            Ok(_) => warn("candidate solution failed independent validation; scene loaded without plan"),
            Err(error) => warn(&format!(
                "candidate solution did not parse ({error}); scene loaded without plan"
            )),
        }
    }
    true
}

fn candidate_solution_valid(domain: &str, problem: &str, solution: &ferroplan::Solution) -> bool {
    if !solution.solved {
        return solution.plan.is_none();
    }
    let Some(plan) = solution.plan.as_ref() else {
        return false;
    };
    if plan.length != plan.steps.len() {
        return false;
    }
    let plan_text = render_plan(plan);
    matches!(
        ferroplan::plan::validate_plan(domain, problem, &plan_text),
        Ok(ferroplan::plan::Validity::Valid)
    )
}

fn render_plan(plan: &ferroplan::Plan) -> String {
    let temporal = plan.steps.iter().any(|step| step.time.is_some());
    let mut output = String::new();
    for step in &plan.steps {
        let args = if step.args.is_empty() {
            String::new()
        } else {
            format!(" {}", step.args.join(" "))
        };
        if temporal {
            output.push_str(&format!(
                "{:.6}: ({}{args}) [{:.6}]\n",
                step.time.unwrap_or(0.0),
                step.action,
                step.duration.unwrap_or(0.0)
            ));
        } else {
            output.push_str(&format!("step {}: {}{args}\n", step.index, step.action));
        }
    }
    output
}

fn warn(message: &str) {
    web_sys::console::warn_1(&format!("ferroplan.handoff: {message}").into());
}

fn read_local_storage(key: &str) -> Option<String> {
    web_sys::window()?
        .local_storage()
        .ok()??
        .get_item(key)
        .ok()?
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn solved_without_plan_is_refused() {
        let solution = ferroplan::Solution {
            solved: true,
            mode: ferroplan::Mode::Ff,
            plan: None,
            statistics: ferroplan::Statistics::default(),
            notes: vec![],
        };
        assert!(!candidate_solution_valid("", "", &solution));
    }
}