//! Persistent Claude Code control-loop MCP server.
//!
//! The ordinary `ferroplan-mcp` server exposes stateless parse/solve/validate
//! operations. This binary exposes the complementary ground-once `Session`
//! surface and a pinned CMCA allocation boundary so an agent can observe a
//! changing repository, preserve a plan while it remains valid, and replan
//! only when admitted observations invalidate the remaining suffix.

#![forbid(unsafe_code)]

use bcinr_cmca::{
    allocator::{
        allocate, AdaptiveUpdate, AdmittedControlState, CertificateReceipt, CertifiedLearning,
        EnvelopeReceipt, OutcomeReceipt,
    },
    fixed::NonNegativeFixed,
    generated::{
        case_studies::{
            LensSpec, PackedSemanticState, ETA, F, K, LAMBDA, LENS_REGISTRY, N, Q,
        },
        stability_profile::CERTIFICATE_DIGEST,
    },
};
use ferroplan::{Options, Plan, Session};
use serde::de::DeserializeOwned;
use serde::Deserialize;
use serde_json::{json, Value};
use std::{
    collections::BTreeMap,
    io::{self, BufRead, Write},
};

const DEFAULT_PROTOCOL_VERSION: &str = "2024-11-05";
const BCINR_REVISION: &str = "fb9321d27882169acc83aaca0639b319cd3b7900";

struct ManagedSession {
    session: Session,
    last_plan: Option<Plan>,
    cursor: usize,
    epoch: u64,
    domain_digest: String,
    problem_digest: String,
    previous_receipt: Option<String>,
}

