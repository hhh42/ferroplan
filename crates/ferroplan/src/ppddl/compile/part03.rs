fn normalize_init_element(value: &Sexp) -> Result<Option<Sexp>, PpddlError> {
    if let Some(target) = init_assignment_target(value) {
        let items = value.list().expect("assignment is a list");
        if reward_target(target) {
            let initial = numeric_constant(&items[2])?;
            if initial.abs() > PROB_EPS {
                return Err(PpddlError::RewardViolation(
                    "the initial value of reward must be zero".into(),
                ));
            }
            return Ok(None);
        }
        return Ok(Some(Sexp::List(vec![
            Sexp::Name("ASSIGN".into()),
            target.clone(),
            items[2].clone(),
        ])));
    }
    let Some(items) = value.list() else {
        return Err(PpddlError::Syntax(
            "initial-state element must be a list".into(),
        ));
    };
    let Some(head) = items.first().and_then(Sexp::name) else {
        return Err(PpddlError::Syntax(
            "initial-state element has no operator".into(),
        ));
    };
    if contains_name(value, "REWARD") {
        return Err(PpddlError::RewardViolation(
            "reward may only be initialized to zero".into(),
        ));
    }
    match head {
        "AND" => {
            let mut normalized = Vec::new();
            for child in &items[1..] {
                if let Some(child) = normalize_init_element(child)? {
                    normalized.push(child);
                }
            }
            Ok(Some(conjunction(normalized)))
        }
        "PROBABILISTIC" => {
            if items.len() < 3 || items.len() % 2 == 0 {
                return Err(PpddlError::Syntax(
                    "probabilistic initial state requires probability/effect pairs".into(),
                ));
            }
            let mut normalized = vec![Sexp::Name("PROBABILISTIC".into())];
            for pair in items[1..].chunks_exact(2) {
                normalized.push(pair[0].clone());
                normalized.push(
                    normalize_init_element(&pair[1])?.unwrap_or_else(noop_effect),
                );
            }
            Ok(Some(Sexp::List(normalized)))
        }
        _ => Ok(Some(value.clone())),
    }
}

fn parse_metric(
    problem_items: &[Sexp],
    rewards: bool,
) -> Result<(DeclaredObjective, Option<String>), PpddlError> {
    let metric = problem_items
        .iter()
        .find(|section| section_head(section) == Some(":METRIC"));
    let Some(metric) = metric else {
        return Ok((
            DeclaredObjective {
                objective: if rewards {
                    ProbabilisticObjective::MaximizeExpectedReward
                } else {
                    ProbabilisticObjective::MaximizeGoalProbability
                },
                metric: None,
            },
            None,
        ));
    };
    let items = metric
        .list()
        .ok_or_else(|| PpddlError::Syntax(":metric must be a list".into()))?;
    if items.len() != 3 {
        return Err(PpddlError::Syntax(
            ":metric requires an optimization direction and expression".into(),
        ));
    }
    let maximize = match items[1].name() {
        Some("MAXIMIZE") => true,
        Some("MINIMIZE") => false,
        Some(other) => {
            return Err(PpddlError::Syntax(format!(
                "unknown PPDDL metric direction {other}"
            )))
        }
        None => {
            return Err(PpddlError::Syntax(
                "PPDDL metric direction must be a name".into(),
            ))
        }
    };
    let expression = &items[2];
    let objective = if expression.head() == Some("GOAL-ACHIEVED")
        || expression.name() == Some("GOAL-ACHIEVED")
    {
        if maximize {
            ProbabilisticObjective::MaximizeGoalProbability
        } else {
            ProbabilisticObjective::MinimizeGoalProbability
        }
    } else if expression.head() == Some("REWARD") || expression.name() == Some("REWARD") {
        if !rewards {
            return Err(PpddlError::RewardViolation(
                "a reward metric requires :rewards or :mdp".into(),
            ));
        }
        if maximize {
            ProbabilisticObjective::MaximizeExpectedReward
        } else {
            ProbabilisticObjective::MinimizeExpectedReward
        }
    } else {
        if contains_name(expression, "REWARD") || contains_name(expression, "GOAL-ACHIEVED") {
            return Err(PpddlError::Unsupported(
                "reward and goal-achieved must be the complete PPDDL metric expression".into(),
            ));
        }
        let objective = if maximize {
            ProbabilisticObjective::MaximizeExpectedMetric
        } else {
            ProbabilisticObjective::MinimizeExpectedMetric
        };
        return Ok((
            DeclaredObjective {
                objective,
                metric: Some(parse_ground_metric(expression)?),
            },
            Some(render(expression)),
        ));
    };
    Ok((
        DeclaredObjective {
            objective,
            metric: None,
        },
        Some(render(expression)),
    ))
}
