// Absorbed from varisat 0.2.2 (https://github.com/jix/varisat, master
// @ 33e87693, file varisat/src/decision.rs); the thin wrappers around
// single Vsids calls (`make_available`, `initialize_var`, `remove_var`)
// were inlined at their call sites.
// Copyright (c) 2017-2019 Jannis Harder, MIT OR Apache-2.0 — ferroplan's
// exact dual license. Ferroplan code from the absorption commit onward;
// see ATTRIBUTION.md.

//! Decision heuristics.

use crate::{
    context::Context,
    prop::{enqueue_assignment, Reason},
};

pub mod vsids;

/// Make a decision and enqueue it.
///
/// Returns `false` if no decision was made because all variables are assigned.
pub fn make_decision(ctx: &mut Context) -> bool {
    let Context {
        assignment,
        impl_graph,
        trail,
        vsids,
        ..
    } = ctx;

    if let Some(decision_var) = vsids.find(|&var| assignment.var_value(var).is_none()) {
        let decision = decision_var.lit(assignment.last_var_value(decision_var));

        trail.new_decision_level();

        enqueue_assignment(assignment, impl_graph, trail, decision, Reason::Unit);

        true
    } else {
        false
    }
}
