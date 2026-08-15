// Absorbed from varisat 0.2.2 (https://github.com/jix/varisat, master
// @ 33e87693, file varisat/src/prop/long.rs); the partial_ref split
// became an explicit `mem::take` of the watchlists and clause allocator
// (nothing reached through the remaining context touches either), and
// assignment reads go through the safe accessors — the raw-pointer scan
// over the watchlist and clause buffer is kept verbatim.
// Copyright (c) 2017-2019 Jannis Harder, MIT OR Apache-2.0 — ferroplan's
// exact dual license. Ferroplan code from the absorption commit onward;
// see ATTRIBUTION.md.

//! Propagation of long clauses.
use std::mem::take;

use crate::{clause::ClauseAlloc, context::Context, lit::Lit};

use super::{enqueue_assignment, Conflict, Reason, Watch, Watchlists};

/// Propagate all literals implied by long clauses watched by the given literal.
///
/// On conflict return the clause propagating the conflicting assignment.
///
/// See [`prop::watch`](crate::prop::watch) for the invariants that this has to uphold.
pub fn propagate_long(ctx: &mut Context, lit: Lit) -> Result<(), Conflict> {
    let mut watchlists = take(&mut ctx.watchlists);
    let mut alloc = take(&mut ctx.clause_alloc);

    let result = scan_watchlist(ctx, &mut watchlists, &mut alloc, lit);

    ctx.watchlists = watchlists;
    ctx.clause_alloc = alloc;

    result
}

/// The watchlist scan of [`propagate_long`], on the parts taken out of the context.
fn scan_watchlist(
    ctx: &mut Context,
    watchlists: &mut Watchlists,
    alloc: &mut ClauseAlloc,
    lit: Lit,
) -> Result<(), Conflict> {
    // The code below is heavily optimized and replaces a much nicer but sadly slower version.
    // Nevertheless it still performs full bound checks. Therefore this function is safe to call
    // even when some other code violated invariants of for example the clause db.
    unsafe {
        let watch_begin;
        let watch_end;
        {
            let watch_list = watchlists.watched_by_mut(lit);
            watch_begin = watch_list.as_mut_ptr();
            watch_end = watch_begin.add(watch_list.len());
        }
        let mut watch_ptr = watch_begin;

        let false_lit = !lit;

        let mut watch_write = watch_ptr;

        'watchers: while watch_ptr != watch_end {
            let watch = *watch_ptr;
            watch_ptr = watch_ptr.add(1);

            // If the blocking literal (which is part of the watched clause) is already true, the
            // watched clause is satisfied and we don't even have to look at it.
            if ctx.assignment.lit_is_true(watch.blocking) {
                *watch_write = watch;
                watch_write = watch_write.add(1);
                continue;
            }

            let cref = watch.cref;

            // Make sure we can access at least 3 lits
            alloc.check_bounds(cref, 3);

            let clause_ptr = alloc.lits_ptr_mut_unchecked(cref);
            let &mut header = alloc.header_unchecked_mut(cref);

            // First we ensure that the literal we're currently propagating is at index 1. This
            // prepares the literal order for further propagations, as the propagating literal has
            // to be at index 0. Doing this here also avoids a similar check later should the clause
            // be satisfied by a non-watched literal, as we can just move it to index 1.
            let mut first = *clause_ptr.add(0);
            if first == false_lit {
                let c1 = *clause_ptr.add(1);
                first = c1;
                *clause_ptr.add(0) = c1;
                *clause_ptr.add(1) = false_lit;
            }

            // We create a new watch with the other watched literal as blocking literal. This will
            // either replace the currently processed watch or be added to another literals watch
            // list.
            let new_watch = Watch {
                cref,
                blocking: first,
            };

            // If the other watched literal (now the first) isn't the blocking literal, check
            // whether that one is true. If so nothing else needs to be done.
            if first != watch.blocking && ctx.assignment.lit_is_true(first) {
                *watch_write = new_watch;
                watch_write = watch_write.add(1);
                continue;
            }

            // At this point we try to find a non-false unwatched literal to replace our current
            // literal as the watched literal.

            let clause_len = header.len();
            let mut lit_ptr = clause_ptr.add(2);
            let lit_end = clause_ptr.add(clause_len);

            // Make sure we can access all clause literals.
            alloc.check_bounds(cref, clause_len);

            while lit_ptr != lit_end {
                let rest_lit = *lit_ptr;
                if !ctx.assignment.lit_is_false(rest_lit) {
                    // We found a non-false literal and make it a watched literal by reordering the
                    // literals and adding the watch to the corresponding watchlist.
                    *clause_ptr.offset(1) = rest_lit;
                    *lit_ptr = false_lit;

                    // We're currently using unsafe to scan the watchlist of lit, so make extra
                    // sure we're not aliasing.
                    assert_ne!(!rest_lit, lit);
                    watchlists.add_watch(!rest_lit, new_watch);
                    continue 'watchers;
                }
                lit_ptr = lit_ptr.add(1);
            }

            // We didn't find a non-false unwatched literal, so either we're propagating or we have
            // a conflict.
            *watch_write = new_watch;
            watch_write = watch_write.add(1);

            // If the other watched literal is false we have a conflict.
            if ctx.assignment.lit_is_false(first) {
                // We move all unprocessed watches and resize the current watchlist.
                while watch_ptr != watch_end {
                    *watch_write = *watch_ptr;
                    watch_write = watch_write.add(1);
                    watch_ptr = watch_ptr.add(1);
                }
                let out_size = ((watch_write as usize) - (watch_begin as usize))
                    / std::mem::size_of::<Watch>();

                watchlists.watched_by_mut(lit).truncate(out_size);

                return Err(Conflict::Long(cref));
            }

            // Otherwise we enqueue a new propagation.
            let Context {
                assignment,
                impl_graph,
                trail,
                ..
            } = ctx;
            enqueue_assignment(assignment, impl_graph, trail, first, Reason::Long(cref));
        }

        let out_size =
            ((watch_write as usize) - (watch_begin as usize)) / std::mem::size_of::<Watch>();
        watchlists.watched_by_mut(lit).truncate(out_size);
    }
    Ok(())
}
