#[derive(Clone)]
struct VariantSpec {
    action_index: usize,
    outcome_index: usize,
    base_name: String,
    variant_name: String,
    marker_name: String,
    probability: f64,
    initial: bool,
}

#[derive(Clone, Debug)]
enum GroundMetricExpr {
    Number(f64),
    Fluent(String),
    Add(Box<GroundMetricExpr>, Box<GroundMetricExpr>),
    Sub(Box<GroundMetricExpr>, Box<GroundMetricExpr>),
    Mul(Box<GroundMetricExpr>, Box<GroundMetricExpr>),
    Div(Box<GroundMetricExpr>, Box<GroundMetricExpr>),
    Neg(Box<GroundMetricExpr>),
    TotalTime,
}

fn parse_ground_metric(value: &Sexp) -> Result<GroundMetricExpr, PpddlError> {
    match value {
        Sexp::Num(number) => Ok(GroundMetricExpr::Number(*number)),
        Sexp::Name(name) if name == "TOTAL-TIME" => Ok(GroundMetricExpr::TotalTime),
        Sexp::Name(name) => Ok(GroundMetricExpr::Fluent(format!("({name})"))),
        Sexp::List(items) if items.len() == 1 => {
            let name = items[0]
                .name()
                .ok_or_else(|| PpddlError::Syntax("metric fluent has no name".into()))?;
            if name == "TOTAL-TIME" {
                Ok(GroundMetricExpr::TotalTime)
            } else {
                Ok(GroundMetricExpr::Fluent(render(value)))
            }
        }
        Sexp::List(items) if items.len() == 2 && matches!(&items[0], Sexp::Dash) => {
            Ok(GroundMetricExpr::Neg(Box::new(parse_ground_metric(&items[1])?)))
        }
        Sexp::List(items)
            if items.len() == 3
                && (matches!(
                    &items[0],
                    Sexp::Op(operator) if operator == "+" || operator == "*" || operator == "/"
                ) || matches!(&items[0], Sexp::Dash)) =>
        {
            let left = Box::new(parse_ground_metric(&items[1])?);
            let right = Box::new(parse_ground_metric(&items[2])?);
            match &items[0] {
                Sexp::Op(operator) if operator == "+" => Ok(GroundMetricExpr::Add(left, right)),
                Sexp::Dash => Ok(GroundMetricExpr::Sub(left, right)),
                Sexp::Op(operator) if operator == "*" => Ok(GroundMetricExpr::Mul(left, right)),
                Sexp::Op(operator) if operator == "/" => Ok(GroundMetricExpr::Div(left, right)),
                _ => Err(PpddlError::Syntax(
                    "unsupported operator in PPDDL ground metric".into(),
                )),
            }
        }
        Sexp::List(items) if !items.is_empty() && items[0].name().is_some() => {
            if items[1..].iter().all(|item| item.name().is_some()) {
                Ok(GroundMetricExpr::Fluent(render(value)))
            } else {
                Err(PpddlError::Syntax(
                    "ground metric fluent arguments must be object names".into(),
                ))
            }
        }
        _ => Err(PpddlError::Syntax(
            "invalid PPDDL ground metric expression".into(),
        )),
    }
}

#[derive(Clone)]
struct DeclaredObjective {
    objective: ProbabilisticObjective,
    metric: Option<GroundMetricExpr>,
}

impl DeclaredObjective {
    fn resolve(&self, configured: ProbabilisticObjective) -> ProbabilisticObjective {
        if configured == ProbabilisticObjective::Auto {
            self.objective
        } else {
            configured
        }
    }
}

struct DomainCompilation {
    source: String,
    domain_name: String,
    variants: HashMap<String, VariantSpec>,
    outcomes_per_action: HashMap<usize, usize>,
    marker_names: Vec<String>,
    probabilistic_actions: usize,
    initial_action_index: usize,
    uses_rewards: bool,
}

struct ProblemAnalysis {
    root: Sexp,
    problem_name: String,
    initial_outcomes: Vec<(f64, Sexp)>,
    goal_reward: Option<GroundMetricExpr>,
    goal_reward_text: Option<String>,
    declared_objective: DeclaredObjective,
    metric_text: Option<String>,
}

struct ProblemCompilation {
    source: String,
    problem_name: String,
}

fn requirement_set(document: &Sexp, kind: &str) -> Result<HashSet<String>, PpddlError> {
    let items = define_items(document, kind)?;
    Ok(items
        .iter()
        .find(|section| section_head(section) == Some(":REQUIREMENTS"))
        .and_then(Sexp::list)
        .map(|requirements| {
            requirements
                .iter()
                .skip(1)
                .filter_map(Sexp::name)
                .map(str::to_string)
                .collect()
        })
        .unwrap_or_default())
}

fn reward_target(value: &Sexp) -> bool {
    value.head() == Some("REWARD") || value.name() == Some("REWARD")
}
