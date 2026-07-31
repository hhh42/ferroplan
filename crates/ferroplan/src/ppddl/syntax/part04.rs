fn expand_effect(
    effect: &Sexp,
    universe: &ObjectUniverse,
    limit: usize,
) -> Result<Vec<(f64, Sexp)>, PpddlError> {
    let Some(items) = effect.list() else {
        return Err(PpddlError::Syntax("effect must be a list".into()));
    };
    let Some(head) = items.first().and_then(Sexp::name) else {
        return Err(PpddlError::Syntax("effect has no operator".into()));
    };
    let mut expanded = match head {
        "PROBABILISTIC" => {
            if items.len() < 3 || items.len() % 2 == 0 {
                return Err(PpddlError::Syntax(
                    "probabilistic effect requires probability/effect pairs".into(),
                ));
            }
            let mut outcomes = Vec::new();
            let mut explicit = 0.0;
            for pair in items[1..].chunks_exact(2) {
                let branch_probability = probability(&pair[0])?;
                explicit += branch_probability;
                for (nested_probability, nested) in
                    expand_effect(&pair[1], universe, limit)?
                {
                    let combined = branch_probability * nested_probability;
                    if combined > PROB_EPS {
                        outcomes.push((combined, nested));
                    }
                }
                if outcomes.len() > limit {
                    return Err(PpddlError::OutcomeLimit {
                        action: "nested probabilistic effect".into(),
                        limit,
                    });
                }
            }
            if explicit > 1.0 + PROB_EPS {
                return Err(PpddlError::InvalidProbability(format!(
                    "probabilistic branches sum to {explicit}, above 1"
                )));
            }
            if explicit < 1.0 - PROB_EPS {
                outcomes.push((1.0 - explicit, noop_effect()));
            }
            outcomes
        }
        "ONEOF" => {
            if items.len() < 2 {
                return Err(PpddlError::Syntax("oneof effect is empty".into()));
            }
            let branch_probability = 1.0 / (items.len() - 1) as f64;
            let mut outcomes = Vec::new();
            for branch in &items[1..] {
                for (nested_probability, nested) in expand_effect(branch, universe, limit)? {
                    outcomes.push((branch_probability * nested_probability, nested));
                }
            }
            outcomes
        }
        "AND" => {
            let mut outcomes = vec![(1.0, noop_effect())];
            for child in &items[1..] {
                let child_outcomes = expand_effect(child, universe, limit)?;
                outcomes = product_outcomes(
                    &outcomes,
                    &child_outcomes,
                    limit,
                    "conjunctive probabilistic effect",
                )?;
            }
            outcomes
        }
        "WHEN" => {
            if items.len() != 3 {
                return Err(PpddlError::Syntax(
                    "when effect requires condition and effect".into(),
                ));
            }
            expand_effect(&items[2], universe, limit)?
                .into_iter()
                .map(|(branch_probability, nested)| {
                    (
                        branch_probability,
                        Sexp::List(vec![
                            Sexp::Name("WHEN".into()),
                            items[1].clone(),
                            nested,
                        ]),
                    )
                })
                .collect()
        }
        "FORALL" => {
            if items.len() != 3 {
                return Err(PpddlError::Syntax(
                    "forall effect requires variables and effect".into(),
                ));
            }
            let variable_items = items[1]
                .list()
                .ok_or_else(|| PpddlError::Syntax("forall variables must be a list".into()))?;
            let variables = parse_typed_items(variable_items)?;
            let bindings = universe.bindings(&variables);
            let mut outcomes = vec![(1.0, noop_effect())];
            for binding in bindings {
                let instantiated = substitute(&items[2], &binding);
                let child_outcomes = expand_effect(&instantiated, universe, limit)?;
                outcomes = product_outcomes(
                    &outcomes,
                    &child_outcomes,
                    limit,
                    "universally quantified probabilistic effect",
                )?;
            }
            outcomes
        }
        _ => vec![(1.0, effect.clone())],
    };

    let mut coalesced: BTreeMap<String, (f64, Sexp)> = BTreeMap::new();
    for (branch_probability, deterministic) in expanded.drain(..) {
        if branch_probability <= PROB_EPS {
            continue;
        }
        let key = render(&deterministic);
        coalesced
            .entry(key)
            .and_modify(|entry| entry.0 += branch_probability)
            .or_insert((branch_probability, deterministic));
    }
    if coalesced.len() > limit {
        return Err(PpddlError::OutcomeLimit {
            action: "normalized probabilistic effect".into(),
            limit,
        });
    }
    let total: f64 = coalesced.values().map(|entry| entry.0).sum();
    if (total - 1.0).abs() > 1e-9 {
        return Err(PpddlError::InvalidProbability(format!(
            "normalized outcome mass is {total}, expected 1"
        )));
    }
    Ok(coalesced.into_values().collect())
}
