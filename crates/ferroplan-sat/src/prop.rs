// Absorbed from varisat 0.2.2 (https://github.com/jix/varisat, master
// @ 33e87693, file varisat/src/prop.rs).
// Copyright (c) 2017-2019 Jannis Harder, MIT OR Apache-2.0 — ferroplan's
// exact dual license. Ferroplan code from the absorption commit onward;
// see ATTRIBUTION.md.

//! Unit propagation.
use crate::context::Context;

pub mod assignment;
pub mod binary;
pub mod graph;
pub mod long;
pub mod watch;

pub use assignment::{backtrack, enqueue_assignment, full_restart, restart, Assignment, Trail};
pub use graph::{Conflict, ImplGraph, Reason};
pub use watch::{enable_watchlists, Watch, Watchlists};

/// Propagate enqueued assignments.
///
/// Returns when all enqueued assignments are propagated, including newly propagated assignments, or
/// if there is a conflict.
///
/// On conflict the first propagation that would assign the opposite value to an already assigned
/// literal is returned.
pub fn propagate(ctx: &mut Context) -> Result<(), Conflict> {
    enable_watchlists(ctx);

    while let Some(lit) = ctx.trail.pop_queue() {
        binary::propagate_binary(ctx, lit)?;
        long::propagate_long(ctx, lit)?;
    }
    Ok(())
}