#[derive(Default)]
struct ServerState {
    sessions: BTreeMap<String, ManagedSession>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct OpenInput {
    session_id: String,
    domain: String,
    problem: String,
    #[serde(default)]
    options: Option<Options>,
    #[serde(default)]
    replace: bool,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct SessionIdInput {
    session_id: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct FactObservation {
    fact: String,
    value: bool,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct FluentObservation {
    fluent: String,
    value: f64,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct ObserveInput {
    session_id: String,
    #[serde(default)]
    facts: Vec<FactObservation>,
    #[serde(default)]
    fluents: Vec<FluentObservation>,
}

#[derive(Debug, Deserialize)]
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

#[derive(Debug, Deserialize)]
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

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct AdvanceInput {
    session_id: String,
    completed_steps: usize,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct CmcaCandidate {
    id: String,
    #[serde(default)]
    parent: Option<usize>,
    factors: Vec<f64>,
    #[serde(default)]
    cost: f64,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct CmcaInput {
    candidates: Vec<CmcaCandidate>,
}

fn main() {
    let stdin = io::stdin();
    let stdout = io::stdout();
    let mut out = stdout.lock();
    let mut state = ServerState::default();

    for line in stdin.lock().lines() {
        let Ok(line) = line else { break };
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        let message: Value = match serde_json::from_str(line) {
            Ok(value) => value,
            Err(error) => {
                send(
                    &mut out,
                    &error_obj(Value::Null, -32700, &format!("parse error: {error}")),
                );
                continue;
            }
        };
        let id = message.get("id").cloned();
        let method = message
            .get("method")
            .and_then(Value::as_str)
            .unwrap_or("");
        let params = message.get("params").cloned().unwrap_or(Value::Null);

        match dispatch(&mut state, method, params) {
            Outcome::Reply(result) => {
                if let Some(id) = id {
                    send(
                        &mut out,
                        &json!({"jsonrpc": "2.0", "id": id, "result": result}),
                    );
                }
            }
            Outcome::Err(code, message) => {
                if let Some(id) = id {
                    send(&mut out, &error_obj(id, code, &message));
                }
            }
            Outcome::Silent => {}
        }
    }
}

enum Outcome {
    Reply(Value),
    Err(i64, String),
    Silent,
}

fn dispatch(state: &mut ServerState, method: &str, params: Value) -> Outcome {
    match method {
        "initialize" => {
            let version = params
                .get("protocolVersion")
                .and_then(Value::as_str)
                .unwrap_or(DEFAULT_PROTOCOL_VERSION);
            Outcome::Reply(json!({
                "protocolVersion": version,
                "capabilities": {"tools": {}},
                "serverInfo": {
                    "name": "ferroplan-session",
                    "version": env!("CARGO_PKG_VERSION")
                },
                "instructions": "Open one grounded repository session, feed it admitted observations, retain the prior plan while its suffix remains valid, invoke CMCA before selecting scarce work, and replan only when the observation receipt proves that the remaining plan broke."
            }))
        }
        "notifications/initialized" | "notifications/cancelled" => Outcome::Silent,
        "ping" => Outcome::Reply(json!({})),
        "tools/list" => Outcome::Reply(json!({"tools": tool_specs()})),
        "tools/call" => call_tool(state, params),
        other => Outcome::Err(-32601, format!("method not found: {other}")),
    }
}

fn tool_specs() -> Value {
    json!([
        {
            "name": "session_open",
            "description": "Parse and ground one persistent Ferroplan Session. Reusing the session makes later repository thinks pay search cost only.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "session_id": {"type": "string"},
                    "domain": {"type": "string"},
                    "problem": {"type": "string"},
                    "options": {"type": "object"},
                    "replace": {"type": "boolean", "default": false}
                },
                "required": ["session_id", "domain", "problem"],
                "additionalProperties": false
            }
        },
        {
            "name": "session_observe",
            "description": "Admit visible repository facts and fluents into a grounded session. Returns exact surprises and whether they invalidate the remaining plan.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "session_id": {"type": "string"},
                    "facts": {"type": "array", "items": {"type": "object", "properties": {"fact": {"type": "string"}, "value": {"type": "boolean"}}, "required": ["fact", "value"], "additionalProperties": false}},
                    "fluents": {"type": "array", "items": {"type": "object", "properties": {"fluent": {"type": "string"}, "value": {"type": "number"}}, "required": ["fluent", "value"], "additionalProperties": false}}
                },
                "required": ["session_id"],
                "additionalProperties": false
            }
        },
        {
            "name": "session_set_goal",
            "description": "Retarget the grounded repository mind to a new ground conjunction without reparsing or regrounding.",
            "inputSchema": {
                "type": "object",
                "properties": {"session_id": {"type": "string"}, "goal": {"type": "string"}},
                "required": ["session_id", "goal"],
                "additionalProperties": false
            }
        },
        {
            "name": "session_think",
            "description": "Return the still-valid prior plan for free, or perform a deterministic bounded rethink. When possible, preserve the prior applicable prefix and search only for a replacement tail.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "session_id": {"type": "string"},
                    "max_evaluated": {"type": "integer", "minimum": 1, "default": 50000},
                    "memory_mb": {"type": "integer", "minimum": 1},
                    "prefer_follow": {"type": "boolean", "default": true}
                },
                "required": ["session_id"],
                "additionalProperties": false
            }
        },
        {
            "name": "session_advance",
            "description": "Advance the cursor over completed plan steps. World effects must still arrive through admitted observations rather than being presumed from intent.",
            "inputSchema": {
                "type": "object",
                "properties": {"session_id": {"type": "string"}, "completed_steps": {"type": "integer", "minimum": 0}},
                "required": ["session_id", "completed_steps"],
                "additionalProperties": false
            }
        },
        {
            "name": "session_status",
            "description": "Inspect the grounded mind, current goal standing, plan cursor, suffix validity, memory split, epoch, and receipt-chain head.",
            "inputSchema": {
                "type": "object",
                "properties": {"session_id": {"type": "string"}},
                "required": ["session_id"],
                "additionalProperties": false
            }
        },
        {
            "name": "session_close",
            "description": "Drop a persistent grounded session and its private belief state.",
            "inputSchema": {
                "type": "object",
                "properties": {"session_id": {"type": "string"}},
                "required": ["session_id"],
                "additionalProperties": false
            }
        },
        {
            "name": "cmca_allocate",
            "description": "Run the pinned Chatman Multifractal Cascade Allocator over exactly eight admitted work nodes. Each node supplies the ten RDF-projected CMCA factors in registry order and an optional parent index and cost.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "candidates": {
                        "type": "array",
                        "minItems": 8,
                        "maxItems": 8,
                        "items": {
                            "type": "object",
                            "properties": {
                                "id": {"type": "string"},
                                "parent": {"type": "integer", "minimum": 0, "maximum": 7},
                                "factors": {"type": "array", "minItems": 10, "maxItems": 10, "items": {"type": "number", "minimum": 0}},
                                "cost": {"type": "number", "minimum": 0, "default": 0}
                            },
                            "required": ["id", "factors"],
                            "additionalProperties": false
                        }
                    }
                },
                "required": ["candidates"],
                "additionalProperties": false
            }
        }
    ])
}

