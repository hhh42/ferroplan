//! Persistent repository planning and CMCA allocation over MCP.
//!
//! `ferroplan-mcp` remains the stateless parse/solve/validate authority. This
//! binary owns ground-once repository minds: observe admitted drift, replay the
//! remaining plan, and search only when the suffix no longer stands.
//!
//! Transport: MCP over stdio via `rmcp`/`tokio`. Session storage is a
//! two-level lock: an outer `BTreeMap<String, Arc<AsyncMutex<ManagedSession>>>`
//! guarded briefly to look up/clone a session's own `Arc<AsyncMutex<..>>`,
//! then the per-session mutex is held for the duration of the call. This
//! means concurrent tool calls against *different* sessions never block each
//! other (the outer lock is only held for the lookup), and concurrent calls
//! against the *same* session queue on that session's own mutex rather than
//! racing or erroring — see `KNOWN LIMITATION` below for what queuing does
//! *not* give you. `session_think` runs its CPU-bound synchronous search
//! (`Session::replan_budgeted`/`replan_following`) via
//! `tokio::task::block_in_place` while still holding the per-session lock,
//! which requires the multi-thread tokio runtime (enabled below via
//! `rt-multi-thread` in Cargo.toml and `#[tokio::main]`'s default flavor).

#![forbid(unsafe_code)]

use bcinr_cmca::{
    allocator::{
        allocate, AdaptiveUpdate, AdmittedControlState, CertificateReceipt, CertifiedLearning,
        EnvelopeReceipt, OutcomeReceipt,
    },
    fixed::NonNegativeFixed,
    generated::{
        case_studies::{LensSpec, PackedSemanticState, ETA, F, K, LAMBDA, LENS_REGISTRY, N, Q},
        stability_profile::CERTIFICATE_DIGEST,
    },
};
use ferroplan::{Options, Plan, Session};
use rmcp::handler::server::router::tool::ToolRouter;
use rmcp::handler::server::wrapper::Parameters;
use rmcp::model::{
    CallToolResult, ContentBlock, ErrorData as McpError, ListResourcesResult,
    ReadResourceRequestParams, ReadResourceResult, Resource, ResourceContents,
    ServerCapabilities, ServerInfo,
};
use rmcp::service::RequestContext;
use rmcp::{tool, tool_handler, tool_router, RoleServer, ServerHandler, ServiceExt};
use schemars::JsonSchema;
use serde::de::DeserializeOwned;
use serde::Deserialize;
use serde_json::{json, Map, Value};
use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;
use tokio::sync::Mutex as AsyncMutex;

const BCINR_REVISION: &str = "fb9321d27882169acc83aaca0639b319cd3b7900";
const SESSION_RECEIPT_DOMAIN: &[u8] = b"urn:chatman:ferroplan-session-chain:v1\0";

// Static per-tool semantic descriptions sourced from
// `plugins/chatman-ecosystem/ontology/ferroplan-domain.ttl`'s `rdfs:comment`
// annotations on the `fp:McpTool` instances for this server (the ontology's
// highest-fidelity server per the migration brief). Generated at compile
// time by `build.rs` — see that file for the extraction logic.
include!(concat!(env!("OUT_DIR"), "/session_ontology.rs"));

struct ManagedSession {
    session: Session,
    last_plan: Option<Plan>,
    cursor: usize,
    epoch: u64,
    domain_digest: String,
    problem_digest: String,
    receipt_head: Option<String>,
}

/// Session map: outer lock briefly guards lookup of a session's own
/// `Arc<AsyncMutex<ManagedSession>>`; the per-session lock then serializes
/// all operations (including `session_think`'s search) against that one
/// session without blocking calls against other sessions.
#[derive(Clone, Default)]
struct ServerState {
    sessions: Arc<AsyncMutex<BTreeMap<String, Arc<AsyncMutex<ManagedSession>>>>>,
}

