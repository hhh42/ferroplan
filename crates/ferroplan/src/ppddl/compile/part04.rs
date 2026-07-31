fn analyze_problem(
    root: Sexp,
    universe: &ObjectUniverse,
    has_probabilistic: bool,
    has_rewards: bool,
    max_initial_outcomes: usize,
) -> Result<ProblemAnalysis, PpddlError> {
    let root_items = define_items(&root, "problem")?;
    let declaration = root_items
        .get(1)
        .and_then(Sexp::list)
        .ok_or_else(|| PpddlError::Syntax("problem declaration missing".into()))?;
    if declaration.first().and_then(Sexp::name) != Some("PROBLEM") {
        return Err(PpddlError::Syntax("expected (problem NAME)".into()));
    }
    let problem_name = declaration
        .get(1)
        .and_then(Sexp::name)
        .ok_or_else(|| PpddlError::Syntax("problem name missing".into()))?
        .to_string();

    let init = root_items
        .iter()
        .find(|section| section_head(section) == Some(":INIT"));
    let init_effect = if let Some(init) = init {
        let items = init.list().unwrap_or_default();
        let mut normalized = Vec::new();
        for element in &items[1..] {
            if let Some(element) = normalize_init_element(element)? {
                normalized.push(element);
            }
        }
        conjunction(normalized)
    } else {
        noop_effect()
    };
    let stochastic_initial = contains_operator(&init_effect, &["PROBABILISTIC", "ONEOF"]);
    if stochastic_initial && !has_probabilistic {
        return Err(PpddlError::Unsupported(format!(
            "probabilistic initial conditions require {PROB_REQ} or :MDP"
        )));
    }
    let initial_outcomes = expand_effect(&init_effect, universe, max_initial_outcomes)
        .map_err(|error| match error {
            PpddlError::OutcomeLimit { .. } => PpddlError::InitialOutcomeLimit {
                limit: max_initial_outcomes,
            },
            other => other,
        })?;

    let goal_section = root_items
        .iter()
        .find(|section| section_head(section) == Some(":GOAL"));
    if goal_section.is_some_and(|goal| contains_name(goal, "REWARD")) {
        return Err(PpddlError::RewardViolation(
            "goals may not reference reward".into(),
        ));
    }

    let goal_reward_section = root_items
        .iter()
        .find(|section| section_head(section) == Some(":GOAL-REWARD"));
    let goal_reward = if let Some(section) = goal_reward_section {
        if !has_rewards {
            return Err(PpddlError::RewardViolation(
                ":goal-reward requires :rewards or :mdp".into(),
            ));
        }
        let items = section
            .list()
            .ok_or_else(|| PpddlError::Syntax(":goal-reward must be a list".into()))?;
        if items.len() != 2 {
            return Err(PpddlError::Syntax(
                ":goal-reward requires one ground numeric expression".into(),
            ));
        }
        if contains_name(&items[1], "REWARD") {
            return Err(PpddlError::RewardViolation(
                ":goal-reward may not depend on accumulated reward".into(),
            ));
        }
        Some(parse_ground_metric(&items[1])?)
    } else if !has_rewards {
        Some(GroundMetricExpr::Number(1.0))
    } else {
        None
    };

    let goal_reward_text = if let Some(section) = goal_reward_section {
        Some(
            section
                .list()
                .and_then(|items| items.get(1))
                .map(render)
                .unwrap_or_default(),
        )
    } else if !has_rewards {
        Some("1".into())
    } else {
        None
    };
    let (declared_objective, metric_text) = parse_metric(root_items, has_rewards)?;
    Ok(ProblemAnalysis {
        root,
        problem_name,
        initial_outcomes,
        goal_reward,
        goal_reward_text,
        declared_objective,
        metric_text,
    })
}

fn gate_action_precondition(action: &mut Sexp) -> Result<(), PpddlError> {
    let items = action
        .list_mut()
        .ok_or_else(|| PpddlError::Syntax("action must be a list".into()))?;
    let gate = Sexp::List(vec![
        Sexp::Name("NOT".into()),
        atom(INIT_PENDING),
    ]);
    if let Some(index) = items
        .iter()
        .position(|item| item.name() == Some(":PRECONDITION"))
    {
        let original = items
            .get(index + 1)
            .cloned()
            .ok_or_else(|| PpddlError::Syntax("action precondition body missing".into()))?;
        items[index + 1] = conjunction([original, gate]);
    } else {
        let effect_index = items
            .iter()
            .position(|item| item.name() == Some(":EFFECT"))
            .unwrap_or(items.len());
        items.insert(effect_index, Sexp::Name(":PRECONDITION".into()));
        items.insert(effect_index + 1, conjunction([gate]));
    }
    Ok(())
}
