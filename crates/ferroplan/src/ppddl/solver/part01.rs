struct DynamicResult {
    values: Vec<Vec<f64>>,
    choices: Vec<Vec<Option<usize>>>,
    iterations: usize,
    converged: bool,
}

fn better(objective: ProbabilisticObjective, candidate: f64, current: f64) -> bool {
    if is_minimizing(objective) {
        candidate < current - PROB_EPS
    } else {
        candidate > current + PROB_EPS
    }
}

fn finite_horizon(
    mdp: &ExplicitMdp,
    options: &ProbabilisticOptions,
    horizon: usize,
) -> Result<DynamicResult, PpddlError> {
    let objective = mdp.model.objective;
    let state_count = mdp.states.len();
    let cells = state_count
        .checked_mul(horizon.saturating_add(1))
        .ok_or(PpddlError::ValueTableLimit {
            limit: options.max_value_cells,
        })?;
    if cells > options.max_value_cells {
        return Err(PpddlError::ValueTableLimit {
            limit: options.max_value_cells,
        });
    }
    let mut layers = vec![base_values(mdp, objective, horizon as f64)?];
    let mut choices = vec![vec![None; state_count]];
    for remaining in 1..=horizon {
        let previous = layers.last().expect("base layer exists");
        let current_time = (horizon - remaining) as f64;
        let mut next = base_values(mdp, objective, current_time)?;
        let mut layer_choices = vec![None; state_count];
        for state in 0..state_count {
            if mdp.goal[state] {
                continue;
            }
            let mut best = if is_minimizing(objective) {
                f64::INFINITY
            } else {
                f64::NEG_INFINITY
            };
            let mut best_action = None;
            for (index, action) in mdp.actions[state].iter().enumerate() {
                let discount = if is_reward_objective(objective) {
                    options.discount
                } else {
                    1.0
                };
                let value = q_value(
                    mdp,
                    objective,
                    discount,
                    action,
                    previous,
                    current_time + 1.0,
                )?;
                if better(objective, value, best) {
                    best = value;
                    best_action = Some(index);
                }
            }
            if let Some(index) = best_action {
                next[state] = best;
                layer_choices[state] = Some(index);
            }
        }
        layers.push(next);
        choices.push(layer_choices);
    }
    Ok(DynamicResult {
        values: layers,
        choices,
        iterations: horizon,
        converged: true,
    })
}

fn infinite_horizon(
    mdp: &ExplicitMdp,
    options: &ProbabilisticOptions,
) -> Result<DynamicResult, PpddlError> {
    let objective = mdp.model.objective;
    let state_count = mdp.states.len();
    let mut values = base_values(mdp, objective, 0.0)?;
    let mut choices = vec![None; state_count];
    let mut converged = false;
    let mut iterations = 0;
    for iteration in 1..=options.max_iterations {
        let previous = values.clone();
        let mut delta = 0.0f64;
        for state in 0..state_count {
            if mdp.goal[state] {
                continue;
            }
            let mut best = if is_minimizing(objective) {
                f64::INFINITY
            } else {
                f64::NEG_INFINITY
            };
            let mut best_action = None;
            for (index, action) in mdp.actions[state].iter().enumerate() {
                let discount = if is_reward_objective(objective) {
                    options.discount
                } else {
                    1.0
                };
                let value = q_value(mdp, objective, discount, action, &previous, 0.0)?;
                if better(objective, value, best) {
                    best = value;
                    best_action = Some(index);
                }
            }
            if let Some(index) = best_action {
                values[state] = best;
                choices[state] = Some(index);
            }
            delta = delta.max((values[state] - previous[state]).abs());
        }
        iterations = iteration;
        if delta <= options.epsilon {
            converged = true;
            break;
        }
    }
    Ok(DynamicResult {
        values: vec![values],
        choices: vec![choices],
        iterations,
        converged,
    })
}

fn decision(
    mdp: &ExplicitMdp,
    state: usize,
    remaining: Option<usize>,
    action_row: usize,
    value: f64,
    successor_time: f64,
) -> Result<PolicyDecision, PpddlError> {
    let state_action = &mdp.actions[state][action_row];
    let action = &mdp.model.actions[state_action.action];
    Ok(PolicyDecision {
        state,
        remaining,
        action: action.base_name.clone(),
        args: action.args.clone(),
        value,
        outcomes: state_action
            .transitions
            .iter()
            .map(|transition| {
                Ok(PolicyOutcome {
                    probability: transition.probability,
                    next_state: transition.next,
                    reward: transition_reward_at(mdp, transition, successor_time)?,
                    goal: mdp.goal[transition.next],
                })
            })
            .collect::<Result<Vec<_>, PpddlError>>()?,
    })
}