impl ServerState {
    /// Look up a session's own lock, briefly holding the outer map lock to
    /// clone the `Arc`, then release it immediately.
    async fn get(&self, id: &str) -> Result<Arc<AsyncMutex<ManagedSession>>, String> {
        let sessions = self.sessions.lock().await;
        sessions
            .get(id)
            .cloned()
            .ok_or_else(|| format!("unknown session `{id}`"))
    }
}

#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct OpenInput {
    session_id: String,
    domain: String,
    problem: String,
    /// Optional solver Options (same shape as `ferroplan-mcp`'s `solve` tool).
    /// `Options`'s own `Deserialize` impl (with its field-level defaults)
    /// covers the "missing means default" case directly.
    #[serde(default)]
    options: Option<Options>,
    #[serde(default)]
    replace: bool,
}

#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct SessionIdInput {
    session_id: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct FactObservation {
    fact: String,
    value: bool,
}

#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct FluentObservation {
    fluent: String,
    value: f64,
}

#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct ObserveInput {
    session_id: String,
    #[serde(default)]
    facts: Vec<FactObservation>,
    #[serde(default)]
    fluents: Vec<FluentObservation>,
}

#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct GoalInput {
    session_id: String,
    goal: String,
}

fn default_budget() -> usize {
    50_000
}

fn default_follow() -> bool {
    true
}

#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct ThinkInput {
    session_id: String,
    #[serde(default = "default_budget")]
    max_evaluated: usize,
    #[serde(default)]
    memory_mb: Option<usize>,
    #[serde(default = "default_follow")]
    prefer_follow: bool,
}

#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct AdvanceInput {
    session_id: String,
    completed_steps: usize,
}

#[derive(Debug, Deserialize, serde::Serialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct CmcaCandidate {
    id: String,
    #[serde(default)]
    parent: Option<usize>,
    factors: Vec<f64>,
    #[serde(default)]
    cost: f64,
}

#[derive(Debug, Deserialize, serde::Serialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct CmcaInput {
    candidates: Vec<CmcaCandidate>,
}

#[derive(Clone)]
struct FerroplanSession {
    tool_router: ToolRouter<Self>,
    state: ServerState,
}

impl FerroplanSession {
    fn new() -> Self {
        Self {
            tool_router: Self::tool_router(),
            state: ServerState::default(),
        }
    }
}

#[tool_router]
impl FerroplanSession {
    #[tool(description = "Parse and ground one persistent Ferroplan Session.")]
    async fn session_open(
        &self,
        Parameters(input): Parameters<OpenInput>,
    ) -> Result<CallToolResult, McpError> {
        to_result(self.do_session_open(input).await)
    }

    #[tool(
        description = "Apply admitted visible facts and finite fluents; return exact surprises \
            and remaining-plan standing."
    )]
    async fn session_observe(
        &self,
        Parameters(input): Parameters<ObserveInput>,
    ) -> Result<CallToolResult, McpError> {
        to_result(self.do_session_observe(input).await)
    }

    #[tool(description = "Retarget the grounded mind to a ground conjunction without regrounding.")]
    async fn session_set_goal(
        &self,
        Parameters(input): Parameters<GoalInput>,
    ) -> Result<CallToolResult, McpError> {
        to_result(self.do_session_set_goal(input).await)
    }

    #[tool(
        description = "Return a valid prior suffix for free or perform a deterministic bounded \
            prefix-following replan. Runs on a blocking thread so other sessions' tool calls are \
            not stalled while a search runs."
    )]
    async fn session_think(
        &self,
        Parameters(input): Parameters<ThinkInput>,
    ) -> Result<CallToolResult, McpError> {
        to_result(self.do_session_think(input).await)
    }

    #[tool(
        description = "Advance the cursor over completed plan steps; effects still enter through \
            observation."
    )]
    async fn session_advance(
        &self,
        Parameters(input): Parameters<AdvanceInput>,
    ) -> Result<CallToolResult, McpError> {
        to_result(self.do_session_advance(input).await)
    }

    #[tool(
        description = "Inspect epoch, goal, cursor, suffix validity, memory split, and \
            receipt-chain head."
    )]
    async fn session_status(
        &self,
        Parameters(input): Parameters<SessionIdInput>,
    ) -> Result<CallToolResult, McpError> {
        to_result(self.do_session_status(input).await)
    }

    #[tool(description = "Drop a persistent grounded mind.")]
    async fn session_close(
        &self,
        Parameters(input): Parameters<SessionIdInput>,
    ) -> Result<CallToolResult, McpError> {
        to_result(self.do_session_close(input).await)
    }

    #[tool(
        description = "Run the pinned Chatman Multifractal Cascade Allocator over exactly eight \
            admitted nodes and ten factors per node."
    )]
    fn cmca_allocate(
        &self,
        Parameters(input): Parameters<CmcaInput>,
    ) -> Result<CallToolResult, McpError> {
        to_result(tool_cmca_allocate(input))
    }
}

