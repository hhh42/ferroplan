// Absorbed from varisat 0.2.2 (https://github.com/jix/varisat, master
// @ 33e87693, file varisat/src/clause/activity.rs).
// Copyright (c) 2017-2019 Jannis Harder, MIT OR Apache-2.0 — ferroplan's
// exact dual license. Ferroplan code from the absorption commit onward;
// see ATTRIBUTION.md.

//! Clause activity.
use crate::{config::SolverConfig, context::Context};

use super::ClauseRef;

/// Clause activity.
///
/// The individual clause activities are stored in the clause allocator. This stores global metadata
/// used for bumping and decaying activities.
pub struct ClauseActivity {
    /// The value to add on bumping.
    bump: f32,
    /// The inverse of the decay factor.
    inv_decay: f32,
}

impl Default for ClauseActivity {
    fn default() -> ClauseActivity {
        ClauseActivity {
            bump: 1.0,
            inv_decay: 1.0 / SolverConfig::default().clause_activity_decay,
        }
    }
}

/// Rescale activities if any value exceeds this value.
fn rescale_limit() -> f32 {
    f32::MAX / 16.0
}

/// Increase a clause's activity.
pub fn bump_clause_activity(ctx: &mut Context, cref: ClauseRef) {
    let bump = ctx.clause_activity.bump;
    let header = ctx.clause_alloc.header_mut(cref);

    let activity = header.activity() + bump;

    header.set_activity(activity);

    if activity > rescale_limit() {
        rescale_clause_activities(ctx);
    }
}

/// Rescale all values to avoid an overflow.
fn rescale_clause_activities(ctx: &mut Context) {
    let rescale_factor = 1.0 / rescale_limit();

    let Context {
        clause_alloc: alloc,
        clause_db: db,
        ..
    } = ctx;

    db.clauses.retain(|&cref| {
        let header = alloc.header_mut(cref);
        if header.deleted() {
            false
        } else {
            let activity = header.activity() * rescale_factor;
            header.set_activity(activity);
            true
        }
    });
    ctx.clause_activity.bump *= rescale_factor;
}

/// Decay the clause activities.
pub fn decay_clause_activities(ctx: &mut Context) {
    let activities = &mut ctx.clause_activity;
    activities.bump *= activities.inv_decay;
    if activities.bump >= rescale_limit() {
        rescale_clause_activities(ctx);
    }
}
