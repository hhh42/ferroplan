// Absorbed from varisat 0.2.2 (https://github.com/jix/varisat, master
// @ 33e87693, file varisat/src/glue.rs); takes its parts explicitly
// instead of a partial context reference.
// Copyright (c) 2017-2019 Jannis Harder, MIT OR Apache-2.0 — ferroplan's
// exact dual license. Ferroplan code from the absorption commit onward;
// see ATTRIBUTION.md.

//! Compute glue levels of clauses.
//!
//! The glue level of a propagating clause is the number of distinct decision levels of the clause's
//! variables. This is also called the literal block distance (LBD). For each clause the smallest
//! glue level observed is used as an indicator of how useful that clause is.

use crate::{lit::Lit, prop::ImplGraph, tmp::TmpFlags};

/// Compute the glue level of a clause.
pub fn compute_glue(tmp_flags: &mut TmpFlags, impl_graph: &ImplGraph, lits: &[Lit]) -> usize {
    let flags = &mut tmp_flags.flags;

    let mut glue = 0;

    for &lit in lits {
        let level = impl_graph.level(lit.var());
        let flag = &mut flags[level];
        if !*flag {
            *flag = true;
            glue += 1
        }
    }

    for &lit in lits {
        let level = impl_graph.level(lit.var());
        flags[level] = false;
    }

    glue
}