#[tool_handler(router = self.tool_router)]
impl ServerHandler for FerroplanSession {
    fn get_info(&self) -> ServerInfo {
        ServerInfo::new(
            ServerCapabilities::builder()
                .enable_tools()
                .enable_resources()
                .build(),
        )
        .with_server_info(rmcp::model::Implementation::new(
            "ferroplan-session",
            env!("CARGO_PKG_VERSION"),
        ))
        .with_instructions(
            "Open one grounded repository mind, feed admitted observations, retain valid plan \
             suffixes, invoke CMCA before scarce-work selection, and use bounded replanning only \
             after real surprise. Read `ferroplan-session://tools/<name>` resources for \
             ontology-sourced semantics.",
        )
    }

    async fn list_resources(
        &self,
        _request: Option<rmcp::model::PaginatedRequestParams>,
        _context: RequestContext<RoleServer>,
    ) -> Result<ListResourcesResult, McpError> {
        let resources = [
            "session_open",
            "session_observe",
            "session_set_goal",
            "session_think",
            "session_advance",
            "session_status",
            "session_close",
            "cmca_allocate",
        ]
        .into_iter()
        .map(|name| {
            Resource::new(
                format!("ferroplan-session://tools/{name}"),
                format!("{name} (semantic summary)"),
            )
            .with_description(format!(
                "Ontology-sourced semantics for the `{name}` tool, from ferroplan-domain.ttl."
            ))
            .with_mime_type("application/json")
        })
        .collect();
        Ok(ListResourcesResult {
            resources,
            next_cursor: None,
            meta: None,
        })
    }

    async fn read_resource(
        &self,
        request: ReadResourceRequestParams,
        _context: RequestContext<RoleServer>,
    ) -> Result<ReadResourceResult, McpError> {
        let name = request
            .uri
            .strip_prefix("ferroplan-session://tools/")
            .ok_or_else(|| McpError::resource_not_found(request.uri.clone(), None))?;
        let ontology_comment = match name {
            "session_open" => OPEN_ONTOLOGY,
            "session_observe" => OBSERVE_ONTOLOGY,
            "session_set_goal" => SET_GOAL_ONTOLOGY,
            "session_think" => THINK_ONTOLOGY,
            "session_advance" => ADVANCE_ONTOLOGY,
            "session_status" => STATUS_ONTOLOGY,
            "session_close" => CLOSE_ONTOLOGY,
            "cmca_allocate" => CMCA_ONTOLOGY,
            _ => return Err(McpError::resource_not_found(request.uri.clone(), None)),
        };
        let body = json!({
            "tool": name,
            "source": "plugins/chatman-ecosystem/ontology/ferroplan-domain.ttl",
            "rdfs_comment": ontology_comment,
        });
        Ok(ReadResourceResult::new(vec![ResourceContents::text(
            serde_json::to_string_pretty(&body).unwrap_or_default(),
            request.uri,
        )]))
    }
}

fn to_result(result: Result<Value, String>) -> Result<CallToolResult, McpError> {
    Ok(match result {
        Ok(value) => {
            let mut r = CallToolResult::success(vec![ContentBlock::text(pretty(&value))]);
            r.structured_content = Some(value);
            r
        }
        Err(message) => CallToolResult::error(vec![ContentBlock::text(message)]),
    })
}

