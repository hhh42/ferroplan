// Absorbed from varisat 0.2.2 (https://github.com/jix/varisat, master
// @ 33e87693, file varisat/src/unit_simplify.rs); the proof steps and
// clause-hash plumbing went with the proof seam.
// Copyright (c) 2017-2019 Jannis Harder, MIT OR Apache-2.0 — ferroplan's
// exact dual license. Ferroplan code from the absorption commit onward;
// see ATTRIBUTION.md.

//! Simplification using unit clauses.

use crate::{
    binary::simplify_binary,
    clause::db::filter_clauses,
    context::Context,
    lit::{Lit, Var},
    prop::{enqueue_assignment, Reason},
    variables,
};

/// Move unit clauses off the trail, remembering them in the implication graph.
pub fn prove_units(ctx: &mut Context) -> bool {
    let mut new_unit = false;

    if ctx.trail.current_level() == 0 {
        let Context {
            impl_graph, trail, ..
        } = ctx;

        for &lit in trail.trail() {
            new_unit = true;
            impl_graph.update_removed_unit(lit.var());
        }

        trail.clear();
    }

    new_unit
}

/// Put a removed unit back onto the trail.
pub fn resurrect_unit(ctx: &mut Context, lit: Lit) {
    if ctx.impl_graph.is_removed_unit(lit.var()) {
        debug_assert!(ctx.assignment.lit_is_true(lit));
        ctx.assignment.unassign_var(lit.var());

        // Because we always enqueue with Reason::Unit this will not cause a unit clause to be
        // proven in `prove_units`.
        let Context {
            assignment,
            impl_graph,
            trail,
            ..
        } = ctx;
        enqueue_assignment(assignment, impl_graph, trail, lit, Reason::Unit);
    }
}

/// Remove satisfied clauses and false literals.
pub fn unit_simplify(ctx: &mut Context) {
    simplify_binary(ctx);

    let mut new_lits: Vec<Lit> = vec![];
    {
        let Context {
            assignment,
            binary_clauses,
            clause_alloc,
            clause_db,
            watchlists,
            ..
        } = ctx;

        filter_clauses(clause_alloc, clause_db, watchlists, |alloc, cref| {
            let clause = alloc.clause_mut(cref);
            new_lits.clear();
            for &lit in clause.lits() {
                match assignment.lit_value(lit) {
                    None => new_lits.push(lit),
                    Some(true) => return false,
                    Some(false) => (),
                }
            }
            if new_lits.len() < clause.lits().len() {
                match new_lits[..] {
                    // Cannot have empty or unit clauses after full propagation. An empty clause
                    // would have been a conflict and a unit clause must be satisfied and thus would
                    // have been dropped above.
                    [] | [_] => unreachable!(),
                    [lit_0, lit_1] => {
                        binary_clauses.add_binary_clause([lit_0, lit_1]);
                        false
                    }
                    ref lits => {
                        clause.lits_mut()[..lits.len()].copy_from_slice(lits);
                        clause.header_mut().set_len(lits.len());
                        true
                    }
                }
            } else {
                true
            }
        });
    }

    for var_index in 0..ctx.assignment.assignment().len() {
        let var = Var::from_index(var_index);
        if !ctx.variables.solver_var_present(var) {
            continue;
        }
        let value = ctx.assignment.assignment()[var_index];
        let var_data = ctx.variables.var_data_solver_mut(var);
        if let Some(value) = value {
            var_data.unit = Some(value);
            var_data.isolated = true;
        }
        if var_data.isolated && !var_data.assumed {
            variables::remove_solver_var(ctx, var);
        }
    }
}
