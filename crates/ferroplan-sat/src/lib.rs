//! ferroplan-sat: the in-tree CDCL SAT solver for ferroplan's SAT
//! compilation wing.
//!
//! Absorbed from [varisat] 0.2.2 by Jannis Harder (MIT OR Apache-2.0,
//! the same dual license as ferroplan) and carried forward as ferroplan
//! code — attribution lives in the file headers and in `ATTRIBUTION.md`.
//! The core is classic CDCL: two-watched-literal propagation with
//! blocking literals, 1UIP conflict analysis with recursive
//! minimization, VSIDS with phase saving, Luby restarts, tiered
//! clause-database reduction, and an incremental assumptions interface.
//!
//! The referee for every change is the DIMACS differential battery in
//! `tests/satdiff.rs`: verdicts must reproduce, and every SAT model is
//! verified against its CNF by direct clause evaluation — the solver is
//! never trusted to referee itself.
//!
//! The first ferroplan-side improvement — planning-specific branching,
//! ordering decisions by the encoding's layer structure — is
//! deliberately NOT taken this cycle: the wing proves out on stock
//! heuristics first, so the solver's contribution stays separable from
//! the encoding's.
//!
//! [varisat]: https://github.com/jix/varisat

pub mod cnf;
pub mod config;
pub mod dimacs;
pub mod lit;
pub mod solver;

mod analyze_conflict;
mod assumptions;
mod binary;
mod cdcl;
mod clause;
mod context;
mod decision;
mod glue;
mod load;
mod model;
mod prop;
mod schedule;
mod state;
mod tmp;
mod unit_simplify;
mod variables;

pub use cnf::{CnfFormula, ExtendFormula};
pub use config::SolverConfig;
pub use lit::{Lit, Var};
pub use solver::{Solver, SolverError};