fn call_tool(state: &mut ServerState, params: Value) -> Outcome {
    let name = params.get("name").and_then(Value::as_str).unwrap_or("");
    let args = params.get("arguments").cloned().unwrap_or(Value::Null);
    let result = match name {
        "session_open" => tool_session_open(state, &args),
        "session_observe" => tool_session_observe(state, &args),
        "session_set_goal" => tool_session_set_goal(state, &args),
        "session_think" => tool_session_think(state, &args),
        "session_advance" => tool_session_advance(state, &args),
        "session_status" => tool_session_status(state, &args),
        "session_close" => tool_session_close(state, &args),
        "cmca_allocate" => tool_cmca_allocate(&args),
        other => return Outcome::Err(-32602, format!("unknown tool: {other}")),
    };
    Outcome::Reply(match result {
        Ok(value) => json!({"content": [text_block(&pretty(&value))], "structuredContent": value}),
        Err(message) => json!({"content": [text_block(&message)], "isError": true}),
    })
}

fn tool_session_open(state: &mut ServerState, args: &Value) -> Result<Value, String> {
    let input: OpenInput = decode(args)?;
    validate_session_id(&input.session_id)?;
    if state.sessions.contains_key(&input.session_id) && !input.replace {
        return Err(format!(
            "session `{}` already exists; set replace=true to discard its belief state",
            input.session_id
        ));
    }
    let options = input.options.unwrap_or_default();
    let session = Session::new(&input.domain, &input.problem, &options)?;
    let mut managed = ManagedSession {
        session,
        last_plan: None,
        cursor: 0,
        epoch: 0,
        domain_digest: digest_bytes(input.domain.as_bytes()),
        problem_digest: digest_bytes(input.problem.as_bytes()),
        previous_receipt: None,
    };
    let event = json!({
        "schema": "urn:chatman:ferroplan-session-event:v1",
        "event": "opened",
        "session_id": input.session_id,
        "domain_digest": managed.domain_digest,
        "problem_digest": managed.problem_digest,
        "world_bytes": managed.session.world_bytes(),
        "mind_bytes": managed.session.mind_bytes()
    });
    let receipt = chain_receipt(&mut managed, &event)?;
    let response = json!({
        "schema": "urn:chatman:ferroplan-session-open:v1",
        "session_id": input.session_id,
        "domain_digest": managed.domain_digest,
        "problem_digest": managed.problem_digest,
        "goal_met": managed.session.goal_met(),
        "world_bytes": managed.session.world_bytes(),
        "mind_bytes": managed.session.mind_bytes(),
        "receipt": receipt
    });
    state.sessions.insert(input.session_id, managed);
    Ok(response)
}

fn tool_session_observe(state: &mut ServerState, args: &Value) -> Result<Value, String> {
    let input: ObserveInput = decode(args)?;
    let managed = get_session_mut(state, &input.session_id)?;
    let facts: Vec<(&str, bool)> = input
        .facts
        .iter()
        .map(|observation| (observation.fact.as_str(), observation.value))
        .collect();
    let surprises = managed.session.observe(&facts)?;
    let mut fluent_surprises = Vec::new();
    for observation in &input.fluents {
        if !observation.value.is_finite() {
            return Err(format!(
                "fluent `{}` observation is not finite",
                observation.fluent
            ));
        }
        let prior = managed.session.fluent(&observation.fluent);
        if prior.map(f64::to_bits) != Some(observation.value.to_bits()) {
            managed
                .session
                .set_fluent(&observation.fluent, observation.value)?;
            fluent_surprises.push(observation.fluent.to_ascii_uppercase());
        }
    }
    if !surprises.is_empty() || !fluent_surprises.is_empty() {
        managed.epoch = managed.epoch.saturating_add(1);
    }
    let plan_valid = current_plan_valid(managed);
    let event = json!({
        "schema": "urn:chatman:ferroplan-session-event:v1",
        "event": "observed",
        "session_id": input.session_id,
        "epoch": managed.epoch,
        "fact_surprises": surprises,
        "fluent_surprises": fluent_surprises,
        "remaining_plan_valid": plan_valid
    });
    let receipt = chain_receipt(managed, &event)?;
    Ok(json!({
        "schema": "urn:chatman:ferroplan-observation:v1",
        "session_id": input.session_id,
        "epoch": managed.epoch,
        "fact_surprises": event["fact_surprises"],
        "fluent_surprises": event["fluent_surprises"],
        "goal_met": managed.session.goal_met(),
        "remaining_plan_valid": plan_valid,
        "replan_required": plan_valid != Some(true),
        "receipt": receipt
    }))
}

