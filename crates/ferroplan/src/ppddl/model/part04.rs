fn build_mdp(
    model: CompiledModel,
    options: &ProbabilisticOptions,
) -> Result<ExplicitMdp, PpddlError> {
    let (mut states, mut goals, initial, mut indexes) =
        initial_distribution(&model, options)?;
    let mut actions_by_state = vec![Vec::new(); states.len()];
    let mut queue = VecDeque::new();
    let mut queued = HashSet::new();
    for mass in &initial {
        if queued.insert(mass.state) {
            queue.push_back(mass.state);
        }
    }
    let mut transition_count = 0usize;

    while let Some(state_index) = queue.pop_front() {
        let state = states[state_index].clone();
        let mut state_actions = Vec::new();
        if !goals[state_index] {
            for (action_index, action) in model.actions.iter().enumerate() {
                let Some(first) = action.outcomes.first() else {
                    continue;
                };
                if !model.task.op_applicable(first.op, &state) {
                    continue;
                }
                let mut transitions: Vec<Transition> = Vec::new();
                for outcome in &action.outcomes {
                    if !model.task.op_applicable(outcome.op, &state) {
                        return Err(PpddlError::GroundingDivergence {
                            action: action.display.clone(),
                            expected: action.outcomes.len(),
                            observed: transitions.len(),
                        });
                    }
                    let mut successor = model.task.apply(outcome.op, &state);
                    let reward = canonicalize_successor(&model, &state, &mut successor);
                    let successor_goal = model.task.goal_met(&successor);
                    let key = model.task.state_key(&successor);
                    let next = if let Some(&known) = indexes.get(&key) {
                        known
                    } else {
                        if states.len() >= options.max_states {
                            return Err(PpddlError::StateLimit {
                                limit: options.max_states,
                            });
                        }
                        let id = states.len();
                        indexes.insert(key, id);
                        states.push(successor);
                        goals.push(successor_goal);
                        actions_by_state.push(Vec::new());
                        queue.push_back(id);
                        id
                    };
                    if let Some(existing) = transitions.iter_mut().find(|transition| {
                        transition.next == next && (transition.reward - reward).abs() <= PROB_EPS
                    }) {
                        existing.probability += outcome.probability;
                    } else {
                        transitions.push(Transition {
                            probability: outcome.probability,
                            next,
                            reward,
                        });
                    }
                    transition_count += 1;
                    if transition_count > options.max_transitions {
                        return Err(PpddlError::TransitionLimit {
                            limit: options.max_transitions,
                        });
                    }
                }
                transitions.sort_by(|left, right| {
                    left.next
                        .cmp(&right.next)
                        .then_with(|| left.reward.total_cmp(&right.reward))
                });
                let mass: f64 = transitions
                    .iter()
                    .map(|transition| transition.probability)
                    .sum();
                if (mass - 1.0).abs() > 1e-9 {
                    return Err(PpddlError::InvalidProbability(format!(
                        "ground action {} has probability mass {mass}",
                        action.display
                    )));
                }
                state_actions.push(StateAction {
                    action: action_index,
                    transitions,
                });
            }
        }
        actions_by_state[state_index] = state_actions;
    }

    Ok(ExplicitMdp {
        model,
        states,
        initial,
        actions: actions_by_state,
        goal: goals,
        transitions: transition_count,
    })
}

fn is_reward_objective(objective: ProbabilisticObjective) -> bool {
    matches!(
        objective,
        ProbabilisticObjective::MaximizeExpectedReward
            | ProbabilisticObjective::MinimizeExpectedReward
    )
}

fn is_minimizing(objective: ProbabilisticObjective) -> bool {
    matches!(
        objective,
        ProbabilisticObjective::MinimizeGoalProbability
            | ProbabilisticObjective::MinimizeExpectedReward
            | ProbabilisticObjective::MinimizeExpectedMetric
    )
}

fn goal_reward_at(
    mdp: &ExplicitMdp,
    state: usize,
    total_time: f64,
) -> Result<f64, PpddlError> {
    if !mdp.goal[state] {
        return Ok(0.0);
    }
    mdp.model
        .goal_reward
        .as_ref()
        .map(|expression| expression.evaluate(&mdp.states[state], total_time))
        .transpose()
        .map(|value| value.unwrap_or(0.0))
}