impl FerroplanSession {
    async fn do_session_open(&self, input: OpenInput) -> Result<Value, String> {
        validate_session_id(&input.session_id)?;
        let options: Options = input.options.unwrap_or_default();

        let mut sessions = self.state.sessions.lock().await;
        if sessions.contains_key(&input.session_id) && !input.replace {
            return Err(format!(
                "session `{}` already exists; set replace=true to discard it",
                input.session_id
            ));
        }

        let domain_digest = digest_bytes(input.domain.as_bytes());
        // (sessions map lock is held across the (cheap, synchronous) grounding
        // below to keep session_open's contains_key/insert atomic; grounding
        // is not the CPU-bound search path this refactor targets.)
        let problem_digest = digest_bytes(input.problem.as_bytes());
        let session = Session::new(&input.domain, &input.problem, &options)?;
        let session_id = input.session_id;
        let mut managed = ManagedSession {
            session,
            last_plan: None,
            cursor: 0,
            epoch: 0,
            domain_digest,
            problem_digest,
            receipt_head: None,
        };
        let event = json!({
            "schema": "urn:chatman:ferroplan-session-event:v1",
            "event": "opened",
            "session_id": &session_id,
            "domain_digest": &managed.domain_digest,
            "problem_digest": &managed.problem_digest,
            "world_bytes": managed.session.world_bytes(),
            "mind_bytes": managed.session.mind_bytes()
        });
        let receipt = chain_receipt(&mut managed, &event)?;
        let response = json!({
            "schema": "urn:chatman:ferroplan-session-open:v1",
            "session_id": &session_id,
            "domain_digest": &managed.domain_digest,
            "problem_digest": &managed.problem_digest,
            "goal_met": managed.session.goal_met(),
            "world_bytes": managed.session.world_bytes(),
            "mind_bytes": managed.session.mind_bytes(),
            "receipt": receipt
        });
        sessions.insert(session_id, Arc::new(AsyncMutex::new(managed)));
        Ok(response)
    }

    async fn do_session_observe(&self, input: ObserveInput) -> Result<Value, String> {
        let session_lock = self.state.get(&input.session_id).await?;
        let mut managed = session_lock.lock().await;
        let managed = &mut *managed;
        let facts: Vec<(&str, bool)> = input
            .facts
            .iter()
            .map(|item| (item.fact.as_str(), item.value))
            .collect();
        let fact_surprises = managed.session.observe(&facts)?;

        let mut fluent_surprises = Vec::new();
        for item in &input.fluents {
            if !item.value.is_finite() {
                return Err(format!("fluent `{}` must be finite", item.fluent));
            }
            let prior = managed.session.fluent(&item.fluent);
            if prior.map(f64::to_bits) != Some(item.value.to_bits()) {
                managed.session.set_fluent(&item.fluent, item.value)?;
                fluent_surprises.push(item.fluent.to_ascii_uppercase());
            }
        }

        if !fact_surprises.is_empty() || !fluent_surprises.is_empty() {
            managed.epoch = managed.epoch.saturating_add(1);
        }
        let remaining_plan_valid = current_plan_valid(managed);
        let event = json!({
            "schema": "urn:chatman:ferroplan-session-event:v1",
            "event": "observed",
            "session_id": &input.session_id,
            "epoch": managed.epoch,
            "fact_surprises": &fact_surprises,
            "fluent_surprises": &fluent_surprises,
            "remaining_plan_valid": remaining_plan_valid
        });
        let receipt = chain_receipt(managed, &event)?;
        Ok(json!({
            "schema": "urn:chatman:ferroplan-observation:v1",
            "session_id": &input.session_id,
            "epoch": managed.epoch,
            "fact_surprises": fact_surprises,
            "fluent_surprises": fluent_surprises,
            "goal_met": managed.session.goal_met(),
            "remaining_plan_valid": remaining_plan_valid,
            "replan_required": remaining_plan_valid != Some(true),
            "receipt": receipt
        }))
    }

