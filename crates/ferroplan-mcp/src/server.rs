//! The MCP tool surface: four stateless planning tools and a stateful
//! **session** surface over [`ferroplan::Session`].
//!
//! The stateless four (`solve` / `parse` / `validate` / `decompose`) are the
//! README's bet — the agent authors PDDL, the planner runs deterministically.
//! They keep their pre-rmcp semantics exactly: pretty-printed JSON in a text
//! block, and a tool-level failure comes back as an `isError` result the agent
//! can read and correct, never as a protocol error.
//!
//! The session surface is the one the library always had and the server never
//! exposed. A `Session` is a GROUNDED world plus one mind's private state: you
//! open it once (grounding is the expensive part), then tell it what changed
//! and ask it to rethink, instead of re-sending and re-grounding the whole
//! domain on every step. `session_fork` is the many-minds primitive — the
//! grounded world is `Arc`-shared, so a fork costs one mind's state, not one
//! world's (`session_state` reports both numbers).
//!
//! Concurrency: sessions live behind a `std::sync::Mutex` and the two
//! genuinely expensive calls — `session_open` (grounding) and `session_replan`
//! (search) — run on `spawn_blocking` so a long search cannot stall the
//! runtime. The lock is taken INSIDE the blocking closure and never held
//! across an await.

use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

use rmcp::handler::server::wrapper::Parameters;
use rmcp::model::{CallToolResult, ContentBlock, ErrorData};
use rmcp::{tool, tool_handler, tool_router, ServerHandler};
use schemars::JsonSchema;
use serde::Deserialize;
use serde_json::json;

use ferroplan::{Options, Session};

/// `Session` is `Arc`-based throughout, so a mind can be parked in a shared
/// map. If that ever stops being true this fails to compile HERE, next to the
/// map that depends on it, rather than somewhere confusing downstream.
const _: fn() = || {
    fn assert_send_sync<T: Send + Sync>() {}
    assert_send_sync::<Session>();
};

#[derive(Clone)]
pub struct Ferroplan {
    sessions: Arc<Mutex<HashMap<String, Session>>>,
    next_id: Arc<AtomicU64>,
}

// --- tool parameter types ---------------------------------------------------

