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

/// Plan introspection (0.18 Phase 3): explain a plan for its domain +
/// problem — causal links (classical), invariant spans (temporal),
/// preference breakdown (PDDL3). `plan_json` is the `plan` field of a
/// `Solution` as returned by [`plan`]; the result is an `Explanation`
/// JSON, or `{"error": "..."}`.
#[wasm_bindgen]
pub fn explain(domain: &str, problem: &str, plan_json: &str) -> String {
    let plan: ferroplan::api::Plan = match serde_json::from_str(plan_json) {
        Ok(p) => p,
        Err(e) => return err_json(&format!("plan: {e}")),
    };
    match ferroplan::introspect::explain(domain, problem, &plan) {
        Ok(ex) => {
            serde_json::to_string(&ex).unwrap_or_else(|e| err_json(&format!("serialize: {e}")))
        }
        Err(e) => err_json(&e),
    }
}

fn parse_mode(m: Option<&str>) -> Mode {
    match m.map(|s| s.to_ascii_lowercase()).as_deref() {
        Some("ff") => Mode::Ff,
        Some("pddl3") => Mode::Pddl3,
        Some("partition") => Mode::Partition,
        Some("temporal") => Mode::Temporal,
        Some("optimal") => Mode::Optimal,
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

/// A live [`ferroplan::Session`] for the browser (0.15 Phase 5): the
/// in-page bazaar drives real minds — fork, scope, think, observe —
/// entirely client-side. The wrapper owns the mind's CURRENT PLAN and
/// cursor so the JS loop mirrors the native `bazaar_live` shape: think
/// stores the plan, `valid()` is the free suffix replay, `step_json()` /
/// `advance()` walk it.
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
        std::panic::set_hook(Box::new(|info| {
            js_console_error(&format!("wasm panic: {info}"));
        }));
        let opts = Options {
            threads: 1,
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
        self.inner.set_goal(goal).map_err(js_err)
    }

    /// Actor scoping by op-display prefix — the bazaar's `restrict_ops`
    /// shape (`"TRADE ALICE "`), and additionally masks any op whose
    /// 5th token (the item RECEIVED) is in `claimed` (comma-separated,
    /// empty = no claims): the loop-side claims policy, client-side.
    pub fn restrict_prefix_claims(&mut self, prefix: String, claimed: String) {
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
        let sol = self.inner.replan_budgeted(evals, Some(mem_mb));
        self.plan = if sol.solved { sol.plan.clone() } else { None };
        self.cursor = 0;
        serde_json::to_string(&sol).unwrap_or_else(|e| err_json(&format!("serialize: {e}")))
    }

    /// Free suffix replay of the stored plan from the cursor.
    pub fn valid(&self) -> bool {
        self.plan
            .as_ref()
            .is_some_and(|p| self.inner.plan_still_valid(p, self.cursor))
    }

    /// The current step as JSON (`null` when the plan is drained/absent).
    pub fn step_json(&self) -> String {
        match self.plan.as_ref().and_then(|p| p.steps.get(self.cursor)) {
            Some(s) => serde_json::to_string(s).unwrap_or_else(|_| "null".into()),
            None => "null".into(),
        }
    }

    /// Remaining plan steps as JSON (for claims + display).
    pub fn suffix_json(&self) -> String {
        match self.plan.as_ref() {
            Some(p) => serde_json::to_string(&p.steps[self.cursor.min(p.steps.len())..])
                .unwrap_or_else(|_| "[]".into()),
            None => "[]".into(),
        }
    }

    pub fn advance(&mut self) {
        self.cursor += 1;
    }

    pub fn drop_plan(&mut self) {
        self.plan = None;
        self.cursor = 0;
    }

    pub fn has_plan(&self) -> bool {
        self.plan.is_some()
    }

    pub fn set_fact(&mut self, name: &str, value: bool) -> Result<(), JsValue> {
        self.inner.set_fact(name, value).map_err(js_err)
    }

    /// Schedule an exogenous flip for `dt` from now (temporal sessions,
    /// positive finite `dt`; use `set_fact` for "now"). Every later think
    /// and `valid()` sees it as a scheduled happening — the "market closes
    /// in five" a plan has to beat or wait through.
    pub fn set_timed_fact(&mut self, dt: f64, name: &str, value: bool) -> Result<(), JsValue> {
        self.inner.set_timed_fact(dt, name, value).map_err(js_err)
    }

    /// Observe a JSON batch `[["(has a b)", true], ...]`; returns the
    /// surprises as a JSON string array.
    pub fn observe(&mut self, sight_json: &str) -> Result<String, JsValue> {
        let sight: Vec<(String, bool)> = serde_json::from_str(sight_json).map_err(js_err)?;
        let refs: Vec<(&str, bool)> = sight.iter().map(|(f, v)| (f.as_str(), *v)).collect();
        let news = self.inner.observe(&refs).map_err(js_err)?;
        serde_json::to_string(&news).map_err(js_err)
    }

    pub fn goal_met(&self) -> bool {
        self.inner.goal_met()
    }

    /// Believed value of a fact (`null` if unknown to the grounding).
    pub fn fact(&self, name: &str) -> JsValue {
        match self.inner.fact(name) {
            Some(v) => JsValue::from_bool(v),
            None => JsValue::NULL,
        }
    }

    /// Actor scoping by substring — the village's `restrict_ops` shape
    /// (`" BOB"` keeps only ops whose display mentions the worker).
    pub fn restrict_contains(&mut self, filter: String) {
        self.inner.restrict_ops(move |d| d.contains(&filter));
    }

    /// Dispatch a durative start NOW (`"(ACTION ARGS)"` plan-step form);
    /// its end fires from a later `elapse`. The village loop's dispatch.
    pub fn apply_start(&mut self, name: &str) -> Result<(), JsValue> {
        self.inner.apply_start(name).map_err(js_err)
    }

    /// Advance world time; returns the fired interval ends as a JSON
    /// string array (the tick loop's world events).
    pub fn elapse(&mut self, dt: f64) -> Result<String, JsValue> {
        let fired = self.inner.elapse(dt).map_err(js_err)?;
        serde_json::to_string(&fired).map_err(js_err)
    }

    pub fn set_fluent(&mut self, name: &str, value: f64) -> Result<(), JsValue> {
        self.inner.set_fluent(name, value).map_err(js_err)
    }

    /// Current value of a fluent (`null` if unknown to the grounding).
    pub fn fluent(&self, name: &str) -> JsValue {
        match self.inner.fluent(name) {
            Some(v) => JsValue::from_f64(v),
            None => JsValue::NULL,
        }
    }

    /// Validity of an ARBITRARY plan (JSON `Plan`) from `from`, judged
    /// against THIS session's state and goal — the village's probe shape:
    /// fork the world, set the worker's contract, ask about their plan.
    pub fn plan_valid_json(&self, plan_json: &str, from: usize) -> bool {
        serde_json::from_str::<ferroplan::api::Plan>(plan_json)
            .map(|p| self.inner.plan_still_valid(&p, from))
            .unwrap_or(false)
    }

    /// Estimated retained bytes of the SHARED grounded payload (op
    /// displays, fact/fluent names, packed CSR arrays) — paid once per
    /// world however many minds `fork` it. Flat array/string bytes only:
    /// a floor, not a full audit.
    pub fn world_bytes(&self) -> usize {
        self.inner.world_bytes()
    }

    /// Estimated retained bytes of THIS mind's private state (facts,
    /// fluents, goal, fluent relevance) — what one more `fork` costs.
    /// Same flat-bytes caveat as `world_bytes`.
    pub fn mind_bytes(&self) -> usize {
        self.inner.mind_bytes()
    }
}

fn js_err(e: impl std::fmt::Display) -> JsValue {
    JsValue::from_str(&e.to_string())
}
