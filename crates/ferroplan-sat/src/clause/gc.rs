// Absorbed from varisat 0.2.2 (https://github.com/jix/varisat, master
// @ 33e87693, file varisat/src/clause/gc.rs).
// Copyright (c) 2017-2019 Jannis Harder, MIT OR Apache-2.0 — ferroplan's
// exact dual license. Ferroplan code from the absorption commit onward;
// see ATTRIBUTION.md.

//! Garbage collection of long clauses.
use crate::{context::Context, prop::Reason};

use super::{ClauseAlloc, Tier};

/// Perform a garbage collection of long clauses if necessary.
pub fn collect_garbage(ctx: &mut Context) {
    // Collecting when a fixed fraction of the allocation is garbage amortizes collection costs.
    if ctx.clause_db.garbage_size * 2 > ctx.clause_alloc.buffer_size() {
        collect_garbage_now(ctx);
    }
}

/// Unconditionally perform a garbage collection of long clauses.
///
/// This needs to invalidate or update any other data structure containing references to
/// clauses.
fn collect_garbage_now(ctx: &mut Context) {
    ctx.watchlists.disable();

    mark_asserting_clauses(ctx);

    let Context {
        clause_alloc: alloc,
        clause_db: db,
        impl_graph,
        ..
    } = ctx;

    assert!(
        db.garbage_size <= alloc.buffer_size(),
        "Inconsistent garbage tracking in ClauseDb"
    );
    let current_size = alloc.buffer_size() - db.garbage_size;

    // Allocating just the current size would lead to an immediate growing when new clauses are
    // learned, overallocating here avoids that.
    let mut new_alloc = ClauseAlloc::with_capacity(current_size * 2);

    let mut new_clauses = vec![];
    let mut new_by_tier: [Vec<_>; Tier::count()] = Default::default();

    db.clauses.retain(|&cref| {
        let clause = alloc.clause(cref);
        let mut header = *clause.header();
        if header.deleted() {
            false
        } else {
            let clause_is_asserting = header.mark();
            header.set_mark(false);

            let new_cref = new_alloc.add_clause(header, clause.lits());

            new_clauses.push(new_cref);
            new_by_tier[header.tier() as usize].push(new_cref);

            if clause_is_asserting {
                let asserted_lit = clause.lits()[0];

                debug_assert_eq!(impl_graph.reason(asserted_lit.var()), &Reason::Long(cref));
                impl_graph.update_reason(asserted_lit.var(), Reason::Long(new_cref));
            }
            true
        }
    });

    *alloc = new_alloc;
    db.clauses = new_clauses;
    db.by_tier = new_by_tier;
    db.garbage_size = 0;
}

/// Mark asserting clauses to track them through GC.
fn mark_asserting_clauses(ctx: &mut Context) {
    let Context {
        clause_alloc: alloc,
        impl_graph,
        trail,
        ..
    } = ctx;

    for &lit in trail.trail().iter() {
        if let Reason::Long(cref) = impl_graph.reason(lit.var()) {
            alloc.header_mut(*cref).set_mark(true);
        }
    }
}
