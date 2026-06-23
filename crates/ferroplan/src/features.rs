//! Process-global overrides for the env-gated planner features.
//!
//! The temporal demand/relevance + decomposer features are normally switched on by
//! `FF_TDEMAND` / `FF_TDECOMP` env vars (great for the CLI). But **WASM can't set
//! env vars** — `std::env::set_var` *panics* on `wasm32-unknown-unknown` — and
//! embedded library callers (e.g. the `sim_core` game) may not want to either. So
//! each feature getter is `override OR env`: the CLI keeps working via the vars, and
//! a library/WASM caller flips the override instead (env *reads* are panic-free on
//! wasm, so the OR is safe there). Single global state, mirroring env semantics; set
//! it once before `solve`.
use std::sync::atomic::{AtomicBool, Ordering::Relaxed};

static TDEMAND: AtomicBool = AtomicBool::new(false);
static TDECOMP: AtomicBool = AtomicBool::new(false);

/// Set both overrides (e.g. from the WASM `flags` arg). Idempotent; `false` clears
/// an override so a later `solve` doesn't inherit a previous caller's choice.
pub fn set_overrides(tdemand: bool, tdecomp: bool) {
    TDEMAND.store(tdemand, Relaxed);
    TDECOMP.store(tdecomp, Relaxed);
}

/// Converging-resource demand guidance + goal-relevance pruning (temporal path).
pub fn tdemand() -> bool {
    TDEMAND.load(Relaxed) || std::env::var("FF_TDEMAND").is_ok()
}

/// The partition-and-resolve decomposer (temporal path).
pub fn tdecomp() -> bool {
    TDECOMP.load(Relaxed) || std::env::var("FF_TDECOMP").is_ok()
}
