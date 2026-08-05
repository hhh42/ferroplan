#[derive(Clone)]
struct Transition {
    probability: f64,
    next: usize,
    reward: f64,
}

#[derive(Clone)]
struct StateAction {
    action: usize,
    transitions: Vec<Transition>,
}

#[derive(Clone)]
struct InitialMass {
    probability: f64,
    state: usize,
}

struct ExplicitMdp {
    model: CompiledModel,
    states: Vec<State>,
    initial: Vec<InitialMass>,
    actions: Vec<Vec<StateAction>>,
    goal: Vec<bool>,
    transitions: usize,
}

type InitialDistribution = (
    Vec<State>,
    Vec<bool>,
    Vec<InitialMass>,
    HashMap<StateKey, usize>,
);

fn canonicalize_successor(model: &CompiledModel, source: &State, successor: &mut State) -> f64 {
    for &fact in &model.marker_facts {
        bitset::clear(&mut successor.bits, fact);
    }
    let mut reward = 0.0;
    if let Some(index) = model.reward_fluent {
        let before = if source.fdef[index] {
            source.fv[index]
        } else {
            0.0
        };
        let after = if successor.fdef[index] {
            successor.fv[index]
        } else {
            before
        };
        reward = after - before;
        successor.fv[index] = before;
        successor.fdef[index] = source.fdef[index];
    }
    reward
}

fn initial_distribution(
    model: &CompiledModel,
    options: &ProbabilisticOptions,
) -> Result<InitialDistribution, PpddlError> {
    let base = model.task.initial();
    let mut states = Vec::new();
    let mut goals = Vec::new();
    let mut initial = Vec::<InitialMass>::new();
    let mut indexes = HashMap::new();
    for outcome in &model.initial_action.outcomes {
        if !model.task.op_applicable(outcome.op, &base) {
            return Err(PpddlError::GroundingDivergence {
                action: INIT_ACTION.into(),
                expected: model.initial_action.outcomes.len(),
                observed: initial.len(),
            });
        }
        let mut state = model.task.apply(outcome.op, &base);
        let reward = canonicalize_successor(model, &base, &mut state);
        if reward.abs() > PROB_EPS {
            return Err(PpddlError::RewardViolation(
                "probabilistic initial conditions may not award reward".into(),
            ));
        }
        let key = model.task.state_key(&state);
        let state_id = if let Some(&known) = indexes.get(&key) {
            known
        } else {
            if states.len() >= options.max_states {
                return Err(PpddlError::StateLimit {
                    limit: options.max_states,
                });
            }
            let id = states.len();
            indexes.insert(key, id);
            goals.push(model.task.goal_met(&state));
            states.push(state);
            id
        };
        if let Some(existing) = initial.iter_mut().find(|mass| mass.state == state_id) {
            existing.probability += outcome.probability;
        } else {
            initial.push(InitialMass {
                probability: outcome.probability,
                state: state_id,
            });
        }
    }
    let mass: f64 = initial.iter().map(|entry| entry.probability).sum();
    if (mass - 1.0).abs() > 1e-9 {
        return Err(PpddlError::InvalidProbability(format!(
            "initial-state probability mass is {mass}"
        )));
    }
    Ok((states, goals, initial, indexes))
}
