//! PPDDL 1.0 probabilistic planning over ferroplan's grounded core.
//!
//! The deterministic engine stores one successor per grounded operator.  PPDDL
//! changes that object into a probability distribution over successors.  This
//! module preserves the deterministic packed task as the transition kernel,
//! expands nested probabilistic effects into weighted deterministic outcomes,
//! and solves the resulting finite explicit MDP with backward induction or
//! value iteration.
//!
//! Supported PPDDL surfaces:
//! - `:probabilistic-effects`, `:rewards`, and `:mdp` requirements;
//! - deterministic or probabilistic initial-state distributions;
//! - nested `(probabilistic p effect ...)` effects, including inside `and`,
//!   `when`, and `forall`;
//! - implicit no-op probability mass when explicit probabilities sum below 1;
//! - `(oneof ...)` as the common uniform probabilistic extension;
//! - `(increase (reward) expression)` transition rewards and `:goal-reward`;
//! - finite-horizon and infinite-horizon reachability policies;
//! - finite-horizon and discounted infinite-horizon expected-reward policies;
//! - deterministic seeded policy simulation and structural policy validation.
//!
//! Explicit-state construction is intentionally bounded.  Inputs whose
//! reachable state graph, transition graph, or normalized outcome product
//! exceeds configured limits are refused rather than silently approximated.

use std::collections::{BTreeMap, HashMap, HashSet, VecDeque};

#[cfg(feature = "schema")]
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::bitset;
use crate::ground::ground_task;
use crate::lexer::{lex, Tok};
use crate::packed::{PackedTask, State, StateKey};
use crate::parser;
use crate::types::{Domain, ParseError, Problem};

const PROB_REQ: &str = ":PROBABILISTIC-EFFECTS";
const REWARD_REQ: &str = ":REWARDS";
const VARIANT_PREFIX: &str = "PPDDL-A";
const MARKER_PREFIX: &str = "PPDDL-MARKER-A";
const INIT_PENDING: &str = "PPDDL-INIT-PENDING";
const INIT_ACTION: &str = "PPDDL-INITIALIZE";
const PROB_EPS: f64 = 1e-12;

fn default_horizon() -> Option<usize> {
    Some(64)
}
fn default_discount() -> f64 {
    1.0
}
fn default_epsilon() -> f64 {
    1e-10
}
fn default_iterations() -> usize {
    10_000
}
fn default_states() -> usize {
    100_000
}
fn default_transitions() -> usize {
    2_000_000
}
fn default_outcomes() -> usize {
    1_024
}
fn default_policy_entries() -> usize {
    200_000
}
fn default_value_cells() -> usize {
    20_000_000
}
fn default_initial_outcomes() -> usize {
    1_024
}
fn default_simulation_steps() -> usize {
    10_000
}

/// Probabilistic optimization objective.
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, Default)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[serde(rename_all = "kebab-case")]
pub enum ProbabilisticObjective {
    /// Follow PPDDL's declared/default metric: reward for `:rewards`, otherwise
    /// probability of goal achievement.
    #[default]
    Auto,
    /// Maximize the probability of reaching the hard goal.
    MaximizeGoalProbability,
    /// Minimize the probability of reaching the hard goal.
    MinimizeGoalProbability,
    /// Maximize expected accumulated transition and goal reward.
    MaximizeExpectedReward,
    /// Minimize expected accumulated transition and goal reward.
    MinimizeExpectedReward,
    /// Maximize an explicitly declared PPDDL ground numeric metric.
    MaximizeExpectedMetric,
    /// Minimize an explicitly declared PPDDL ground numeric metric.
    MinimizeExpectedMetric,
}

