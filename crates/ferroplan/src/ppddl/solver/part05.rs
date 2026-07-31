#[derive(Clone, Copy)]
struct XorShift64(u64);

impl XorShift64 {
    fn new(seed: u64) -> Self {
        Self(if seed == 0 {
            0x9e37_79b9_7f4a_7c15
        } else {
            seed
        })
    }

    fn next_f64(&mut self) -> f64 {
        let mut value = self.0;
        value ^= value << 13;
        value ^= value >> 7;
        value ^= value << 17;
        self.0 = value;
        (value as f64) / (u64::MAX as f64 + 1.0)
    }
}

fn sample_weighted<'a, T>(
    rng: &mut XorShift64,
    values: &'a [T],
    probability: impl Fn(&T) -> f64,
) -> Option<&'a T> {
    let sample = rng.next_f64();
    let mut cumulative = 0.0;
    values
        .iter()
        .find(|value| {
            cumulative += probability(value);
            sample <= cumulative + PROB_EPS
        })
        .or(values.last())
}

/// Solve and execute a policy in the exact compiled transition graph using a
/// deterministic seed. This is a simulation receipt, not a substitute for the
/// dynamic-programming value.
pub fn simulate_ppddl(
    domain_src: &str,
    problem_src: &str,
    options: &ProbabilisticOptions,
    episodes: usize,
    seed: u64,
) -> Result<SimulationReport, PpddlError> {
    if episodes == 0 {
        return Err(PpddlError::InvalidOptions(
            "simulation episodes must be positive".into(),
        ));
    }
    let model = compile_model(domain_src, problem_src, options)?;
    let mdp = build_mdp(model, options)?;
    let (solution, _) = solve_model(&mdp, options)?;
    let policy = policy_map(&solution);
    let mut rng = XorShift64::new(seed);
    let mut reached_goal = 0usize;
    let mut total_reward = 0.0;
    let mut total_discounted_reward = 0.0;
    let mut total_steps = 0usize;

    for _ in 0..episodes {
        let initial = sample_weighted(&mut rng, &mdp.initial, |entry| entry.probability)
            .expect("PPDDL initial distribution is non-empty");
        let mut state = initial.state;
        let mut remaining = options.horizon;
        let initial_goal_reward = if is_reward_objective(mdp.model.objective) {
            goal_reward_at(&mdp, state, 0.0)?
        } else {
            0.0
        };
        let mut episode_reward = initial_goal_reward;
        let mut discounted_reward = initial_goal_reward;
        let mut discount = 1.0;
        let mut steps = 0usize;
        while !mdp.goal[state] && steps < options.simulation_max_steps {
            if matches!(remaining, Some(0)) {
                break;
            }
            let decision = policy
                .get(&(state, remaining))
                .or_else(|| policy.get(&(state, None)));
            let Some(decision) = decision else {
                break;
            };
            let selected = sample_weighted(&mut rng, &decision.outcomes, |outcome| {
                outcome.probability
            })
            .expect("policy decisions have outcomes");
            episode_reward += selected.reward;
            discounted_reward += discount * selected.reward;
            discount *= options.discount;
            state = selected.next_state;
            steps += 1;
            if let Some(value) = &mut remaining {
                *value = value.saturating_sub(1);
            }
        }
        if mdp.goal[state] {
            reached_goal += 1;
        }
        total_reward += episode_reward;
        total_discounted_reward += discounted_reward;
        total_steps += steps;
    }
    Ok(SimulationReport {
        episodes,
        reached_goal,
        goal_rate: reached_goal as f64 / episodes as f64,
        average_reward: total_reward / episodes as f64,
        average_discounted_reward: total_discounted_reward / episodes as f64,
        average_steps: total_steps as f64 / episodes as f64,
        seed,
    })
}
