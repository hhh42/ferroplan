// Absorbed from varisat 0.2.2 (https://github.com/jix/varisat, master
// @ 33e87693, file varisat/src/model.rs); the model-in-proof plumbing
// went with the proof seam.
// Copyright (c) 2017-2019 Jannis Harder, MIT OR Apache-2.0 — ferroplan's
// exact dual license. Ferroplan code from the absorption commit onward;
// see ATTRIBUTION.md.

//! Global model reconstruction

use crate::{context::Context, state::SatState};

/// Global model reconstruction
#[derive(Default)]
pub struct Model {
    /// Assignment of the global model.
    ///
    /// Whenever the solver state is SAT this must be up to date.
    assignment: Vec<Option<bool>>,
}

impl Model {
    /// Assignment of the global model.
    ///
    /// Only valid if the solver state is SAT.
    pub fn assignment(&self) -> &[Option<bool>] {
        &self.assignment
    }
}

pub fn reconstruct_global_model(ctx: &mut Context) {
    {
        let Context {
            variables,
            model,
            assignment,
            ..
        } = ctx;

        model.assignment.clear();
        model.assignment.resize(variables.global_watermark(), None);

        for global_var in variables.global_var_iter() {
            let value = if let Some(solver_var) = variables.solver_from_global().get(global_var) {
                assignment.var_value(solver_var)
            } else {
                Some(variables.var_data_global(global_var).unit.unwrap_or(false))
            };

            model.assignment[global_var.index()] = value;
        }
    }

    ctx.solver_state.sat_state = SatState::Sat;
}
