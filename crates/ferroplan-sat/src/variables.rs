// Absorbed from varisat 0.2.2 (https://github.com/jix/varisat, master
// @ 33e87693, file varisat/src/variables.rs); `rustc-hash` was replaced
// by std's `HashSet`, the proof steps went with the proof seam.
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

use data::{SamplingMode, VarData};
use var_map::{VarBiMap, VarBiMapMut, VarMap};

/// Variable mapping and metadata.
#[derive(Default)]
pub struct Variables {
    /// Bidirectional mapping from user variables to global variables.
    ///
    /// Initially this is the identity mapping. This ensures that in the non-assumptions setting the
    /// map from used user variables to global variables is the identity.
    global_from_user: VarBiMap,
    /// Bidirectional mapping from global variables to solver variables.
    ///
    /// This starts with the empty mapping, so only used variables are allocated.
    solver_from_global: VarBiMap,
    /// User variables that were explicitly hidden by the user.
    user_freelist: HashSet<Var>,
    /// Global variables that can be recycled without increasing the global_watermark.
    global_freelist: HashSet<Var>,
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

    /// Mutable global to user mapping.
    pub fn user_from_global_mut(&mut self) -> VarBiMapMut<'_> {
        self.global_from_user.bwd_mut()
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

    /// Get an unmapped global variable.
    pub fn next_unmapped_global(&self) -> Var {
        self.global_freelist
            .iter()
            .next()
            .cloned()
            .unwrap_or_else(|| Var::from_index(self.global_watermark()))
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
        self.user_freelist
            .iter()
            .next()
            .cloned()
            .unwrap_or_else(|| Var::from_index(self.user_watermark()))
    }
}

/// Maps a user variable into a global variable.
///
/// If no matching global variable exists a new global variable is allocated.
pub fn global_from_user(ctx: &mut Context, user: Var, require_sampling: bool) -> Var {
    let variables = &mut ctx.variables;

    if user.index() > variables.user_watermark() {
        for index in variables.user_watermark()..user.index() {
            global_from_user(ctx, Var::from_index(index), false);
        }
    }

    let variables = &mut ctx.variables;

    match variables.global_from_user().get(user) {
        Some(global) => {
            if require_sampling
                && variables.var_data[global.index()].sampling_mode != SamplingMode::Sample
            {
                panic!("witness variables cannot be constrained");
            }
            global
        }
        None => {
            // Can we add an identity mapping?
            let global = match variables.var_data.get(user.index()) {
                Some(var_data) if var_data.deleted => user,
                None => user,
                _ => variables.next_unmapped_global(),
            };

            *variables.var_data_global_mut(global) = VarData::user_default();

            variables.global_from_user_mut().insert(global, user);
            variables.global_freelist.remove(&global);
            variables.user_freelist.remove(&user);

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
pub fn solver_from_user(ctx: &mut Context, user: Var, require_sampling: bool) -> Var {
    let global = global_from_user(ctx, user, require_sampling);
    solver_from_global(ctx, global)
}

/// Allocates a currently unused user variable.
///
/// This is either a user variable above any user variable used so far, or a user variable that was
/// previously hidden by the user.
pub fn new_user_var(ctx: &mut Context) -> Var {
    let user_var = ctx.variables.next_unmapped_user();
    global_from_user(ctx, user_var, false);
    user_var
}

/// Maps a slice of user lits to solver lits using [`solver_from_user`].
pub fn solver_from_user_lits(
    ctx: &mut Context,
    solver_lits: &mut Vec<Lit>,
    user_lits: &[Lit],
    require_sampling: bool,
) {
    solver_lits.clear();
    for user_lit in user_lits {
        let solver_var = solver_from_user(ctx, user_lit.var(), require_sampling);
        solver_lits.push(user_lit.map_var(|_| solver_var));
    }
}

/// Changes the sampling mode of a global variable.
///
/// If the mode is changed to hidden, an existing user mapping is automatically removed.
///
/// If the mode is changed from hidden, a new user mapping is allocated and the user variable is
/// returned.
pub fn set_sampling_mode(ctx: &mut Context, global: Var, mode: SamplingMode) -> Option<Var> {
    let variables = &mut ctx.variables;

    let var_data = &mut variables.var_data[global.index()];

    assert!(!var_data.deleted);

    if var_data.assumed {
        panic!("cannot change sampling mode of assumption variable")
    }

    let previous_mode = var_data.sampling_mode;

    if previous_mode == mode {
        return None;
    }

    var_data.sampling_mode = mode;

    let mut result = None;

    if previous_mode == SamplingMode::Hide {
        let user = variables.next_unmapped_user();
        variables.user_from_global_mut().insert(user, global);
        variables.user_freelist.remove(&user);

        result = Some(user);
    } else if mode == SamplingMode::Hide {
        if let Some(user) = variables.user_from_global_mut().remove(global) {
            variables.user_freelist.insert(user);
        }

        delete_global_if_unused(ctx, global);
    }

    result
}

/// Turns all hidden vars into witness vars and returns them.
pub fn observe_internal_vars(ctx: &mut Context) -> Vec<Var> {
    let mut result = vec![];
    for global_index in 0..ctx.variables.global_watermark() {
        let global = Var::from_index(global_index);
        let var_data = &ctx.variables.var_data[global.index()];
        if !var_data.deleted && var_data.sampling_mode == SamplingMode::Hide {
            let user = set_sampling_mode(ctx, global, SamplingMode::Witness).unwrap();
            result.push(user);
        }
    }
    result
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
/// If the variable is isolated and hidden, the global variable is also removed.
pub fn remove_solver_var(ctx: &mut Context, solver: Var) {
    ctx.vsids.make_unavailable(solver);

    let variables = &mut ctx.variables;

    let global = variables
        .global_from_solver_mut()
        .remove(solver)
        .expect("no existing global var for solver var");

    variables.solver_freelist.insert(solver);

    delete_global_if_unused(ctx, global);
}

/// Delete a global variable if it is unused
fn delete_global_if_unused(ctx: &mut Context, global: Var) {
    let variables = &mut ctx.variables;

    if variables.user_from_global().get(global).is_some() {
        return;
    }

    if variables.solver_from_global().get(global).is_some() {
        return;
    }

    let data = &mut variables.var_data[global.index()];

    assert!(data.sampling_mode == SamplingMode::Hide);

    if !data.isolated {
        return;
    }

    data.deleted = true;

    variables.global_freelist.insert(global);
}
