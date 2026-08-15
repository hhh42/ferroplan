// Absorbed from varisat 0.2.2 (https://github.com/jix/varisat, master
// @ 33e87693, file varisat/src/binary.rs); the proof steps went with the
// proof seam.
// Copyright (c) 2017-2019 Jannis Harder, MIT OR Apache-2.0 — ferroplan's
// exact dual license. Ferroplan code from the absorption commit onward;
// see ATTRIBUTION.md.

//! Binary clauses.

use crate::{context::Context, lit::Lit};

/// Binary clauses.
#[derive(Default)]
pub struct BinaryClauses {
    by_lit: Vec<Vec<Lit>>,
    count: usize,
}

impl BinaryClauses {
    /// Update structures for a new variable count.
    pub fn set_var_count(&mut self, count: usize) {
        self.by_lit.resize(count * 2, vec![]);
    }

    /// Add a binary clause.
    pub fn add_binary_clause(&mut self, lits: [Lit; 2]) {
        for i in 0..2 {
            self.by_lit[(!lits[i]).code()].push(lits[i ^ 1]);
        }
        self.count += 1;
    }

    /// Implications of a given literal
    pub fn implied(&self, lit: Lit) -> &[Lit] {
        &self.by_lit[lit.code()]
    }

    /// Number of binary clauses.
    #[cfg(test)]
    pub fn count(&self) -> usize {
        self.count
    }
}

/// Remove binary clauses that have an assigned literal.
pub fn simplify_binary(ctx: &mut Context) {
    let Context {
        binary_clauses,
        assignment,
        ..
    } = ctx;

    let mut double_count = 0;

    for (code, implied) in binary_clauses.by_lit.iter_mut().enumerate() {
        let lit = Lit::from_code(code);

        if !assignment.lit_is_unk(lit) {
            implied.clear();
        } else {
            implied.retain(|&other_lit| assignment.lit_is_unk(other_lit));

            double_count += implied.len();
        }
    }

    binary_clauses.count = double_count / 2;
}
