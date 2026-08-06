//! WebAssembly bindings for ferroplan — run the planner in the browser.
//!
//! The historical `plan` function is retained for compatibility. New hosts
//! should use `plan_production`, which returns the same bounded, typed,
//! candidate-only operation envelope exposed by the native adapters.

use ferroplan::{
    capability_manifest, solve, solve_production, Mode, Options, ProductionLimits, Search,
};
use wasm_bindgen::prelude::*;

const WASM_HARD_INPUT_BYTES: usize = 64 * 1024 * 1024;
const WASM_TEXT_FIELD_BYTES: usize = 1024 * 1024;
const WASM_JSON_FIELD_BYTES: usize = 16 * 1024 * 1024;
const WASM_MAX_THINK_EVALS: usize = 1_000_000;
const WASM_MAX_THINK_MEMORY_MB: usize = 2_048;

macro_rules! serialize_or_error {
    ($value:expr) => {
        serde_json::to_string($value)
            .unwrap_or_else(|error| err_json("FP_ADAPTER", &format!("serialize: {error}")))
    };
}

/// Compatibility solve surface. Returns legacy `Solution` JSON or an error JSON.
#[wasm_bindgen]
pub fn plan(
    domain: &str,
    problem: &str,
    mode: Option<String>,
    flags: Option<String>,
    search: Option<String>,
) -> String {
    if let Err(message) = ensure_compat_input(domain, problem) {
        return err_json("FP_LIMIT_INPUT", &message);
    }
    apply_flags(flags.as_deref());
    let opts = Options {
        mode: parse_mode(mode.as_deref()),
        search: parse_search(search.as_deref()),
        threads: 1,
        ..Default::default()
    };
    match solve(domain, problem, &opts) {
        Ok(sol) => serialize_or_error!(&sol),
        Err(e) => err_json("FP_ADAPTER", &e.to_string()),
    }
}

/// Bounded browser production solve. The returned JSON is a versioned
/// `OperationEnvelope<Solution>` and is always candidate-only.
#[wasm_bindgen]
#[allow(clippy::too_many_arguments)]
pub fn plan_production(
    domain: &str,
    problem: &str,
    mode: Option<String>,
    search: Option<String>,
    max_evaluated: Option<usize>,
    max_plan_steps: Option<usize>,
    max_output_bytes: Option<usize>,
    request_id: Option<String>,
) -> String {
    let mode = match parse_mode_strict(mode.as_deref()) {
        Ok(mode) => mode,
        Err(message) => return adapter_refusal_json(request_id.as_deref(), &message),
    };
    let search = match parse_search_strict(search.as_deref()) {
        Ok(search) => search,
        Err(message) => return adapter_refusal_json(request_id.as_deref(), &message),
    };
    let defaults = ProductionLimits::default();
    let limits = ProductionLimits {
        max_evaluated: max_evaluated.unwrap_or(defaults.max_evaluated),
        max_plan_steps: max_plan_steps.unwrap_or(defaults.max_plan_steps),
        max_output_bytes: max_output_bytes.unwrap_or(defaults.max_output_bytes),
        max_workers: 1,
        ..defaults
    };
    let options = Options {
        mode,
        search,
        threads: 1,
        max_evaluated,
        ..Default::default()
    };
    serialize_or_error!(&solve_production(
        domain,
        problem,
        &options,
        &limits,
        request_id.as_deref(),
    ))
}

/// Canonical capability contract and deterministic manifest fingerprint.
#[wasm_bindgen]
pub fn readiness() -> String {
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

/// Map short feature names to ferroplan's in-process feature overrides. WASM is
/// single-threaded; the complete managed set is reset on every call.
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

/// Explain a plan for its domain + problem.
#[wasm_bindgen]
pub fn explain(domain: &str, problem: &str, plan_json: &str) -> String {
    if let Err(message) = ensure_compat_input(domain, problem) {
        return err_json("FP_LIMIT_INPUT", &message);
    }
    if plan_json.len() > WASM_JSON_FIELD_BYTES {
        return err_json(
            "FP_LIMIT_INPUT",
            "plan JSON exceeds the browser adapter input limit",
        );
    }
    let plan: ferroplan::api::Plan = match serde_json::from_str(plan_json) {
        Ok(p) => p,
        Err(e) => return err_json("FP_PARSE", &format!("plan: {e}")),
    };
    match ferroplan::introspect::explain(domain, problem, &plan) {
        Ok(ex) => serialize_or_error!(&ex),
        Err(e) => err_json("FP_VALIDATION", &e),
    }
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
        Some(other) => Err(format!("unsupported mode `{other}`")),
    }
}

