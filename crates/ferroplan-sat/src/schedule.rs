// Absorbed from varisat 0.2.2 (https://github.com/jix/varisat, master
// @ 33e87693, file varisat/src/schedule.rs); the `log` telemetry went
// with the external dependencies, and a per-solve conflict budget was
// added — the wing's horizon ramp needs "budget exhausted" as an honest
// third verdict.
// Copyright (c) 2017-2019 Jannis Harder, MIT OR Apache-2.0 — ferroplan's
// exact dual license. Ferroplan code from the absorption commit onward;
// see ATTRIBUTION.md.

//! Scheduling of processing and solving steps.

use crate::{
    cdcl::conflict_step,
    clause::{
        collect_garbage,
        reduce::{reduce_locals, reduce_mids},
    },
    context::Context,
    prop::restart,
    state::SatState,
};

mod luby;

use luby::LubySequence;

/// Scheduling of processing and solving steps.
#[derive(Default)]
pub struct Schedule {
    conflicts: u64,
    next_restart: u64,
    restarts: u64,
    luby: LubySequence,
    /// Conflict budget for the current `solve` call; `None` is unlimited.
    pub conflict_limit: Option<u64>,
    /// Conflicts spent in the current `solve` call.
    pub conflicts_this_solve: u64,
}

/// Perform one step of the schedule.
pub fn schedule_step(ctx: &mut Context) -> bool {
    if ctx.solver_state.sat_state != SatState::Unknown {
        return false;
    }

    if let Some(limit) = ctx.schedule.conflict_limit {
        if ctx.schedule.conflicts_this_solve >= limit {
            // Budget exhausted: stop with the state honestly Unknown.
            return false;
        }
    }

    if ctx.schedule.next_restart == ctx.schedule.conflicts {
        restart(ctx);
        ctx.schedule.restarts += 1;
        let interval = ctx.schedule.luby.advance();
        ctx.schedule.next_restart += ctx.solver_config.luby_restart_interval_scale * interval;
    }

    if ctx.schedule.conflicts % ctx.solver_config.reduce_locals_interval == 0 {
        reduce_locals(ctx);
    }
    if ctx.schedule.conflicts % ctx.solver_config.reduce_mids_interval == 0 {
        reduce_mids(ctx);
    }

    collect_garbage(ctx);

    conflict_step(ctx);
    ctx.schedule.conflicts += 1;
    ctx.schedule.conflicts_this_solve += 1;
    true
}