#[derive(Debug, Deserialize, JsonSchema)]
pub struct SolveArgs {
    /// PDDL domain source.
    pub domain: String,
    /// PDDL problem source.
    pub problem: String,
    /// Optional solver options; omitted fields use defaults.
    #[serde(default)]
    pub options: Option<Options>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct ParseArgs {
    /// A PDDL domain OR problem source string (auto-detected).
    pub pddl: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct ValidateArgs {
    /// PDDL domain source.
    pub domain: String,
    /// PDDL problem source.
    pub problem: String,
    /// Classical `step N: (action args)` lines, or a temporal
    /// `t: (action args) [dur]` plan.
    pub plan: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct SessionId {
    /// Session handle from `session_open` or `session_fork`.
    pub session_id: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct SessionSetArgs {
    pub session_id: String,
    /// Facts to flip now: `[["(at v1 field)", true], ...]`.
    #[serde(default)]
    pub facts: Vec<(String, bool)>,
    /// Fluents to set now: `[["(grain)", 3.0], ...]`.
    #[serde(default)]
    pub fluents: Vec<(String, f64)>,
    /// Exogenous flips scheduled `dt` from now (temporal sessions):
    /// `[[5.0, "(market-open)", false], ...]` — the "closes in five" a plan
    /// must beat or wait through.
    #[serde(default)]
    pub timed_facts: Vec<(f64, String, bool)>,
    /// Replace the goal, as a PDDL goal expression.
    #[serde(default)]
    pub goal: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct SessionObserveArgs {
    pub session_id: String,
    /// What the mind now sees: `[["(has a b)", true], ...]`. Returns the
    /// SURPRISES — the sightings that contradicted what it believed.
    pub sight: Vec<(String, bool)>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct SessionElapseArgs {
    pub session_id: String,
    /// Advance the world clock by `dt`, firing due timed facts and the ends of
    /// in-flight durative actions. Returns what fired.
    pub dt: f64,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct SessionApplyStartArgs {
    pub session_id: String,
    /// Op display name, e.g. `FIRE-KILN POT1`. For a durative action this
    /// starts the interval; its end fires from `session_elapse`.
    pub action: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct SessionReplanArgs {
    pub session_id: String,
    /// Cap on evaluated nodes — a budgeted-think contract. Omit for the
    /// session's configured behaviour.
    #[serde(default)]
    pub max_evaluated: Option<usize>,
    /// Memory bound in MiB for the budgeted think.
    #[serde(default)]
    pub memory_mb: Option<usize>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct SessionStateArgs {
    pub session_id: String,
    /// Fact display names to read back, e.g. `["(at v1 field)"]`.
    #[serde(default)]
    pub facts: Vec<String>,
    /// Fluent display names to read back, e.g. `["(grain)"]`.
    #[serde(default)]
    pub fluents: Vec<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct SessionOpenArgs {
    /// PDDL domain source.
    pub domain: String,
    /// PDDL problem source.
    pub problem: String,
    /// Optional solver options; omitted fields use defaults.
    #[serde(default)]
    pub options: Option<Options>,
}

// --- helpers ----------------------------------------------------------------

/// Pretty JSON in a text block — the shape the pre-rmcp server returned, kept
/// so existing agent prompts and transcripts still read the same.
fn ok_json<T: serde::Serialize>(v: &T) -> Result<CallToolResult, ErrorData> {
    match serde_json::to_string_pretty(v) {
        Ok(s) => Ok(CallToolResult::success(vec![ContentBlock::text(s)])),
        Err(e) => Ok(fail(format!("could not serialize result: {e}"))),
    }
}

fn ok_text(s: impl Into<String>) -> Result<CallToolResult, ErrorData> {
    Ok(CallToolResult::success(vec![ContentBlock::text(s.into())]))
}

/// A TOOL-level failure: the request routed fine, the tool could not do it.
/// MCP wants this visible to the agent (so it can fix its PDDL), not an opaque
/// protocol error — see `CallToolResult::error` vs `Err(ErrorData)`.
fn fail(msg: impl Into<String>) -> CallToolResult {
    CallToolResult::error(vec![ContentBlock::text(msg.into())])
}

#[tool_router]
impl Ferroplan {
    pub fn new() -> Self {
        Self {
            sessions: Arc::new(Mutex::new(HashMap::new())),
            next_id: Arc::new(AtomicU64::new(1)),
        }
    }

    fn mint_id(&self) -> String {
        format!("s{}", self.next_id.fetch_add(1, Ordering::Relaxed))
    }

    /// Run `f` against one session under the lock. Cheap mutations only —
    /// anything that searches or grounds goes through `spawn_blocking`.
    fn with_session<T>(
        &self,
        id: &str,
        f: impl FnOnce(&mut Session) -> T,
    ) -> Result<T, CallToolResult> {
        let mut guard = match self.sessions.lock() {
            Ok(g) => g,
            Err(p) => p.into_inner(), // a poisoned map is still readable state
        };
        match guard.get_mut(id) {
            Some(s) => Ok(f(s)),
            None => Err(fail(format!(
                "no such session `{id}` — open one with `session_open` \
                 (handles do not survive a server restart)"
            ))),
        }
    }

    // --- the stateless four -------------------------------------------------

    #[tool(
        name = "solve",
        description = "Plan a PDDL domain + problem and return the structured Solution \
            (typed steps, makespan/metric, statistics). Handles STRIPS, typing, ADL, \
            numeric fluents, derived axioms, PDDL3 preferences, and PDDL2.1 temporal \
            durative actions — mode is auto-detected. A solved:false result is a normal \
            answer, not an error."
    )]
    async fn solve(
        &self,
        Parameters(a): Parameters<SolveArgs>,
    ) -> Result<CallToolResult, ErrorData> {
        let opts = a.options.unwrap_or_default();
        let out = tokio::task::spawn_blocking(move || {
            ferroplan::solve(&a.domain, &a.problem, &opts).map_err(|e| e.to_string())
        })
        .await
        .map_err(|e| ErrorData::internal_error(format!("solve task failed: {e}"), None))?;
        match out {
            Ok(sol) => ok_json(&sol),
            Err(e) => Ok(fail(e)),
        }
    }

    #[tool(
        name = "parse",
        description = "Syntax-check a PDDL source string and summarize its structure \
            WITHOUT grounding or solving — fast feedback while authoring. Auto-detects \
            domain vs problem; reports ok/error with a line number, plus name, \
            requirements and counts. Use it to catch mistakes before `solve`."
    )]
    fn parse(&self, Parameters(a): Parameters<ParseArgs>) -> Result<CallToolResult, ErrorData> {
        ok_json(&ferroplan::parse(&a.pddl))
    }

    #[tool(
        name = "validate",
        description = "Independently validate a plan against a domain + problem under \
            ferroplan's own execution semantics (auto-detects classical vs temporal). \
            Reports whether the plan is executable and goal-reaching, with a reason if not."
    )]
    fn validate(
        &self,
        Parameters(a): Parameters<ValidateArgs>,
    ) -> Result<CallToolResult, ErrorData> {
        match ferroplan::plan::validate_plan(&a.domain, &a.problem, &a.plan) {
            Ok(ferroplan::plan::Validity::Valid) => ok_text("Plan valid"),
            Ok(ferroplan::plan::Validity::Invalid(why)) => ok_text(format!("Plan invalid: {why}")),
            Err(e) => Ok(fail(e)),
        }
    }

    #[tool(
        name = "decompose",
        description = "Decompose a temporal goal too big for one-shot search into ordered, \
            individually-solved contracts stitched into one validated plan. Returns each \
            contract's sub-goal, sub-plan and timeline offset, plus the stitched plan. A \
            goal that cannot be split falls back to one monolithic contract, reported honestly."
    )]
    async fn decompose(
        &self,
        Parameters(a): Parameters<SolveArgs>,
    ) -> Result<CallToolResult, ErrorData> {
        let opts = a.options.unwrap_or_default();
        let out = tokio::task::spawn_blocking(move || {
            ferroplan::decompose(&a.domain, &a.problem, &opts).map_err(|e| e.to_string())
        })
        .await
        .map_err(|e| ErrorData::internal_error(format!("decompose task failed: {e}"), None))?;
        match out {
            Ok(d) => ok_json(&d),
            Err(e) => Ok(fail(e)),
        }
    }

    // --- the session surface ------------------------------------------------

    #[tool(
        name = "session_open",
        description = "Ground a domain + problem ONCE and keep it open as a live session, \
            returning a session_id. Thereafter tell the session what changed \
            (session_set / session_observe / session_elapse / session_apply_start) and ask \
            it to rethink (session_replan) — no re-sending or re-grounding the domain per \
            step. This is the tool to use for a running world; `solve` is for one-shot questions."
    )]
    async fn session_open(
        &self,
        Parameters(a): Parameters<SessionOpenArgs>,
    ) -> Result<CallToolResult, ErrorData> {
        let opts = a.options.unwrap_or_default();
        let built = tokio::task::spawn_blocking(move || Session::new(&a.domain, &a.problem, &opts))
            .await
            .map_err(|e| {
                ErrorData::internal_error(format!("session_open task failed: {e}"), None)
            })?;
        let session = match built {
            Ok(s) => s,
            Err(e) => return Ok(fail(e)),
        };
        let id = self.mint_id();
        let (goal_met, world_bytes, mind_bytes) = (
            session.goal_met(),
            session.world_bytes(),
            session.mind_bytes(),
        );
        match self.sessions.lock() {
            Ok(mut g) => {
                g.insert(id.clone(), session);
            }
            Err(p) => {
                p.into_inner().insert(id.clone(), session);
            }
        }
        ok_json(&json!({
            "session_id": id,
            "goal_met": goal_met,
            "world_bytes": world_bytes,
            "mind_bytes": mind_bytes,
        }))
    }

