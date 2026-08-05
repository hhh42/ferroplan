//! Bounded native implementations for every planning family admitted by
//! [`crate::planning_types::PlanningType`].
//!
//! These planners operate over one explicit, serializable state-transition
//! model.  PDDL/PPDDL front-ends may project into this model; RDF, A2A, and MCP
//! front-ends may manufacture it directly.  The module performs no actuation.

use crate::planning_types::PlanningType;
use serde::{Deserialize, Serialize};
use std::cmp::Reverse;
use std::collections::{BTreeMap, BTreeSet, BinaryHeap, VecDeque};
use std::fmt;

const PROBABILITY_SCALE: u64 = 1_000_000;

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct State {
    pub id: String,
    #[serde(default)]
    pub facts: BTreeSet<String>,
    #[serde(default)]
    pub fluents: BTreeMap<String, i64>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Goal {
    #[serde(default)]
    pub facts: BTreeSet<String>,
    #[serde(default)]
    pub numeric_min: BTreeMap<String, i64>,
    #[serde(default)]
    pub numeric_max: BTreeMap<String, i64>,
}

impl Goal {
    fn holds(&self, state: &State) -> bool {
        self.facts.is_subset(&state.facts)
            && self
                .numeric_min
                .iter()
                .all(|(name, value)| state.fluents.get(name).is_some_and(|v| v >= value))
            && self
                .numeric_max
                .iter()
                .all(|(name, value)| state.fluents.get(name).is_some_and(|v| v <= value))
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Transition {
    pub action: String,
    pub from: String,
    pub to: String,
    #[serde(default = "one")]
    pub cost: u64,
    #[serde(default = "one")]
    pub duration: u64,
    #[serde(default)]
    pub reward: i64,
    /// Probability in millionths. Deterministic edges use 1_000_000.
    #[serde(default = "probability_one")]
    pub probability_ppm: u32,
    #[serde(default)]
    pub observation: Option<String>,
    #[serde(default)]
    pub requires: BTreeSet<String>,
}

fn one() -> u64 {
    1
}
fn probability_one() -> u32 {
    PROBABILITY_SCALE as u32
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Task {
    pub id: String,
    #[serde(default)]
    pub primitive_action: Option<String>,
    #[serde(default)]
    pub requires: BTreeSet<String>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Method {
    pub id: String,
    pub task: String,
    #[serde(default)]
    pub subtasks: Vec<String>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorkflowEdge {
    pub before: String,
    pub after: String,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct QueueState {
    pub id: String,
    pub current_wip: u64,
    pub max_wip: u64,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Agent {
    pub id: String,
    #[serde(default)]
    pub capabilities: BTreeSet<String>,
    #[serde(default = "one")]
    pub capacity: u64,
    #[serde(default)]
    pub current_wip: u64,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Tool {
    pub id: String,
    #[serde(default)]
    pub capabilities: BTreeSet<String>,
    #[serde(default)]
    pub authority_bound: bool,
    #[serde(default)]
    pub verifier_bound: bool,
    #[serde(default)]
    pub receipt_bound: bool,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct RdfTriple {
    pub subject: String,
    pub predicate: String,
    pub object: String,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlanningProblem {
    #[serde(default)]
    pub states: Vec<State>,
    #[serde(default)]
    pub initial_states: Vec<String>,
    #[serde(default)]
    pub goal: Goal,
    #[serde(default)]
    pub unsafe_states: BTreeSet<String>,
    #[serde(default)]
    pub soft_goal_facts: BTreeMap<String, u64>,
    #[serde(default)]
    pub transitions: Vec<Transition>,
    #[serde(default)]
    pub tasks: Vec<Task>,
    #[serde(default)]
    pub root_tasks: Vec<String>,
    #[serde(default)]
    pub methods: Vec<Method>,
    #[serde(default)]
    pub workflow_edges: Vec<WorkflowEdge>,
    #[serde(default)]
    pub queues: Vec<QueueState>,
    #[serde(default)]
    pub agents: Vec<Agent>,
    #[serde(default)]
    pub tools: Vec<Tool>,
    #[serde(default)]
    pub rdf: Vec<RdfTriple>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlannerLimits {
    #[serde(default = "default_depth")]
    pub max_depth: usize,
    #[serde(default = "default_states")]
    pub max_states: usize,
    #[serde(default = "default_iterations")]
    pub max_iterations: usize,
}

fn default_depth() -> usize {
    128
}
fn default_states() -> usize {
    100_000
}
fn default_iterations() -> usize {
    512
}

impl Default for PlannerLimits {
    fn default() -> Self {
        Self {
            max_depth: default_depth(),
            max_states: default_states(),
            max_iterations: default_iterations(),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct UniversalPlanningRequest {
    pub planning_type: PlanningType,
    pub problem: PlanningProblem,
    #[serde(default)]
    pub limits: PlannerLimits,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlanStep {
    pub action: String,
    #[serde(default)]
    pub from: Option<String>,
    #[serde(default)]
    pub to: Option<String>,
    #[serde(default)]
    pub start: u64,
    #[serde(default)]
    pub duration: u64,
    #[serde(default)]
    pub agent: Option<String>,
    #[serde(default)]
    pub tool: Option<String>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct PolicyOutcome {
    pub state: String,
    pub probability_ppm: u32,
    #[serde(default)]
    pub observation: Option<String>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct PolicyEntry {
    pub state: String,
    pub action: String,
    #[serde(default)]
    pub outcomes: Vec<PolicyOutcome>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct UniversalPlan {
    pub planning_type: Option<PlanningType>,
    pub solved: bool,
    #[serde(default)]
    pub steps: Vec<PlanStep>,
    #[serde(default)]
    pub policy: Vec<PolicyEntry>,
    #[serde(default)]
    pub decomposition: Vec<String>,
    #[serde(default)]
    pub notes: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "code", rename_all = "SCREAMING_SNAKE_CASE")]
pub enum PlannerError {
    EmptyInitialState,
    UnknownState {
        state: String,
    },
    InvalidProbabilityMass {
        state: String,
        action: String,
        mass: u64,
    },
    ResourceBound {
        resource: String,
        limit: usize,
    },
    NoPlan,
    HierarchyCycle {
        task: String,
    },
    UnknownTask {
        task: String,
    },
    NoMethod {
        task: String,
    },
    WorkflowCycle,
    WipBoundExceeded {
        queue: String,
        current: u64,
        max: u64,
    },
    CapabilityUncovered {
        item: String,
        missing: BTreeSet<String>,
    },
    AuthorityUnbound {
        tool: String,
    },
    VerifierUnbound {
        tool: String,
    },
    ReceiptUnbound {
        tool: String,
    },
    InvalidRdfProjection {
        reason: String,
    },
}

impl fmt::Display for PlannerError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{self:?}")
    }
}
impl std::error::Error for PlannerError {}

/// Execute the bounded native planner corresponding to the request type.
pub fn solve_planning_type(
    request: &UniversalPlanningRequest,
) -> Result<UniversalPlan, PlannerError> {
    validate_problem(&request.problem)?;
    let mut plan = match request.planning_type {
        PlanningType::Classical => shortest_path(&request.problem, Metric::Steps, &request.limits),
        PlanningType::CostOptimal => shortest_path(&request.problem, Metric::Cost, &request.limits),
        PlanningType::Numeric => shortest_path(&request.problem, Metric::Cost, &request.limits),
        PlanningType::Temporal => {
            shortest_path(&request.problem, Metric::Duration, &request.limits)
        }
        PlanningType::Preferences => preference_plan(&request.problem, &request.limits),
        PlanningType::Probabilistic => probabilistic_policy(&request.problem, &request.limits),
        PlanningType::Fond => fond_policy(&request.problem, &request.limits),
        PlanningType::Conformant => conformant_plan(&request.problem, &request.limits),
        PlanningType::Contingent => contingent_policy(&request.problem, &request.limits),
        PlanningType::Hierarchical => hierarchical_plan(&request.problem, &request.limits),
        PlanningType::PartialOrder | PlanningType::Workflow => {
            workflow_plan(&request.problem, &request.limits)
        }
        PlanningType::FlowConstrained => flow_plan(&request.problem, &request.limits),
        PlanningType::ResolutionAdaptive => resolution_plan(&request.problem, &request.limits),
        PlanningType::MultiAgent => multi_agent_plan(&request.problem, &request.limits),
        PlanningType::RdfDerived => rdf_plan(&request.problem, &request.limits),
        PlanningType::A2aDelegated => delegated_plan(&request.problem, &request.limits),
        PlanningType::McpBound => mcp_plan(&request.problem, &request.limits),
    }?;
    plan.planning_type = Some(request.planning_type);
    Ok(plan)
}

fn validate_problem(problem: &PlanningProblem) -> Result<(), PlannerError> {
    if problem.initial_states.is_empty()
        && !matches!(problem.tasks.as_slice(), [_, ..])
        && problem.rdf.is_empty()
    {
        return Err(PlannerError::EmptyInitialState);
    }
    let states = problem
        .states
        .iter()
        .map(|state| state.id.as_str())
        .collect::<BTreeSet<_>>();
    for initial in &problem.initial_states {
        if !states.contains(initial.as_str()) {
            return Err(PlannerError::UnknownState {
                state: initial.clone(),
            });
        }
    }
    for transition in &problem.transitions {
        for state in [&transition.from, &transition.to] {
            if !states.contains(state.as_str()) {
                return Err(PlannerError::UnknownState {
                    state: state.clone(),
                });
            }
        }
    }
    let mut masses = BTreeMap::<(&str, &str), u64>::new();
    for edge in &problem.transitions {
        *masses.entry((&edge.from, &edge.action)).or_default() += u64::from(edge.probability_ppm);
    }
    for ((state, action), mass) in masses {
        if mass != PROBABILITY_SCALE {
            return Err(PlannerError::InvalidProbabilityMass {
                state: state.to_owned(),
                action: action.to_owned(),
                mass,
            });
        }
    }
    Ok(())
}

#[derive(Clone, Copy)]
enum Metric {
    Steps,
    Cost,
    Duration,
}

fn edge_weight(edge: &Transition, metric: Metric) -> u64 {
    match metric {
        Metric::Steps => 1,
        Metric::Cost => edge.cost,
        Metric::Duration => edge.duration,
    }
}

fn state_index(problem: &PlanningProblem) -> BTreeMap<&str, &State> {
    problem
        .states
        .iter()
        .map(|state| (state.id.as_str(), state))
        .collect()
}

fn grouped_edges(problem: &PlanningProblem) -> BTreeMap<&str, Vec<&Transition>> {
    let mut grouped = BTreeMap::<&str, Vec<&Transition>>::new();
    for edge in &problem.transitions {
        grouped.entry(edge.from.as_str()).or_default().push(edge);
    }
    grouped
}

fn action_groups(problem: &PlanningProblem) -> BTreeMap<(&str, &str), Vec<&Transition>> {
    let mut grouped = BTreeMap::<(&str, &str), Vec<&Transition>>::new();
    for edge in &problem.transitions {
        grouped
            .entry((edge.from.as_str(), edge.action.as_str()))
            .or_default()
            .push(edge);
    }
    grouped
}

fn reconstruct(goal: &str, parent: &BTreeMap<String, (String, Transition)>) -> Vec<PlanStep> {
    let mut cursor = goal.to_owned();
    let mut reverse = Vec::new();
    while let Some((previous, edge)) = parent.get(&cursor) {
        reverse.push(PlanStep {
            action: edge.action.clone(),
            from: Some(edge.from.clone()),
            to: Some(edge.to.clone()),
            duration: edge.duration,
            ..PlanStep::default()
        });
        cursor = previous.clone();
    }
    reverse.reverse();
    let mut start = 0;
    for step in &mut reverse {
        step.start = start;
        start += step.duration;
    }
    reverse
}

fn shortest_path(
    problem: &PlanningProblem,
    metric: Metric,
    limits: &PlannerLimits,
) -> Result<UniversalPlan, PlannerError> {
    let states = state_index(problem);
    let outgoing = grouped_edges(problem);
    let mut heap = BinaryHeap::new();
    let mut distance = BTreeMap::<String, u64>::new();
    let mut parent = BTreeMap::<String, (String, Transition)>::new();
    for initial in &problem.initial_states {
        distance.insert(initial.clone(), 0);
        heap.push(Reverse((0_u64, initial.clone())));
    }
    let mut visited = 0;
    while let Some(Reverse((cost, state_id))) = heap.pop() {
        if distance.get(&state_id).is_some_and(|best| cost != *best) {
            continue;
        }
        visited += 1;
        if visited > limits.max_states {
            return Err(PlannerError::ResourceBound {
                resource: "states".to_owned(),
                limit: limits.max_states,
            });
        }
        let state = states[state_id.as_str()];
        if problem.goal.holds(state) {
            return Ok(UniversalPlan {
                solved: true,
                steps: reconstruct(&state_id, &parent),
                ..UniversalPlan::default()
            });
        }
        for edge in outgoing.get(state_id.as_str()).into_iter().flatten() {
            if problem.unsafe_states.contains(&edge.to) {
                continue;
            }
            let next = cost.saturating_add(edge_weight(edge, metric));
            if distance.get(&edge.to).map_or(true, |known| next < *known) {
                distance.insert(edge.to.clone(), next);
                parent.insert(edge.to.clone(), (state_id.clone(), (*edge).clone()));
                heap.push(Reverse((next, edge.to.clone())));
            }
        }
    }
    Err(PlannerError::NoPlan)
}

fn preference_plan(
    problem: &PlanningProblem,
    limits: &PlannerLimits,
) -> Result<UniversalPlan, PlannerError> {
    let states = state_index(problem);
    let outgoing = grouped_edges(problem);
    let mut heap = BinaryHeap::new();
    let mut distance = BTreeMap::<String, u64>::new();
    let mut parent = BTreeMap::<String, (String, Transition)>::new();
    let mut best_goal: Option<(u64, String)> = None;
    for initial in &problem.initial_states {
        distance.insert(initial.clone(), 0);
        heap.push(Reverse((0_u64, initial.clone())));
    }
    let mut visited = 0;
    while let Some(Reverse((cost, state_id))) = heap.pop() {
        if distance.get(&state_id).is_some_and(|best| cost != *best) {
            continue;
        }
        visited += 1;
        if visited > limits.max_states {
            return Err(PlannerError::ResourceBound {
                resource: "states".to_owned(),
                limit: limits.max_states,
            });
        }
        let state = states[state_id.as_str()];
        if problem.goal.holds(state) {
            let penalty = problem
                .soft_goal_facts
                .iter()
                .filter(|(fact, _)| !state.facts.contains(*fact))
                .map(|(_, penalty)| *penalty)
                .sum::<u64>();
            let score = cost.saturating_add(penalty);
            if best_goal.as_ref().map_or(true, |(best, _)| score < *best) {
                best_goal = Some((score, state_id.clone()));
            }
        }
        for edge in outgoing.get(state_id.as_str()).into_iter().flatten() {
            if problem.unsafe_states.contains(&edge.to) {
                continue;
            }
            let next = cost.saturating_add(edge.cost);
            if distance.get(&edge.to).map_or(true, |known| next < *known) {
                distance.insert(edge.to.clone(), next);
                parent.insert(edge.to.clone(), (state_id.clone(), (*edge).clone()));
                heap.push(Reverse((next, edge.to.clone())));
            }
        }
    }
    let (_, goal) = best_goal.ok_or(PlannerError::NoPlan)?;
    Ok(UniversalPlan {
        solved: true,
        steps: reconstruct(&goal, &parent),
        ..UniversalPlan::default()
    })
}

fn probabilistic_policy(
    problem: &PlanningProblem,
    limits: &PlannerLimits,
) -> Result<UniversalPlan, PlannerError> {
    let _states = state_index(problem);
    let groups = action_groups(problem);
    let mut values = problem
        .states
        .iter()
        .map(|state| {
            let value = if problem.goal.holds(state) {
                PROBABILITY_SCALE
            } else {
                0
            };
            (state.id.clone(), value as i128)
        })
        .collect::<BTreeMap<_, _>>();
    let mut choices = BTreeMap::<String, String>::new();
    for _ in 0..limits.max_iterations {
        let mut changed = false;
        let mut next_values = values.clone();
        for state in &problem.states {
            if problem.goal.holds(state) || problem.unsafe_states.contains(&state.id) {
                continue;
            }
            let mut best: Option<(i128, String)> = None;
            for ((from, action), edges) in &groups {
                if *from != state.id {
                    continue;
                }
                let score = edges
                    .iter()
                    .map(|edge| {
                        i128::from(edge.probability_ppm)
                            * (values[&edge.to] + i128::from(edge.reward))
                    })
                    .sum::<i128>()
                    / i128::from(PROBABILITY_SCALE);
                if best.as_ref().map_or(true, |(value, _)| score > *value) {
                    best = Some((score, (*action).to_owned()));
                }
            }
            if let Some((value, action)) = best {
                if next_values[&state.id] != value {
                    changed = true;
                }
                next_values.insert(state.id.clone(), value);
                choices.insert(state.id.clone(), action);
            }
        }
        values = next_values;
        if !changed {
            break;
        }
    }
    let initial_has_value = problem
        .initial_states
        .iter()
        .any(|state| values.get(state).is_some_and(|value| *value > 0));
    if !initial_has_value {
        return Err(PlannerError::NoPlan);
    }
    Ok(policy_from_choices(
        problem,
        choices,
        "bounded value iteration",
    ))
}

fn policy_from_choices(
    problem: &PlanningProblem,
    choices: BTreeMap<String, String>,
    note: &str,
) -> UniversalPlan {
    let groups = action_groups(problem);
    let policy = choices
        .into_iter()
        .map(|(state, action)| PolicyEntry {
            outcomes: groups
                .get(&(state.as_str(), action.as_str()))
                .into_iter()
                .flatten()
                .map(|edge| PolicyOutcome {
                    state: edge.to.clone(),
                    probability_ppm: edge.probability_ppm,
                    observation: edge.observation.clone(),
                })
                .collect(),
            state,
            action,
        })
        .collect();
    UniversalPlan {
        solved: true,
        policy,
        notes: vec![note.to_owned()],
        ..UniversalPlan::default()
    }
}

fn fond_policy(
    problem: &PlanningProblem,
    limits: &PlannerLimits,
) -> Result<UniversalPlan, PlannerError> {
    let states = state_index(problem);
    let groups = action_groups(problem);
    let mut winning = problem
        .states
        .iter()
        .filter(|state| problem.goal.holds(state))
        .map(|state| state.id.clone())
        .collect::<BTreeSet<_>>();
    let mut choices = BTreeMap::<String, String>::new();
    for _ in 0..limits.max_iterations {
        let mut changed = false;
        for state in &problem.states {
            if winning.contains(&state.id) || problem.unsafe_states.contains(&state.id) {
                continue;
            }
            for ((from, action), outcomes) in &groups {
                if *from == state.id
                    && !outcomes.is_empty()
                    && outcomes.iter().all(|edge| winning.contains(&edge.to))
                {
                    winning.insert(state.id.clone());
                    choices.insert(state.id.clone(), (*action).to_owned());
                    changed = true;
                    break;
                }
            }
        }
        if !changed {
            break;
        }
    }
    if !problem
        .initial_states
        .iter()
        .all(|state| winning.contains(state))
    {
        return Err(PlannerError::NoPlan);
    }
    // Goal states need no policy entry; all non-goal winning states do.
    for state in &winning {
        if !problem.goal.holds(states[state.as_str()]) && !choices.contains_key(state) {
            return Err(PlannerError::NoPlan);
        }
    }
    Ok(policy_from_choices(
        problem,
        choices,
        "strong FOND fixed point",
    ))
}

fn conformant_plan(
    problem: &PlanningProblem,
    limits: &PlannerLimits,
) -> Result<UniversalPlan, PlannerError> {
    let states = state_index(problem);
    let groups = action_groups(problem);
    let start = problem
        .initial_states
        .iter()
        .cloned()
        .collect::<BTreeSet<_>>();
    let mut queue = VecDeque::from([(start.clone(), Vec::<String>::new())]);
    let mut seen = BTreeSet::from([start]);
    while let Some((belief, actions)) = queue.pop_front() {
        if belief
            .iter()
            .all(|id| problem.goal.holds(states[id.as_str()]))
        {
            return Ok(UniversalPlan {
                solved: true,
                steps: actions
                    .into_iter()
                    .map(|action| PlanStep {
                        action,
                        ..PlanStep::default()
                    })
                    .collect(),
                notes: vec!["belief-state breadth-first search".to_owned()],
                ..UniversalPlan::default()
            });
        }
        if actions.len() >= limits.max_depth || seen.len() > limits.max_states {
            continue;
        }
        let candidates = groups
            .keys()
            .filter(|(state, _)| belief.contains(*state))
            .map(|(_, action)| (*action).to_owned())
            .collect::<BTreeSet<_>>();
        for action in candidates {
            let mut successor = BTreeSet::new();
            let mut applicable = true;
            for state in &belief {
                if let Some(outcomes) = groups.get(&(state.as_str(), action.as_str())) {
                    successor.extend(outcomes.iter().map(|edge| edge.to.clone()));
                } else {
                    applicable = false;
                    break;
                }
            }
            if applicable
                && successor
                    .iter()
                    .all(|state| !problem.unsafe_states.contains(state))
                && seen.insert(successor.clone())
            {
                let mut next_actions = actions.clone();
                next_actions.push(action);
                queue.push_back((successor, next_actions));
            }
        }
    }
    Err(PlannerError::NoPlan)
}

fn contingent_policy(
    problem: &PlanningProblem,
    limits: &PlannerLimits,
) -> Result<UniversalPlan, PlannerError> {
    let states = state_index(problem);
    let groups = action_groups(problem);
    let start = problem
        .initial_states
        .iter()
        .cloned()
        .collect::<BTreeSet<_>>();
    let mut memo = BTreeMap::<BTreeSet<String>, Option<Vec<PolicyEntry>>>::new();
    fn solve_belief(
        belief: &BTreeSet<String>,
        depth: usize,
        problem: &PlanningProblem,
        states: &BTreeMap<&str, &State>,
        groups: &BTreeMap<(&str, &str), Vec<&Transition>>,
        limits: &PlannerLimits,
        memo: &mut BTreeMap<BTreeSet<String>, Option<Vec<PolicyEntry>>>,
    ) -> Option<Vec<PolicyEntry>> {
        if belief
            .iter()
            .all(|state| problem.goal.holds(states[state.as_str()]))
        {
            return Some(Vec::new());
        }
        if depth >= limits.max_depth || memo.len() >= limits.max_states {
            return None;
        }
        if let Some(cached) = memo.get(belief) {
            return cached.clone();
        }
        memo.insert(belief.clone(), None);
        let actions = groups
            .keys()
            .filter(|(state, _)| belief.contains(*state))
            .map(|(_, action)| (*action).to_owned())
            .collect::<BTreeSet<_>>();
        for action in actions {
            let mut branches = BTreeMap::<String, BTreeSet<String>>::new();
            let mut applicable = true;
            for state in belief {
                let Some(outcomes) = groups.get(&(state.as_str(), action.as_str())) else {
                    applicable = false;
                    break;
                };
                for edge in outcomes {
                    let observation = edge.observation.clone().unwrap_or_else(|| edge.to.clone());
                    branches
                        .entry(observation)
                        .or_default()
                        .insert(edge.to.clone());
                }
            }
            if !applicable
                || branches
                    .values()
                    .flatten()
                    .any(|state| problem.unsafe_states.contains(state))
            {
                continue;
            }
            let mut combined = Vec::new();
            let mut all_solved = true;
            for branch in branches.values() {
                if let Some(mut child) =
                    solve_belief(branch, depth + 1, problem, states, groups, limits, memo)
                {
                    combined.append(&mut child);
                } else {
                    all_solved = false;
                    break;
                }
            }
            if all_solved {
                combined.push(PolicyEntry {
                    state: belief.iter().cloned().collect::<Vec<_>>().join("|"),
                    action,
                    outcomes: branches
                        .into_iter()
                        .map(|(observation, states)| PolicyOutcome {
                            state: states.into_iter().collect::<Vec<_>>().join("|"),
                            probability_ppm: 0,
                            observation: Some(observation),
                        })
                        .collect(),
                });
                memo.insert(belief.clone(), Some(combined.clone()));
                return Some(combined);
            }
        }
        None
    }
    let policy = solve_belief(&start, 0, problem, &states, &groups, limits, &mut memo)
        .ok_or(PlannerError::NoPlan)?;
    Ok(UniversalPlan {
        solved: true,
        policy,
        notes: vec!["AND-OR contingent belief policy".to_owned()],
        ..UniversalPlan::default()
    })
}

fn task_map(problem: &PlanningProblem) -> BTreeMap<&str, &Task> {
    problem
        .tasks
        .iter()
        .map(|task| (task.id.as_str(), task))
        .collect()
}

fn hierarchical_plan(
    problem: &PlanningProblem,
    limits: &PlannerLimits,
) -> Result<UniversalPlan, PlannerError> {
    let tasks = task_map(problem);
    let mut methods = BTreeMap::<&str, Vec<&Method>>::new();
    for method in &problem.methods {
        methods
            .entry(method.task.as_str())
            .or_default()
            .push(method);
    }
    fn expand(
        task_id: &str,
        tasks: &BTreeMap<&str, &Task>,
        methods: &BTreeMap<&str, Vec<&Method>>,
        stack: &mut BTreeSet<String>,
        output: &mut Vec<String>,
        depth: usize,
        limits: &PlannerLimits,
    ) -> Result<(), PlannerError> {
        if depth > limits.max_depth || output.len() > limits.max_states {
            return Err(PlannerError::ResourceBound {
                resource: "hierarchy".to_owned(),
                limit: limits.max_states,
            });
        }
        let task = tasks
            .get(task_id)
            .ok_or_else(|| PlannerError::UnknownTask {
                task: task_id.to_owned(),
            })?;
        if let Some(action) = &task.primitive_action {
            output.push(action.clone());
            return Ok(());
        }
        if !stack.insert(task_id.to_owned()) {
            return Err(PlannerError::HierarchyCycle {
                task: task_id.to_owned(),
            });
        }
        let method = methods
            .get(task_id)
            .and_then(|candidates| candidates.first())
            .ok_or_else(|| PlannerError::NoMethod {
                task: task_id.to_owned(),
            })?;
        for subtask in &method.subtasks {
            expand(subtask, tasks, methods, stack, output, depth + 1, limits)?;
        }
        stack.remove(task_id);
        Ok(())
    }
    let mut actions = Vec::new();
    let mut stack = BTreeSet::new();
    for root in &problem.root_tasks {
        expand(root, &tasks, &methods, &mut stack, &mut actions, 0, limits)?;
    }
    Ok(UniversalPlan {
        solved: true,
        decomposition: actions.clone(),
        steps: actions
            .into_iter()
            .map(|action| PlanStep {
                action,
                ..PlanStep::default()
            })
            .collect(),
        ..UniversalPlan::default()
    })
}

fn workflow_plan(
    problem: &PlanningProblem,
    limits: &PlannerLimits,
) -> Result<UniversalPlan, PlannerError> {
    let tasks = task_map(problem);
    let mut indegree = problem
        .tasks
        .iter()
        .map(|task| (task.id.clone(), 0_usize))
        .collect::<BTreeMap<_, _>>();
    let mut outgoing = BTreeMap::<String, Vec<String>>::new();
    for edge in &problem.workflow_edges {
        if !tasks.contains_key(edge.before.as_str()) {
            return Err(PlannerError::UnknownTask {
                task: edge.before.clone(),
            });
        }
        if !tasks.contains_key(edge.after.as_str()) {
            return Err(PlannerError::UnknownTask {
                task: edge.after.clone(),
            });
        }
        *indegree.entry(edge.after.clone()).or_default() += 1;
        outgoing
            .entry(edge.before.clone())
            .or_default()
            .push(edge.after.clone());
    }
    let mut ready = indegree
        .iter()
        .filter(|(_, degree)| **degree == 0)
        .map(|(task, _)| task.clone())
        .collect::<BTreeSet<_>>();
    let mut order = Vec::new();
    while let Some(task) = ready.pop_first() {
        order.push(task.clone());
        if order.len() > limits.max_states {
            return Err(PlannerError::ResourceBound {
                resource: "workflow".to_owned(),
                limit: limits.max_states,
            });
        }
        for child in outgoing.get(&task).into_iter().flatten() {
            let degree = indegree.get_mut(child).expect("workflow child exists");
            *degree -= 1;
            if *degree == 0 {
                ready.insert(child.clone());
            }
        }
    }
    if order.len() != problem.tasks.len() {
        return Err(PlannerError::WorkflowCycle);
    }
    let steps = order
        .iter()
        .filter_map(|task| tasks[task.as_str()].primitive_action.clone())
        .map(|action| PlanStep {
            action,
            ..PlanStep::default()
        })
        .collect();
    Ok(UniversalPlan {
        solved: true,
        steps,
        decomposition: order,
        notes: vec!["stable topological order".to_owned()],
        ..UniversalPlan::default()
    })
}

fn flow_plan(
    problem: &PlanningProblem,
    limits: &PlannerLimits,
) -> Result<UniversalPlan, PlannerError> {
    for queue in &problem.queues {
        if queue.current_wip >= queue.max_wip {
            return Err(PlannerError::WipBoundExceeded {
                queue: queue.id.clone(),
                current: queue.current_wip,
                max: queue.max_wip,
            });
        }
    }
    let mut plan = shortest_path(problem, Metric::Cost, limits)?;
    plan.notes
        .push("Little-law admission bounds satisfied".to_owned());
    Ok(plan)
}

fn resolution_plan(
    problem: &PlanningProblem,
    limits: &PlannerLimits,
) -> Result<UniversalPlan, PlannerError> {
    let mut plan = hierarchical_plan(problem, limits)?;
    plan.notes
        .push("expanded until every leaf was primitive".to_owned());
    Ok(plan)
}

fn assign_agents(
    steps: &mut [PlanStep],
    requirements: &BTreeMap<String, BTreeSet<String>>,
    agents: &[Agent],
) -> Result<(), PlannerError> {
    let mut loads = agents
        .iter()
        .map(|agent| (agent.id.clone(), agent.current_wip))
        .collect::<BTreeMap<_, _>>();
    for step in steps {
        let required = requirements.get(&step.action).cloned().unwrap_or_default();
        let candidate = agents
            .iter()
            .filter(|agent| {
                required.is_subset(&agent.capabilities)
                    && loads.get(&agent.id).copied().unwrap_or_default() < agent.capacity
            })
            .min_by_key(|agent| (loads.get(&agent.id).copied().unwrap_or_default(), &agent.id));
        let Some(agent) = candidate else {
            return Err(PlannerError::CapabilityUncovered {
                item: step.action.clone(),
                missing: required,
            });
        };
        *loads.entry(agent.id.clone()).or_default() += 1;
        step.agent = Some(agent.id.clone());
    }
    Ok(())
}

fn action_requirements(problem: &PlanningProblem) -> BTreeMap<String, BTreeSet<String>> {
    let mut requirements = BTreeMap::new();
    for transition in &problem.transitions {
        requirements
            .entry(transition.action.clone())
            .or_insert_with(|| transition.requires.clone());
    }
    for task in &problem.tasks {
        if let Some(action) = &task.primitive_action {
            requirements
                .entry(action.clone())
                .or_insert_with(|| task.requires.clone());
        }
    }
    requirements
}

fn multi_agent_plan(
    problem: &PlanningProblem,
    limits: &PlannerLimits,
) -> Result<UniversalPlan, PlannerError> {
    let mut plan = shortest_path(problem, Metric::Cost, limits)?;
    assign_agents(
        &mut plan.steps,
        &action_requirements(problem),
        &problem.agents,
    )?;
    plan.notes
        .push("capacity-aware multi-agent assignment".to_owned());
    Ok(plan)
}

fn delegated_plan(
    problem: &PlanningProblem,
    limits: &PlannerLimits,
) -> Result<UniversalPlan, PlannerError> {
    let mut plan = if problem.root_tasks.is_empty() {
        shortest_path(problem, Metric::Cost, limits)?
    } else {
        hierarchical_plan(problem, limits)?
    };
    assign_agents(
        &mut plan.steps,
        &action_requirements(problem),
        &problem.agents,
    )?;
    plan.notes.push("A2A capability delegation".to_owned());
    Ok(plan)
}

fn mcp_plan(
    problem: &PlanningProblem,
    limits: &PlannerLimits,
) -> Result<UniversalPlan, PlannerError> {
    let mut plan = if problem.root_tasks.is_empty() {
        shortest_path(problem, Metric::Cost, limits)?
    } else {
        hierarchical_plan(problem, limits)?
    };
    let requirements = action_requirements(problem);
    for step in &mut plan.steps {
        let required = requirements.get(&step.action).cloned().unwrap_or_default();
        let Some(tool) = problem
            .tools
            .iter()
            .filter(|tool| required.is_subset(&tool.capabilities))
            .min_by_key(|tool| &tool.id)
        else {
            return Err(PlannerError::CapabilityUncovered {
                item: step.action.clone(),
                missing: required,
            });
        };
        if !tool.authority_bound {
            return Err(PlannerError::AuthorityUnbound {
                tool: tool.id.clone(),
            });
        }
        if !tool.verifier_bound {
            return Err(PlannerError::VerifierUnbound {
                tool: tool.id.clone(),
            });
        }
        if !tool.receipt_bound {
            return Err(PlannerError::ReceiptUnbound {
                tool: tool.id.clone(),
            });
        }
        step.tool = Some(tool.id.clone());
    }
    plan.notes
        .push("MCP primitive capability binding".to_owned());
    Ok(plan)
}

fn rdf_plan(
    problem: &PlanningProblem,
    limits: &PlannerLimits,
) -> Result<UniversalPlan, PlannerError> {
    let mut projected = problem.clone();
    if projected.rdf.is_empty() {
        return Err(PlannerError::InvalidRdfProjection {
            reason: "empty graph".to_owned(),
        });
    }
    let mut records = BTreeMap::<String, BTreeMap<String, Vec<String>>>::new();
    for triple in &projected.rdf {
        records
            .entry(triple.subject.clone())
            .or_default()
            .entry(triple.predicate.clone())
            .or_default()
            .push(triple.object.clone());
    }
    let mut state_ids = BTreeSet::new();
    for (subject, predicates) in &records {
        if predicates.contains_key("state") {
            state_ids.insert(subject.clone());
        }
        if predicates.contains_key("initial") {
            projected.initial_states.push(subject.clone());
            state_ids.insert(subject.clone());
        }
        if predicates.contains_key("goal") {
            projected.goal.facts.insert(format!("goal:{subject}"));
            state_ids.insert(subject.clone());
        }
    }
    for state in &state_ids {
        let mut facts = BTreeSet::new();
        if records[state].contains_key("goal") {
            facts.insert(format!("goal:{state}"));
        }
        projected.states.push(State {
            id: state.clone(),
            facts,
            fluents: BTreeMap::new(),
        });
    }
    for (subject, predicates) in &records {
        let Some(from) = predicates.get("from").and_then(|v| v.first()) else {
            continue;
        };
        let Some(to) = predicates.get("to").and_then(|v| v.first()) else {
            continue;
        };
        let action = predicates
            .get("action")
            .and_then(|v| v.first())
            .cloned()
            .unwrap_or_else(|| subject.clone());
        projected.transitions.push(Transition {
            action,
            from: from.clone(),
            to: to.clone(),
            cost: 1,
            duration: 1,
            reward: 0,
            probability_ppm: probability_one(),
            observation: None,
            requires: BTreeSet::new(),
        });
    }
    projected.rdf.clear();
    // The source problem may already carry projections; retain only one state per ID.
    let mut unique = BTreeMap::new();
    for state in projected.states {
        unique.insert(state.id.clone(), state);
    }
    projected.states = unique.into_values().collect();
    projected.initial_states.sort();
    projected.initial_states.dedup();
    validate_problem(&projected)?;
    let mut plan = shortest_path(&projected, Metric::Cost, limits)?;
    plan.notes
        .push("RDF graph projected into bounded state space".to_owned());
    Ok(plan)
}