fn tool_session_set_goal(state: &mut ServerState, args: &Value) -> Result<Value, String> {
    let input: GoalInput = decode(args)?;
    let managed = get_session_mut(state, &input.session_id)?;
    managed.session.set_goal(&input.goal)?;
    managed.cursor = 0;
    managed.epoch = managed.epoch.saturating_add(1);
    let plan_valid = current_plan_valid(managed);
    let event = json!({
        "schema": "urn:chatman:ferroplan-session-event:v1",
        "event": "goal-retargeted",
        "session_id": input.session_id,
        "epoch": managed.epoch,
        "goal": input.goal,
        "remaining_plan_valid": plan_valid
    });
    let receipt = chain_receipt(managed, &event)?;
    Ok(json!({
        "schema": "urn:chatman:ferroplan-goal:v1",
        "session_id": input.session_id,
        "epoch": managed.epoch,
        "goal_met": managed.session.goal_met(),
        "remaining_plan_valid": plan_valid,
        "receipt": receipt
    }))
}

fn tool_session_think(state: &mut ServerState, args: &Value) -> Result<Value, String> {
    let input: ThinkInput = decode(args)?;
    if input.max_evaluated == 0 {
        return Err("max_evaluated must be greater than zero".to_owned());
    }
    let managed = get_session_mut(state, &input.session_id)?;
    if current_plan_valid(managed) == Some(true) {
        let plan = managed.last_plan.as_ref().expect("valid plan exists");
        let event = json!({
            "schema": "urn:chatman:ferroplan-session-event:v1",
            "event": "plan-retained",
            "session_id": input.session_id,
            "epoch": managed.epoch,
            "cursor": managed.cursor,
            "plan_digest": digest_value(&serde_json::to_value(plan).map_err(|e| e.to_string())?)?
        });
        let receipt = chain_receipt(managed, &event)?;
        return Ok(json!({
            "schema": "urn:chatman:ferroplan-think:v1",
            "session_id": input.session_id,
            "decision": "follow",
            "searched": false,
            "cursor": managed.cursor,
            "plan": plan,
            "receipt": receipt
        }));
    }

    let solution = match managed.last_plan.as_ref() {
        Some(prior) if input.prefer_follow => managed.session.replan_following(
            prior,
            managed.cursor,
            input.max_evaluated,
            input.memory_mb,
        ),
        _ => managed
            .session
            .replan_budgeted(input.max_evaluated, input.memory_mb),
    };
    managed.cursor = 0;
    managed.last_plan = solution.plan.clone();
    let solution_value = serde_json::to_value(&solution).map_err(|error| error.to_string())?;
    let event = json!({
        "schema": "urn:chatman:ferroplan-session-event:v1",
        "event": "planned",
        "session_id": input.session_id,
        "epoch": managed.epoch,
        "max_evaluated": input.max_evaluated,
        "memory_mb": input.memory_mb,
        "solution_digest": digest_value(&solution_value)?
    });
    let receipt = chain_receipt(managed, &event)?;
    Ok(json!({
        "schema": "urn:chatman:ferroplan-think:v1",
        "session_id": input.session_id,
        "decision": if solution.solved { "replan" } else { "bounded-refusal" },
        "searched": true,
        "solution": solution_value,
        "receipt": receipt
    }))
}

fn tool_session_advance(state: &mut ServerState, args: &Value) -> Result<Value, String> {
    let input: AdvanceInput = decode(args)?;
    let managed = get_session_mut(state, &input.session_id)?;
    let length = managed
        .last_plan
        .as_ref()
        .map_or(0, |plan| plan.steps.len());
    let next = managed.cursor.saturating_add(input.completed_steps);
    if next > length {
        return Err(format!(
            "cursor advance reaches {next}, beyond admitted plan length {length}"
        ));
    }
    managed.cursor = next;
    let plan_valid = current_plan_valid(managed);
    let event = json!({
        "schema": "urn:chatman:ferroplan-session-event:v1",
        "event": "cursor-advanced",
        "session_id": input.session_id,
        "epoch": managed.epoch,
        "cursor": managed.cursor,
        "remaining_plan_valid": plan_valid
    });
    let receipt = chain_receipt(managed, &event)?;
    Ok(json!({
        "schema": "urn:chatman:ferroplan-advance:v1",
        "session_id": input.session_id,
        "cursor": managed.cursor,
        "plan_length": length,
        "remaining_plan_valid": plan_valid,
        "receipt": receipt
    }))
}

