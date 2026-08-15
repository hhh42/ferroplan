// Absorbed from varisat 0.2.2 (https://github.com/jix/varisat, master
// @ 33e87693, file varisat/src/clause/db.rs); `filter_clauses` and
// `clauses_iter` take their parts explicitly instead of a partial
// context reference.
// Copyright (c) 2017-2019 Jannis Harder, MIT OR Apache-2.0 — ferroplan's
// exact dual license. Ferroplan code from the absorption commit onward;
// see ATTRIBUTION.md.

//! Database for long clauses.
use std::mem::transmute;

use crate::{context::Context, lit::Lit, prop::Reason};

use super::{header::HEADER_LEN, ClauseAlloc, ClauseHeader, ClauseRef};

/// Partitions of the clause database.
///
/// The long clauses are partitioned into 4 [`Tier`]s. This follows the approach described by
/// Chanseok Oh in ["Between SAT and UNSAT: The Fundamental Difference in CDCL
/// SAT"](https://doi.org/10.1007/978-3-319-24318-4_23), section 4.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
#[repr(u8)]
pub enum Tier {
    Irred = 0,
    Core = 1,
    Mid = 2,
    Local = 3,
}

impl Tier {
    /// Total number of tiers.
    pub const fn count() -> usize {
        4
    }

    /// Cast an index into the corresponding tier.
    ///
    /// # Safety
    ///
    /// The index must be below [`Tier::count()`].
    pub unsafe fn from_index(index: usize) -> Tier {
        debug_assert!(index < Tier::count());
        transmute::<u8, Tier>(index as u8)
    }
}

/// Database for long clauses.
///
/// Removal of clauses from the `clauses` and the `by_tier` fields can be delayed. The clause
/// header's deleted and tier fields need to be checked when iterating over these. `by_tier` may
/// also contain duplicate entries.
#[derive(Default)]
pub struct ClauseDb {
    /// May contain deleted clauses, see above
    pub(crate) clauses: Vec<ClauseRef>,
    /// May contain deleted and moved clauses, see above
    pub(super) by_tier: [Vec<ClauseRef>; Tier::count()],
    /// These counts should always be up to date
    pub(super) count_by_tier: [usize; Tier::count()],
    /// Size of deleted but not collected clauses
    pub(super) garbage_size: usize,
}

impl ClauseDb {
    /// The number of long clauses of a given tier.
    #[cfg(test)]
    pub fn count_by_tier(&self, tier: Tier) -> usize {
        self.count_by_tier[tier as usize]
    }
}

/// Add a long clause to the database.
pub fn add_clause(ctx: &mut Context, header: ClauseHeader, lits: &[Lit]) -> ClauseRef {
    let tier = header.tier();

    let cref = ctx.clause_alloc.add_clause(header, lits);

    ctx.watchlists.watch_clause(cref, [lits[0], lits[1]]);

    let db = &mut ctx.clause_db;

    db.clauses.push(cref);
    db.by_tier[tier as usize].push(cref);
    db.count_by_tier[tier as usize] += 1;

    cref
}

/// Change the tier of a long clause.
///
/// This is a noop for a clause already of the specified tier.
pub fn set_clause_tier(ctx: &mut Context, cref: ClauseRef, tier: Tier) {
    let Context {
        clause_alloc: alloc,
        clause_db: db,
        ..
    } = ctx;

    let old_tier = alloc.header(cref).tier();
    if old_tier != tier {
        db.count_by_tier[old_tier as usize] -= 1;
        db.count_by_tier[tier as usize] += 1;

        alloc.header_mut(cref).set_tier(tier);
        db.by_tier[tier as usize].push(cref);
    }
}

/// Delete a long clause from the database.
pub fn delete_clause(ctx: &mut Context, cref: ClauseRef) {
    // TODO Don't force a rebuild of all watchlists here
    ctx.watchlists.disable();

    let Context {
        clause_alloc: alloc,
        clause_db: db,
        ..
    } = ctx;

    let header = alloc.header_mut(cref);

    debug_assert!(
        !header.deleted(),
        "delete_clause for already deleted clause"
    );

    header.set_deleted(true);

    db.count_by_tier[header.tier() as usize] -= 1;

    db.garbage_size += header.len() + HEADER_LEN;
}

