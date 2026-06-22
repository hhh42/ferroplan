//! # ferroplan
//!
//! A fast, data-parallel [PDDL](https://en.wikipedia.org/wiki/Planning_Domain_Definition_Language)
//! planner in Rust. The core is a delete-relaxation FF heuristic over a
//! data-oriented (bitset / structure-of-arrays) task representation, with
//! parallel grounding and parallel batch heuristic evaluation. On top of the
//! engine it offers an SGPlan-style **partition-and-resolve** mode and a
//! **PDDL3** preferences/metric mode.
//!
//! ## Quick start
//! ```no_run
//! let domain = std::fs::read_to_string("domain.pddl").unwrap();
//! let problem = std::fs::read_to_string("problem.pddl").unwrap();
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
pub mod derived;
pub mod ground;
pub mod hash;
pub mod heuristic;
pub mod invariants;
pub mod lexer;
pub mod output;
pub mod packed;
pub mod par;
pub mod parser;
pub mod resource;
pub mod search;
pub mod types;

// modes (built on the engine)
pub mod espc;
pub mod partition;
pub mod pddl3;
pub mod report;
pub mod resolve;
pub mod temporal;
pub mod trace;
pub mod verify;
pub mod viz;

// orchestration + smart public API
pub mod api;
pub mod planner;

pub use api::{solve, Metric, Mode, Options, Plan, Search, Solution, SolveError, Statistics, Step};
pub use planner::{run_ff, run_planner};
pub use trace::{trace, StateSnapshot};
pub use types::ParseError;
