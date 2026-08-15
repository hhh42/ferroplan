// Absorbed from varisat 0.2.2 (https://github.com/jix/varisat, master
// @ 33e87693, file varisat/src/context.rs); the `partial_ref` context
// machinery was replaced by a plain struct — functions take `&mut
// Context` and split borrows are explicit field destructures.
// Copyright (c) 2017-2019 Jannis Harder, MIT OR Apache-2.0 — ferroplan's
// exact dual license. Ferroplan code from the absorption commit onward;
// see ATTRIBUTION.md.

//! Central solver data structure.
//!
//! This module defines the `Context` data structure which holds all data used by the solver. It
//! also contains global notification functions that likely need to be extended when new parts are
//! added to the solver.

use crate::{
    analyze_conflict::AnalyzeConflict,
    assumptions::Assumptions,
    binary::BinaryClauses,
    clause::{ClauseActivity, ClauseAlloc, ClauseDb},
    config::SolverConfig,
    decision::vsids::Vsids,
    model::Model,
    prop::{Assignment, ImplGraph, Trail, Watchlists},
    schedule::Schedule,
    state::SolverState,
    tmp::{TmpData, TmpFlags},
    variables::Variables,
};

/// Central solver data structure.
///
/// This struct contains all data kept by the solver. Functions operating on multiple fields take
/// `&mut Context` and destructure it into disjoint field borrows where needed; the handful of
/// places that must hold a part across a whole-context call temporarily `mem::take` the part out
/// (documented at each site).
#[derive(Default)]
pub struct Context {
    pub analyze_conflict: AnalyzeConflict,
    pub assignment: Assignment,
    pub assumptions: Assumptions,
    pub binary_clauses: BinaryClauses,
    pub clause_activity: ClauseActivity,
    pub clause_alloc: ClauseAlloc,
    pub clause_db: ClauseDb,
    pub impl_graph: ImplGraph,
    pub model: Model,
    pub schedule: Schedule,
    pub solver_config: SolverConfig,
    pub solver_state: SolverState,
    pub tmp_data: TmpData,
    pub tmp_flags: TmpFlags,
    pub trail: Trail,
    pub variables: Variables,
    pub vsids: Vsids,
    pub watchlists: Watchlists,
}

/// Update structures for a new variable count.
pub fn set_var_count(ctx: &mut Context, count: usize) {
    ctx.analyze_conflict.set_var_count(count);
    ctx.assignment.set_var_count(count);
    ctx.binary_clauses.set_var_count(count);
    ctx.impl_graph.set_var_count(count);
    ctx.tmp_flags.set_var_count(count);
    ctx.vsids.set_var_count(count);
    ctx.watchlists.set_var_count(count);
}
