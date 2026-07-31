fn solve_model(
    mdp: &ExplicitMdp,
    options: &ProbabilisticOptions,
) -> Result<(ProbabilisticSolution, DynamicResult), PpddlError> {
    let dynamic = if let Some(horizon) = options.horizon {
        finite_horizon(mdp, options, horizon)?
    } else {
        infinite_horizon(mdp, options)?
    };
    let policy = if let Some(horizon) = options.horizon {
        extract_finite_policy(mdp, &dynamic, horizon, options.max_policy_entries)?
    } else {
        extract_stationary_policy(mdp, &dynamic, options.max_policy_entries)?
    };
    let values = if let Some(horizon) = options.horizon {
        &dynamic.values[horizon]
    } else {
        &dynamic.values[0]
    };
    let initial_value = initial_value(mdp, values, mdp.model.objective)?;
    let initial_action = initial_action_name(mdp, &policy, options.horizon);
    let map = policy
        .iter()
        .map(|decision| ((decision.state, decision.remaining), decision))
        .collect::<HashMap<_, _>>();
    let policy_closed = mdp.initial.iter().all(|initial| {
        mdp.goal[initial.state] || map.contains_key(&(initial.state, options.horizon))
    });
    let all_initial_goals = mdp.initial.iter().all(|entry| mdp.goal[entry.state]);
    let solved = all_initial_goals
        || (policy_closed && (options.horizon.is_some() || dynamic.converged));
    let mut notes = Vec::new();
    if options.horizon.is_none() && !dynamic.converged {
        notes.push(format!(
            "value iteration reached max_iterations={} before epsilon={}",
            options.max_iterations, options.epsilon
        ));
    }
    if all_initial_goals {
        notes.push("every initial state already satisfies the absorbing goal".into());
    }
    if mdp.initial.len() > 1 {
        notes.push(format!(
            "policy synthesized over {} initial states",
            mdp.initial.len()
        ));
    }
    Ok((
        ProbabilisticSolution {
            solved,
            objective: mdp.model.objective,
            initial_value,
            initial_distribution: mdp
                .initial
                .iter()
                .map(|entry| InitialStateProbability {
                    state: entry.state,
                    probability: entry.probability,
                    goal: mdp.goal[entry.state],
                })
                .collect(),
            states: project_states(mdp),
            initial_action,
            horizon: options.horizon,
            discount: options.discount,
            declared_metric: mdp.model.metric_text.clone(),
            policy,
            statistics: ProbabilisticStatistics {
                grounded_facts: mdp.model.task.n_reach_facts,
                grounded_outcome_operators: mdp.model.task.n_reach_actions,
                grounded_actions: mdp.model.actions.len(),
                initial_states: mdp.initial.len(),
                reachable_states: mdp.states.len(),
                transitions: mdp.transitions,
                iterations: dynamic.iterations,
                converged: dynamic.converged,
                threads: mdp.model.threads,
            },
            notes,
        },
        dynamic,
    ))
}

/// Compile a PPDDL domain/problem into an explicit MDP and synthesize a policy.
pub fn solve_ppddl(
    domain_src: &str,
    problem_src: &str,
    options: &ProbabilisticOptions,
) -> Result<ProbabilisticSolution, PpddlError> {
    let model = compile_model(domain_src, problem_src, options)?;
    let mdp = build_mdp(model, options)?;
    solve_model(&mdp, options).map(|result| result.0)
}
