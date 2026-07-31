fn compile_sources(
    domain_src: &str,
    problem_src: &str,
    max_outcomes: usize,
    max_initial_outcomes: usize,
) -> Result<CompiledSources, PpddlError> {
    let domain_root = parse_sexp(domain_src)?;
    let problem_root = parse_sexp(problem_src)?;
    let universe = ObjectUniverse::from_documents(&domain_root, &problem_root)?;
    let mut requirements = requirement_set(&domain_root, "domain")?;
    requirements.extend(requirement_set(&problem_root, "problem")?);
    let mdp = requirements.contains(":MDP");
    let has_probabilistic = mdp || requirements.contains(PROB_REQ);
    let has_rewards = mdp || requirements.contains(REWARD_REQ);
    let analysis = analyze_problem(
        problem_root,
        &universe,
        has_probabilistic,
        has_rewards,
        max_initial_outcomes,
    )?;
    let domain_compilation = compile_domain_source(
        &domain_root,
        &universe,
        &requirements,
        &analysis.initial_outcomes,
        max_outcomes,
    )?;
    let problem_compilation =
        compile_problem_source(&analysis, domain_compilation.uses_rewards)?;
    let domain = parser::parse_domain(&domain_compilation.source).map_err(PpddlError::DomainParse)?;
    let problem =
        parser::parse_problem(&problem_compilation.source).map_err(PpddlError::ProblemParse)?;
    if domain.name != problem.domain_name {
        return Err(PpddlError::Syntax(format!(
            "problem references domain {}, expected {}",
            problem.domain_name, domain.name
        )));
    }
    if domain.name != domain_compilation.domain_name {
        return Err(PpddlError::Syntax("normalized domain name drift".into()));
    }
    if problem.name != problem_compilation.problem_name {
        return Err(PpddlError::Syntax("normalized problem name drift".into()));
    }
    if !domain.durative_actions.is_empty() || !problem.til.is_empty() {
        return Err(PpddlError::Unsupported(
            "PPDDL1.0 is discrete-time and does not include durative actions or timed initial literals"
                .into(),
        ));
    }
    if !domain.constraints.is_empty() || !problem.constraints.is_empty() {
        return Err(PpddlError::Unsupported(
            "PDDL3 trajectory constraints are outside PPDDL1.0".into(),
        ));
    }
    if !domain.derived.is_empty() {
        return Err(PpddlError::Unsupported(
            "derived predicates are outside PPDDL1.0".into(),
        ));
    }
    let (domain, problem) =
        crate::derived::compile(&domain, &problem).map_err(PpddlError::Derived)?;
    Ok(CompiledSources {
        domain,
        problem,
        variants: domain_compilation.variants,
        outcomes_per_action: domain_compilation.outcomes_per_action,
        marker_names: domain_compilation.marker_names,
        probabilistic_actions: domain_compilation.probabilistic_actions,
        initial_action_index: domain_compilation.initial_action_index,
        initial_outcomes: analysis.initial_outcomes.len(),
        uses_rewards: domain_compilation.uses_rewards,
        goal_reward: analysis.goal_reward,
        goal_reward_text: analysis.goal_reward_text,
        declared_objective: analysis.declared_objective,
        metric_text: analysis.metric_text,
    })
}
