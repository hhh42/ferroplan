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

use ferroplan::{solve, Mode, Options};
use wasm_bindgen::prelude::*;

/// Solve a PDDL domain+problem; returns a JSON string of the `Solution`, or
/// `{"error": "..."}` on a parse/solve error. `mode` is one of "auto", "ff",
/// "pddl3", "partition" (case-insensitive; unknown falls back to Auto).
#[wasm_bindgen]
pub fn plan(domain: &str, problem: &str, mode: Option<String>) -> String {
    let opts = Options {
        mode: parse_mode(mode.as_deref()),
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
        _ => Mode::Auto,
    }
}

fn err_json(msg: &str) -> String {
    serde_json::json!({ "error": msg }).to_string()
}