fn tool_session_status(state: &mut ServerState, args: &Value) -> Result<Value, String> {
    let input: SessionIdInput = decode(args)?;
    let managed = get_session(state, &input.session_id)?;
    Ok(json!({
        "schema": "urn:chatman:ferroplan-session-status:v1",
        "session_id": input.session_id,
        "epoch": managed.epoch,
        "domain_digest": managed.domain_digest,
        "problem_digest": managed.problem_digest,
        "goal_met": managed.session.goal_met(),
        "cursor": managed.cursor,
        "plan_length": managed.last_plan.as_ref().map(|plan| plan.steps.len()),
        "remaining_plan_valid": current_plan_valid(managed),
        "world_bytes": managed.session.world_bytes(),
        "mind_bytes": managed.session.mind_bytes(),
        "receipt_chain_head": managed.previous_receipt
    }))
}

fn tool_session_close(state: &mut ServerState, args: &Value) -> Result<Value, String> {
    let input: SessionIdInput = decode(args)?;
    let removed = state.sessions.remove(&input.session_id).is_some();
    Ok(json!({
        "schema": "urn:chatman:ferroplan-session-close:v1",
        "session_id": input.session_id,
        "closed": removed
    }))
}

fn tool_cmca_allocate(args: &Value) -> Result<Value, String> {
    let input: CmcaInput = decode(args)?;
    if input.candidates.len() != N {
        return Err(format!(
            "CMCA v1 requires exactly {N} admitted nodes; received {}",
            input.candidates.len()
        ));
    }
    let mut states = [PackedSemanticState {
        id: 0,
        factors: [NonNegativeFixed::ZERO; F],
    }; N];
    let mut parent = [-1_i32; N];
    let mut costs = [NonNegativeFixed::ZERO; N];
    for (index, candidate) in input.candidates.iter().enumerate() {
        if candidate.id.trim().is_empty() {
            return Err(format!("candidate {index} has an empty id"));
        }
        if candidate.factors.len() != F {
            return Err(format!(
                "candidate `{}` requires exactly {F} factors; received {}",
                candidate.id,
                candidate.factors.len()
            ));
        }
        let mut factors = [NonNegativeFixed::ZERO; F];
        for (factor_index, value) in candidate.factors.iter().copied().enumerate() {
            factors[factor_index] = fixed(value, &format!("{}.factors[{factor_index}]", candidate.id))?;
        }
        states[index] = PackedSemanticState {
            id: index as u32,
            factors,
        };
        parent[index] = match candidate.parent {
            None => -1,
            Some(parent_index) if parent_index < N && parent_index != index => parent_index as i32,
            Some(parent_index) => {
                return Err(format!(
                    "candidate `{}` has invalid parent index {parent_index}",
                    candidate.id
                ))
            }
        };
        costs[index] = fixed(candidate.cost, &format!("{}.cost", candidate.id))?;
    }
    validate_forest(&parent)?;

    let mut weights = [[NonNegativeFixed::ONE; 2 * Q]; N];
    let payoffs = [[NonNegativeFixed::ZERO; 2 * Q]; N];
    let mu = [NonNegativeFixed::ZERO; N];
    let mut last_switch_t = 0;
    let mut previous_mode = 0;
    let proof = AdaptiveUpdate::admit_adaptive_update(
        AdmittedControlState::admit_control_state(0),
        CertificateReceipt::admit_certificate(0),
        EnvelopeReceipt::admit_envelope(0),
        OutcomeReceipt::admit_outcome(0),
        NonNegativeFixed::ZERO,
        NonNegativeFixed::ONE,
        CertifiedLearning::admit_learning(),
    );
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
        &mu,
        &costs,
        0,
        &mut last_switch_t,
        &mut previous_mode,
        500,
        CERTIFICATE_DIGEST,
        proof.as_ref(),
    )
    .map_err(|refusal| format!("CMCA refused allocation: {refusal:?}"))?;

    let allocations: Vec<Value> = input
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
    let payload = json!({
        "schema": "urn:chatman:cmca-allocation:v1",
        "name": "Chatman Multifractal Cascade Allocator",
        "bcinr_revision": BCINR_REVISION,
        "node_count": N,
        "factor_count": F,
        "lens_count": Q,
        "measure_count": K,
        "lenses": lens_receipt(&LENS_REGISTRY),
        "allocations": allocations
    });
    let digest = digest_value(&payload)?;
    Ok(json!({
        "payload": payload,
        "payload_digest": digest
    }))
}

