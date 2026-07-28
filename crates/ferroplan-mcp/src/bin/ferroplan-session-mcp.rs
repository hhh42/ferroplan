//! Persistent repository planning and CMCA allocation over MCP.
//!
//! `ferroplan-mcp` remains the stateless parse/solve/validate authority. This
//! binary owns ground-once repository minds: observe admitted drift, replay the
//! remaining plan, and search only when the suffix no longer stands.

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
use serde::de::DeserializeOwned;
use serde::Deserialize;
use serde_json::{json, Map, Value};
use std::{
    collections::{BTreeMap, BTreeSet},
    io::{self, BufRead, Write},
};

const PROTOCOL_VERSION: &str = "2024-11-05";
const BCINR_REVISION: &str = "fb9321d27882169acc83aaca0639b319cd3b7900";
const SESSION_RECEIPT_DOMAIN: &[u8] = b"urn:chatman:ferroplan-session-chain:v1\0";

struct ManagedSession {
    session: Session,
    last_plan: Option<Plan>,
    cursor: usize,
    epoch: u64,
    domain_digest: String,
    problem_digest: String,
    receipt_head: Option<String>,
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

enum Outcome {
    Reply(Value),
    Error(i64, String),
    Silent,
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
                    &rpc_error(Value::Null, -32700, &format!("parse error: {error}")),
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
            Outcome::Error(code, message) => {
                if let Some(id) = id {
                    send(&mut out, &rpc_error(id, code, &message));
                }
            }
            Outcome::Silent => {}
        }
    }
}

fn dispatch(state: &mut ServerState, method: &str, params: Value) -> Outcome {
    match method {
        "initialize" => Outcome::Reply(json!({
            "protocolVersion": params
                .get("protocolVersion")
                .and_then(Value::as_str)
                .unwrap_or(PROTOCOL_VERSION),
            "capabilities": {"tools": {}},
            "serverInfo": {
                "name": "ferroplan-session",
                "version": env!("CARGO_PKG_VERSION")
            },
            "instructions": "Open one grounded repository mind, feed admitted observations, retain valid plan suffixes, invoke CMCA before scarce-work selection, and use bounded replanning only after real surprise."
        })),
        "notifications/initialized" | "notifications/cancelled" => Outcome::Silent,
        "ping" => Outcome::Reply(json!({})),
        "tools/list" => Outcome::Reply(json!({"tools": tool_specs()})),
        "tools/call" => call_tool(state, params),
        other => Outcome::Error(-32601, format!("method not found: {other}")),
    }
}

fn tool_specs() -> Value {
    json!([
        {
            "name": "session_open",
            "description": "Parse and ground one persistent Ferroplan Session.",
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
            "description": "Apply admitted visible facts and finite fluents; return exact surprises and remaining-plan standing.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "session_id": {"type": "string"},
                    "facts": {"type": "array", "items": {
                        "type": "object",
                        "properties": {"fact": {"type": "string"}, "value": {"type": "boolean"}},
                        "required": ["fact", "value"],
                        "additionalProperties": false
                    }},
                    "fluents": {"type": "array", "items": {
                        "type": "object",
                        "properties": {"fluent": {"type": "string"}, "value": {"type": "number"}},
                        "required": ["fluent", "value"],
                        "additionalProperties": false
                    }}
                },
                "required": ["session_id"],
                "additionalProperties": false
            }
        },
        {
            "name": "session_set_goal",
            "description": "Retarget the grounded mind to a ground conjunction without regrounding.",
            "inputSchema": {
                "type": "object",
                "properties": {"session_id": {"type": "string"}, "goal": {"type": "string"}},
                "required": ["session_id", "goal"],
                "additionalProperties": false
            }
        },
        {
            "name": "session_think",
            "description": "Return a valid prior suffix for free or perform a deterministic bounded prefix-following replan.",
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
            "description": "Advance the cursor over completed plan steps; effects still enter through observation.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "session_id": {"type": "string"},
                    "completed_steps": {"type": "integer", "minimum": 0}
                },
                "required": ["session_id", "completed_steps"],
                "additionalProperties": false
            }
        },
        {
            "name": "session_status",
            "description": "Inspect epoch, goal, cursor, suffix validity, memory split, and receipt-chain head.",
            "inputSchema": {
                "type": "object",
                "properties": {"session_id": {"type": "string"}},
                "required": ["session_id"],
                "additionalProperties": false
            }
        },
        {
            "name": "session_close",
            "description": "Drop a persistent grounded mind.",
            "inputSchema": {
                "type": "object",
                "properties": {"session_id": {"type": "string"}},
                "required": ["session_id"],
                "additionalProperties": false
            }
        },
        {
            "name": "cmca_allocate",
            "description": "Run the pinned Chatman Multifractal Cascade Allocator over exactly eight admitted nodes and ten factors per node.",
            "inputSchema": {
                "type": "object",
                "properties": {"candidates": {
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
                }},
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
        other => return Outcome::Error(-32602, format!("unknown tool: {other}")),
    };

    Outcome::Reply(match result {
        Ok(value) => json!({
            "content": [{"type": "text", "text": pretty(&value)}],
            "structuredContent": value
        }),
        Err(message) => json!({
            "content": [{"type": "text", "text": message}],
            "isError": true
        }),
    })
}

