// Absorbed from varisat 0.2.2 (https://github.com/jix/varisat, master
// @ 33e87693, file varisat/src/solver.rs); the proof-writing/self-check
// surface (write_proof, close_proof, enable_self_checking,
// add_proof_processor, ProofFormat, the error plumbing they needed) went
// with the proof seam, `add_dimacs_cnf` with the unabsorbed dimacs crate
// — use `dimacs::parse_dimacs` + `add_formula`. A per-solve conflict
// budget was added for the wing's horizon ramp.
// Copyright (c) 2017-2019 Jannis Harder, MIT OR Apache-2.0 — ferroplan's
// exact dual license. Ferroplan code from the absorption commit onward;
// see ATTRIBUTION.md.

//! Boolean satisfiability solver.
use std::fmt;

use crate::{
    assumptions::set_assumptions,
    cnf::{CnfFormula, ExtendFormula},
    context::Context,
    lit::{Lit, Var},
    load::load_clause,
    schedule::schedule_step,
    state::SatState,
    variables,
};

/// Possible errors while solving a formula.
#[derive(Debug)]
#[non_exhaustive]
pub enum SolverError {
    /// The solver was interrupted: the conflict budget set via
    /// [`Solver::set_conflict_limit`] was exhausted before a verdict.
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
    ctx: Box<Context>,
}

impl Solver {
    /// Create a new solver.
    pub fn new() -> Solver {
        Solver::default()
    }

    /// Add a formula to the solver.
    pub fn add_formula(&mut self, formula: &CnfFormula) {
        for clause in formula.iter() {
            load_clause(&mut self.ctx, clause);
        }
    }

    /// Seed branching priorities: each `(variable index, activity)` raises
    /// that variable's initial VSIDS activity (`Vsids::seed`, the
    /// planning-specific branching hook — the reason this solver is
    /// in-tree). Indices beyond the current variable count are ignored;
    /// call after the formula is loaded.
    pub fn seed_activity(&mut self, seeds: &[(usize, f32)]) {
        // VSIDS runs on GLOBAL vars; seeds arrive as user indices —
        // translate exactly as `model` does, skipping unmapped vars.
        let Context {
            variables, vsids, ..
        } = &mut *self.ctx;
        let n = vsids.var_count();
        for &(idx, act) in seeds {
            let user = Var::from_index(idx);
            if let Some(global) = variables.global_from_user().get(user) {
                if global.index() < n {
                    vsids.seed(global, act);
                }
            }
        }
    }

    /// Limit the number of conflicts per `solve` call; `None` removes the limit.
    ///
    /// When the budget is exhausted before a verdict, [`Solver::solve`] returns
    /// `Err(SolverError::Interrupted)`. The interrupt is recoverable: raise or
    /// clear the limit and call `solve` again to continue where it stopped.
    pub fn set_conflict_limit(&mut self, limit: Option<u64>) {
        self.ctx.schedule.conflict_limit = limit;
    }

    /// Check the satisfiability of the current formula.
    pub fn solve(&mut self) -> Result<bool, SolverError> {
        self.ctx.schedule.conflicts_this_solve = 0;

        while schedule_step(&mut self.ctx) {}

        match self.ctx.solver_state.sat_state {
            SatState::Unknown => Err(SolverError::Interrupted),
            SatState::Sat => Ok(true),
            SatState::Unsat | SatState::UnsatUnderAssumptions => Ok(false),
        }
    }

    /// Assume given literals for future calls to solve.
    ///
    /// This replaces the current set of assumed literals.
    pub fn assume(&mut self, assumptions: &[Lit]) {
        set_assumptions(&mut self.ctx, assumptions);
    }

    /// Set of literals that satisfy the formula.
    pub fn model(&self) -> Option<Vec<Lit>> {
        if self.ctx.solver_state.sat_state == SatState::Sat {
            let variables = &self.ctx.variables;
            let model = &self.ctx.model;
            Some(
                variables
                    .user_var_iter()
                    .flat_map(|user_var| {
                        let global_var = variables
                            .global_from_user()
                            .get(user_var)
                            .expect("no existing global var for user var");
                        model.assignment()[global_var.index()].map(|value| user_var.lit(value))
                    })
                    .collect(),
            )
        } else {
            None
        }
    }

    /// Subset of the assumptions that made the formula unsatisfiable.
    ///
    /// This is not guaranteed to be minimal and may just return all assumptions every time.
    pub fn failed_core(&self) -> Option<&[Lit]> {
        match self.ctx.solver_state.sat_state {
            SatState::UnsatUnderAssumptions => Some(self.ctx.assumptions.user_failed_core()),
            SatState::Unsat => Some(&[]),
            SatState::Unknown | SatState::Sat => None,
        }
    }
}

impl ExtendFormula for Solver {
    /// Add a clause to the solver.
    fn add_clause(&mut self, clause: &[Lit]) {
        load_clause(&mut self.ctx, clause);
    }

    /// Add a new variable to the solver.
    fn new_var(&mut self) -> Var {
        variables::new_user_var(&mut self.ctx)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn lits(lits: &[isize]) -> Vec<Lit> {
        lits.iter().map(|&l| Lit::from_dimacs(l)).collect()
    }

    #[test]
    fn sat_then_unsat_incremental() {
        let mut solver = Solver::new();

        solver.add_clause(&lits(&[1, 2, 3]));
        solver.add_clause(&lits(&[-1, -2]));
        assert_eq!(solver.solve().ok(), Some(true));

        let model = solver.model().unwrap();
        assert!(
            model.contains(&Lit::from_dimacs(1))
                || model.contains(&Lit::from_dimacs(2))
                || model.contains(&Lit::from_dimacs(3))
        );

        solver.add_clause(&lits(&[-1]));
        solver.add_clause(&lits(&[-2]));
        solver.add_clause(&lits(&[-3]));
        assert_eq!(solver.solve().ok(), Some(false));
        assert_eq!(solver.model(), None);
        assert_eq!(solver.failed_core(), Some(&[][..]));
    }
}