    async fn do_session_set_goal(&self, input: GoalInput) -> Result<Value, String> {
        let session_lock = self.state.get(&input.session_id).await?;
        let mut managed = session_lock.lock().await;
        let managed = &mut *managed;
        managed.session.set_goal(&input.goal)?;
        managed.cursor = 0;
        managed.epoch = managed.epoch.saturating_add(1);
        let remaining_plan_valid = current_plan_valid(managed);
        let event = json!({
            "schema": "urn:chatman:ferroplan-session-event:v1",
            "event": "goal-retargeted",
            "session_id": &input.session_id,
            "epoch": managed.epoch,
            "goal": &input.goal,
            "remaining_plan_valid": remaining_plan_valid
        });
        let receipt = chain_receipt(managed, &event)?;
        Ok(json!({
            "schema": "urn:chatman:ferroplan-goal:v1",
            "session_id": &input.session_id,
            "epoch": managed.epoch,
            "goal_met": managed.session.goal_met(),
            "remaining_plan_valid": remaining_plan_valid,
            "receipt": receipt
        }))
    }

    /// Concurrency: the per-session lock (see `ServerState::get`) means a
    /// second `session_think` (or any other) call against the *same*
    /// session_id queues on this session's mutex until this call finishes,
    /// rather than racing a remove/reinsert or seeing "unknown session".
    /// Cancellation `KNOWN LIMITATION` (documented, not silently dropped):
    /// rmcp's `notifications/cancelled` reaches `ServerHandler::on_cancelled`,
    /// but there is no cooperative-cancellation hook inside
    /// `Session::replan_budgeted`/`replan_following` to abort an in-flight
    /// search early — a prior decision explicitly rejected adding one. A
    /// cancelled `session_think` call still runs its search to completion
    /// while holding the per-session lock (queued callers wait it out) and
    /// its result is applied normally rather than being interrupted.
    async fn do_session_think(&self, input: ThinkInput) -> Result<Value, String> {
        if input.max_evaluated == 0 {
            return Err("max_evaluated must be greater than zero".to_owned());
        }

        let session_lock = self.state.get(&input.session_id).await?;
        let mut managed = session_lock.lock().await;
        let managed = &mut *managed;

        // Fast path: if the stored plan suffix is still valid, answer without
        // running a search.
        if current_plan_valid(managed) == Some(true) {
            let plan = managed
                .last_plan
                .clone()
                .ok_or_else(|| "validity reported without a stored plan".to_owned())?;
            let plan_value = serde_json::to_value(&plan).map_err(|error| error.to_string())?;
            let plan_digest = digest_value(&plan_value)?;
            let event = json!({
                "schema": "urn:chatman:ferroplan-session-event:v1",
                "event": "plan-retained",
                "session_id": &input.session_id,
                "epoch": managed.epoch,
                "cursor": managed.cursor,
                "plan_digest": &plan_digest
            });
            let receipt = chain_receipt(managed, &event)?;
            return Ok(json!({
                "schema": "urn:chatman:ferroplan-think:v1",
                "session_id": &input.session_id,
                "decision": "follow",
                "searched": false,
                "cursor": managed.cursor,
                "plan_digest": plan_digest,
                "plan": plan,
                "receipt": receipt
            }));
        }

        // Slow path: run the CPU-bound search in place, while still holding
        // the per-session lock, via `block_in_place` (requires the
        // multi-thread tokio runtime — see Cargo.toml's `rt-multi-thread`
        // feature and this binary's `#[tokio::main]`). Other sessions'
        // calls are unaffected; calls against *this* session queue behind
        // the lock we're holding.
        let max_evaluated = input.max_evaluated;
        let memory_mb = input.memory_mb;
        let prefer_follow = input.prefer_follow;
        let solution = tokio::task::block_in_place(|| {
            let prior = managed.last_plan.clone();
            let solution = match prior.as_ref() {
                Some(plan) if prefer_follow => {
                    managed
                        .session
                        .replan_following(plan, managed.cursor, max_evaluated, memory_mb)
                }
                _ => managed.session.replan_budgeted(max_evaluated, memory_mb),
            };
            managed.cursor = 0;
            managed.last_plan = solution.plan.clone();
            solution
        });

        let solution_value = serde_json::to_value(&solution).map_err(|error| error.to_string())?;
        let plan_digest = solution
            .plan
            .as_ref()
            .map(serde_json::to_value)
            .transpose()
            .map_err(|error| error.to_string())?
            .as_ref()
            .map(digest_value)
            .transpose()?;
        let event = json!({
            "schema": "urn:chatman:ferroplan-session-event:v1",
            "event": "planned",
            "session_id": &input.session_id,
            "epoch": managed.epoch,
            "max_evaluated": max_evaluated,
            "memory_mb": memory_mb,
            "solution_digest": digest_value(&solution_value)?,
            "plan_digest": &plan_digest
        });
        let receipt = chain_receipt(managed, &event)?;
        Ok(json!({
            "schema": "urn:chatman:ferroplan-think:v1",
            "session_id": &input.session_id,
            "decision": if solution.solved { "replan" } else { "bounded-refusal" },
            "searched": true,
            "plan_digest": plan_digest,
            "solution": solution_value,
            "receipt": receipt
        }))
    }