fn parse_search(s: Option<&str>) -> Search {
    match s.map(|s| s.to_ascii_lowercase()).as_deref() {
        Some("ehc") => Search::Ehc,
        Some("best-first") => Search::BestFirst,
        Some("ehc-then-bf") => Search::EhcThenBestFirst,
        _ => Search::Auto,
    }
}

fn parse_search_strict(s: Option<&str>) -> Result<Search, String> {
    match s.map(|s| s.to_ascii_lowercase()).as_deref() {
        None | Some("auto") => Ok(Search::Auto),
        Some("ehc") => Ok(Search::Ehc),
        Some("best-first") => Ok(Search::BestFirst),
        Some("ehc-then-bf") | Some("ehc-then-best-first") => Ok(Search::EhcThenBestFirst),
        Some(other) => Err(format!("unsupported search strategy `{other}`")),
    }
}

fn ensure_compat_input(domain: &str, problem: &str) -> Result<(), String> {
    if domain.len() > WASM_HARD_INPUT_BYTES || problem.len() > WASM_HARD_INPUT_BYTES {
        return Err("domain or problem exceeds the browser hard input limit".to_string());
    }
    Ok(())
}

fn adapter_refusal_json(request_id: Option<&str>, message: &str) -> String {
    serde_json::json!({
        "schema_version": "ferroplan.operation.v1",
        "request_id": request_id.unwrap_or("wasm-adapter-refusal"),
        "capability_id": "fp.wasm",
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

/// A live [`ferroplan::Session`] for the browser. The wrapper owns the current
/// plan and cursor; all untrusted string/JSON boundaries are explicitly bounded.
#[wasm_bindgen]
pub struct WasmSession {
    inner: ferroplan::Session,
    plan: Option<ferroplan::api::Plan>,
    cursor: usize,
}

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = console, js_name = error)]
    fn js_console_error(s: &str);
}

#[wasm_bindgen]
impl WasmSession {
    /// Ground a world. Errors as a JS string.
    #[wasm_bindgen(constructor)]
    pub fn new(domain: &str, problem: &str) -> Result<WasmSession, JsValue> {
        ensure_js_len(
            domain,
            ProductionLimits::default().max_domain_bytes,
            "domain",
        )?;
        ensure_js_len(
            problem,
            ProductionLimits::default().max_problem_bytes,
            "problem",
        )?;
        std::panic::set_hook(Box::new(|info| {
            js_console_error(&format!("wasm panic: {info}"));
        }));
        let opts = Options {
            threads: 1,
            max_evaluated: Some(ProductionLimits::default().max_evaluated),
            ..Default::default()
        };
        Ok(WasmSession {
            inner: ferroplan::Session::new(domain, problem, &opts).map_err(js_err)?,
            plan: None,
            cursor: 0,
        })
    }

    /// Cheap mind: shares the grounded payload, private state.
    pub fn fork(&self) -> WasmSession {
        WasmSession {
            inner: self.inner.fork(),
            plan: None,
            cursor: 0,
        }
    }

    pub fn set_goal(&mut self, goal: &str) -> Result<(), JsValue> {
        ensure_js_len(goal, WASM_TEXT_FIELD_BYTES, "goal")?;
        self.inner.set_goal(goal).map_err(js_err)
    }

    pub fn restrict_prefix_claims(&mut self, prefix: String, claimed: String) {
        let prefix = bounded_string(prefix, 4_096);
        let claimed = bounded_string(claimed, WASM_TEXT_FIELD_BYTES);
        let claimed: std::collections::HashSet<String> = claimed
            .split(',')
            .map(|s| s.trim().to_ascii_uppercase())
            .filter(|s| !s.is_empty())
            .collect();
        self.inner.restrict_ops(move |d| {
            d.starts_with(&prefix)
                && d.split_whitespace()
                    .nth(4)
                    .map(|y| !claimed.contains(y.trim_end_matches(')')))
                    .unwrap_or(true)
        });
    }

    /// Bounded think; stores the plan internally and returns the whole
    /// `Solution` as JSON for display.
    pub fn think(&mut self, evals: usize, mem_mb: usize) -> String {
        if evals == 0 || evals > WASM_MAX_THINK_EVALS {
            return err_json(
                "FP_LIMIT_SEARCH",
                "evals must be within the browser production budget",
            );
        }
        if mem_mb == 0 || mem_mb > WASM_MAX_THINK_MEMORY_MB {
            return err_json(
                "FP_LIMIT_MEMORY",
                "mem_mb must be within the browser production budget",
            );
        }
        let sol = self.inner.replan_budgeted(evals, Some(mem_mb));
        self.plan = if sol.solved { sol.plan.clone() } else { None };
        self.cursor = 0;
        serialize_or_error!(&sol)
    }

    /// Free suffix replay of the stored plan from the cursor.
    pub fn valid(&self) -> bool {
        self.plan
            .as_ref()
            .is_some_and(|p| self.inner.plan_still_valid(p, self.cursor))
    }

