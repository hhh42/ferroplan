// Absorbed from varisat 0.2.2 (https://github.com/jix/varisat, master
// @ 33e87693, file varisat/src/clause/assess.rs).
// Copyright (c) 2017-2019 Jannis Harder, MIT OR Apache-2.0 — ferroplan's
// exact dual license. Ferroplan code from the absorption commit onward;
// see ATTRIBUTION.md.

//! Clause assessment.
use crate::{clause::db, clause::ClauseRef, context::Context, glue::compute_glue, lit::Lit};

use super::{activity::bump_clause_activity, ClauseHeader, Tier};

/// Assess the newly learned clause and generate a clause header.
pub fn assess_learned_clause(ctx: &mut Context, lits: &[Lit]) -> ClauseHeader {
    // This is called while the clause is still in conflict, thus the computed glue level is one
    // higher than it'll be after backtracking when the clause becomes asserting.
    let Context {
        tmp_flags,
        impl_graph,
        ..
    } = ctx;
    let glue = compute_glue(tmp_flags, impl_graph, lits) - 1;

    let mut header = ClauseHeader::new();

    header.set_glue(glue);
    header.set_tier(select_tier(glue));

    header
}

/// Compute the tier for a redundant clause with a given glue level.
fn select_tier(glue: usize) -> Tier {
    if glue <= 2 {
        Tier::Core
    } else if glue <= 6 {
        Tier::Mid
    } else {
        Tier::Local
    }
}

/// Update stats for clauses involved in the conflict.
pub fn bump_clause(ctx: &mut Context, cref: ClauseRef) {
    bump_clause_activity(ctx, cref);

    let new_tier;
    {
        let Context {
            clause_alloc,
            tmp_flags,
            impl_graph,
            ..
        } = ctx;

        let clause = clause_alloc.clause_mut(cref);

        let glue = compute_glue(tmp_flags, impl_graph, clause.lits());

        clause.header_mut().set_active(true);

        if glue < clause.header().glue() {
            clause.header_mut().set_glue(glue);
            new_tier = Some(select_tier(glue));
        } else {
            new_tier = None;
        }
    }

    if let Some(tier) = new_tier {
        db::set_clause_tier(ctx, cref, tier);
    }
}