fn lens_receipt(lenses: &[LensSpec; Q]) -> Vec<Value> {
    lenses
        .iter()
        .map(|lens| json!({"id": lens.id, "q_q16_16": lens.q.val}))
        .collect()
}

fn validate_forest(parent: &[i32; N]) -> Result<(), String> {
    if !parent.iter().any(|parent| *parent == -1) {
        return Err("CMCA parent relation has no root".to_owned());
    }
    for start in 0..N {
        let mut seen = [false; N];
        let mut current = start as i32;
        for _ in 0..=N {
            if current == -1 {
                break;
            }
            let index = current as usize;
            if index >= N {
                return Err(format!("parent relation escapes node registry at {index}"));
            }
            if seen[index] {
                return Err(format!("parent relation contains a cycle through node {index}"));
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
        return Err(format!(
            "{surface} must be finite and within [0, {maximum}]"
        ));
    }
    Ok(NonNegativeFixed::from_bits(
        (value * 65_536.0).round() as u32,
    ))
}

fn current_plan_valid(managed: &ManagedSession) -> Option<bool> {
    managed
        .last_plan
        .as_ref()
        .map(|plan| managed.session.plan_still_valid(plan, managed.cursor))
}

fn get_session<'a>(state: &'a ServerState, session_id: &str) -> Result<&'a ManagedSession, String> {
    state
        .sessions
        .get(session_id)
        .ok_or_else(|| format!("unknown session `{session_id}`"))
}

fn get_session_mut<'a>(
    state: &'a mut ServerState,
    session_id: &str,
) -> Result<&'a mut ManagedSession, String> {
    state
        .sessions
        .get_mut(session_id)
        .ok_or_else(|| format!("unknown session `{session_id}`"))
}

fn validate_session_id(session_id: &str) -> Result<(), String> {
    if session_id.is_empty()
        || !session_id.bytes().all(|byte| {
            byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.' | b':')
        })
    {
        return Err(format!("session id is not canonical: `{session_id}`"));
    }
    Ok(())
}

fn chain_receipt(managed: &mut ManagedSession, event: &Value) -> Result<String, String> {
    let event_digest = digest_value(event)?;
    let mut hasher = blake3::Hasher::new();
    hasher.update(b"urn:chatman:ferroplan-session-chain:v1\0");
    if let Some(previous) = &managed.previous_receipt {
        hasher.update(previous.as_bytes());
    }
    hasher.update(&[0]);
    hasher.update(event_digest.as_bytes());
    let receipt = hasher.finalize().to_hex().to_string();
    managed.previous_receipt = Some(receipt.clone());
    Ok(receipt)
}

fn digest_value(value: &Value) -> Result<String, String> {
    let bytes = serde_json::to_vec(value).map_err(|error| error.to_string())?;
    Ok(digest_bytes(&bytes))
}

fn digest_bytes(bytes: &[u8]) -> String {
    blake3::hash(bytes).to_hex().to_string()
}

fn decode<T: DeserializeOwned>(value: &Value) -> Result<T, String> {
    serde_json::from_value(value.clone()).map_err(|error| error.to_string())
}

fn pretty(value: &Value) -> String {
    serde_json::to_string_pretty(value).unwrap_or_else(|_| value.to_string())
}

fn text_block(text: &str) -> Value {
    json!({"type": "text", "text": text})
}

fn error_obj(id: Value, code: i64, message: &str) -> Value {
    json!({"jsonrpc": "2.0", "id": id, "error": {"code": code, "message": message}})
}

fn send(out: &mut impl Write, message: &Value) {
    if writeln!(out, "{message}").is_ok() {
        let _ = out.flush();
    }
}
