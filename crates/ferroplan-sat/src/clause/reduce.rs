// Absorbed from varisat 0.2.2 (https://github.com/jix/varisat, master
// @ 33e87693, file varisat/src/clause/reduce.rs); `ordered-float` was
// replaced by `f32::total_cmp` and `vec_mut_scan` by explicit
// keep-vectors, proof steps went with the proof seam.
// Copyright (c) 2017-2019 Jannis Harder, MIT OR Apache-2.0 — ferroplan's
// exact dual license. Ferroplan code from the absorption commit onward;
// see ATTRIBUTION.md.

//! Clause database reduction.
use std::mem::take;

use crate::context::Context;

use super::db::{set_clause_tier, try_delete_clause, Tier};

/// Remove deleted and duplicate entries from the by_tier clause lists.
///
/// This has the side effect of setting the mark bit on all clauses of the tier.
pub fn dedup_and_mark_by_tier(ctx: &mut Context, tier: Tier) {
    let Context {
        clause_alloc: alloc,
        clause_db: db,
        ..
    } = ctx;
    let by_tier = &mut db.by_tier[tier as usize];

    by_tier.retain(|&cref| {
        let header = alloc.header_mut(cref);
        let retain = !header.deleted() && !header.mark() && header.tier() == tier;
        if retain {
            header.set_mark(true);
        }
        retain
    })
}

/// Reduce the number of local tier clauses by deleting half of them.
pub fn reduce_locals(ctx: &mut Context) {
    dedup_and_mark_by_tier(ctx, Tier::Local);

    let mut locals = take(&mut ctx.clause_db.by_tier[Tier::Local as usize]);

    {
        let alloc = &ctx.clause_alloc;
        locals.sort_unstable_by(|&a, &b| {
            alloc
                .header(a)
                .activity()
                .total_cmp(&alloc.header(b).activity())
                .then_with(|| a.cmp(&b))
        });
    }

    let mut to_delete = locals.len() / 2;
    let mut kept = Vec::with_capacity(locals.len() - to_delete);

    for &cref in locals.iter() {
        ctx.clause_alloc.header_mut(cref).set_mark(false);

        if to_delete > 0 && try_delete_clause(ctx, cref) {
            to_delete -= 1;
        } else {
            kept.push(cref);
        }
    }

    ctx.clause_db.count_by_tier[Tier::Local as usize] = kept.len();
    ctx.clause_db.by_tier[Tier::Local as usize] = kept;
}

/// Reduce the number of mid tier clauses by moving inactive ones to the local tier.
pub fn reduce_mids(ctx: &mut Context) {
    dedup_and_mark_by_tier(ctx, Tier::Mid);

    let mut mids = take(&mut ctx.clause_db.by_tier[Tier::Mid as usize]);

    mids.retain(|&cref| {
        let header = ctx.clause_alloc.header_mut(cref);
        header.set_mark(false);

        if header.active() {
            header.set_active(false);
            true
        } else {
            set_clause_tier(ctx, cref, Tier::Local);
            false
        }
    });

    ctx.clause_db.count_by_tier[Tier::Mid as usize] = mids.len();
    ctx.clause_db.by_tier[Tier::Mid as usize] = mids;
}
