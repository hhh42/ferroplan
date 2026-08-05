//! Production PPDDL policy synthesis, validation, simulation, and receipt emission.

use std::path::PathBuf;

use anyhow::{Context, Result};
use clap::{Parser, ValueEnum};
use ferroplan::{
    simulate_ppddl, solve_ppddl, validate_ppddl_policy, ProbabilisticObjective,
    ProbabilisticOptions,
};
use serde_json::json;

#[derive(Parser, Debug)]
#[command(
    name = "ferroplan-ppddl",
    version,
    about = "Bounded PPDDL policy synthesis with validation, replay, and receipts"
)]
struct Cli {
    /// PPDDL domain file.
    #[arg(short = 'o', long = "domain", value_name = "DOMAIN")]
    domain: PathBuf,

    /// PPDDL problem file.
    #[arg(short = 'f', long = "problem", value_name = "PROBLEM")]
    problem: PathBuf,

    /// Optimization objective.
    #[arg(long, value_enum, default_value_t = ObjectiveArg::Auto)]
    objective: ObjectiveArg,

    /// Finite planning horizon. Ignored when --infinite is set.
    #[arg(long, default_value_t = 64)]
    horizon: usize,

    /// Use infinite-horizon value iteration.
    #[arg(long)]
    infinite: bool,

    /// Discount factor in [0,1]. Infinite expected reward requires < 1.
    #[arg(long, default_value_t = 1.0)]
    discount: f64,

    /// Value-iteration convergence epsilon.
    #[arg(long, default_value_t = 1e-10)]
    epsilon: f64,

    #[arg(long, default_value_t = 10_000)]
    max_iterations: usize,

    #[arg(long, default_value_t = 100_000)]
    max_states: usize,

    #[arg(long, default_value_t = 2_000_000)]
    max_transitions: usize,

    #[arg(long, default_value_t = 1_024)]
    max_outcomes_per_action: usize,

    #[arg(long, default_value_t = 200_000)]
    max_policy_entries: usize,

    #[arg(long, default_value_t = 20_000_000)]
    max_value_cells: usize,

    #[arg(long, default_value_t = 1_024)]
    max_initial_outcomes: usize,

    #[arg(long, default_value_t = 10_000)]
    simulation_max_steps: usize,

    /// Grounding workers; 0 uses the engine default.
    #[arg(long, default_value_t = 0)]
    threads: usize,

    /// Deterministic seeded replay episode count. Zero disables simulation.
    #[arg(long, default_value_t = 1_000)]
    episodes: usize,

    /// Seed for deterministic replay.
    #[arg(long, default_value_t = 42)]
    seed: u64,

    /// Skip independent structural policy validation.
    #[arg(long)]
    no_validate: bool,
}

#[derive(Copy, Clone, Debug, ValueEnum)]
enum ObjectiveArg {
    Auto,
    MaximizeGoalProbability,
    MinimizeGoalProbability,
    MaximizeExpectedReward,
    MinimizeExpectedReward,
    MaximizeExpectedMetric,
    MinimizeExpectedMetric,
}

impl From<ObjectiveArg> for ProbabilisticObjective {
    fn from(value: ObjectiveArg) -> Self {
        match value {
            ObjectiveArg::Auto => Self::Auto,
            ObjectiveArg::MaximizeGoalProbability => Self::MaximizeGoalProbability,
            ObjectiveArg::MinimizeGoalProbability => Self::MinimizeGoalProbability,
            ObjectiveArg::MaximizeExpectedReward => Self::MaximizeExpectedReward,
            ObjectiveArg::MinimizeExpectedReward => Self::MinimizeExpectedReward,
            ObjectiveArg::MaximizeExpectedMetric => Self::MaximizeExpectedMetric,
            ObjectiveArg::MinimizeExpectedMetric => Self::MinimizeExpectedMetric,
        }
    }
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    let domain = std::fs::read_to_string(&cli.domain)
        .with_context(|| format!("reading {}", cli.domain.display()))?;
    let problem = std::fs::read_to_string(&cli.problem)
        .with_context(|| format!("reading {}", cli.problem.display()))?;

    let options = ProbabilisticOptions {
        objective: cli.objective.into(),
        horizon: if cli.infinite {
            None
        } else {
            Some(cli.horizon)
        },
        discount: cli.discount,
        epsilon: cli.epsilon,
        max_iterations: cli.max_iterations,
        max_states: cli.max_states,
        max_transitions: cli.max_transitions,
        max_outcomes_per_action: cli.max_outcomes_per_action,
        max_policy_entries: cli.max_policy_entries,
        max_value_cells: cli.max_value_cells,
        max_initial_outcomes: cli.max_initial_outcomes,
        simulation_max_steps: cli.simulation_max_steps,
        threads: cli.threads,
    };

    let solution = solve_ppddl(&domain, &problem, &options)?;
    let validation = if cli.no_validate {
        None
    } else {
        Some(validate_ppddl_policy(
            &domain, &problem, &options, &solution,
        )?)
    };
    let simulation = if cli.episodes == 0 {
        None
    } else {
        Some(simulate_ppddl(
            &domain,
            &problem,
            &options,
            cli.episodes,
            cli.seed,
        )?)
    };

    let validation_alive = validation.as_ref().map_or(true, |report| report.valid);
    let standing = if solution.solved && validation_alive {
        "ALIVE"
    } else {
        "PARTIAL_ALIVE"
    };

    let receipt = json!({
        "schema": "ferroplan-ppddl-production-receipt/v1",
        "standing": standing,
        "subject": {
            "domain_path": cli.domain,
            "problem_path": cli.problem,
            "domain_blake3": blake3::hash(domain.as_bytes()).to_hex().to_string(),
            "problem_blake3": blake3::hash(problem.as_bytes()).to_hex().to_string()
        },
        "execution": {
            "solver": "ferroplan::solve_ppddl",
            "validation": if cli.no_validate { "SKIPPED" } else { "EXECUTED" },
            "simulation_episodes": cli.episodes,
            "simulation_seed": cli.seed,
            "ci_used": false
        },
        "options": options,
        "solution": solution,
        "validation": validation,
        "simulation": simulation
    });

    println!("{}", serde_json::to_string_pretty(&receipt)?);
    std::process::exit(if standing == "ALIVE" { 0 } else { 1 });
}
