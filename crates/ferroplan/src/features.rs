//! Process-global overrides for the env-gated planner features.
//!
//! Each feature has a *default* (whether it's on when nothing is configured) and two
//! ways to override it: an env var (great for the CLI) and an in-process override
//! (for **WASM**, where `std::env::set_var` *panics* on `wasm32-unknown-unknown`, and
//! embedded library callers like the `sim_core` game). Env *reads* are panic-free on
//! wasm, so a getter that consults both is safe there.
//!
//! The override is **tri-state** (`Unset` / `On` / `Off`): `Unset` falls back to the
//! default + env, while `On`/`Off` are definitive. This matters now that `tdemand`
//! defaults ON — a WASM caller must be able to force it *off*, which a plain bool
//! "override OR env" could not express. Set the override once before `solve`.
use std::sync::atomic::{AtomicU8, Ordering::Relaxed};

// Tri-state override packed into an AtomicU8.
const UNSET: u8 = 0;
const ON: u8 = 1;
const OFF: u8 = 2;

static TDEMAND: AtomicU8 = AtomicU8::new(UNSET);
static TDECOMP: AtomicU8 = AtomicU8::new(UNSET);
static TCONC: AtomicU8 = AtomicU8::new(UNSET);
static ESCALATE: AtomicU8 = AtomicU8::new(UNSET);

/// Set the overrides (e.g. from the WASM `flags` arg). Each bool is definitive for
/// this and subsequent solves — `true` forces the feature on, `false` forces it off
/// (overriding the default), so a later `solve` can't inherit a previous caller's
/// choice. To return a feature to its default + env behavior, use [`clear_overrides`].
pub fn set_overrides(tdemand: bool, tdecomp: bool, tconc: bool) {
    TDEMAND.store(if tdemand { ON } else { OFF }, Relaxed);
    TDECOMP.store(if tdecomp { ON } else { OFF }, Relaxed);
    TCONC.store(if tconc { ON } else { OFF }, Relaxed);
}

/// In-process override for the escalation ladder (see [`escalate`]) — the WASM /
/// embedded analog of `FF_NO_ESCALATE`, since env *writes* panic on wasm32.
/// Definitive until [`clear_overrides`].
pub fn set_escalate_override(on: bool) {
    ESCALATE.store(if on { ON } else { OFF }, Relaxed);
}

/// Clear all in-process overrides back to `Unset` (default + env decide).
pub fn clear_overrides() {
    TDEMAND.store(UNSET, Relaxed);
    TDECOMP.store(UNSET, Relaxed);
    TCONC.store(UNSET, Relaxed);
    ESCALATE.store(UNSET, Relaxed);
}

#[inline]
fn resolve(state: &AtomicU8, default: bool) -> bool {
    match state.load(Relaxed) {
        ON => true,
        OFF => false,
        _ => default,
    }
}

/// How much temporal demand guidance to apply. The feature graduated from a single
/// opt-in `FF_TDEMAND` to a **default-on `Numeric`** tier in v0.2 — but only the
/// numeric-goal half, because the predicate-goal-threshold half can regress makespan
/// on renewable-resource concurrency domains (it reads a `(>= (avail) 1)` guard on a
/// net-zero pool as accumulation demand and serializes). So the safe, measured win
/// (multi-round *numeric* goals: `steel >= 2`, `grain >= 10`, `coin >= 15`) is on by
/// default; the structural/predicate half stays explicit.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum DemandMode {
    /// No demand guidance and no relevance pruning — bit-identical to the pre-v0.2
    /// default.
    Off,
    /// Default (v0.2): seed demand from NUMERIC goals only (no predicate-threshold
    /// seeding). Goal-relevance pruning is also on (v0.3.0), with an unmasked
    /// complete backstop pass; `FF_NOREL` disables pruning alone.
    Numeric,
    /// Full (`FF_TDEMAND` for whole solves): additionally seed demand from
    /// predicate-goal thresholds — for the conjunctive/structural builds. The
    /// escalation ladder also retries failed default-tier searches at this tier
    /// automatically (see [`escalate`]), so the flag now mainly forces it *first*.
    Full,
}

/// Resolve the active demand tier from the override / env / default.
pub fn demand_mode() -> DemandMode {
    match TDEMAND.load(Relaxed) {
        ON => DemandMode::Full,
        OFF => DemandMode::Off,
        _ => {
            if std::env::var("FF_TDEMAND").is_ok() {
                DemandMode::Full
            } else if std::env::var("FF_NO_TDEMAND").is_ok() {
                DemandMode::Off
            } else {
                DemandMode::Numeric
            }
        }
    }
}

/// Whether *any* demand seed is built (`Numeric` or `Full`). Predicate-threshold
/// seeding is gated separately on [`demand_mode`] `== Full`; goal-relevance pruning
/// rides any non-`Off` tier (minus `FF_NOREL`).
pub fn tdemand() -> bool {
    demand_mode() != DemandMode::Off
}

/// The partition-and-resolve decomposer (temporal path). Opt-in via `FF_TDECOMP`.
pub fn tdecomp() -> bool {
    resolve(&TDECOMP, std::env::var("FF_TDECOMP").is_ok())
}

/// The on-failure escalation ladder in [`crate::temporal::solve`]: when the
/// default-tier monolithic search fails, retry at the `Full` demand tier, then
/// hand the goal to the decomposer. Each rung runs ONLY after the previous one
/// failed, so no instance that solves today can change its plan — escalation
/// spends extra time on (would-be) failures to convert them into solves.
/// Default ON; `FF_NO_ESCALATE` (or [`set_escalate_override`]`(false)` in-process)
/// disables the ladder alone, and `FF_NO_TDEMAND` (the master "pristine pre-v0.2
/// path" switch) disables it too.
pub fn escalate() -> bool {
    resolve(&ESCALATE, std::env::var("FF_NO_ESCALATE").is_err())
}

/// The concurrent scheduling phase: repack a temporal plan onto the domain's actor
/// objects to minimise makespan (so more workers finish faster). See [`crate::tsched`].
/// Opt-in via `FF_TCONC`.
pub fn tconc() -> bool {
    resolve(&TCONC, std::env::var("FF_TCONC").is_ok())
}
