#[derive(Clone)]
struct GroundOutcome {
    probability: f64,
    op: usize,
    outcome_index: usize,
}

#[derive(Clone)]
struct GroundAction {
    display: String,
    base_name: String,
    args: Vec<String>,
    outcomes: Vec<GroundOutcome>,
}


#[derive(Clone)]
enum ResolvedMetricExpr {
    Number(f64),
    Fluent { index: usize, display: String },
    Add(Box<ResolvedMetricExpr>, Box<ResolvedMetricExpr>),
    Sub(Box<ResolvedMetricExpr>, Box<ResolvedMetricExpr>),
    Mul(Box<ResolvedMetricExpr>, Box<ResolvedMetricExpr>),
    Div(Box<ResolvedMetricExpr>, Box<ResolvedMetricExpr>),
    Neg(Box<ResolvedMetricExpr>),
    TotalTime,
}

impl ResolvedMetricExpr {
    fn resolve(expression: &GroundMetricExpr, task: &PackedTask) -> Result<Self, PpddlError> {
        match expression {
            GroundMetricExpr::Number(value) => Ok(Self::Number(*value)),
            GroundMetricExpr::Fluent(display) => task
                .fluent_id(display)
                .map(|index| Self::Fluent {
                    index,
                    display: display.clone(),
                })
                .ok_or_else(|| {
                    PpddlError::Unsupported(format!(
                        "PPDDL numeric expression references unknown fluent {display}"
                    ))
                }),
            GroundMetricExpr::Add(left, right) => Ok(Self::Add(
                Box::new(Self::resolve(left, task)?),
                Box::new(Self::resolve(right, task)?),
            )),
            GroundMetricExpr::Sub(left, right) => Ok(Self::Sub(
                Box::new(Self::resolve(left, task)?),
                Box::new(Self::resolve(right, task)?),
            )),
            GroundMetricExpr::Mul(left, right) => Ok(Self::Mul(
                Box::new(Self::resolve(left, task)?),
                Box::new(Self::resolve(right, task)?),
            )),
            GroundMetricExpr::Div(left, right) => Ok(Self::Div(
                Box::new(Self::resolve(left, task)?),
                Box::new(Self::resolve(right, task)?),
            )),
            GroundMetricExpr::Neg(value) => {
                Ok(Self::Neg(Box::new(Self::resolve(value, task)?)))
            }
            GroundMetricExpr::TotalTime => Ok(Self::TotalTime),
        }
    }

    fn collect_fluents(&self, output: &mut Vec<usize>) {
        match self {
            Self::Fluent { index, .. } => output.push(*index),
            Self::Add(left, right)
            | Self::Sub(left, right)
            | Self::Mul(left, right)
            | Self::Div(left, right) => {
                left.collect_fluents(output);
                right.collect_fluents(output);
            }
            Self::Neg(value) => value.collect_fluents(output),
            Self::Number(_) | Self::TotalTime => {}
        }
    }

    fn uses_total_time(&self) -> bool {
        match self {
            Self::TotalTime => true,
            Self::Add(left, right)
            | Self::Sub(left, right)
            | Self::Mul(left, right)
            | Self::Div(left, right) => left.uses_total_time() || right.uses_total_time(),
            Self::Neg(value) => value.uses_total_time(),
            Self::Number(_) | Self::Fluent { .. } => false,
        }
    }

    fn evaluate(&self, state: &State, total_time: f64) -> Result<f64, PpddlError> {
        let value = match self {
            Self::Number(value) => *value,
            Self::Fluent { index, display } => {
                if !state.fdef[*index] {
                    return Err(PpddlError::Unsupported(format!(
                        "PPDDL numeric-expression fluent {display} is undefined in a reachable state"
                    )));
                }
                state.fv[*index]
            }
            Self::Add(left, right) => left.evaluate(state, total_time)? + right.evaluate(state, total_time)?,
            Self::Sub(left, right) => left.evaluate(state, total_time)? - right.evaluate(state, total_time)?,
            Self::Mul(left, right) => left.evaluate(state, total_time)? * right.evaluate(state, total_time)?,
            Self::Div(left, right) => {
                let denominator = right.evaluate(state, total_time)?;
                if denominator == 0.0 {
                    return Err(PpddlError::Unsupported(
                        "PPDDL numeric expression divides by zero in a reachable state".into(),
                    ));
                }
                left.evaluate(state, total_time)? / denominator
            }
            Self::Neg(value) => -value.evaluate(state, total_time)?,
            Self::TotalTime => total_time,
        };
        if !value.is_finite() {
            return Err(PpddlError::Unsupported(
                "PPDDL numeric expression produced a non-finite value".into(),
            ));
        }
        Ok(value)
    }
}

struct CompiledModel {
    task: PackedTask,
    actions: Vec<GroundAction>,
    initial_action: GroundAction,
    marker_facts: Vec<usize>,
    reward_fluent: Option<usize>,
    goal_reward: Option<ResolvedMetricExpr>,
    objective: ProbabilisticObjective,
    metric: Option<ResolvedMetricExpr>,
    metric_text: Option<String>,
    threads: usize,
}
