// Absorbed from varisat 0.2.2 (https://github.com/jix/varisat, master
// @ 33e87693, file varisat/src/config.rs); the `ConfigUpdate`/`DocDefault`
// derive machinery (varisat-internal-macros) was left behind — this is a
// plain struct with a hand-written `Default`.
// Copyright (c) 2017-2019 Jannis Harder, MIT OR Apache-2.0 — ferroplan's
// exact dual license. Ferroplan code from the absorption commit onward;
// see ATTRIBUTION.md.

//! Solver configuration.

/// Configurable parameters used during solving.
pub struct SolverConfig {
    /// Multiplicative decay for the VSIDS decision heuristic.
    ///
    /// Default: 0.95, range 0.5..1.0.
    pub vsids_decay: f32,

    /// Multiplicative decay for clause activities.
    ///
    /// Default: 0.999, range 0.5..1.0.
    pub clause_activity_decay: f32,

    /// Number of conflicts between local clause reductions.
    ///
    /// Default: 15000.
    pub reduce_locals_interval: u64,

    /// Number of conflicts between mid clause reductions.
    ///
    /// Default: 10000.
    pub reduce_mids_interval: u64,

    /// Scaling factor for luby sequence based restarts (number of conflicts).
    ///
    /// Default: 128.
    pub luby_restart_interval_scale: u64,
}

impl Default for SolverConfig {
    fn default() -> SolverConfig {
        SolverConfig {
            vsids_decay: 0.95,
            clause_activity_decay: 0.999,
            reduce_locals_interval: 15_000,
            reduce_mids_interval: 10_000,
            luby_restart_interval_scale: 128,
        }
    }
}