/// Delete a long clause from the database unless it is asserting.
///
/// Returns true if the clause was deleted.
pub fn try_delete_clause(ctx: &mut Context, cref: ClauseRef) -> bool {
    let initial_lit = ctx.clause_alloc.clause(cref).lits()[0];
    let asserting = ctx.assignment.lit_is_true(initial_lit)
        && ctx.impl_graph.reason(initial_lit.var()) == &Reason::Long(cref);

    if !asserting {
        delete_clause(ctx, cref);
    }
    !asserting
}

/// Iterator over all long clauses.
///
/// This filters deleted (but uncollected) clauses on the fly.
pub fn clauses_iter<'a>(
    db: &'a ClauseDb,
    alloc: &'a ClauseAlloc,
) -> impl Iterator<Item = ClauseRef> + 'a {
    db.clauses
        .iter()
        .cloned()
        .filter(move |&cref| !alloc.header(cref).deleted())
}

/// Iterate over all and remove some long clauses.
///
/// Takes a closure that returns true for each clause that should be kept and false for each that
/// should be deleted.
pub fn filter_clauses<F>(
    alloc: &mut ClauseAlloc,
    db: &mut ClauseDb,
    watchlists: &mut crate::prop::Watchlists,
    mut filter: F,
) where
    F: FnMut(&mut ClauseAlloc, ClauseRef) -> bool,
{
    watchlists.disable();

    let ClauseDb {
        clauses,
        count_by_tier,
        garbage_size,
        ..
    } = db;

    clauses.retain(|&cref| {
        if alloc.header(cref).deleted() {
            false
        } else if filter(alloc, cref) {
            true
        } else {
            let header = alloc.header_mut(cref);

            header.set_deleted(true);

            count_by_tier[header.tier() as usize] -= 1;

            *garbage_size += header.len() + HEADER_LEN;

            false
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::context::set_var_count;
    use crate::lit::Lit;

    fn clause(lits: &[isize]) -> Vec<Lit> {
        lits.iter().map(|&l| Lit::from_dimacs(l)).collect()
    }

    #[test]
    fn set_tiers_and_deletes() {
        let mut ctx = Context::default();

        let clauses: Vec<Vec<Lit>> = vec![
            clause(&[1, 2, 3]),
            clause(&[4, -5, 6]),
            clause(&[-2, 3, -4]),
            clause(&[-3, 5, 2, 7, 5]),
        ];

        let max_var = 7;
        set_var_count(&mut ctx, max_var);

        let tiers = [Tier::Irred, Tier::Core, Tier::Mid, Tier::Local];
        let new_tiers = [Tier::Irred, Tier::Local, Tier::Local, Tier::Core];

        let mut crefs = vec![];

        for (clause, &tier) in clauses.iter().zip(tiers.iter()) {
            let mut header = ClauseHeader::new();
            header.set_tier(tier);
            let cref = add_clause(&mut ctx, header, clause);
            crefs.push(cref);
        }

        for (&cref, &tier) in crefs.iter().rev().zip(new_tiers.iter().rev()) {
            set_clause_tier(&mut ctx, cref, tier);
        }

        // We only check presence, as deletion from these lists is delayed
        assert!(ctx.clause_db.by_tier[Tier::Irred as usize].contains(&crefs[0]));
        assert!(ctx.clause_db.by_tier[Tier::Core as usize].contains(&crefs[3]));
        assert!(ctx.clause_db.by_tier[Tier::Local as usize].contains(&crefs[1]));
        assert!(ctx.clause_db.by_tier[Tier::Local as usize].contains(&crefs[2]));

        assert_eq!(ctx.clause_db.count_by_tier(Tier::Irred), 1);
        assert_eq!(ctx.clause_db.count_by_tier(Tier::Core), 1);
        assert_eq!(ctx.clause_db.count_by_tier(Tier::Mid), 0);
        assert_eq!(ctx.clause_db.count_by_tier(Tier::Local), 2);

        delete_clause(&mut ctx, crefs[0]);
        delete_clause(&mut ctx, crefs[2]);

        assert_eq!(ctx.clause_db.count_by_tier(Tier::Irred), 0);
        assert_eq!(ctx.clause_db.count_by_tier(Tier::Core), 1);
        assert_eq!(ctx.clause_db.count_by_tier(Tier::Mid), 0);
        assert_eq!(ctx.clause_db.count_by_tier(Tier::Local), 1);
    }
}
