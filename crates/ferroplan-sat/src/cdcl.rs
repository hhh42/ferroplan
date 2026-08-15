// Absorbed from varisat 0.2.2 (https://github.com/jix/varisat, master
// @ 33e87693, file varisat/src/cdcl.rs); the learned-clause proof step
// went with the proof seam, and the analyze-conflict split became an
// explicit `mem::take` (nothing below touches `ctx.analyze_conflict`
// while it is out).
// Copyright (c) 2017-2019 Jannis Harder, MIT OR Apache-2.0 — ferroplan's
// exact dual license. Ferroplan code from the absorption commit onward;
// see ATTRIBUTION.md.

//! Conflict driven clause learning.

use std::mem::take;

use crate::{
    analyze_conflict::analyze_conflict,
    assumptions::{enqueue_assumption, EnqueueAssumption},
    clause::{
        assess::{assess_learned_clause, bump_clause},
        db, decay_clause_activities,
    },
    context::Context,
    decision::make_decision,
    model::reconstruct_global_model,
    prop::{backtrack, enqueue_assignment, propagate, Conflict, Reason},
    state::SatState,
    unit_simplify::{prove_units, unit_simplify},
};

/// Find a conflict, learn a clause and backtrack.
pub fn conflict_step(ctx: &mut Context) {
    let conflict = match find_conflict(ctx) {
        Ok(()) => {
            reconstruct_global_model(ctx);
            return;
        }
        Err(FoundConflict::Assumption) => {
            ctx.solver_state.sat_state = SatState::UnsatUnderAssumptions;
            return;
        }
        Err(FoundConflict::Conflict(conflict)) => conflict,
    };

    let backtrack_to = analyze_conflict(ctx, conflict);

    let analyze = take(&mut ctx.analyze_conflict);

    for &cref in analyze.involved() {
        bump_clause(ctx, cref);
    }

    decay_clause_activities(ctx);

    backtrack(ctx, backtrack_to);

    let clause = analyze.clause();

    let reason = match clause.len() {
        0 => {
            ctx.solver_state.sat_state = SatState::Unsat;
            ctx.analyze_conflict = analyze;
            return;
        }
        1 => Reason::Unit,
        2 => {
            ctx.binary_clauses.add_binary_clause([clause[0], clause[1]]);
            Reason::Binary([clause[1]])
        }
        _ => {
            let header = assess_learned_clause(ctx, clause);
            let cref = db::add_clause(ctx, header, clause);
            Reason::Long(cref)
        }
    };

    {
        let Context {
            assignment,
            impl_graph,
            trail,
            ..
        } = ctx;
        enqueue_assignment(assignment, impl_graph, trail, clause[0], reason);
    }

    ctx.analyze_conflict = analyze;
}

/// Return type of [`find_conflict`].
///
/// Specifies whether a conflict was found during propagation or while enqueuing assumptions.
enum FoundConflict {
    Conflict(Conflict),
    Assumption,
}

impl From<Conflict> for FoundConflict {
    fn from(conflict: Conflict) -> FoundConflict {
        FoundConflict::Conflict(conflict)
    }
}

/// Find a conflict.
///
/// Returns `Err` if a conflict was found and `Ok` if a satisfying assignment was found instead.
fn find_conflict(ctx: &mut Context) -> Result<(), FoundConflict> {
    loop {
        let propagation_result = propagate(ctx);

        let new_unit = prove_units(ctx);

        propagation_result?;

        if new_unit {
            unit_simplify(ctx);
        }

        match enqueue_assumption(ctx) {
            EnqueueAssumption::Enqueued => continue,
            EnqueueAssumption::Conflict => return Err(FoundConflict::Assumption),
            EnqueueAssumption::Done => (),
        }

        if !make_decision(ctx) {
            return Ok(());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::{lit::Lit, load::load_clause};

    fn lits(lits: &[isize]) -> Vec<Lit> {
        lits.iter().map(|&l| Lit::from_dimacs(l)).collect()
    }

    #[test]
    fn level_0_unsat() {
        let mut ctx = Context::default();

        let formula: Vec<Vec<Lit>> = vec![
            lits(&[1, 2, 3]),
            lits(&[-1]),
            lits(&[1, -2]),
            lits(&[2, -3]),
        ];

        for clause in formula.iter() {
            load_clause(&mut ctx, clause);
        }

        while ctx.solver_state.sat_state == SatState::Unknown {
            conflict_step(&mut ctx);
        }

        assert_eq!(ctx.solver_state.sat_state, SatState::Unsat);
    }
}
