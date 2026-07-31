fn compile_problem_source(
    analysis: &ProblemAnalysis,
    uses_rewards: bool,
) -> Result<ProblemCompilation, PpddlError> {
    let root_items = define_items(&analysis.root, "problem")?;
    let mut output = vec![root_items[0].clone(), root_items[1].clone()];
    for original in root_items.iter().skip(2) {
        match section_head(original) {
            Some(":INIT") | Some(":GOAL-REWARD") | Some(":METRIC") => {}
            _ => {
                let mut section = original.clone();
                strip_ppddl_requirements(&mut section);
                output.push(section);
            }
        }
    }
    let mut init = vec![Sexp::Name(":INIT".into()), atom(INIT_PENDING)];
    if uses_rewards {
        init.push(Sexp::List(vec![
            Sexp::Op("=".into()),
            atom("REWARD"),
            Sexp::Num(0.0),
        ]));
    }
    output.push(Sexp::List(init));
    Ok(ProblemCompilation {
        source: render(&Sexp::List(output)),
        problem_name: analysis.problem_name.clone(),
    })
}

/// Parse and normalize PPDDL without constructing its explicit state graph.
pub fn parse_ppddl(domain_src: &str, problem_src: &str) -> PpddlParseReport {
    match compile_sources(
        domain_src,
        problem_src,
        default_outcomes(),
        default_initial_outcomes(),
    ) {
        Ok(compiled) => PpddlParseReport {
            ok: true,
            domain: Some(compiled.domain.name.to_lowercase()),
            problem: Some(compiled.problem.name.to_lowercase()),
            probabilistic_actions: compiled.probabilistic_actions,
            normalized_outcomes: compiled
                .variants
                .values()
                .filter(|variant| !variant.initial)
                .count(),
            initial_outcomes: compiled.initial_outcomes,
            uses_rewards: compiled.uses_rewards,
            goal_reward: compiled.goal_reward_text,
            error: None,
        },
        Err(error) => PpddlParseReport {
            ok: false,
            domain: None,
            problem: None,
            probabilistic_actions: 0,
            normalized_outcomes: 0,
            initial_outcomes: 0,
            uses_rewards: false,
            goal_reward: None,
            error: Some(error.to_string()),
        },
    }
}

struct CompiledSources {
    domain: Domain,
    problem: Problem,
    variants: HashMap<String, VariantSpec>,
    outcomes_per_action: HashMap<usize, usize>,
    marker_names: Vec<String>,
    probabilistic_actions: usize,
    initial_action_index: usize,
    initial_outcomes: usize,
    uses_rewards: bool,
    goal_reward: Option<GroundMetricExpr>,
    goal_reward_text: Option<String>,
    declared_objective: DeclaredObjective,
    metric_text: Option<String>,
}