    async fn do_session_advance(&self, input: AdvanceInput) -> Result<Value, String> {
        let session_lock = self.state.get(&input.session_id).await?;
        let mut managed = session_lock.lock().await;
        let managed = &mut *managed;
        let plan_length = managed
            .last_plan
            .as_ref()
            .map_or(0, |plan| plan.steps.len());
        let next = managed.cursor.saturating_add(input.completed_steps);
        if next > plan_length {
            return Err(format!(
                "cursor advance reaches {next}, beyond plan length {plan_length}"
            ));
        }
        managed.cursor = next;
        let remaining_plan_valid = current_plan_valid(managed);
        let event = json!({
            "schema": "urn:chatman:ferroplan-session-event:v1",
            "event": "cursor-advanced",
            "session_id": &input.session_id,
            "epoch": managed.epoch,
            "cursor": managed.cursor,
            "remaining_plan_valid": remaining_plan_valid
        });
        let receipt = chain_receipt(managed, &event)?;
        Ok(json!({
            "schema": "urn:chatman:ferroplan-advance:v1",
            "session_id": &input.session_id,
            "cursor": managed.cursor,
            "plan_length": plan_length,
            "remaining_plan_valid": remaining_plan_valid,
            "receipt": receipt
        }))
    }

    async fn do_session_status(&self, input: SessionIdInput) -> Result<Value, String> {
        let session_lock = self.state.get(&input.session_id).await?;
        let managed = session_lock.lock().await;
        let managed = &*managed;
        Ok(json!({
            "schema": "urn:chatman:ferroplan-session-status:v1",
            "session_id": &input.session_id,
            "epoch": managed.epoch,
            "domain_digest": &managed.domain_digest,
            "problem_digest": &managed.problem_digest,
            "goal_met": managed.session.goal_met(),
            "cursor": managed.cursor,
            "plan_length": managed.last_plan.as_ref().map(|plan| plan.steps.len()),
            "remaining_plan_valid": current_plan_valid(managed),
            "world_bytes": managed.session.world_bytes(),
            "mind_bytes": managed.session.mind_bytes(),
            "receipt_chain_head": &managed.receipt_head
        }))
    }

    async fn do_session_close(&self, input: SessionIdInput) -> Result<Value, String> {
        let mut sessions = self.state.sessions.lock().await;
        // Removes the per-session `Arc<AsyncMutex<ManagedSession>>` from the
        // map. If a search (or any other call) is in-flight and holds the
        // inner lock, that Arc keeps the ManagedSession alive until the
        // in-flight call releases it, but the session is no longer reachable
        // via the map for any *new* caller as soon as this returns — a
        // caller racing a concurrent session_close sees "unknown session"
        // rather than being served by the soon-to-be-orphaned session.
        Ok(json!({
            "schema": "urn:chatman:ferroplan-session-close:v1",
            "session_id": &input.session_id,
            "closed": sessions.remove(&input.session_id).is_some()
        }))
    }
}

