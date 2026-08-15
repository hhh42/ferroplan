// Absorbed from varisat 0.2.2 (https://github.com/jix/varisat, master
// @ 33e87693, file varisat/src/load.rs); the proof steps went with the
// proof seam, the tmp buffers are `mem::take`n for the duration (the
// partial_ref split made explicit), and `vec_mut_scan` became a
// write-index partition.
// Copyright (c) 2017-2019 Jannis Harder, MIT OR Apache-2.0 — ferroplan's
// exact dual license. Ferroplan code from the absorption commit onward;
// see ATTRIBUTION.md.

//! Loading a formula into the solver.
use std::mem::take;

use crate::{
    clause::{db, ClauseHeader, Tier},
    context::Context,
    lit::Lit,
    prop::{enqueue_assignment, full_restart, Reason},
    state::SatState,
    unit_simplify::resurrect_unit,
    variables,
};

/// Adds a clause to the current formula.
///
/// The input uses user variable names.
///
/// Removes duplicated literals, ignores tautological clauses (eg. x v -x v y), handles empty
/// clauses and dispatches among unit, binary and long clauses.
pub fn load_clause(ctx: &mut Context, user_lits: &[Lit]) {
    match ctx.solver_state.sat_state {
        SatState::Unsat => return,
        SatState::Sat => {
            ctx.solver_state.sat_state = SatState::Unknown;
        }
        _ => {}
    }

    // Restart the search when the user adds new clauses.
    full_restart(ctx);

    // Convert the clause from user to solver literals. The tmp buffers are taken out of the
    // context for the duration; nothing below touches `ctx.tmp_data`.
    let mut lits = take(&mut ctx.tmp_data.lits);
    let mut false_lits = take(&mut ctx.tmp_data.lits_2);

    variables::solver_from_user_lits(ctx, &mut lits, user_lits);

    lits.sort_unstable();
    lits.dedup();

    // Detect tautological clauses
    let mut tautology = false;
    let mut last = None;

    for &lit in lits.iter() {
        if last == Some(!lit) {
            tautology = true;
            break;
        }
        last = Some(lit);
    }

    if !tautology {
        // If we're not a unit clause the contained variables are not isolated anymore.
        if lits.len() > 1 {
            for &lit in lits.iter() {
                ctx.variables.var_data_solver_mut(lit.var()).isolated = false;
            }
        }

        // Remove satisfied clauses and handle false literals. We move unassigned literals to the
        // beginning (preserving order) to make sure we're going to watch unassigned literals.
        false_lits.clear();

        let mut clause_is_true = false;
        let mut write = 0;

        for read in 0..lits.len() {
            let lit = lits[read];
            match ctx.assignment.lit_value(lit) {
                Some(true) => {
                    clause_is_true = true;
                    break;
                }
                Some(false) => false_lits.push(lit),
                None => {
                    lits[write] = lit;
                    write += 1;
                }
            }
        }

        if !clause_is_true {
            lits.truncate(write);

            let will_conflict = lits.is_empty();

            // We resurrect any removed false literals to ensure propagation by this new clause.
            // This is also required to eventually simplify this clause.
            for &lit in false_lits.iter() {
                resurrect_unit(ctx, !lit);
            }

            lits.extend_from_slice(&false_lits);

            match lits[..] {
                [] => ctx.solver_state.sat_state = SatState::Unsat,
                [lit] => {
                    if will_conflict {
                        ctx.solver_state.sat_state = SatState::Unsat
                    } else {
                        let Context {
                            assignment,
                            impl_graph,
                            trail,
                            ..
                        } = ctx;
                        enqueue_assignment(assignment, impl_graph, trail, lit, Reason::Unit)
                    }
                }
                [lit_0, lit_1] => {
                    ctx.binary_clauses.add_binary_clause([lit_0, lit_1]);
                }
                _ => {
                    let mut header = ClauseHeader::new();
                    header.set_tier(Tier::Irred);

                    db::add_clause(ctx, header, &lits);
                }
            }
        }
    }

    ctx.tmp_data.lits = lits;
    ctx.tmp_data.lits_2 = false_lits;
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::clause::Tier;

    fn lits(lits: &[isize]) -> Vec<Lit> {
        lits.iter().map(|&l| Lit::from_dimacs(l)).collect()
    }

    #[test]
    fn unsat_on_empty_clause() {
        let mut ctx = Context::default();

        load_clause(&mut ctx, &[]);

        assert_eq!(ctx.solver_state.sat_state, SatState::Unsat);
    }

    #[test]
    fn unit_clauses() {
        let mut ctx = Context::default();

        load_clause(&mut ctx, &lits(&[1]));

        assert_eq!(ctx.trail.trail().len(), 1);

        load_clause(&mut ctx, &lits(&[3, -3]));

        assert_eq!(ctx.trail.trail().len(), 1);

        load_clause(&mut ctx, &lits(&[-2]));

        assert_eq!(ctx.trail.trail().len(), 2);

        load_clause(&mut ctx, &lits(&[1, 1]));

        assert_eq!(ctx.trail.trail().len(), 2);

        assert_eq!(ctx.solver_state.sat_state, SatState::Unknown);

        load_clause(&mut ctx, &lits(&[2]));

        assert_eq!(ctx.trail.trail().len(), 2);

        assert_eq!(ctx.solver_state.sat_state, SatState::Unsat);
    }

    #[test]
    fn binary_clauses() {
        let mut ctx = Context::default();

        load_clause(&mut ctx, &lits(&[1, 2]));

        assert_eq!(ctx.binary_clauses.count(), 1);

        load_clause(&mut ctx, &lits(&[-1, 3, 3]));

        assert_eq!(ctx.binary_clauses.count(), 2);

        load_clause(&mut ctx, &lits(&[4, -4]));

        assert_eq!(ctx.binary_clauses.count(), 2);

        assert_eq!(ctx.solver_state.sat_state, SatState::Unknown);
    }

    #[test]
    fn long_clauses() {
        let mut ctx = Context::default();

        load_clause(&mut ctx, &lits(&[1, 2, 3]));

        assert_eq!(ctx.clause_db.count_by_tier(Tier::Irred), 1);

        load_clause(&mut ctx, &lits(&[-2, 3, 3, 4]));

        assert_eq!(ctx.clause_db.count_by_tier(Tier::Irred), 2);

        load_clause(&mut ctx, &lits(&[4, -5, 5, 2]));

        assert_eq!(ctx.clause_db.count_by_tier(Tier::Irred), 2);

        assert_eq!(ctx.solver_state.sat_state, SatState::Unknown);
    }
}
