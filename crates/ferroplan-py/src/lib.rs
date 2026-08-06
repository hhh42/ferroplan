//! Python bindings for ferroplan (via pyo3).
//!
//! The historical `plan` function remains compatibility-stable. New service
//! integrations should use `plan_production`, which returns the same bounded,
//! typed, candidate-only envelope as the Rust, CLI, WASM, and MCP+ surfaces.

use ferroplan_core::{
    capability_manifest, solve, solve_production, Mode, Options, ProductionLimits,
};
use pyo3::prelude::*;

/// Solve a PDDL domain + problem. Returns a JSON string of the legacy `Solution`
/// (parse it with `json.loads`), or `{"error": "..."}` on a parse/solve error.
///
/// This compatibility function is intentionally retained. New deployments
/// should use `plan_production` for bounded inputs and typed terminal outcomes.
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
        Ok(sol) => serialize_or_error(&sol),
        Err(e) => err_json("FP_ADAPTER", &e.to_string()),
    }
}

/// Bounded production solve. Returns a versioned `OperationEnvelope<Solution>`
/// JSON object for success, refusal, no-plan, limit, and internal failure.
/// Planner output remains candidate-only and carries no actuation authority.
#[pyfunction]
#[pyo3(signature = (
    domain,
    problem,
    mode=None,
    threads=None,
    max_evaluated=None,
    request_id=None,
    max_domain_bytes=None,
    max_problem_bytes=None,
    max_plan_steps=None,
    max_output_bytes=None,
    max_workers=None
))]
#[allow(clippy::too_many_arguments)]
fn plan_production(
    domain: &str,
    problem: &str,
    mode: Option<&str>,
    threads: Option<usize>,
    max_evaluated: Option<usize>,
    request_id: Option<&str>,
    max_domain_bytes: Option<usize>,
    max_problem_bytes: Option<usize>,
    max_plan_steps: Option<usize>,
    max_output_bytes: Option<usize>,
    max_workers: Option<usize>,
) -> String {
    let mode = match parse_mode_strict(mode) {
        Ok(mode) => mode,
        Err(message) => return adapter_refusal_json(request_id, &message),
    };
    let defaults = ProductionLimits::default();
    let limits = ProductionLimits {
        max_domain_bytes: max_domain_bytes.unwrap_or(defaults.max_domain_bytes),
        max_problem_bytes: max_problem_bytes.unwrap_or(defaults.max_problem_bytes),
        max_evaluated: max_evaluated.unwrap_or(defaults.max_evaluated),
        max_plan_steps: max_plan_steps.unwrap_or(defaults.max_plan_steps),
        max_output_bytes: max_output_bytes.unwrap_or(defaults.max_output_bytes),
        max_workers: max_workers.unwrap_or(defaults.max_workers),
    };
    let options = Options {
        mode,
        threads: threads.unwrap_or(1),
        max_evaluated,
        ..Default::default()
    };
    serialize_or_error(&solve_production(
        domain,
        problem,
        &options,
        &limits,
        request_id,
    ))
}

/// Canonical capability contract and deterministic manifest fingerprint.
/// This reports the contract, not a self-authored admission verdict.
#[pyfunction]
fn readiness() -> String {
    let manifest = capability_manifest();
    match manifest.fingerprint() {
        Ok(fingerprint) => serde_json::json!({
            "schema_version": "ferroplan.readiness-contract.v1",
            "product_version": env!("CARGO_PKG_VERSION"),
            "manifest_fingerprint": fingerprint,
            "contract_valid": true,
            "admission_state": "declared",
            "admission_notice": "Admission is verifier-derived from exact-source evidence.",
            "manifest": manifest,
        })
        .to_string(),
        Err(error) => err_json("FP_INVARIANT", &error.to_string()),
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
        Some("portfolio") => Mode::Portfolio,
        Some("optimal") => Mode::Optimal,
        _ => Mode::Auto,
    }
}

fn parse_mode_strict(m: Option<&str>) -> Result<Mode, String> {
    match m.map(|s| s.to_ascii_lowercase()).as_deref() {
        None | Some("auto") => Ok(Mode::Auto),
        Some("ff") => Ok(Mode::Ff),
        Some("pddl3") => Ok(Mode::Pddl3),
        Some("partition") => Ok(Mode::Partition),
        Some("temporal") => Ok(Mode::Temporal),
        Some("portfolio") => Ok(Mode::Portfolio),
        Some("optimal") => Ok(Mode::Optimal),
        Some(other) => Err(format!(
            "unsupported mode `{other}`; expected auto, ff, pddl3, partition, temporal, portfolio, or optimal"
        )),
    }
}

fn serialize_or_error(value: &impl serde::Serialize) -> String {
    serde_json::to_string(value)
        .unwrap_or_else(|error| err_json("FP_ADAPTER", &format!("serialize: {error}")))
}

fn adapter_refusal_json(request_id: Option<&str>, message: &str) -> String {
    serde_json::json!({
        "schema_version": "ferroplan.operation.v1",
        "request_id": request_id.unwrap_or("python-adapter-refusal"),
        "capability_id": "fp.python",
        "capability_version": env!("CARGO_PKG_VERSION"),
        "authority": "candidate_only",
        "outcome": "refused",
        "validation": "not_applicable",
        "payload": null,
        "error": {
            "code": "FP_INVALID_REQUEST",
            "message": message,
            "retryable": false
        }
    })
    .to_string()
}

fn err_json(code: &str, msg: &str) -> String {
    serde_json::json!({
        "error": {
            "code": code,
            "message": msg,
            "retryable": false
        }
    })
    .to_string()
}

#[pymodule]
fn ferroplan(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(plan, m)?)?;
    m.add_function(wrap_pyfunction!(plan_production, m)?)?;
    m.add_function(wrap_pyfunction!(readiness, m)?)?;
    m.add_function(wrap_pyfunction!(version, m)?)?;
    Ok(())
}