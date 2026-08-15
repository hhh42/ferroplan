// Absorbed from varisat 0.2.2 (https://github.com/jix/varisat, master
// @ 33e87693, file varisat/src/variables.rs); `rustc-hash` was replaced
// by std's `HashSet`, the proof steps went with the proof seam, and the
// sampling-mode surface (set_sampling_mode, observe_internal_vars, the
// hide/witness freelists, the require_sampling threading) was stripped —
// user variables are never hidden here, so the user↔global mapping stays
// the identity and globals are never deleted. The three-level
// user/global/solver mapping itself is KEPT: unit_simplify recycles
// solver variables through it (a real solver feature, not proof
// residue).
// Copyright (c) 2017-2019 Jannis Harder, MIT OR Apache-2.0 — ferroplan's
// exact dual license. Ferroplan code from the absorption commit onward;
// see ATTRIBUTION.md.

//! Variable mapping and metadata.

use std::collections::HashSet;

use crate::{
    context::{set_var_count, Context},
    lit::{Lit, Var},
};

pub mod data;
pub mod var_map;

use data::VarData;
use var_map::{VarBiMap, VarBiMapMut, VarMap};

/// Variable mapping and metadata.
#[derive(Default)]
pub struct Variables {
    /// Bidirectional mapping from user variables to global variables.
    ///
    /// Always the identity mapping (nothing removes user variables anymore); kept bidirectional so
    /// the shape survives for a future ferroplan-side use.
    global_from_user: VarBiMap,
    /// Bidirectional mapping from global variables to solver variables.
    ///
    /// This starts with the empty mapping, so only used variables are allocated.
    solver_from_global: VarBiMap,
    /// Solver variables that are unused and can be recycled.
    solver_freelist: HashSet<Var>,

    /// Variable metadata.
    ///
    /// Indexed by global variable indices.
    var_data: Vec<VarData>,
}

impl Variables {
    /// Number of allocated solver variables.
    pub fn solver_watermark(&self) -> usize {
        self.global_from_solver().watermark()
    }

    /// Number of allocated global variables.
    pub fn global_watermark(&self) -> usize {
        self.var_data.len()
    }

    /// Number of allocated user variables.
    pub fn user_watermark(&self) -> usize {
        self.global_from_user().watermark()
    }

    /// Iterator over all user variables that are in use.
    pub fn user_var_iter(&self) -> impl Iterator<Item = Var> + '_ {
        let global_from_user = self.global_from_user.fwd();
        (0..self.global_from_user().watermark())
            .map(Var::from_index)
            .filter(move |&user_var| global_from_user.get(user_var).is_some())
    }

    /// Iterator over all global variables that are in use.
    pub fn global_var_iter(&self) -> impl Iterator<Item = Var> + '_ {
        (0..self.global_watermark())
            .map(Var::from_index)
            .filter(move |&global_var| !self.var_data[global_var.index()].deleted)
    }

    /// The user to global mapping.
    pub fn global_from_user(&self) -> &VarMap {
        self.global_from_user.fwd()
    }

    /// Mutable user to global mapping.
    pub fn global_from_user_mut(&mut self) -> VarBiMapMut<'_> {
        self.global_from_user.fwd_mut()
    }

    /// The global to solver mapping.
    pub fn solver_from_global(&self) -> &VarMap {
        self.solver_from_global.fwd()
    }

    /// Mutable global to solver mapping.
    pub fn solver_from_global_mut(&mut self) -> VarBiMapMut<'_> {
        self.solver_from_global.fwd_mut()
    }

    /// The global to user mapping.
    pub fn user_from_global(&self) -> &VarMap {
        self.global_from_user.bwd()
    }

    /// The solver to global mapping.
    pub fn global_from_solver(&self) -> &VarMap {
        self.solver_from_global.bwd()
    }

    /// Mutable solver to global mapping.
    pub fn global_from_solver_mut(&mut self) -> VarBiMapMut<'_> {
        self.solver_from_global.bwd_mut()
    }

    /// Get an existing user var from a solver var.
    pub fn existing_user_from_solver(&self, solver: Var) -> Var {
        let global = self
            .global_from_solver()
            .get(solver)
            .expect("no existing global var for solver var");
        self.user_from_global()
            .get(global)
            .expect("no existing user var for global var")
    }

    /// Mutable reference to the var data for a global variable.
    pub fn var_data_global_mut(&mut self, global: Var) -> &mut VarData {
        if self.var_data.len() <= global.index() {
            self.var_data.resize(global.index() + 1, VarData::default());
        }
        &mut self.var_data[global.index()]
    }

    /// Mutable reference to the var data for a solver variable.
    pub fn var_data_solver_mut(&mut self, solver: Var) -> &mut VarData {
        let global = self
            .global_from_solver()
            .get(solver)
            .expect("no existing global var for solver var");
        &mut self.var_data[global.index()]
    }

    /// Var data for a global variable.
    pub fn var_data_global(&self, global: Var) -> &VarData {
        &self.var_data[global.index()]
    }

    /// Check if a solver var is mapped to a global var
    pub fn solver_var_present(&self, solver: Var) -> bool {
        self.global_from_solver().get(solver).is_some()
    }

    /// Get an unmapped solver variable.
    pub fn next_unmapped_solver(&self) -> Var {
        self.solver_freelist
            .iter()
            .next()
            .cloned()
            .unwrap_or_else(|| Var::from_index(self.solver_watermark()))
    }

    /// Get an unmapped user variable.
    pub fn next_unmapped_user(&self) -> Var {
        Var::from_index(self.user_watermark())
    }
}

