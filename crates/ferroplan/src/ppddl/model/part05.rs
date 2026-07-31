fn transition_reward_at(
    mdp: &ExplicitMdp,
    transition: &Transition,
    successor_time: f64,
) -> Result<f64, PpddlError> {
    Ok(transition.reward + goal_reward_at(mdp, transition.next, successor_time)?)
}

fn q_value(
    mdp: &ExplicitMdp,
    objective: ProbabilisticObjective,
    discount: f64,
    action: &StateAction,
    values: &[f64],
    successor_time: f64,
) -> Result<f64, PpddlError> {
    action
        .transitions
        .iter()
        .map(|transition| {
            let immediate = if is_reward_objective(objective) {
                transition_reward_at(mdp, transition, successor_time)?
            } else {
                0.0
            };
            Ok(transition.probability * (immediate + discount * values[transition.next]))
        })
        .sum()
}

fn base_values(
    mdp: &ExplicitMdp,
    objective: ProbabilisticObjective,
    total_time: f64,
) -> Result<Vec<f64>, PpddlError> {
    if matches!(
        objective,
        ProbabilisticObjective::MaximizeExpectedMetric
            | ProbabilisticObjective::MinimizeExpectedMetric
    ) {
        let metric = mdp.model.metric.as_ref().ok_or_else(|| {
            PpddlError::InvalidOptions(
                "an expected-metric objective requires a declared PPDDL metric".into(),
            )
        })?;
        return mdp
            .states
            .iter()
            .map(|state| metric.evaluate(state, total_time))
            .collect();
    }
    Ok(mdp
        .goal
        .iter()
        .map(|&goal| {
            if goal
                && matches!(
                    objective,
                    ProbabilisticObjective::MaximizeGoalProbability
                        | ProbabilisticObjective::MinimizeGoalProbability
                )
            {
                1.0
            } else {
                0.0
            }
        })
        .collect())
}

fn initial_value(
    mdp: &ExplicitMdp,
    values: &[f64],
    objective: ProbabilisticObjective,
) -> Result<f64, PpddlError> {
    mdp.initial
        .iter()
        .map(|entry| {
            let initial_goal_reward = if is_reward_objective(objective) {
                goal_reward_at(mdp, entry.state, 0.0)?
            } else {
                0.0
            };
            Ok(entry.probability * (initial_goal_reward + values[entry.state]))
        })
        .sum()
}
