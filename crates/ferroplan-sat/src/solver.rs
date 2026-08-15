//! Boolean satisfiability solver facade.
//!
//! FIXTURES-FIRST STUB: this commit carries the satdiff battery and the
//! public API surface only. `solve` answers UNSAT for everything, `model`
//! and `failed_core` answer nothing — the battery must be RED against
//! this stub and turn GREEN only when the absorbed varisat core lands.

use std::fmt;

use crate::{CnfFormula, ExtendFormula, Lit, Var};

/// Possible errors while solving a formula.
#[derive(Debug)]
#[non_exhaustive]
pub enum SolverError {
    /// The solver was interrupted (conflict budget exhausted).
    Interrupted,
}

impl fmt::Display for SolverError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            SolverError::Interrupted => write!(f, "the solver was interrupted"),
        }
    }
}

impl std::error::Error for SolverError {}

impl SolverError {
    /// Whether a Solver instance can be used after producing such an error.
    pub fn is_recoverable(&self) -> bool {
        matches!(self, SolverError::Interrupted)
    }
}

/// A boolean satisfiability solver.
#[derive(Default)]
pub struct Solver {
    var_count: usize,
}

impl Solver {
    /// Create a new solver.
    pub fn new() -> Solver {
        Solver::default()
    }

    /// Add a formula to the solver.
    pub fn add_formula(&mut self, formula: &CnfFormula) {
        for clause in formula.iter() {
            self.add_clause(clause);
        }
    }

    /// Limit the number of conflicts per `solve` call; `None` removes the limit.
    pub fn set_conflict_limit(&mut self, _limit: Option<u64>) {}

    /// Check the satisfiability of the current formula.
    pub fn solve(&mut self) -> Result<bool, SolverError> {
        Ok(false) // stub: the battery must catch this lie
    }

    /// Assume given literals for future calls to solve.
    ///
    /// This replaces the current set of assumed literals.
    pub fn assume(&mut self, _assumptions: &[Lit]) {}

    /// Set of literals that satisfy the formula.
    pub fn model(&self) -> Option<Vec<Lit>> {
        None
    }

    /// Subset of the assumptions that made the formula unsatisfiable.
    ///
    /// This is not guaranteed to be minimal and may just return all assumptions every time.
    pub fn failed_core(&self) -> Option<&[Lit]> {
        None
    }
}

impl ExtendFormula for Solver {
    fn add_clause(&mut self, clause: &[Lit]) {
        for lit in clause {
            self.var_count = self.var_count.max(lit.index() + 1);
        }
    }

    fn new_var(&mut self) -> Var {
        let var = Var::from_index(self.var_count);
        self.var_count += 1;
        var
    }
}
