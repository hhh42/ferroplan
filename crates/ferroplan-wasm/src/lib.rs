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
}

fn js_err(e: impl std::fmt::Display) -> JsValue {
    JsValue::from_str(&e.to_string())
}
