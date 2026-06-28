//! WebAssembly bindings for ferroplan — run the planner in the browser.
//!
//! Exposes [`plan`]: given a PDDL domain + problem (and optional mode/threads),
//! return the structured [`ferroplan::Solution`] as a JSON string. WASM has no
//! threads here, so we always solve with `threads = 1` (the lib's data-parallel
//! map falls back to a sequential pass, producing an identical result).
//!
//! Build: `cargo build -p ferroplan-wasm --release --target wasm32-unknown-unknown`
//! then `wasm-bindgen --target web --out-dir web/pkg target/wasm32-unknown-unknown/release/ferroplan_wasm.wasm`.
//! See `web/index.html` for a self-contained demo.

use ferroplan::{solve, Mode, Options, Search};
use wasm_bindgen::prelude::*;

/// Solve a PDDL domain+problem; returns a JSON string of the `Solution`, or
/// `{"error": "..."}` on a parse/solve error. `mode` is one of "auto", "ff",
/// "pddl3", "partition", "temporal" (case-insensitive; unknown falls back to Auto,
/// which itself routes durative-action problems to the temporal solver).
///
/// `flags` is a comma-separated list of the planner's env-gated feature switches to
/// enable for this solve (e.g. "tdemand,tdecomp"): `tdemand` = converging-resource
/// demand guidance + goal-relevance pruning, `tdecomp` = the partition-and-resolve
/// decomposer — what the genuinely-hard temporal problems need. They're env vars in
/// the lib; WASM is single-threaded, so we set them in-process here and reset the
/// whole managed set each call so one pick never leaks into the next.
#[wasm_bindgen]
pub fn plan(
    domain: &str,
    problem: &str,
    mode: Option<String>,
    flags: Option<String>,
    search: Option<String>,
) -> String {
    apply_flags(flags.as_deref());
    let opts = Options {
        mode: parse_mode(mode.as_deref()),
        search: parse_search(search.as_deref()),
        threads: 1,
        ..Default::default()
    };
    match solve(domain, problem, &opts) {
        Ok(sol) => {
            serde_json::to_string(&sol).unwrap_or_else(|e| err_json(&format!("serialize: {e}")))
        }
        Err(e) => err_json(&e.to_string()),
    }
}

/// Map the demo's short feature names to ferroplan's feature overrides for this
/// solve (env vars panic on wasm, so we use the in-process override instead).
/// Resets the whole managed set each call so a previous pick can't carry over.
fn apply_flags(flags: Option<&str>) {
    let want: std::collections::HashSet<&str> = flags
        .unwrap_or("")
        .split(',')
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
        .collect();
    ferroplan::features::set_overrides(
        want.contains("tdemand"),
        want.contains("tdecomp"),
        want.contains("tconc"),
    );
}

/// ferroplan's version, for the demo footer.
#[wasm_bindgen]
pub fn version() -> String {
    env!("CARGO_PKG_VERSION").to_string()
}

fn parse_mode(m: Option<&str>) -> Mode {
    match m.map(|s| s.to_ascii_lowercase()).as_deref() {
        Some("ff") => Mode::Ff,
        Some("pddl3") => Mode::Pddl3,
        Some("partition") => Mode::Partition,
        Some("temporal") => Mode::Temporal,
        _ => Mode::Auto,
    }
}

/// Map the demo's search names to [`Search`]; unknown / `auto` ⇒ the engine default.
fn parse_search(s: Option<&str>) -> Search {
    match s.map(|s| s.to_ascii_lowercase()).as_deref() {
        Some("ehc") => Search::Ehc,
        Some("best-first") => Search::BestFirst,
        Some("ehc-then-bf") => Search::EhcThenBestFirst,
        _ => Search::Auto,
    }
}

fn err_json(msg: &str) -> String {
    serde_json::json!({ "error": msg }).to_string()
}
