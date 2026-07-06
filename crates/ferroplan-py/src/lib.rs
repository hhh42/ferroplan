//! Python bindings for ferroplan (via pyo3).
//!
//! ```python
//! import ferroplan, json
//! sol = json.loads(ferroplan.plan(domain_pddl, problem_pddl))
//! print(sol["solved"], sol["plan"]["length"])
//! ```
//!
//! Build: `pip install maturin && maturin develop` (in this crate), or
//! `maturin build --release` for a wheel.

use ferroplan_core::{solve, Mode, Options};
use pyo3::prelude::*;

/// Solve a PDDL domain + problem. Returns a JSON string of the `Solution`
/// (parse it with `json.loads`), or `{"error": "..."}` on a parse/solve error.
///
/// `mode` ∈ "auto" | "ff" | "pddl3" | "partition" | "temporal" (default "auto").
/// `threads` defaults to the planner's auto count; pass 1 for deterministic
/// single-thread.
#[pyfunction]
#[pyo3(signature = (domain, problem, mode=None, threads=None))]
fn plan(domain: &str, problem: &str, mode: Option<&str>, threads: Option<usize>) -> String {
    let mut opts = Options {
        mode: parse_mode(mode),
        ..Default::default()
    };
    if let Some(t) = threads {
        opts.threads = t;
    }
    match solve(domain, problem, &opts) {
        Ok(sol) => {
            serde_json::to_string(&sol).unwrap_or_else(|e| err_json(&format!("serialize: {e}")))
        }
        Err(e) => err_json(&e.to_string()),
    }
}

/// ferroplan's version string.
#[pyfunction]
fn version() -> String {
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

fn err_json(msg: &str) -> String {
    serde_json::json!({ "error": msg }).to_string()
}

#[pymodule]
fn ferroplan(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(plan, m)?)?;
    m.add_function(wrap_pyfunction!(version, m)?)?;
    Ok(())
}
