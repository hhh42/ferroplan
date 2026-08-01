fn extract_finite_policy(
    mdp: &ExplicitMdp,
    dynamic: &DynamicResult,
    horizon: usize,
    limit: usize,
) -> Result<Vec<PolicyDecision>, PpddlError> {
    let mut policy = Vec::new();
    let mut queue = VecDeque::new();
    for initial in &mdp.initial {
        queue.push_back((initial.state, horizon));
    }
    let mut seen = HashSet::new();
    while let Some((state, remaining)) = queue.pop_front() {
        if remaining == 0 || mdp.goal[state] || !seen.insert((state, remaining)) {
            continue;
        }
        let Some(action_row) = dynamic.choices[remaining][state] else {
            continue;
        };
        if policy.len() >= limit {
            return Err(PpddlError::PolicyLimit { limit });
        }
        let selected = &mdp.actions[state][action_row];
        for transition in &selected.transitions {
            queue.push_back((transition.next, remaining - 1));
        }
        policy.push(decision(
            mdp,
            state,
            Some(remaining),
            action_row,
            dynamic.values[remaining][state],
            (horizon - remaining + 1) as f64,
        )?);
    }
    policy.sort_by_key(|entry| (entry.remaining.unwrap_or(usize::MAX), entry.state));
    Ok(policy)
}

fn extract_stationary_policy(
    mdp: &ExplicitMdp,
    dynamic: &DynamicResult,
    limit: usize,
) -> Result<Vec<PolicyDecision>, PpddlError> {
    let values = &dynamic.values[0];
    let choices = &dynamic.choices[0];
    let mut policy = Vec::new();
    let mut queue = VecDeque::new();
    for initial in &mdp.initial {
        queue.push_back(initial.state);
    }
    let mut seen = HashSet::new();
    while let Some(state) = queue.pop_front() {
        if mdp.goal[state] || !seen.insert(state) {
            continue;
        }
        let Some(action_row) = choices[state] else {
            continue;
        };
        if policy.len() >= limit {
            return Err(PpddlError::PolicyLimit { limit });
        }
        let selected = &mdp.actions[state][action_row];
        for transition in &selected.transitions {
            queue.push_back(transition.next);
        }
        policy.push(decision(mdp, state, None, action_row, values[state], 0.0)?);
    }
    policy.sort_by_key(|entry| entry.state);
    Ok(policy)
}

fn policy_map(
    solution: &ProbabilisticSolution,
) -> HashMap<(usize, Option<usize>), &PolicyDecision> {
    solution
        .policy
        .iter()
        .map(|decision| ((decision.state, decision.remaining), decision))
        .collect()
}

fn initial_action_name(
    mdp: &ExplicitMdp,
    policy: &[PolicyDecision],
    horizon: Option<usize>,
) -> Option<String> {
    let policy = policy
        .iter()
        .map(|decision| ((decision.state, decision.remaining), decision))
        .collect::<HashMap<_, _>>();
    let mut selected = None::<String>;
    for initial in &mdp.initial {
        if mdp.goal[initial.state] {
            continue;
        }
        let decision = policy.get(&(initial.state, horizon))?;
        let display = std::iter::once(decision.action.clone())
            .chain(decision.args.iter().cloned())
            .collect::<Vec<_>>()
            .join(" ");
        match &selected {
            Some(existing) if existing != &display => return None,
            Some(_) => {}
            None => selected = Some(display),
        }
    }
    selected
}

fn project_states(mdp: &ExplicitMdp) -> Vec<ProbabilisticState> {
    let initial = mdp
        .initial
        .iter()
        .map(|entry| (entry.state, entry.probability))
        .collect::<HashMap<_, _>>();
    mdp.states
        .iter()
        .enumerate()
        .map(|(id, state)| {
            let facts = mdp
                .model
                .task
                .fact_names
                .iter()
                .enumerate()
                .filter(|(index, name)| {
                    bitset::test(&state.bits, *index)
                        && !name.starts_with("(PPDDL-MARKER-")
                        && name.as_str() != "(PPDDL-INIT-PENDING)"
                })
                .map(|(_, name)| name.clone())
                .collect();
            let fluents = mdp
                .model
                .task
                .fluent_names
                .iter()
                .enumerate()
                .filter(|(index, name)| state.fdef[*index] && name.as_str() != "(REWARD)")
                .map(|(index, name)| (name.clone(), state.fv[index]))
                .collect();
            ProbabilisticState {
                id,
                facts,
                fluents,
                goal: mdp.goal[id],
                initial_probability: initial.get(&id).copied().unwrap_or(0.0),
            }
        })
        .collect()
}