/// Maps a user variable into a global variable.
///
/// If no matching global variable exists a new global variable is allocated.
pub fn global_from_user(ctx: &mut Context, user: Var) -> Var {
    let variables = &mut ctx.variables;

    if user.index() > variables.user_watermark() {
        for index in variables.user_watermark()..user.index() {
            global_from_user(ctx, Var::from_index(index));
        }
    }

    let variables = &mut ctx.variables;

    match variables.global_from_user().get(user) {
        Some(global) => global,
        None => {
            // Nothing ever removes user variables, so the mapping is the identity.
            let global = user;

            *variables.var_data_global_mut(global) = VarData::user_default();

            variables.global_from_user_mut().insert(global, user);

            global
        }
    }
}

/// Maps an existing global variable to a solver variable.
///
/// If no matching solver variable exists a new one is allocated.
pub fn solver_from_global(ctx: &mut Context, global: Var) -> Var {
    debug_assert!(!ctx.variables.var_data[global.index()].deleted);

    match ctx.variables.solver_from_global().get(global) {
        Some(solver) => solver,
        None => {
            let variables = &mut ctx.variables;

            let solver = variables.next_unmapped_solver();

            let old_watermark = variables.global_from_solver().watermark();

            variables.solver_from_global_mut().insert(solver, global);
            variables.solver_freelist.remove(&solver);

            let new_watermark = variables.global_from_solver().watermark();

            if new_watermark > old_watermark {
                set_var_count(ctx, new_watermark);
            }

            initialize_solver_var(ctx, solver, global);

            solver
        }
    }
}

/// Maps a user variable to a solver variable.
///
/// Allocates global and solver variables as required.
pub fn solver_from_user(ctx: &mut Context, user: Var) -> Var {
    let global = global_from_user(ctx, user);
    solver_from_global(ctx, global)
}

/// Allocates a currently unused user variable.
pub fn new_user_var(ctx: &mut Context) -> Var {
    let user_var = ctx.variables.next_unmapped_user();
    global_from_user(ctx, user_var);
    user_var
}

/// Maps a slice of user lits to solver lits using [`solver_from_user`].
pub fn solver_from_user_lits(ctx: &mut Context, solver_lits: &mut Vec<Lit>, user_lits: &[Lit]) {
    solver_lits.clear();
    for user_lit in user_lits {
        let solver_var = solver_from_user(ctx, user_lit.var());
        solver_lits.push(user_lit.map_var(|_| solver_var));
    }
}

/// Initialize a newly allocated solver variable
pub fn initialize_solver_var(ctx: &mut Context, solver: Var, global: Var) {
    let Context {
        variables,
        assignment,
        impl_graph,
        vsids,
        ..
    } = ctx;

    let data = &variables.var_data[global.index()];

    // This recovers the state of a variable that has a known value and was already propagated. This
    // is important so that when new clauses containing this variable are added, load_clause knows
    // to reenqueue the assignment.
    assignment.set_var(solver, data.unit);
    if data.unit.is_some() {
        impl_graph.update_removed_unit(solver);
    }
    vsids.reset(solver);
    if data.unit.is_none() {
        vsids.make_available(solver);
    }
}

/// Remove a solver var.
///
/// The global variable (and with it the user mapping and the remembered unit value) stays; only
/// the solver-side slot is recycled.
pub fn remove_solver_var(ctx: &mut Context, solver: Var) {
    ctx.vsids.make_unavailable(solver);

    let variables = &mut ctx.variables;

    variables
        .global_from_solver_mut()
        .remove(solver)
        .expect("no existing global var for solver var");

    variables.solver_freelist.insert(solver);
}