fn tool_cmca_allocate(input: CmcaInput) -> Result<Value, String> {
    if input.candidates.len() != N {
        return Err(format!(
            "CMCA requires exactly {N} nodes; received {}",
            input.candidates.len()
        ));
    }

    let mut ids = BTreeSet::new();
    let mut states = [PackedSemanticState {
        id: 0,
        factors: [NonNegativeFixed::ZERO; F],
    }; N];
    let mut parent = [-1_i32; N];
    let mut costs = [NonNegativeFixed::ZERO; N];

    for (index, candidate) in input.candidates.iter().enumerate() {
        let id = candidate.id.trim();
        if id.is_empty() || !ids.insert(id) {
            return Err(format!("candidate {index} has an empty or duplicate id"));
        }
        if candidate.factors.len() != F {
            return Err(format!(
                "candidate `{id}` requires {F} factors; received {}",
                candidate.factors.len()
            ));
        }

        let mut factors = [NonNegativeFixed::ZERO; F];
        for (factor_index, value) in candidate.factors.iter().copied().enumerate() {
            factors[factor_index] = fixed(value, &format!("{id}.factors[{factor_index}]"))?;
        }
        states[index] = PackedSemanticState {
            id: index as u32,
            factors,
        };
        parent[index] = match candidate.parent {
            None => -1,
            Some(parent_index) if parent_index < N && parent_index != index => parent_index as i32,
            Some(parent_index) => {
                return Err(format!("candidate `{id}` has invalid parent {parent_index}"))
            }
        };
        costs[index] = fixed(candidate.cost, &format!("{id}.cost"))?;
    }
    validate_forest(&parent)?;

    let input_value =
        serde_json::to_value(&input).map_err(|e| format!("failed to serialize input: {e}"))?;
    let input_digest = digest_value(&canonicalize(&input_value))?;
    let proof_digest = u64::from_be_bytes(
        blake3::hash(input_digest.as_bytes()).as_bytes()[..8]
            .try_into()
            .map_err(|_| "failed to derive proof digest".to_owned())?,
    );
    let proof = AdaptiveUpdate::admit_adaptive_update(
        AdmittedControlState::admit_control_state(proof_digest),
        CertificateReceipt::admit_certificate(proof_digest),
        EnvelopeReceipt::admit_envelope(proof_digest),
        OutcomeReceipt::admit_outcome(proof_digest),
        NonNegativeFixed::ZERO,
        NonNegativeFixed::ONE,
        CertifiedLearning::admit_learning(),
    );

    let mut weights = [[NonNegativeFixed::ONE; 2 * Q]; N];
    let payoffs = [[NonNegativeFixed::ZERO; 2 * Q]; N];
    let prices = [NonNegativeFixed::ZERO; N];
    let mut last_switch_t = 0;
    let mut previous_mode = 0;
    let allocation = allocate(
        &states,
        &LENS_REGISTRY,
        &LAMBDA,
        ETA,
        &parent,
        &mut weights,
        &payoffs,
        NonNegativeFixed::ZERO,
        NonNegativeFixed::ZERO,
        &prices,
        &costs,
        0,
        &mut last_switch_t,
        &mut previous_mode,
        500,
        CERTIFICATE_DIGEST,
        proof.as_ref(),
    )
    .map_err(|refusal| format!("CMCA refused allocation: {refusal:?}"))?;

    let rows: Vec<Value> = input
        .candidates
        .iter()
        .zip(allocation)
        .map(|(candidate, share)| {
            json!({
                "id": candidate.id,
                "q16_16": share.to_bits(),
                "share": f64::from(share.to_bits()) / 65_536.0
            })
        })
        .collect();
    let payload = canonicalize(&json!({
        "schema": "urn:chatman:cmca-allocation:v1",
        "name": "Chatman Multifractal Cascade Allocator",
        "bcinr_revision": BCINR_REVISION,
        "input_digest": input_digest,
        "node_count": N,
        "factor_count": F,
        "lens_count": Q,
        "measure_count": K,
        "lenses": lens_receipt(&LENS_REGISTRY),
        "allocations": rows
    }));
    Ok(json!({
        "payload_digest": digest_value(&payload)?,
        "payload": payload
    }))
}

