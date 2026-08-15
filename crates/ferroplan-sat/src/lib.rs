//! ferroplan-sat: the in-tree CDCL SAT solver for ferroplan's SAT
//! compilation wing.
//!
//! Absorbed from [varisat] 0.2.2 by Jannis Harder (MIT OR Apache-2.0,
//! the same dual license as ferroplan) and carried forward as ferroplan
//! code — attribution lives in the file headers and in `ATTRIBUTION.md`.
//! The referee for every change is the DIMACS differential battery in
//! `tests/satdiff.rs`: verdicts must reproduce, and every SAT model is
//! verified against its CNF by direct clause evaluation — the solver is
//! never trusted to referee itself.
//!
//! [varisat]: https://github.com/jix/varisat

pub mod cnf;
pub mod dimacs;
pub mod lit;
pub mod solver;

pub use cnf::{CnfFormula, ExtendFormula};
pub use lit::{Lit, Var};
pub use solver::{Solver, SolverError};
