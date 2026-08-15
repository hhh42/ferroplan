// Absorbed from varisat 0.2.2 (https://github.com/jix/varisat, master
// @ 33e87693, file varisat/src/variables/data.rs); the sampling-mode
// partition (Sample/Witness/Hide) was stripped with the witness/hide
// surface — every user variable is a plain sampling variable here.
// Copyright (c) 2017-2019 Jannis Harder, MIT OR Apache-2.0 — ferroplan's
// exact dual license. Ferroplan code from the absorption commit onward;
// see ATTRIBUTION.md.

//! Data associated with variables.

/// Data associated with variables.
///
/// This is available for each _global_ variable, even if eliminated within the solver.
#[derive(Clone)]
pub struct VarData {
    /// Whether the variable is forced by a unit clause.
    ///
    /// This is used to remember unit clauses after they are removed from the solver.
    pub unit: Option<bool>,
    /// True if there are no clauses containing this variable and other variables.
    ///
    /// This is the case if there are no clauses containing this variable or just a unit clause with
    /// this variable.
    pub isolated: bool,
    /// True if this variable is part of the current assumptions.
    pub assumed: bool,
    /// Whether the global variable was deleted.
    pub deleted: bool,
}

impl Default for VarData {
    fn default() -> VarData {
        VarData {
            unit: None,
            isolated: true,
            assumed: false,
            deleted: true,
        }
    }
}

impl VarData {
    /// Default variable data for a new user variable.
    pub fn user_default() -> VarData {
        VarData {
            deleted: false,
            ..VarData::default()
        }
    }
}