/// Bounded explicit-MDP solver configuration.
#[derive(Serialize, Deserialize, Clone, Debug)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
pub struct ProbabilisticOptions {
    #[serde(default)]
    pub objective: ProbabilisticObjective,
    /// `Some(h)` uses exact finite-horizon backward induction.  `None` uses
    /// value iteration.  Infinite expected reward requires `discount < 1`.
    #[serde(default = "default_horizon")]
    pub horizon: Option<usize>,
    #[serde(default = "default_discount")]
    pub discount: f64,
    #[serde(default = "default_epsilon")]
    pub epsilon: f64,
    #[serde(default = "default_iterations")]
    pub max_iterations: usize,
    #[serde(default = "default_states")]
    pub max_states: usize,
    #[serde(default = "default_transitions")]
    pub max_transitions: usize,
    #[serde(default = "default_outcomes")]
    pub max_outcomes_per_action: usize,
    #[serde(default = "default_policy_entries")]
    pub max_policy_entries: usize,
    /// Maximum number of state/value cells allocated by finite-horizon
    /// backward induction (`reachable_states * (horizon + 1)`).
    #[serde(default = "default_value_cells")]
    pub max_value_cells: usize,
    #[serde(default = "default_initial_outcomes")]
    pub max_initial_outcomes: usize,
    #[serde(default = "default_simulation_steps")]
    pub simulation_max_steps: usize,
    /// Grounding workers; `0` uses ferroplan's configured thread count.
    #[serde(default)]
    pub threads: usize,
}

impl Default for ProbabilisticOptions {
    fn default() -> Self {
        Self {
            objective: ProbabilisticObjective::default(),
            horizon: default_horizon(),
            discount: default_discount(),
            epsilon: default_epsilon(),
            max_iterations: default_iterations(),
            max_states: default_states(),
            max_transitions: default_transitions(),
            max_outcomes_per_action: default_outcomes(),
            max_policy_entries: default_policy_entries(),
            max_value_cells: default_value_cells(),
            max_initial_outcomes: default_initial_outcomes(),
            simulation_max_steps: default_simulation_steps(),
            threads: 0,
        }
    }
}

impl ProbabilisticOptions {
    fn validate(&self) -> Result<(), PpddlError> {
        if !self.discount.is_finite() || !(0.0..=1.0).contains(&self.discount) {
            return Err(PpddlError::InvalidOptions(
                "discount must be finite and in [0, 1]".into(),
            ));
        }
        if !self.epsilon.is_finite() || self.epsilon <= 0.0 {
            return Err(PpddlError::InvalidOptions(
                "epsilon must be finite and positive".into(),
            ));
        }
        if self.max_iterations == 0
            || self.max_states == 0
            || self.max_transitions == 0
            || self.max_outcomes_per_action == 0
            || self.max_policy_entries == 0
            || self.max_value_cells == 0
            || self.max_initial_outcomes == 0
            || self.simulation_max_steps == 0
        {
            return Err(PpddlError::InvalidOptions(
                "all PPDDL resource bounds must be positive".into(),
            ));
        }
        if self.horizon.is_none()
            && matches!(
                self.objective,
                ProbabilisticObjective::MaximizeExpectedReward
                    | ProbabilisticObjective::MinimizeExpectedReward
            )
            && self.discount >= 1.0
        {
            return Err(PpddlError::InvalidOptions(
                "infinite-horizon expected reward requires discount < 1".into(),
            ));
        }
        Ok(())
    }
}