    #[tool(
        name = "session_close",
        description = "Close a session and free its state. Idempotent-ish: closing an \
            unknown handle is reported, not fatal."
    )]
    fn session_close(
        &self,
        Parameters(a): Parameters<SessionId>,
    ) -> Result<CallToolResult, ErrorData> {
        let removed = match self.sessions.lock() {
            Ok(mut g) => g.remove(&a.session_id).is_some(),
            Err(p) => p.into_inner().remove(&a.session_id).is_some(),
        };
        if removed {
            ok_text(format!("closed {}", a.session_id))
        } else {
            Ok(fail(format!("no such session `{}`", a.session_id)))
        }
    }

    #[tool(
        name = "session_list",
        description = "List open sessions with their goal status and memory split \
            (world_bytes is the SHARED grounded payload; mind_bytes is one mind's private \
            state — what one more fork costs)."
    )]
    fn session_list(&self) -> Result<CallToolResult, ErrorData> {
        let guard = match self.sessions.lock() {
            Ok(g) => g,
            Err(p) => p.into_inner(),
        };
        let mut rows: Vec<_> = guard
            .iter()
            .map(|(id, s)| {
                json!({
                    "session_id": id,
                    "goal_met": s.goal_met(),
                    "world_bytes": s.world_bytes(),
                    "mind_bytes": s.mind_bytes(),
                })
            })
            .collect();
        rows.sort_by_key(|v| v["session_id"].as_str().unwrap_or("").to_string());
        ok_json(&json!({ "sessions": rows }))
    }

    #[tool(
        name = "session_fork",
        description = "Fork a session into a second, independent mind sharing the same \
            grounded world. This is the many-minds primitive: the world is shared, so a \
            fork costs mind_bytes, not world_bytes. Use it to give each actor its own \
            beliefs and goal over one simulation."
    )]
    fn session_fork(
        &self,
        Parameters(a): Parameters<SessionId>,
    ) -> Result<CallToolResult, ErrorData> {
        let forked = match self.with_session(&a.session_id, |s| s.fork()) {
            Ok(f) => f,
            Err(e) => return Ok(e),
        };
        let id = self.mint_id();
        let mind_bytes = forked.mind_bytes();
        match self.sessions.lock() {
            Ok(mut g) => {
                g.insert(id.clone(), forked);
            }
            Err(p) => {
                p.into_inner().insert(id.clone(), forked);
            }
        }
        ok_json(&json!({
            "session_id": id, "forked_from": a.session_id, "mind_bytes": mind_bytes
        }))
    }

    #[tool(
        name = "session_set",
        description = "Edit a session's world: flip facts, set fluents, schedule exogenous \
            timed facts, and/or replace the goal — in one call. Applies in that order and \
            stops at the first rejection, reporting what was applied. Static facts and \
            compiler-minted tokens are fenced and will be refused by name."
    )]
    fn session_set(
        &self,
        Parameters(a): Parameters<SessionSetArgs>,
    ) -> Result<CallToolResult, ErrorData> {
        let SessionSetArgs {
            session_id,
            facts,
            fluents,
            timed_facts,
            goal,
        } = a;
        let outcome = self.with_session(&session_id, move |s| {
            let mut applied = json!({
                "facts": 0, "fluents": 0, "timed_facts": 0, "goal": false
            });
            for (n, v) in &facts {
                if let Err(e) = s.set_fact(n, *v) {
                    return Err((format!("set_fact {n}: {e}"), applied));
                }
                applied["facts"] = json!(applied["facts"].as_u64().unwrap_or(0) + 1);
            }
            for (n, v) in &fluents {
                if let Err(e) = s.set_fluent(n, *v) {
                    return Err((format!("set_fluent {n}: {e}"), applied));
                }
                applied["fluents"] = json!(applied["fluents"].as_u64().unwrap_or(0) + 1);
            }
            for (dt, n, v) in &timed_facts {
                if let Err(e) = s.set_timed_fact(*dt, n, *v) {
                    return Err((format!("set_timed_fact {n}: {e}"), applied));
                }
                applied["timed_facts"] = json!(applied["timed_facts"].as_u64().unwrap_or(0) + 1);
            }
            if let Some(g) = &goal {
                if let Err(e) = s.set_goal(g) {
                    return Err((format!("set_goal: {e}"), applied));
                }
                applied["goal"] = json!(true);
            }
            applied["goal_met"] = json!(s.goal_met());
            Ok(applied)
        });
        match outcome {
            Err(e) => Ok(e),
            Ok(Ok(applied)) => ok_json(&applied),
            Ok(Err((why, applied))) => Ok(fail(format!(
                "{why}\napplied before the failure: {applied}"
            ))),
        }
    }

    #[tool(
        name = "session_observe",
        description = "Tell a session what it now sees — `[[\"(has a b)\", true], ...]` — \
            and get back the SURPRISES: the sightings that contradicted what it believed. \
            An empty surprise list means the world matched its model."
    )]
    fn session_observe(
        &self,
        Parameters(a): Parameters<SessionObserveArgs>,
    ) -> Result<CallToolResult, ErrorData> {
        let sight = a.sight;
        let outcome = self.with_session(&a.session_id, move |s| {
            let borrowed: Vec<(&str, bool)> = sight.iter().map(|(n, v)| (n.as_str(), *v)).collect();
            s.observe(&borrowed)
                .map(|sur| json!({ "surprises": sur, "goal_met": s.goal_met() }))
        });
        match outcome {
            Err(e) => Ok(e),
            Ok(Ok(v)) => ok_json(&v),
            Ok(Err(e)) => Ok(fail(e)),
        }
    }

    #[tool(
        name = "session_elapse",
        description = "Advance a session's clock by dt: fires due scheduled facts and the \
            ends of in-flight durative actions, and returns what fired. Temporal sessions \
            only — this is how a world moves between thinks."
    )]
    fn session_elapse(
        &self,
        Parameters(a): Parameters<SessionElapseArgs>,
    ) -> Result<CallToolResult, ErrorData> {
        let dt = a.dt;
        let outcome = self.with_session(&a.session_id, move |s| {
            s.elapse(dt)
                .map(|ev| json!({ "fired": ev, "goal_met": s.goal_met() }))
        });
        match outcome {
            Err(e) => Ok(e),
            Ok(Ok(v)) => ok_json(&v),
            Ok(Err(e)) => Ok(fail(e)),
        }
    }

    #[tool(
        name = "session_apply_start",
        description = "Commit a session to STARTING an action (op display name, e.g. \
            `FIRE-KILN POT1`). For a durative action this puts the interval in flight — the \
            mind can rethink while it runs, and its end fires from session_elapse."
    )]
    fn session_apply_start(
        &self,
        Parameters(a): Parameters<SessionApplyStartArgs>,
    ) -> Result<CallToolResult, ErrorData> {
        let action = a.action;
        let outcome = self.with_session(&a.session_id, move |s| {
            s.apply_start(&action)
                .map(|()| json!({ "started": action, "goal_met": s.goal_met() }))
        });
        match outcome {
            Err(e) => Ok(e),
            Ok(Ok(v)) => ok_json(&v),
            Ok(Err(e)) => Ok(fail(e)),
        }
    }

    #[tool(
        name = "session_replan",
        description = "Rethink from the session's CURRENT state and return the structured \
            Solution. Optionally budgeted (max_evaluated / memory_mb) — a budgeted think is \
            a contract: it returns what it found within the budget rather than running to \
            exhaustion. solved:false is a normal answer."
    )]
    async fn session_replan(
        &self,
        Parameters(a): Parameters<SessionReplanArgs>,
    ) -> Result<CallToolResult, ErrorData> {
        let sessions = self.sessions.clone();
        let id = a.session_id.clone();
        let (cap, mem) = (a.max_evaluated, a.memory_mb);
        // Search is the long pole: fork the mind, drop the lock, think off the
        // runtime. A replan must never hold the map while it runs, or one deep
        // search freezes every other session on the server.
        let mind = tokio::task::spawn_blocking(move || {
            let guard = match sessions.lock() {
                Ok(g) => g,
                Err(p) => p.into_inner(),
            };
            guard.get(&id).map(|s| s.fork())
        })
        .await
        .map_err(|e| ErrorData::internal_error(format!("replan task failed: {e}"), None))?;
        let Some(mind) = mind else {
            return Ok(fail(format!("no such session `{}`", a.session_id)));
        };
        let sol = tokio::task::spawn_blocking(move || match cap {
            Some(c) => mind.replan_budgeted(c, mem),
            None => mind.replan(),
        })
        .await
        .map_err(|e| ErrorData::internal_error(format!("replan task failed: {e}"), None))?;
        ok_json(&sol)
    }

    #[tool(
        name = "session_state",
        description = "Read a session back: goal_met, the memory split, and the current \
            value of any facts/fluents you name. Use it to check what the mind believes \
            before deciding what to tell it."
    )]
    fn session_state(
        &self,
        Parameters(a): Parameters<SessionStateArgs>,
    ) -> Result<CallToolResult, ErrorData> {
        let (want_f, want_n) = (a.facts, a.fluents);
        let outcome = self.with_session(&a.session_id, move |s| {
            let facts: serde_json::Map<String, serde_json::Value> = want_f
                .iter()
                .map(|n| (n.clone(), json!(s.fact(n))))
                .collect();
            let fluents: serde_json::Map<String, serde_json::Value> = want_n
                .iter()
                .map(|n| (n.clone(), json!(s.fluent(n))))
                .collect();
            json!({
                "goal_met": s.goal_met(),
                "world_bytes": s.world_bytes(),
                "mind_bytes": s.mind_bytes(),
                "facts": facts,
                "fluents": fluents,
            })
        });
        match outcome {
            Err(e) => Ok(e),
            Ok(v) => ok_json(&v),
        }
    }
}

impl Default for Ferroplan {
    fn default() -> Self {
        Self::new()
    }
}

#[tool_handler(
    name = "ferroplan",
    instructions = "Two ways to use this planner. For a one-shot question, author a PDDL \
domain + problem and call `solve` (or `decompose` when the goal is too big for one \
search); `parse` syntax-checks while you author and `validate` independently checks a \
plan. For a RUNNING world, call `session_open` once to ground it, then loop: tell the \
session what changed (`session_set`, `session_observe`, `session_elapse`, \
`session_apply_start`) and call `session_replan`. `session_fork` gives another actor its \
own mind over the same shared grounded world."
)]
impl ServerHandler for Ferroplan {}
