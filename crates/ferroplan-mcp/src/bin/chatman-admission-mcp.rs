//! Independent receipt admission for the Claude Code self-hosting loop.
//!
//! Ferroplan establishes candidate-plan and suffix-validity claims. BCINR/CMCA
//! establishes bounded allocation claims. This server binds their exact inputs
//! and outputs, the hook observation frontier, independent validation evidence,
//! and a predecessor receipt into canonical BLAKE3 envelopes.

#![forbid(unsafe_code)]

use serde::de::DeserializeOwned;
use serde::Deserialize;
use serde_json::{json, Map, Value};
use std::io::{self, BufRead, Write};

const DEFAULT_PROTOCOL_VERSION: &str = "2024-11-05";
const BCINR_REVISION: &str = "fb9321d27882169acc83aaca0639b319cd3b7900";
const RECEIPT_DOMAIN: &[u8] = b"urn:chatman:claude-code-admission:v1\0";

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct BindAllocationInput {
    candidates: Value,
    allocation_result: Value,
    observation_frontier: Value,
    #[serde(default)]
    previous_receipt: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct BindPlanInput {
    session_think: Value,
    allocation_receipt: String,
    observation_frontier: Value,
    validator_result: Value,
    #[serde(default)]
    previous_receipt: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct VerifyInput {
    envelope: Value,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct DigestInput {
    value: Value,
}

fn main() {
    let stdin = io::stdin();
    let stdout = io::stdout();
    let mut out = stdout.lock();

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

        match dispatch(method, params) {
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

fn dispatch(method: &str, params: Value) -> Outcome {
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
                    "name": "chatman-admission",
                    "version": env!("CARGO_PKG_VERSION")
                },
                "instructions": "Bind canonical observation, allocation, plan, validation, and predecessor commitments. This server admits evidence; it does not plan or actuate."
            }))
        }
        "notifications/initialized" | "notifications/cancelled" => Outcome::Silent,
        "ping" => Outcome::Reply(json!({})),
        "tools/list" => Outcome::Reply(json!({"tools": tool_specs()})),
        "tools/call" => call_tool(params),
        other => Outcome::Err(-32601, format!("method not found: {other}")),
    }
}

fn tool_specs() -> Value {
    json!([
        {
            "name": "canonical_digest",
            "description": "Compute a BLAKE3 digest over recursively key-sorted canonical JSON.",
            "inputSchema": {
                "type": "object",
                "properties": {"value": {}},
                "required": ["value"],
                "additionalProperties": false
            }
        },
        {
            "name": "bind_allocation_receipt",
            "description": "Bind the exact eight CMCA candidates, allocation result, observation frontier, BCINR revision, and predecessor into a replayable receipt envelope.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "candidates": {"type": "array", "minItems": 8, "maxItems": 8},
                    "allocation_result": {"type": "object"},
                    "observation_frontier": {"type": "object"},
                    "previous_receipt": {"type": "string", "pattern": "^[0-9a-f]{64}$"}
                },
                "required": ["candidates", "allocation_result", "observation_frontier"],
                "additionalProperties": false
            }
        },
        {
            "name": "bind_plan_receipt",
            "description": "Bind a solved persistent-session plan, allocation receipt, exact observation frontier, independent validator result, and predecessor receipt.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "session_think": {"type": "object"},
                    "allocation_receipt": {"type": "string", "pattern": "^[0-9a-f]{64}$"},
                    "observation_frontier": {"type": "object"},
                    "validator_result": {"type": "object"},
                    "previous_receipt": {"type": "string", "pattern": "^[0-9a-f]{64}$"}
                },
                "required": ["session_think", "allocation_receipt", "observation_frontier", "validator_result"],
                "additionalProperties": false
            }
        },
        {
            "name": "verify_receipt",
            "description": "Recompute and verify a Chatman admission envelope without trusting its declared receipt or payload digest.",
            "inputSchema": {
                "type": "object",
                "properties": {"envelope": {"type": "object"}},
                "required": ["envelope"],
                "additionalProperties": false
            }
        }
    ])
}

