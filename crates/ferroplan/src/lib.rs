//! # ferroplan
//!
//! A fast, data-parallel [PDDL](https://en.wikipedia.org/wiki/Planning_Domain_Definition_Language)
//! planner in Rust — a deterministic planning core for the age of AI. The engine is a
//! delete-relaxation FF heuristic over a data-oriented (bitset / structure-of-arrays)
//! task representation, with enforced hill-climbing + best-first fallback and parallel
//! grounding / heuristic evaluation.
//!
//! PDDL coverage: STRIPS, typing, ADL (conditional/`forall` effects, equality),
//! numeric fluents, derived axioms, **PDDL3** soft-goal preferences/metric, and
//! **PDDL2.1 temporal** durative actions (constant / parameter-dependent durations,
//! duration inequalities, timed initial literals). Plus an SGPlan-style
//! **partition-and-resolve** mode.
//!
//! ## The public API (all `serde`-serializable)
//!
//! - [`solve`] — plan a domain + problem; returns a [`Solution`] (mode auto-detected).
//! - [`decompose`] — split a temporal goal too big for one-shot search into ordered,
//!   individually-solved [`Contract`]s, stitched into one validated plan
//!   ([`Decomposition`]).
//! - [`parse`] — syntax-check PDDL and summarize its structure ([`ParseReport`])
//!   *without* grounding or solving — fast feedback for an authoring loop.
//! - [`Session`] — ground once, **replan many**: hold a mutable world state
//!   (`set_fact`/`set_fluent`) and re-solve per tick paying only the search
//!   (~10x per-tick on small contracts) — the embedding API for games/simulations.
//! - [`plan::validate_plan`] — independently check a plan under ferroplan's semantics.
//!
//! ## Quick start
//! ```no_run
//! let domain = std::fs::read_to_string("domain.pddl").unwrap();
//! let problem = std::fs::read_to_string("problem.pddl").unwrap();
//!
//! // Catch syntax mistakes before solving.
//! let report = ferroplan::parse(&domain);
//! assert!(report.ok, "{:?}", report.error);
//!
//! let solution = ferroplan::solve(&domain, &problem, &ferroplan::Options::default()).unwrap();
//! if let Some(plan) = solution.plan {
//!     for step in &plan.steps { println!("{}", step.action); }
//! }
//! ```
//!
//! The lower-level text-rendering entry points (`run_planner`, `run_ff`) produce
//! classic Metric-FF / IPC output and back the `ff` binary.

// engine (data-oriented core)
pub mod bitset;
pub mod clock;
pub mod derived;
pub mod features;
pub mod ground;
pub mod hash;
pub mod heuristic;
pub mod invariants;
pub mod lama;
pub mod landmarks;
pub mod lexer;
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
pub mod report;
pub mod resolve;
pub mod selection;
pub mod temporal;
pub mod trace;
pub mod tresolve;
pub mod tsched;
pub mod verify;
pub mod viz;

// orchestration + smart public API
pub mod api;
pub mod planner;
pub mod session;

pub use api::{
    decompose, parse, solve, Contract, Decomposition, DomainSummary, Metric, Mode, Options,
    ParseReport, Plan, ProblemSummary, Search, Solution, SolveError, Statistics, Step,
};
pub use planner::{run_ff, run_planner};
pub use session::Session;
pub use trace::{trace, StateSnapshot};
pub use types::ParseError;