fn current_plan_valid(managed: &ManagedSession) -> Option<bool> {
    managed
        .last_plan
        .as_ref()
        .map(|plan| managed.session.plan_still_valid(plan, managed.cursor))
}

fn chain_receipt(managed: &mut ManagedSession, event: &Value) -> Result<String, String> {
    let event_digest = digest_value(&canonicalize(event))?;
    let mut hasher = blake3::Hasher::new();
    hasher.update(SESSION_RECEIPT_DOMAIN);
    update_framed(
        &mut hasher,
        managed.receipt_head.as_deref().unwrap_or("").as_bytes(),
    );
    update_framed(&mut hasher, event_digest.as_bytes());
    let receipt = hasher.finalize().to_hex().to_string();
    managed.receipt_head = Some(receipt.clone());
    Ok(receipt)
}

fn update_framed(hasher: &mut blake3::Hasher, bytes: &[u8]) {
    hasher.update(&(bytes.len() as u64).to_be_bytes());
    hasher.update(bytes);
}

fn validate_forest(parent: &[i32; N]) -> Result<(), String> {
    if !parent.contains(&-1) {
        return Err("CMCA parent relation has no root".to_owned());
    }
    for start in 0..N {
        let mut seen = [false; N];
        let mut current = start as i32;
        for _ in 0..=N {
            if current == -1 {
                break;
            }
            let index = usize::try_from(current)
                .map_err(|_| format!("parent relation contains invalid index {current}"))?;
            if index >= N {
                return Err(format!("parent relation escapes registry at {index}"));
            }
            if seen[index] {
                return Err(format!("parent relation contains a cycle through {index}"));
            }
            seen[index] = true;
            current = parent[index];
        }
    }
    Ok(())
}

fn fixed(value: f64, surface: &str) -> Result<NonNegativeFixed, String> {
    let maximum = f64::from(u32::MAX) / 65_536.0;
    if !value.is_finite() || value < 0.0 || value > maximum {
        return Err(format!("{surface} must be finite and within [0, {maximum}]"));
    }
    Ok(NonNegativeFixed::from_bits(
        (value * 65_536.0).round() as u32,
    ))
}

fn lens_receipt(lenses: &[LensSpec; Q]) -> Vec<Value> {
    lenses
        .iter()
        .map(|lens| json!({"id": lens.id, "q_q16_16": lens.q.val}))
        .collect()
}

fn validate_session_id(id: &str) -> Result<(), String> {
    if id.is_empty()
        || !id.bytes().all(|byte| {
            byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.' | b':')
        })
    {
        return Err(format!("session id is not canonical: `{id}`"));
    }
    Ok(())
}

fn canonicalize(value: &Value) -> Value {
    match value {
        Value::Array(values) => Value::Array(values.iter().map(canonicalize).collect()),
        Value::Object(object) => {
            let mut keys: Vec<&String> = object.keys().collect();
            keys.sort_unstable();
            let mut result = Map::new();
            for key in keys {
                result.insert(key.clone(), canonicalize(&object[key]));
            }
            Value::Object(result)
        }
        _ => value.clone(),
    }
}

fn digest_value(value: &Value) -> Result<String, String> {
    let bytes = serde_json::to_vec(&canonicalize(value)).map_err(|error| error.to_string())?;
    Ok(digest_bytes(&bytes))
}

fn digest_bytes(bytes: &[u8]) -> String {
    blake3::hash(bytes).to_hex().to_string()
}

#[allow(dead_code)]
fn decode<T: DeserializeOwned>(value: &Value) -> Result<T, String> {
    serde_json::from_value(value.clone()).map_err(|error| error.to_string())
}

fn pretty(value: &Value) -> String {
    serde_json::to_string_pretty(value).unwrap_or_else(|_| value.to_string())
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let service = FerroplanSession::new()
        .serve(rmcp::transport::stdio())
        .await
        .map_err(|e| {
            eprintln!("serving error: {e}");
            e
        })?;
    service.waiting().await?;
    Ok(())
}