fn call_tool(params: Value) -> Outcome {
    let name = params.get("name").and_then(Value::as_str).unwrap_or("");
    let args = params.get("arguments").cloned().unwrap_or(Value::Null);
    let result = match name {
        "canonical_digest" => tool_canonical_digest(&args),
        "bind_allocation_receipt" => tool_bind_allocation(&args),
        "bind_plan_receipt" => tool_bind_plan(&args),
        "verify_receipt" => tool_verify(&args),
        other => return Outcome::Err(-32602, format!("unknown tool: {other}")),
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

fn tool_canonical_digest(args: &Value) -> Result<Value, String> {
    let input: DigestInput = decode(args)?;
    let canonical = canonicalize(&input.value);
    Ok(json!({
        "schema": "urn:chatman:canonical-digest:v1",
        "algorithm": "BLAKE3",
        "digest": digest_value(&canonical)?,
        "canonical": canonical
    }))
}

fn tool_bind_allocation(args: &Value) -> Result<Value, String> {
    let input: BindAllocationInput = decode(args)?;
    validate_receipt(input.previous_receipt.as_deref(), "previous_receipt")?;

    let candidates = canonicalize(&input.candidates);
    let candidate_rows = candidates
        .as_array()
        .ok_or_else(|| "candidates must be an array".to_owned())?;
    if candidate_rows.len() != 8 {
        return Err(format!(
            "CMCA allocation admission requires exactly eight candidates; received {}",
            candidate_rows.len()
        ));
    }

    let allocation_result = canonicalize(&input.allocation_result);
    let revision = allocation_result
        .pointer("/payload/bcinr_revision")
        .and_then(Value::as_str)
        .ok_or_else(|| "allocation_result lacks payload.bcinr_revision".to_owned())?;
    if revision != BCINR_REVISION {
        return Err(format!(
            "allocation_result BCINR revision `{revision}` does not match admitted `{BCINR_REVISION}`"
        ));
    }
    let allocations = allocation_result
        .pointer("/payload/allocations")
        .and_then(Value::as_array)
        .ok_or_else(|| "allocation_result lacks payload.allocations".to_owned())?;
    if allocations.len() != 8 {
        return Err(format!(
            "allocation_result requires eight allocation rows; received {}",
            allocations.len()
        ));
    }

    let observation_frontier = canonicalize(&input.observation_frontier);
    let candidates_digest = digest_value(&candidates)?;
    let allocation_result_digest = digest_value(&allocation_result)?;
    let observation_frontier_digest = digest_value(&observation_frontier)?;
    let payload = canonicalize(&json!({
        "schema": "urn:chatman:allocation-admission-payload:v1",
        "bcinr_revision": BCINR_REVISION,
        "candidates": candidates,
        "candidates_digest": candidates_digest,
        "allocation_result": allocation_result,
        "allocation_result_digest": allocation_result_digest,
        "observation_frontier": observation_frontier,
        "observation_frontier_digest": observation_frontier_digest
    }));
    make_envelope("allocation", payload, input.previous_receipt)
}

fn tool_bind_plan(args: &Value) -> Result<Value, String> {
    let input: BindPlanInput = decode(args)?;
    validate_receipt(Some(&input.allocation_receipt), "allocation_receipt")?;
    validate_receipt(input.previous_receipt.as_deref(), "previous_receipt")?;

    let session_think = canonicalize(&input.session_think);
    let plan = session_think
        .get("plan")
        .filter(|value| !value.is_null())
        .or_else(|| {
            session_think
                .pointer("/solution/plan")
                .filter(|value| !value.is_null())
        })
        .ok_or_else(|| "session_think does not contain a solved plan".to_owned())?;
    let session_receipt = session_think
        .get("receipt")
        .and_then(Value::as_str)
        .ok_or_else(|| "session_think lacks a receipt".to_owned())?;
    validate_receipt(Some(session_receipt), "session_think.receipt")?;

    let validator_result = canonicalize(&input.validator_result);
    let validator_valid = validator_result
        .get("valid")
        .and_then(Value::as_bool)
        .or_else(|| validator_result.get("ok").and_then(Value::as_bool))
        .ok_or_else(|| "validator_result must declare boolean `valid` or `ok`".to_owned())?;
    if !validator_valid {
        return Err("independent validator did not admit the candidate plan".to_owned());
    }

    let canonical_plan = canonicalize(plan);
    let observation_frontier = canonicalize(&input.observation_frontier);
    let plan_digest = digest_value(&canonical_plan)?;
    let observation_frontier_digest = digest_value(&observation_frontier)?;
    let validator_result_digest = digest_value(&validator_result)?;
    let payload = canonicalize(&json!({
        "schema": "urn:chatman:plan-admission-payload:v1",
        "session_receipt": session_receipt,
        "session_think": session_think,
        "plan": canonical_plan,
        "plan_digest": plan_digest,
        "allocation_receipt": input.allocation_receipt,
        "observation_frontier": observation_frontier,
        "observation_frontier_digest": observation_frontier_digest,
        "validator_result": validator_result,
        "validator_result_digest": validator_result_digest
    }));
    make_envelope("plan", payload, input.previous_receipt)
}

fn tool_verify(args: &Value) -> Result<Value, String> {
    let input: VerifyInput = decode(args)?;
    let object = input
        .envelope
        .as_object()
        .ok_or_else(|| "envelope must be an object".to_owned())?;
    let kind = object
        .get("kind")
        .and_then(Value::as_str)
        .ok_or_else(|| "envelope lacks kind".to_owned())?;
    let payload = canonicalize(
        object
            .get("payload")
            .ok_or_else(|| "envelope lacks payload".to_owned())?,
    );
    let previous_receipt = object
        .get("previous_receipt")
        .and_then(Value::as_str)
        .map(str::to_owned);
    validate_receipt(previous_receipt.as_deref(), "previous_receipt")?;
    let declared_receipt = object
        .get("receipt")
        .and_then(Value::as_str)
        .ok_or_else(|| "envelope lacks receipt".to_owned())?;
    validate_receipt(Some(declared_receipt), "receipt")?;
    let declared_payload_digest = object
        .get("payload_digest")
        .and_then(Value::as_str)
        .ok_or_else(|| "envelope lacks payload_digest".to_owned())?;
    validate_receipt(Some(declared_payload_digest), "payload_digest")?;

    let expected_payload_digest = digest_value(&payload)?;
    let expected_receipt = receipt_for(kind, &payload, previous_receipt.as_deref())?;
    let receipt_valid = expected_receipt == declared_receipt;
    let payload_digest_valid = expected_payload_digest == declared_payload_digest;
    Ok(json!({
        "schema": "urn:chatman:receipt-verification:v1",
        "valid": receipt_valid && payload_digest_valid,
        "receipt_valid": receipt_valid,
        "payload_digest_valid": payload_digest_valid,
        "declared_receipt": declared_receipt,
        "expected_receipt": expected_receipt,
        "declared_payload_digest": declared_payload_digest,
        "expected_payload_digest": expected_payload_digest,
        "kind": kind
    }))
}

fn make_envelope(
    kind: &str,
    payload: Value,
    previous_receipt: Option<String>,
) -> Result<Value, String> {
    let payload = canonicalize(&payload);
    let payload_digest = digest_value(&payload)?;
    let receipt = receipt_for(kind, &payload, previous_receipt.as_deref())?;
    Ok(json!({
        "schema": "urn:chatman:admission-envelope:v1",
        "kind": kind,
        "algorithm": "BLAKE3",
        "payload_digest": payload_digest,
        "payload": payload,
        "previous_receipt": previous_receipt,
        "receipt": receipt
    }))
}

fn receipt_for(kind: &str, payload: &Value, previous: Option<&str>) -> Result<String, String> {
    let bytes = canonical_bytes(payload)?;
    let mut hasher = blake3::Hasher::new();
    hasher.update(RECEIPT_DOMAIN);
    update_framed(&mut hasher, kind.as_bytes());
    update_framed(&mut hasher, previous.unwrap_or("").as_bytes());
    update_framed(&mut hasher, &bytes);
    Ok(hasher.finalize().to_hex().to_string())
}

fn update_framed(hasher: &mut blake3::Hasher, bytes: &[u8]) {
    hasher.update(&(bytes.len() as u64).to_be_bytes());
    hasher.update(bytes);
}

fn canonicalize(value: &Value) -> Value {
    match value {
        Value::Array(values) => Value::Array(values.iter().map(canonicalize).collect()),
        Value::Object(object) => {
            let mut keys: Vec<&String> = object.keys().collect();
            keys.sort_unstable();
            let mut canonical = Map::new();
            for key in keys {
                canonical.insert(key.clone(), canonicalize(&object[key]));
            }
            Value::Object(canonical)
        }
        _ => value.clone(),
    }
}

fn canonical_bytes(value: &Value) -> Result<Vec<u8>, String> {
    serde_json::to_vec(&canonicalize(value)).map_err(|error| error.to_string())
}

fn digest_value(value: &Value) -> Result<String, String> {
    Ok(blake3::hash(&canonical_bytes(value)?).to_hex().to_string())
}

fn validate_receipt(value: Option<&str>, field: &str) -> Result<(), String> {
    let Some(value) = value else { return Ok(()) };
    if value.len() != 64 || !value.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return Err(format!("{field} must be a 64-character hexadecimal digest"));
    }
    Ok(())
}

fn decode<T: DeserializeOwned>(value: &Value) -> Result<T, String> {
    serde_json::from_value(value.clone()).map_err(|error| error.to_string())
}

fn pretty(value: &Value) -> String {
    serde_json::to_string_pretty(value).unwrap_or_else(|_| value.to_string())
}

fn error_obj(id: Value, code: i64, message: &str) -> Value {
    json!({"jsonrpc": "2.0", "id": id, "error": {"code": code, "message": message}})
}

fn send(out: &mut impl Write, message: &Value) {
    if writeln!(out, "{message}").is_ok() {
        let _ = out.flush();
    }
}