/// PPDDL compilation or planning failure.
#[derive(thiserror::Error, Debug)]
pub enum PpddlError {
    #[error("PPDDL syntax error: {0}")]
    Syntax(String),
    #[error("domain parse error after PPDDL normalization: {0}")]
    DomainParse(ParseError),
    #[error("problem parse error after PPDDL normalization: {0}")]
    ProblemParse(ParseError),
    #[error("derived predicate error: {0}")]
    Derived(String),
    #[error("unsupported PPDDL combination: {0}")]
    Unsupported(String),
    #[error("invalid PPDDL probability: {0}")]
    InvalidProbability(String),
    #[error("invalid probabilistic options: {0}")]
    InvalidOptions(String),
    #[error("PPDDL outcome product exceeded configured bound {limit} for action {action}")]
    OutcomeLimit { action: String, limit: usize },
    #[error("explicit PPDDL state bound exceeded ({limit})")]
    StateLimit { limit: usize },
    #[error("explicit PPDDL transition bound exceeded ({limit})")]
    TransitionLimit { limit: usize },
    #[error("grounding failed for normalized PPDDL task")]
    GroundingFailed,
    #[error("probabilistic grounding diverged for {action}: expected {expected} outcomes, observed {observed}")]
    GroundingDivergence {
        action: String,
        expected: usize,
        observed: usize,
    },
    #[error("probabilistic initial-state outcome bound exceeded ({limit})")]
    InitialOutcomeLimit { limit: usize },
    #[error("invalid PPDDL reward use: {0}")]
    RewardViolation(String),
    #[error("policy entry bound exceeded ({limit})")]
    PolicyLimit { limit: usize },
    #[error("finite-horizon value-table bound exceeded ({limit} cells)")]
    ValueTableLimit { limit: usize },
}

/// Summary returned by [`parse_ppddl`].
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct PpddlParseReport {
    pub ok: bool,
    pub domain: Option<String>,
    pub problem: Option<String>,
    pub probabilistic_actions: usize,
    pub normalized_outcomes: usize,
    pub initial_outcomes: usize,
    pub uses_rewards: bool,
    pub goal_reward: Option<String>,
    pub error: Option<String>,
}

/// One stochastic outcome in a synthesized policy.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct PolicyOutcome {
    pub probability: f64,
    pub next_state: usize,
    pub reward: f64,
    pub goal: bool,
}

/// One policy choice. `remaining` is present for a finite-horizon policy.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct PolicyDecision {
    pub state: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub remaining: Option<usize>,
    pub action: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub args: Vec<String>,
    pub value: f64,
    pub outcomes: Vec<PolicyOutcome>,
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct ProbabilisticStatistics {
    pub grounded_facts: usize,
    pub grounded_outcome_operators: usize,
    pub grounded_actions: usize,
    pub initial_states: usize,
    pub reachable_states: usize,
    pub transitions: usize,
    pub iterations: usize,
    pub converged: bool,
    pub threads: usize,
}

/// Observable projection of one reachable PPDDL state.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ProbabilisticState {
    pub id: usize,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub facts: Vec<String>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub fluents: BTreeMap<String, f64>,
    pub goal: bool,
    #[serde(default, skip_serializing_if = "is_zero_probability")]
    pub initial_probability: f64,
}

fn is_zero_probability(value: &f64) -> bool {
    value.abs() <= PROB_EPS
}

/// One member of the PPDDL initial-state distribution.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct InitialStateProbability {
    pub state: usize,
    pub probability: f64,
    pub goal: bool,
}

/// Result of PPDDL policy synthesis.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ProbabilisticSolution {
    pub solved: bool,
    pub objective: ProbabilisticObjective,
    pub initial_value: f64,
    pub initial_distribution: Vec<InitialStateProbability>,
    pub states: Vec<ProbabilisticState>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub initial_action: Option<String>,
    pub horizon: Option<usize>,
    pub discount: f64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub declared_metric: Option<String>,
    pub policy: Vec<PolicyDecision>,
    pub statistics: ProbabilisticStatistics,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub notes: Vec<String>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct SimulationReport {
    pub episodes: usize,
    pub reached_goal: usize,
    pub goal_rate: f64,
    pub average_reward: f64,
    pub average_discounted_reward: f64,
    pub average_steps: f64,
    pub seed: u64,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct PolicyValidation {
    pub valid: bool,
    pub checked_decisions: usize,
    pub max_probability_error: f64,
    pub errors: Vec<String>,
}


include!("ppddl/syntax.rs");
include!("ppddl/compile.rs");
include!("ppddl/model.rs");
include!("ppddl/solver.rs");