fn tool_session_open(state: &mut ServerState, args: &Value) -> Result<Value, String> {
    let input: OpenInput = decode(args)?;
    validate_session_id(&input.session_id)?;
    if state.sessions.contains_key(&input.session_id) && !input.replace {
        return Err(format!(
            "session `{}` already exists; set replace=true to discard it",
            input.session_id
        ));
    }

    let domain_digest = digest_bytes(input.domain.as_bytes());
    let problem_digest = digest_bytes(input.problem.as_bytes());
    let session = Session::new(
        &input.domain,
        &input.problem,
        &input.options.unwrap_or_default(),
    )?;
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
    state.sessions.insert(session_id, managed);
    Ok(response)
}

fn tool_session_observe(state: &mut ServerState, args: &Value) -> Result<Value, String> {
    let input: ObserveInput = decode(args)?;
    let managed = get_session_mut(state, &input.session_id)?;
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

fn tool_session_set_goal(state: &mut ServerState, args: &Value) -> Result<Value, String> {
    let input: GoalInput = decode(args)?;
    let managed = get_session_mut(state, &input.session_id)?;
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

fn tool_session_think(state: &mut ServerState, args: &Value) -> Result<Value, String> {
    let input: ThinkInput = decode(args)?;
    if input.max_evaluated == 0 {
        return Err("max_evaluated must be greater than zero".to_owned());
    }
    let managed = get_session_mut(state, &input.session_id)?;

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

    let prior = managed.last_plan.clone();
    let solution = match prior.as_ref() {
        Some(plan) if input.prefer_follow => managed.session.replan_following(
            plan,
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
        "max_evaluated": input.max_evaluated,
        "memory_mb": input.memory_mb,
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

fn tool_session_advance(state: &mut ServerState, args: &Value) -> Result<Value, String> {
    let input: AdvanceInput = decode(args)?;
    let managed = get_session_mut(state, &input.session_id)?;
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

fn tool_session_status(state: &ServerState, args: &Value) -> Result<Value, String> {
    let input: SessionIdInput = decode(args)?;
    let managed = get_session(state, &input.session_id)?;
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

fn tool_session_close(state: &mut ServerState, args: &Value) -> Result<Value, String> {
    let input: SessionIdInput = decode(args)?;
    Ok(json!({
        "schema": "urn:chatman:ferroplan-session-close:v1",
        "session_id": &input.session_id,
        "closed": state.sessions.remove(&input.session_id).is_some()
    }))
}

fn tool_cmca_allocate(args: &Value) -> Result<Value, String> {
    let input: CmcaInput = decode(args)?;
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

    let input_digest = digest_value(&canonicalize(args))?;
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
    if !parent.iter().any(|value| *value == -1) {
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

fn get_session<'a>(state: &'a ServerState, id: &str) -> Result<&'a ManagedSession, String> {
    state
        .sessions
        .get(id)
        .ok_or_else(|| format!("unknown session `{id}`"))
}

fn get_session_mut<'a>(
    state: &'a mut ServerState,
    id: &str,
) -> Result<&'a mut ManagedSession, String> {
    state
        .sessions
        .get_mut(id)
        .ok_or_else(|| format!("unknown session `{id}`"))
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

fn decode<T: DeserializeOwned>(value: &Value) -> Result<T, String> {
    serde_json::from_value(value.clone()).map_err(|error| error.to_string())
}

fn pretty(value: &Value) -> String {
    serde_json::to_string_pretty(value).unwrap_or_else(|_| value.to_string())
}

fn rpc_error(id: Value, code: i64, message: &str) -> Value {
    json!({"jsonrpc": "2.0", "id": id, "error": {"code": code, "message": message}})
}

fn send(out: &mut impl Write, message: &Value) {
    if writeln!(out, "{message}").is_ok() {
        let _ = out.flush();
    }
}
