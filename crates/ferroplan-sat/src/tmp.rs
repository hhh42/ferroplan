// Absorbed from varisat 0.2.2 (https://github.com/jix/varisat, master
// @ 33e87693, file varisat/src/tmp.rs).
// Copyright (c) 2017-2019 Jannis Harder, MIT OR Apache-2.0 — ferroplan's
// exact dual license. Ferroplan code from the absorption commit onward;
// see ATTRIBUTION.md.

//! Temporary data.
use crate::lit::Lit;

/// Temporary data used by various parts of the solver.
///
/// Make sure to check any documented invariants when using this. Also make sure to check all
/// existing users when adding invariants.
#[derive(Default)]
pub struct TmpData {
    pub lits: Vec<Lit>,
    pub lits_2: Vec<Lit>,
}

/// Temporary data that is automatically resized.
///
/// This contains buffers that are automatically resized when the variable count of the solver
/// changes. They are also always kept in a clean state, so using them doesn't come with costs
/// proportional to the number of variables.
///
/// Make sure to check any documented invariants when using this. Also make sure to check all
/// existing users when adding invariants.
#[derive(Default)]
pub struct TmpFlags {
    /// A boolean for each literal.
    ///
    /// Reset to all-false, keep size.
    pub flags: Vec<bool>,
}

impl TmpFlags {
    /// Update structures for a new variable count.
    pub fn set_var_count(&mut self, count: usize) {
        self.flags.resize(count * 2, false);
    }
}
