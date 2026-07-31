/// Recompile and structurally validate a synthesized policy against the PPDDL
/// transition graph. This checks initial probability mass, action identity,
/// successor identity, reward, closure, and transition probability mass.
pub fn validate_ppddl_policy(
    domain_src: &str,
    problem_src: &str,
    options: &ProbabilisticOptions,
    solution: &ProbabilisticSolution,
) -> Result<PolicyValidation, PpddlError> {
    let model = compile_model(domain_src, problem_src, options)?;
    let mdp = build_mdp(model, options)?;
    let dynamic = if let Some(horizon) = options.horizon {
        finite_horizon(&mdp, options, horizon)?
    } else {
        infinite_horizon(&mdp, options)?
    };
    let values = if let Some(horizon) = options.horizon {
        &dynamic.values[horizon]
    } else {
        &dynamic.values[0]
    };
    let expected_initial_value = initial_value(&mdp, values, mdp.model.objective)?;
    let policy = policy_map(solution);
    let mut errors = Vec::new();
    let mut max_probability_error = 0.0f64;

    if solution.objective != mdp.model.objective {
        errors.push("policy objective differs from the PPDDL objective".into());
    }
    if solution.horizon != options.horizon {
        errors.push("policy horizon differs from the verifier horizon".into());
    }
    if (solution.discount - options.discount).abs() > 1e-12 {
        errors.push("policy discount differs from the verifier discount".into());
    }
    if (solution.initial_value - expected_initial_value).abs() > 1e-8 {
        errors.push("reported initial value differs from independent dynamic programming".into());
    }
    if solution.declared_metric != mdp.model.metric_text {
        errors.push("declared metric projection differs from the PPDDL problem".into());
    }
    if policy.len() != solution.policy.len() {
        errors.push("policy contains duplicate state/horizon decisions".into());
    }
    let expected_initial_mass: f64 = mdp.initial.iter().map(|entry| entry.probability).sum();
    let observed_initial_mass: f64 = solution
        .initial_distribution
        .iter()
        .map(|entry| entry.probability)
        .sum();
    max_probability_error =
        max_probability_error.max((observed_initial_mass - 1.0).abs());
    if (expected_initial_mass - observed_initial_mass).abs() > 1e-9
        || solution.initial_distribution.len() != mdp.initial.len()
    {
        errors.push("initial-state distribution shape changed".into());
    } else {
        for (expected, observed) in mdp.initial.iter().zip(&solution.initial_distribution) {
            if expected.state != observed.state
                || (expected.probability - observed.probability).abs() > 1e-9
                || mdp.goal[expected.state] != observed.goal
            {
                errors.push("initial-state distribution mismatch".into());
            }
        }
    }

    let expected_states = project_states(&mdp);
    if solution.states.len() != expected_states.len() {
        errors.push("reachable-state projection shape changed".into());
    } else {
        for (expected, observed) in expected_states.iter().zip(&solution.states) {
            if expected.id != observed.id
                || expected.facts != observed.facts
                || expected.fluents != observed.fluents
                || expected.goal != observed.goal
                || (expected.initial_probability - observed.initial_probability).abs() > 1e-9
            {
                errors.push(format!("reachable-state projection mismatch at state {}", expected.id));
            }
        }
    }

    for decision in &solution.policy {
        if options.horizon.is_some() != decision.remaining.is_some() {
            errors.push(format!(
                "state {} decision has the wrong finite/stationary policy shape",
                decision.state
            ));
        }
        if decision.state >= mdp.states.len() {
            errors.push(format!("policy references unknown state {}", decision.state));
            continue;
        }
        let action_row = mdp.actions[decision.state].iter().position(|candidate| {
            let action = &mdp.model.actions[candidate.action];
            action.base_name == decision.action && action.args == decision.args
        });
        let Some(action_row) = action_row else {
            errors.push(format!(
                "state {} has no applicable action {} {}",
                decision.state,
                decision.action,
                decision.args.join(" ")
            ));
            continue;
        };
        let state_action = &mdp.actions[decision.state][action_row];
        let successor_time = decision
            .remaining
            .and_then(|remaining| {
                options.horizon.and_then(|horizon| {
                    horizon
                        .checked_sub(remaining)
                        .map(|elapsed| (elapsed + 1) as f64)
                })
            })
            .unwrap_or(0.0);
        let expected = &state_action.transitions;
        let (chosen_value, optimal_value) = if let Some(remaining) = decision.remaining {
            if remaining == 0 || remaining >= dynamic.values.len() {
                errors.push(format!(
                    "state {} has a decision outside the finite-horizon value layers",
                    decision.state
                ));
                (f64::NAN, f64::NAN)
            } else {
                let discount = if is_reward_objective(mdp.model.objective) {
                    options.discount
                } else {
                    1.0
                };
                (
                    q_value(
                        &mdp,
                        mdp.model.objective,
                        discount,
                        state_action,
                        &dynamic.values[remaining - 1],
                        successor_time,
                    )?,
                    dynamic.values[remaining][decision.state],
                )
            }
        } else {
            let discount = if is_reward_objective(mdp.model.objective) {
                options.discount
            } else {
                1.0
            };
            (
                q_value(
                    &mdp,
                    mdp.model.objective,
                    discount,
                    state_action,
                    &dynamic.values[0],
                    0.0,
                )?,
                dynamic.values[0][decision.state],
            )
        };
        if chosen_value.is_finite()
            && ((chosen_value - optimal_value).abs() > 1e-8
                || (decision.value - optimal_value).abs() > 1e-8)
        {
            errors.push(format!(
                "state {} action {} fails the Bellman optimality witness",
                decision.state, decision.action
            ));
        }
        let expected_mass: f64 = expected
            .iter()
            .map(|transition| transition.probability)
            .sum();
        let observed_mass: f64 = decision
            .outcomes
            .iter()
            .map(|outcome| outcome.probability)
            .sum();
        max_probability_error =
            max_probability_error.max((observed_mass - 1.0).abs());
        if (expected_mass - observed_mass).abs() > 1e-9
            || expected.len() != decision.outcomes.len()
        {
            errors.push(format!(
                "state {} action {} outcome distribution shape changed",
                decision.state, decision.action
            ));
            continue;
        }
        for (expected, observed) in expected.iter().zip(&decision.outcomes) {
            let expected_reward = transition_reward_at(&mdp, expected, successor_time)?;
            if (expected.probability - observed.probability).abs() > 1e-9
                || expected.next != observed.next_state
                || (expected_reward - observed.reward).abs() > 1e-9
                || mdp.goal[expected.next] != observed.goal
            {
                errors.push(format!(
                    "state {} action {} outcome mismatch",
                    decision.state, decision.action
                ));
            }
        }
        if let Some(remaining) = decision.remaining {
            if remaining == 0 {
                errors.push(format!(
                    "state {} has a zero-horizon policy decision",
                    decision.state
                ));
            }
            for outcome in &decision.outcomes {
                if remaining > 1
                    && !outcome.goal
                    && !policy.contains_key(&(outcome.next_state, Some(remaining - 1)))
                    && !mdp.actions[outcome.next_state].is_empty()
                {
                    errors.push(format!(
                        "finite policy is not closed at state {} with {} steps remaining",
                        outcome.next_state,
                        remaining - 1
                    ));
                }
            }
        } else {
            for outcome in &decision.outcomes {
                if !outcome.goal
                    && !policy.contains_key(&(outcome.next_state, None))
                    && !mdp.actions[outcome.next_state].is_empty()
                {
                    errors.push(format!(
                        "stationary policy is not closed at state {}",
                        outcome.next_state
                    ));
                }
            }
        }
    }

    for initial in &mdp.initial {
        if !mdp.goal[initial.state]
            && !policy.contains_key(&(initial.state, options.horizon))
        {
            errors.push(format!(
                "policy has no decision for initial state {}",
                initial.state
            ));
        }
    }

    Ok(PolicyValidation {
        valid: errors.is_empty(),
        checked_decisions: solution.policy.len(),
        max_probability_error,
        errors,
    })
}
