//! # ferroplan
//!
//! A fast, data-parallel [PDDL](https://en.wikipedia.org/wiki/Planning_Domain_Definition_Language)
//! planner in Rust — a deterministic planning core for the age of AI. The engine is a
//! delete-relaxation FF heuristic over a data-oriented (bitset / structure-of-arrays)
//! task representation, with enforced hill-climbing + best-first fallback and parallel
//! grounding / heuristic evaluation.
//!
//! PDDL coverage: STRIPS, typing, ADL (conditional/`forall` effects, equality),
//! numeric fluents, derived axioms, **PDDL3** soft-goal preferences/metric,
//! **PDDL2.1 temporal** durative actions, and **PPDDL 1.0 probabilistic planning**
//! with explicit-MDP policy synthesis, rewards, simulation, and policy validation.
//! Plus an SGPlan-style **partition-and-resolve** mode.
//!
//! The [`eve`] module adds the relational outer contract: human purpose is
//! grounded against a Genesis ontology, projected through SPARQL, decomposed
//! through HDDL, governed through PPDDL when uncertainty is explicit, and
//! handed to ggen/MCP+ with Truex receipt and replay obligations intact.
//!
//! ## The public API (all `serde`-serializable)
//!
//! - [`solve`] — plan a deterministic domain + problem; returns a [`Solution`].
//! - [`solve_ppddl`] — synthesize a bounded stochastic policy for PPDDL.
//! - [`simulate_ppddl`] / [`validate_ppddl_policy`] — probabilistic execution receipts.
//! - [`decompose`] — split and solve a temporal goal as ordered [`Contract`]s.
//! - [`Eve::enter`] — compile human purpose into the Genesis/HDDL/PPDDL/ggen/MCP+
//!   consequence handoff without granting actuation authority.
//! - [`parse`] / [`parse_ppddl`] — fast syntax and structure feedback.
//! - [`Session`] — ground once, replan many for mutable deterministic worlds.
//! - [`plan::validate_plan`] — independently check a deterministic plan.
//!
//! ## Quick start
//! ```no_run
//! let domain = std::fs::read_to_string("domain.pddl").unwrap();
//! let problem = std::fs::read_to_string("problem.pddl").unwrap();
//!
//! let solution = ferroplan::solve(&domain, &problem, &ferroplan::Options::default()).unwrap();
//! if let Some(plan) = solution.plan {
//!     for step in &plan.steps { println!("{}", step.action); }
//! }
//! ```

// engine (data-oriented core)
pub mod bitset;
pub mod clock;
pub mod derived;
pub mod features;
pub mod ground;
pub mod hash;
pub mod heuristic;
pub mod introspect;
pub mod invariants;
pub mod lama;
pub mod landmarks;
pub mod lexer;
pub mod novelty;
pub mod optimal;
pub mod orbits;
pub mod output;
pub mod packed;
pub mod par;
pub mod parser;
pub mod resource;
pub mod search;
pub mod types;

// modes (built on the engine)
pub mod constraints;
pub mod costs;
pub mod espc;
pub mod partition;
pub mod pddl3;
pub mod plan;
pub mod portfolio;
pub mod ppddl;
pub mod report;
pub mod resolve;
pub mod selection;
pub mod temporal;
pub mod trace;
pub mod tresolve;
pub mod tsched;
pub mod verify;
pub mod viz;

// relational orchestration + smart public API
pub mod api;
pub mod eve;
pub mod planner;
pub mod session;

pub use api::{
    decompose, parse, solve, Contract, Decomposition, DomainSummary, Metric, Mode, Options,
    ParseReport, Plan, ProblemSummary, Search, Solution, SolveError, Statistics, Step,
};
pub use eve::{
    Activator, CapabilityTarget, Eve, EveError, EveHandoff, EveRequest, EveStage,
    GenesisProjection, GenesisWorld, GgenManufacturingRequest, GroundedGoal,
    HddlDecompositionRequest, HddlSurface, HumanPurpose, ManufactureTarget, McpPlusHandoff,
    PlanningRegime, PpddlPolicyRequest, PpddlSurface, SplitDirective, TruexContinuation,
    MAX_PRIMARY_ACTIVATORS,
};
pub use planner::{run_ff, run_planner};
pub use ppddl::{
    parse_ppddl, simulate_ppddl, solve_ppddl, validate_ppddl_policy, InitialStateProbability,
    PolicyDecision, PolicyOutcome, PolicyValidation, PpddlError, PpddlParseReport,
    ProbabilisticObjective, ProbabilisticOptions, ProbabilisticSolution, ProbabilisticState,
    ProbabilisticStatistics, SimulationReport,
};
pub use session::Session;
pub use trace::{trace, StateSnapshot};
pub use types::ParseError;
