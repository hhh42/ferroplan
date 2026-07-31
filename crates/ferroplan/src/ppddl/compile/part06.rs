fn compile_domain_source(
    root: &Sexp,
    universe: &ObjectUniverse,
    requirements: &HashSet<String>,
    initial_outcomes: &[(f64, Sexp)],
    max_outcomes: usize,
) -> Result<DomainCompilation, PpddlError> {
    validate_reserved_symbols(root)?;
    let root_items = define_items(root, "domain")?;
    let domain_decl = root_items
        .get(1)
        .and_then(Sexp::list)
        .ok_or_else(|| PpddlError::Syntax("domain declaration missing".into()))?;
    if domain_decl.first().and_then(Sexp::name) != Some("DOMAIN") {
        return Err(PpddlError::Syntax("expected (domain NAME)".into()));
    }
    let domain_name = domain_decl
        .get(1)
        .and_then(Sexp::name)
        .ok_or_else(|| PpddlError::Syntax("domain name missing".into()))?
        .to_string();

    let mdp = requirements.contains(":MDP");
    let has_probabilistic = mdp || requirements.contains(PROB_REQ);
    let has_rewards = mdp || requirements.contains(REWARD_REQ);
    let mut output = vec![root_items[0].clone(), root_items[1].clone()];
    let mut action_sections = Vec::new();
    let mut marker_names = Vec::new();
    let mut variants = HashMap::new();
    let mut outcomes_per_action = HashMap::new();
    let mut probabilistic_actions = 0usize;
    let mut action_index = 0usize;
    let mut reward_used = false;

    for original in root_items.iter().skip(2) {
        if section_head(original) != Some(":ACTION") {
            let mut section = original.clone();
            strip_ppddl_requirements(&mut section);
            output.push(section);
            continue;
        }
        let items = original
            .list()
            .ok_or_else(|| PpddlError::Syntax("action must be a list".into()))?;
        let base_name = items
            .get(1)
            .and_then(Sexp::name)
            .ok_or_else(|| PpddlError::Syntax("action name missing".into()))?
            .to_string();
        if let Some(precondition_index) = items
            .iter()
            .position(|item| item.name() == Some(":PRECONDITION"))
        {
            let precondition = items.get(precondition_index + 1).ok_or_else(|| {
                PpddlError::Syntax(format!("action {base_name} precondition missing"))
            })?;
            if contains_name(precondition, "REWARD") {
                return Err(PpddlError::RewardViolation(format!(
                    "action {base_name} precondition references reward"
                )));
            }
        }
        let effect_keyword = items
            .iter()
            .position(|item| item.name() == Some(":EFFECT"))
            .ok_or_else(|| PpddlError::Syntax(format!("action {base_name} has no :effect")))?;
        let effect = items.get(effect_keyword + 1).ok_or_else(|| {
            PpddlError::Syntax(format!("action {base_name} has no effect body"))
        })?;
        reward_used |= validate_reward_effect(effect)?;
        let stochastic = contains_operator(effect, &["PROBABILISTIC", "ONEOF"]);
        if stochastic && !has_probabilistic {
            return Err(PpddlError::Unsupported(format!(
                "action {base_name} uses probabilistic effects without {PROB_REQ} or :MDP"
            )));
        }
        if stochastic {
            probabilistic_actions += 1;
        }
        let outcomes = expand_effect(effect, universe, max_outcomes).map_err(|error| match error {
            PpddlError::OutcomeLimit { limit, .. } => PpddlError::OutcomeLimit {
                action: base_name.clone(),
                limit,
            },
            other => other,
        })?;
        outcomes_per_action.insert(action_index, outcomes.len());
        for (outcome_index, (probability, deterministic)) in outcomes.into_iter().enumerate() {
            let variant_name =
                format!("{VARIANT_PREFIX}{action_index}-O{outcome_index}-{base_name}");
            let marker_name = format!("{MARKER_PREFIX}{action_index}-O{outcome_index}");
            marker_names.push(marker_name.clone());
            let mut variant = original.clone();
            {
                let variant_items = variant
                    .list_mut()
                    .ok_or_else(|| PpddlError::Syntax("action must be a list".into()))?;
                variant_items[1] = Sexp::Name(variant_name.clone());
                let effect_index = variant_items
                    .iter()
                    .position(|item| item.name() == Some(":EFFECT"))
                    .expect("source action has effect");
                variant_items[effect_index + 1] =
                    join_effects(&deterministic, &atom(&marker_name));
            }
            gate_action_precondition(&mut variant)?;
            action_sections.push(variant);
            variants.insert(
                variant_name.clone(),
                VariantSpec {
                    action_index,
                    outcome_index,
                    base_name: base_name.clone(),
                    variant_name,
                    marker_name,
                    probability,
                    initial: false,
                },
            );
        }
        action_index += 1;
    }

    if reward_used && !has_rewards {
        return Err(PpddlError::RewardViolation(format!(
            "reward effects require {REWARD_REQ} or :MDP"
        )));
    }
    let initial_action_index = action_index;
    outcomes_per_action.insert(initial_action_index, initial_outcomes.len());
    for (outcome_index, (probability, deterministic)) in initial_outcomes.iter().enumerate() {
        let variant_name = format!(
            "{VARIANT_PREFIX}{initial_action_index}-O{outcome_index}-{INIT_ACTION}"
        );
        let marker_name =
            format!("{MARKER_PREFIX}{initial_action_index}-O{outcome_index}");
        marker_names.push(marker_name.clone());
        let effect = conjunction([
            deterministic.clone(),
            Sexp::List(vec![Sexp::Name("NOT".into()), atom(INIT_PENDING)]),
            atom(&marker_name),
        ]);
        action_sections.push(Sexp::List(vec![
            Sexp::Name(":ACTION".into()),
            Sexp::Name(variant_name.clone()),
            Sexp::Name(":PARAMETERS".into()),
            Sexp::List(Vec::new()),
            Sexp::Name(":PRECONDITION".into()),
            atom(INIT_PENDING),
            Sexp::Name(":EFFECT".into()),
            effect,
        ]));
        variants.insert(
            variant_name.clone(),
            VariantSpec {
                action_index: initial_action_index,
                outcome_index,
                base_name: INIT_ACTION.into(),
                variant_name,
                marker_name,
                probability: *probability,
                initial: true,
            },
        );
    }

    normalize_requirements(&mut output);
    let marker_declarations = std::iter::once(atom(INIT_PENDING))
        .chain(marker_names.iter().cloned().map(atom))
        .collect::<Vec<_>>();
    if let Some(index) = output
        .iter()
        .position(|section| section_head(section) == Some(":PREDICATES"))
    {
        output[index]
            .list_mut()
            .expect("predicate section is a list")
            .extend(marker_declarations);
    } else {
        let mut predicates = vec![Sexp::Name(":PREDICATES".into())];
        predicates.extend(marker_declarations);
        output.push(Sexp::List(predicates));
    }

    let uses_rewards = has_rewards || reward_used;
    if uses_rewards {
        let functions_index = output
            .iter()
            .position(|section| section_head(section) == Some(":FUNCTIONS"));
        let reward_declared = functions_index.is_some_and(|index| {
            output[index]
                .list()
                .is_some_and(|items| items.iter().skip(1).any(reward_target))
        });
        if !reward_declared {
            if let Some(index) = functions_index {
                output[index]
                    .list_mut()
                    .expect("function section is a list")
                    .push(atom("REWARD"));
            } else {
                output.push(Sexp::List(vec![
                    Sexp::Name(":FUNCTIONS".into()),
                    atom("REWARD"),
                ]));
            }
        }
    }
    output.extend(action_sections);

    Ok(DomainCompilation {
        source: render(&Sexp::List(output)),
        domain_name,
        variants,
        outcomes_per_action,
        marker_names,
        probabilistic_actions,
        initial_action_index,
        uses_rewards,
    })
}
