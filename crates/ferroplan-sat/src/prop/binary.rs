// Absorbed from varisat 0.2.2 (https://github.com/jix/varisat, master
// @ 33e87693, file varisat/src/prop/binary.rs); the split borrow became
// an index walk — the implication list cannot change while it is walked.
// Copyright (c) 2017-2019 Jannis Harder, MIT OR Apache-2.0 — ferroplan's
// exact dual license. Ferroplan code from the absorption commit onward;
// see ATTRIBUTION.md.

//! Propagation of binary clauses.
use crate::{context::Context, lit::Lit};

use super::{enqueue_assignment, Conflict, Reason};

/// Propagate all literals implied by the given literal via binary clauses.
///
/// On conflict return the binary clause propagating the conflicting assignment.
pub fn propagate_binary(ctx: &mut Context, lit: Lit) -> Result<(), Conflict> {
    let mut i = 0;
    while i < ctx.binary_clauses.implied(lit).len() {
        let implied = ctx.binary_clauses.implied(lit)[i];
        i += 1;

        if ctx.assignment.lit_is_false(implied) {
            return Err(Conflict::Binary([implied, !lit]));
        } else if !ctx.assignment.lit_is_true(implied) {
            let Context {
                assignment,
                impl_graph,
                trail,
                ..
            } = ctx;
            enqueue_assignment(
                assignment,
                impl_graph,
                trail,
                implied,
                Reason::Binary([!lit]),
            );
        }
    }

    Ok(())
}