    pub fn step_json(&self) -> String {
        match self.plan.as_ref().and_then(|p| p.steps.get(self.cursor)) {
            Some(s) => serde_json::to_string(s).unwrap_or_else(|_| "null".into()),
            None => "null".into(),
        }
    }

    pub fn suffix_json(&self) -> String {
        match self.plan.as_ref() {
            Some(p) => serde_json::to_string(&p.steps[self.cursor.min(p.steps.len())..])
                .unwrap_or_else(|_| "[]".into()),
            None => "[]".into(),
        }
    }

    pub fn advance(&mut self) {
        self.cursor = self.cursor.saturating_add(1);
    }

    pub fn drop_plan(&mut self) {
        self.plan = None;
        self.cursor = 0;
    }

    pub fn has_plan(&self) -> bool {
        self.plan.is_some()
    }

    pub fn set_fact(&mut self, name: &str, value: bool) -> Result<(), JsValue> {
        ensure_js_len(name, 4_096, "fact name")?;
        self.inner.set_fact(name, value).map_err(js_err)
    }

    pub fn set_timed_fact(&mut self, dt: f64, name: &str, value: bool) -> Result<(), JsValue> {
        ensure_js_len(name, 4_096, "fact name")?;
        self.inner.set_timed_fact(dt, name, value).map_err(js_err)
    }

    pub fn observe(&mut self, sight_json: &str) -> Result<String, JsValue> {
        ensure_js_len(sight_json, WASM_JSON_FIELD_BYTES, "observation JSON")?;
        let sight: Vec<(String, bool)> = serde_json::from_str(sight_json).map_err(js_err)?;
        if sight.len() > 100_000 {
            return Err(JsValue::from_str("observation contains too many facts"));
        }
        let refs: Vec<(&str, bool)> = sight.iter().map(|(f, v)| (f.as_str(), *v)).collect();
        let news = self.inner.observe(&refs).map_err(js_err)?;
        serde_json::to_string(&news).map_err(js_err)
    }

    pub fn goal_met(&self) -> bool {
        self.inner.goal_met()
    }

    pub fn fact(&self, name: &str) -> JsValue {
        if name.len() > 4_096 {
            return JsValue::NULL;
        }
        match self.inner.fact(name) {
            Some(v) => JsValue::from_bool(v),
            None => JsValue::NULL,
        }
    }

    pub fn restrict_contains(&mut self, filter: String) {
        let filter = bounded_string(filter, 4_096);
        self.inner.restrict_ops(move |d| d.contains(&filter));
    }

    pub fn apply_start(&mut self, name: &str) -> Result<(), JsValue> {
        ensure_js_len(name, 4_096, "action")?;
        self.inner.apply_start(name).map_err(js_err)
    }

    pub fn elapse(&mut self, dt: f64) -> Result<String, JsValue> {
        let fired = self.inner.elapse(dt).map_err(js_err)?;
        serde_json::to_string(&fired).map_err(js_err)
    }

    pub fn set_fluent(&mut self, name: &str, value: f64) -> Result<(), JsValue> {
        ensure_js_len(name, 4_096, "fluent name")?;
        self.inner.set_fluent(name, value).map_err(js_err)
    }

    pub fn fluent(&self, name: &str) -> JsValue {
        if name.len() > 4_096 {
            return JsValue::NULL;
        }
        match self.inner.fluent(name) {
            Some(v) => JsValue::from_f64(v),
            None => JsValue::NULL,
        }
    }

    pub fn plan_valid_json(&self, plan_json: &str, from: usize) -> bool {
        if plan_json.len() > WASM_JSON_FIELD_BYTES {
            return false;
        }
        serde_json::from_str::<ferroplan::api::Plan>(plan_json)
            .map(|p| self.inner.plan_still_valid(&p, from))
            .unwrap_or(false)
    }

    pub fn world_bytes(&self) -> usize {
        self.inner.world_bytes()
    }

    pub fn mind_bytes(&self) -> usize {
        self.inner.mind_bytes()
    }
}

fn ensure_js_len(value: &str, max_bytes: usize, label: &str) -> Result<(), JsValue> {
    if value.len() > max_bytes {
        Err(JsValue::from_str(&format!(
            "{label} exceeds the {max_bytes}-byte limit"
        )))
    } else {
        Ok(())
    }
}

fn bounded_string(mut value: String, max_bytes: usize) -> String {
    if value.len() <= max_bytes {
        return value;
    }
    let mut end = max_bytes;
    while end > 0 && !value.is_char_boundary(end) {
        end -= 1;
    }
    value.truncate(end);
    value
}

fn js_err(e: impl std::fmt::Display) -> JsValue {
    JsValue::from_str(&e.to_string())
}
