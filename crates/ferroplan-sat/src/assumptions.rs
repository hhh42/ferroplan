// Absorbed from varisat 0.2.2 (https://github.com/jix/varisat, master
// @ 33e87693, file varisat/src/assumptions.rs); the proof steps and
// clause-hash plumbing went with the proof seam. KEPT after the strip
// audit: ~200 lines, zero dependencies, no proof residue — and the
// temporal face's CEGAR loop may want failed cores; the wing's
// re-encode-per-horizon design keeps it optional either way.
// Copyright (c) 2017-2019 Jannis Harder, MIT OR Apache-2.0 — ferroplan's
// exact dual license. Ferroplan code from the absorption commit onward;
// see ATTRIBUTION.md.

//! Incremental solving.

use std::mem::take;

use crate::{
    context::Context,
    lit::Lit,
    prop::{enqueue_assignment, full_restart, Reason},
    state::SatState,
    variables,
};

/// Incremental solving.
#[derive(Default)]
pub struct Assumptions {
    assumptions: Vec<Lit>,
    failed_core: Vec<Lit>,
    user_failed_core: Vec<Lit>,
    assumption_levels: usize,
}

impl Assumptions {
    /// Current number of decision levels used for assumptions.
    pub fn assumption_levels(&self) -> usize {
        self.assumption_levels
    }

    /// Resets assumption_levels to zero on a full restart.
    pub fn full_restart(&mut self) {
        self.assumption_levels = 0;
    }

    /// Subset of assumptions that made the formula unsatisfiable.
    pub fn user_failed_core(&self) -> &[Lit] {
        &self.user_failed_core
    }
}

/// Return type of [`enqueue_assumption`].
pub enum EnqueueAssumption {
    Done,
    Enqueued,
    Conflict,
}

/// Change the currently active assumptions.
///
/// The input uses user variable names.
pub fn set_assumptions(ctx: &mut Context, user_assumptions: &[Lit]) {
    full_restart(ctx);

    let state = &mut ctx.solver_state;

    state.sat_state = match state.sat_state {
        SatState::Unsat => SatState::Unsat,
        SatState::Sat | SatState::UnsatUnderAssumptions | SatState::Unknown => SatState::Unknown,
    };

    // The assumption buffer is taken out of the context for the duration; nothing below touches
    // `ctx.assumptions.assumptions`.
    let mut assumptions = take(&mut ctx.assumptions.assumptions);

    for lit in assumptions.iter() {
        ctx.variables.var_data_solver_mut(lit.var()).assumed = false;
    }

    variables::solver_from_user_lits(ctx, &mut assumptions, user_assumptions, true);

    for lit in assumptions.iter() {
        ctx.variables.var_data_solver_mut(lit.var()).assumed = true;
    }

    ctx.assumptions.assumptions = assumptions;
}

/// Enqueue another assumption if possible.
///
/// Returns whether an assumption was enqueued, whether no assumptions are left or whether the
/// assumptions result in a conflict.
pub fn enqueue_assumption(ctx: &mut Context) -> EnqueueAssumption {
    while let Some(&assumption) = ctx.assumptions.assumptions.get(ctx.trail.current_level()) {
        match ctx.assignment.lit_value(assumption) {
            Some(false) => {
                analyze_assumption_conflict(ctx, assumption);
                return EnqueueAssumption::Conflict;
            }
            Some(true) => {
                // The next assumption is already implied by other assumptions so we can remove it.
                let level = ctx.trail.current_level();
                ctx.assumptions.assumptions.swap_remove(level);
            }
            None => {
                ctx.trail.new_decision_level();
                {
                    let Context {
                        assignment,
                        impl_graph,
                        trail,
                        ..
                    } = ctx;
                    enqueue_assignment(assignment, impl_graph, trail, assumption, Reason::Unit);
                }
                ctx.assumptions.assumption_levels = ctx.trail.current_level();
                return EnqueueAssumption::Enqueued;
            }
        }
    }
    EnqueueAssumption::Done
}

/// Analyze a conflicting set of assumptions.
///
/// Compute a set of incompatible assumptions given an assumption that is incompatible with the
/// assumptions enqueued so far.
fn analyze_assumption_conflict(ctx: &mut Context, assumption: Lit) {
    let Context {
        assumptions,
        tmp_flags,
        trail,
        impl_graph,
        clause_alloc,
        variables,
        ..
    } = ctx;

    let flags = &mut tmp_flags.flags;

    assumptions.failed_core.clear();
    assumptions.failed_core.push(assumption);

    flags[assumption.index()] = true;
    let mut flag_count = 1;

    for &lit in trail.trail().iter().rev() {
        if flags[lit.index()] {
            flags[lit.index()] = false;
            flag_count -= 1;

            match impl_graph.reason(lit.var()) {
                Reason::Unit => {
                    if impl_graph.level(lit.var()) > 0 {
                        assumptions.failed_core.push(lit);
                    }
                }
                reason => {
                    for &reason_lit in reason.lits(clause_alloc) {
                        if !flags[reason_lit.index()] {
                            flags[reason_lit.index()] = true;
                            flag_count += 1;
                        }
                    }
                }
            }

            if flag_count == 0 {
                break;
            }
        }
    }

    assumptions.user_failed_core.clear();
    for solver_lit in assumptions.failed_core.iter() {
        assumptions
            .user_failed_core
            .push(solver_lit.map_var(|solver_var| variables.existing_user_from_solver(solver_var)));
    }
}
