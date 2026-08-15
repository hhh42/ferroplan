// Absorbed from varisat 0.2.2 (https://github.com/jix/varisat, master
// @ 33e87693, file varisat/src/state.rs); the proof-error bookkeeping
// (`solver_error`, `state_is_invalid`, `formula_is_empty`,
// `solver_invoked`) went with the proof seam.
// Copyright (c) 2017-2019 Jannis Harder, MIT OR Apache-2.0 — ferroplan's
// exact dual license. Ferroplan code from the absorption commit onward;
// see ATTRIBUTION.md.

//! Miscellaneous solver state.

/// Satisfiability state.
#[derive(Copy, Clone, Eq, PartialEq, Debug, Default)]
pub enum SatState {
    #[default]
    Unknown,
    Sat,
    Unsat,
    UnsatUnderAssumptions,
}

/// Miscellaneous solver state.
///
/// Anything larger or any larger group of related state variables should be moved into a separate
/// part of [`Context`](crate::context::Context).
#[derive(Default)]
pub struct SolverState {
    pub sat_state: SatState,
}
