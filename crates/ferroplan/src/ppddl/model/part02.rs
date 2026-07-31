fn compile_model(
    domain_src: &str,
    problem_src: &str,
    options: &ProbabilisticOptions,
) -> Result<CompiledModel, PpddlError> {
    options.validate()?;
    let compiled = compile_sources(
        domain_src,
        problem_src,
        options.max_outcomes_per_action,
        options.max_initial_outcomes,
    )?;
    let objective = compiled.declared_objective.resolve(options.objective);
    if options.horizon.is_none()
        && matches!(
            objective,
            ProbabilisticObjective::MaximizeExpectedReward
                | ProbabilisticObjective::MinimizeExpectedReward
        )
        && options.discount >= 1.0
    {
        return Err(PpddlError::InvalidOptions(
            "infinite-horizon expected reward requires discount < 1".into(),
        ));
    }
    let threads = if options.threads == 0 {
        crate::par::num_threads()
    } else {
        options.threads
    };
    let mut task = ground_task(&compiled.domain, &compiled.problem, threads)
        .ok_or(PpddlError::GroundingFailed)?;

    let marker_facts = compiled
        .marker_names
        .iter()
        .map(|name| {
            task.fact_id(&format!("({name})")).ok_or_else(|| {
                PpddlError::GroundingDivergence {
                    action: name.clone(),
                    expected: 1,
                    observed: 0,
                }
            })
        })
        .collect::<Result<Vec<_>, _>>()?;

    let mut groups: BTreeMap<(usize, String), Vec<GroundOutcome>> = BTreeMap::new();
    let mut displays: HashMap<(usize, String), (String, Vec<String>, bool)> = HashMap::new();
    for (op, display) in task.op_display.iter().enumerate() {
        let mut words = display.split_whitespace();
        let Some(variant_name) = words.next() else {
            continue;
        };
        let Some(spec) = compiled.variants.get(variant_name) else {
            continue;
        };
        debug_assert_eq!(spec.variant_name, variant_name);
        debug_assert!(spec.marker_name.starts_with(MARKER_PREFIX));
        let args = words.map(str::to_string).collect::<Vec<_>>();
        let args_key = args.join("\u{1f}");
        let key = (spec.action_index, args_key);
        groups.entry(key.clone()).or_default().push(GroundOutcome {
            probability: spec.probability,
            op,
            outcome_index: spec.outcome_index,
        });
        displays.insert(key, (spec.base_name.clone(), args, spec.initial));
    }

    let mut actions = Vec::new();
    let mut initial_action = None;
    for (key, mut outcomes) in groups {
        outcomes.sort_by_key(|outcome| outcome.outcome_index);
        let expected = compiled
            .outcomes_per_action
            .get(&key.0)
            .copied()
            .unwrap_or_default();
        if outcomes.len() != expected {
            let (base, args, _) = displays
                .get(&key)
                .cloned()
                .unwrap_or_else(|| (format!("action-{}", key.0), Vec::new(), false));
            let action = std::iter::once(base)
                .chain(args)
                .collect::<Vec<_>>()
                .join(" ");
            return Err(PpddlError::GroundingDivergence {
                action,
                expected,
                observed: outcomes.len(),
            });
        }
        let (base_name, args, initial) =
            displays.remove(&key).expect("display inserted with group");
        let display = std::iter::once(base_name.clone())
            .chain(args.iter().cloned())
            .collect::<Vec<_>>()
            .join(" ");
        let action = GroundAction {
            display,
            base_name,
            args,
            outcomes,
        };
        if initial {
            if initial_action.replace(action).is_some() {
                return Err(PpddlError::GroundingDivergence {
                    action: INIT_ACTION.into(),
                    expected: 1,
                    observed: 2,
                });
            }
        } else {
            actions.push(action);
        }
    }
    let initial_action = initial_action.ok_or_else(|| PpddlError::GroundingDivergence {
        action: INIT_ACTION.into(),
        expected: compiled
            .outcomes_per_action
            .get(&compiled.initial_action_index)
            .copied()
            .unwrap_or(1),
        observed: 0,
    })?;

    let reward_fluent = task.fluent_id("(REWARD)");
    if compiled.uses_rewards && reward_fluent.is_none() {
        return Err(PpddlError::GroundingDivergence {
            action: "REWARD".into(),
            expected: 1,
            observed: 0,
        });
    }
    if let Some(reward) = reward_fluent {
        if task.relevant_fluent.get(reward).copied().unwrap_or(false) {
            return Err(PpddlError::RewardViolation(
                "reward may not control action applicability or goals".into(),
            ));
        }
        let initial = task.initial();
        if initial.fdef[reward] && initial.fv[reward].abs() > PROB_EPS {
            return Err(PpddlError::RewardViolation(
                "the initial value of reward must be zero".into(),
            ));
        }
    }

    let goal_reward = compiled
        .goal_reward
        .as_ref()
        .map(|expression| ResolvedMetricExpr::resolve(expression, &task))
        .transpose()?;
    let metric = if matches!(
        objective,
        ProbabilisticObjective::MaximizeExpectedMetric
            | ProbabilisticObjective::MinimizeExpectedMetric
    ) {
        let expression = compiled.declared_objective.metric.as_ref().ok_or_else(|| {
            PpddlError::InvalidOptions(
                "an expected-metric objective requires a declared PPDDL metric".into(),
            )
        })?;
        Some(ResolvedMetricExpr::resolve(expression, &task)?)
    } else {
        None
    };

    if options.horizon.is_none()
        && goal_reward
            .as_ref()
            .is_some_and(ResolvedMetricExpr::uses_total_time)
    {
        return Err(PpddlError::InvalidOptions(
            "infinite-horizon goal rewards may not reference total-time".into(),
        ));
    }
    if options.horizon.is_none()
        && metric
            .as_ref()
            .is_some_and(ResolvedMetricExpr::uses_total_time)
    {
        return Err(PpddlError::InvalidOptions(
            "infinite-horizon terminal metrics may not reference total-time".into(),
        ));
    }

    let mut state_fluents = Vec::new();
    if let Some(reward) = reward_fluent {
        let mut reward_reads = Vec::new();
        for op in 0..task.n_ops {
            for effect in task.num_eff.slice(op) {
                if effect.target as usize == reward {
                    effect.value.collect_fluents(&mut reward_reads);
                }
            }
            for conditional in task.cond_effs(op) {
                for effect in &conditional.num {
                    if effect.target as usize == reward {
                        effect.value.collect_fluents(&mut reward_reads);
                    }
                }
            }
        }
        state_fluents.extend(reward_reads.into_iter().map(|fluent| fluent as usize));
    }
    if let Some(expression) = &goal_reward {
        expression.collect_fluents(&mut state_fluents);
    }
    if let Some(expression) = &metric {
        expression.collect_fluents(&mut state_fluents);
    }
    for fluent in state_fluents {
        if Some(fluent) == reward_fluent {
            return Err(PpddlError::RewardViolation(
                "goal reward and terminal metrics may not depend on accumulated reward".into(),
            ));
        }
        if !task.relevant_fluent[fluent] {
            task.relevant_fluent[fluent] = true;
            task.rel_fluents.push(fluent as u32);
        }
    }
    task.rel_fluents.sort_unstable();
    task.rel_fluents.dedup();
    task.n_relevant_fluents = task.rel_fluents.len();

    Ok(CompiledModel {
        task,
        actions,
        initial_action,
        marker_facts,
        reward_fluent,
        goal_reward,
        objective,
        metric,
        metric_text: compiled.metric_text,
        threads,
    })
}
