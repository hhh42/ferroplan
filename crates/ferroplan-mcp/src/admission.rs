//! Canonical evidence admission tools, merged into the single `ferroplan-mcp`
//! binary's `Ferroplan` tool handler (see `crate::main` for the merge).
//!
//! These tools do not plan, allocate, validate, or actuate. They bind the
//! exact outputs of those independent authorities into replayable BLAKE3
//! envelopes with explicit predecessor commitments.

use rmcp::handler::server::wrapper::Parameters;
use rmcp::model::{CallToolResult, ContentBlock, ErrorData as McpError};
use rmcp::tool;
use rmcp::tool_router;
use schemars::JsonSchema;
use serde::de::DeserializeOwned;
use serde::Deserialize;
use serde_json::{json, Map, Value};

use crate::Ferroplan;

const BCINR_REVISION: &str = "fb9321d27882169acc83aaca0639b319cd3b7900";
const RECEIPT_DOMAIN: &[u8] = b"urn:chatman:claude-code-admission:v1\0";

// Static per-tool semantic descriptions sourced from
// `plugins/chatman-ecosystem/ontology/ferroplan-domain.ttl`'s `rdfs:comment`
// annotations. The ontology flags this module's tool schemas as
// UNVERIFIED/lower-fidelity relative to session-mcp's, so field shapes here
// follow the actual Rust source (this file), not the ontology — only the
// prose semantic summary below is drawn from the ontology. Generated at
// compile time by `build.rs` — see that file for the extraction logic. These
// constants are read by `crate::main`'s merged
// `list_resources`/`read_resource`.
include!(concat!(env!("OUT_DIR"), "/admission_ontology.rs"));

pub(crate) const RESOURCE_TOOLS: &[&str] = &[
    "canonical_digest",
    "bind_allocation_receipt",
    "bind_plan_receipt",
    "verify_receipt",
];

pub(crate) fn ontology_comment(name: &str) -> Option<&'static str> {
    Some(match name {
        "canonical_digest" => DIGEST_ONTOLOGY,
        "bind_allocation_receipt" => BIND_ALLOC_ONTOLOGY,
        "bind_plan_receipt" => BIND_PLAN_ONTOLOGY,
        "verify_receipt" => VERIFY_ONTOLOGY,
        _ => return None,
    })
}

/// Schema for an unconstrained JSON value.
///
/// `schemars` renders `serde_json::Value` as the boolean schema `true`. That is
/// valid JSON Schema, but MCP clients reject a boolean where an object subschema
/// is required, which fails the whole `tools/list` response. An empty object
/// schema accepts exactly the same instances and validates as an object.
fn any_json(_: &mut schemars::SchemaGenerator) -> schemars::Schema {
    schemars::json_schema!({})
}

/// Some MCP clients, faced with the unconstrained `any_json` schema above,
/// serialize the argument as a JSON-encoded *string* instead of passing the
/// array/object natively — observed reproducibly against `canonical_digest`,
/// `bind_allocation_receipt`, and `bind_plan_receipt` in the same session
/// (the returned "canonical" form preserved the caller's original key order
/// instead of `canonicalize`'s alphabetical sort, proving the value never
/// reached `Value::Object`/`Value::Array` in the first place).
///
/// Defensively re-parse: a string that itself parses as a JSON array or
/// object is treated as that parsed value. A plain string that is not JSON
/// (or that parses to a scalar) is left untouched, so a legitimately
/// string-typed field can never be corrupted by this — it only ever
/// recovers structure that was already lost in transit, never invents any.
fn coerce_stringified_json(value: Value) -> Value {
    if let Value::String(text) = &value {
        if let Ok(parsed) = serde_json::from_str::<Value>(text) {
            if matches!(parsed, Value::Array(_) | Value::Object(_)) {
                return parsed;
            }
        }
    }
    value
}

#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct DigestInput {
    #[schemars(schema_with = "any_json")]
    value: Value,
}

#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct BindAllocationInput {
    #[schemars(schema_with = "any_json")]
    candidates: Value,
    #[schemars(schema_with = "any_json")]
    allocation_result: Value,
    #[schemars(schema_with = "any_json")]
    observation_frontier: Value,
    #[serde(default)]
    previous_receipt: Option<String>,
    /// The complete, previously-bound allocation envelope this call descends
    /// from (recursive CMCA: this local frontier is the eight-candidate
    /// expansion of one node inside that parent allocation). Must pair with
    /// `selected_node`. The envelope is independently re-verified, not
    /// trusted from its own `receipt` field, so a tampered or fabricated
    /// parent envelope is refused rather than silently bound.
    #[serde(default)]
    #[schemars(schema_with = "any_json")]
    parent_allocation: Option<Value>,
    /// The `id` of the parent allocation's candidate this local eight-node
    /// frontier expands. Must exist in `parent_allocation`'s candidate array.
    #[serde(default)]
    selected_node: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct BindPlanInput {
    #[schemars(schema_with = "any_json")]
    session_think: Value,
    allocation_receipt: String,
    #[schemars(schema_with = "any_json")]
    observation_frontier: Value,
    #[schemars(schema_with = "any_json")]
    validator_result: Value,
    #[serde(default)]
    previous_receipt: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct VerifyInput {
    #[schemars(schema_with = "any_json")]
    envelope: Value,
}

#[tool_router(router = admission_router, vis = "pub")]
impl Ferroplan {
    #[tool(description = "Compute a BLAKE3 digest over recursively key-sorted canonical JSON.")]
    fn canonical_digest(
        &self,
        Parameters(input): Parameters<DigestInput>,
    ) -> Result<CallToolResult, McpError> {
        to_result(tool_canonical_digest(input))
    }

    #[tool(
        description = "Bind exactly eight CMCA candidates, the allocation result, the \
            observation frontier, the admitted BCINR revision, and an optional predecessor. \
            Pass parent_allocation (a prior, independently re-verified allocation envelope) \
            and selected_node (one of its candidate ids) together to bind this local eight-node \
            frontier as a recursive descent from that parent node."
    )]
    fn bind_allocation_receipt(
        &self,
        Parameters(input): Parameters<BindAllocationInput>,
    ) -> Result<CallToolResult, McpError> {
        to_result(tool_bind_allocation(input))
    }

    #[tool(
        description = "Bind a solved Session result, allocation receipt, observation frontier, \
            independent validator result, and optional predecessor."
    )]
    fn bind_plan_receipt(
        &self,
        Parameters(input): Parameters<BindPlanInput>,
    ) -> Result<CallToolResult, McpError> {
        to_result(tool_bind_plan(input))
    }

    #[tool(
        description = "Recompute both payload digest and chained receipt without trusting the \
            envelope declarations."
    )]
    fn verify_receipt(
        &self,
        Parameters(input): Parameters<VerifyInput>,
    ) -> Result<CallToolResult, McpError> {
        to_result(tool_verify(input))
    }
}

/// Map the existing `Result<Value, String>` tool-body convention onto rmcp's
/// `CallToolResult`, preserving the prior `structuredContent` behavior on success.
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

fn tool_canonical_digest(input: DigestInput) -> Result<Value, String> {
    let canonical = canonicalize(&coerce_stringified_json(input.value));
    Ok(json!({
        "schema": "urn:chatman:canonical-digest:v1",
        "algorithm": "BLAKE3",
        "digest": digest_value(&canonical)?,
        "canonical": canonical
    }))
}

fn tool_bind_allocation(input: BindAllocationInput) -> Result<Value, String> {
    validate_digest(input.previous_receipt.as_deref(), "previous_receipt")?;

    let candidates = canonicalize(&coerce_stringified_json(input.candidates));
    require_array_len(&candidates, "candidates", 8)?;

    let allocation_result = canonicalize(&coerce_stringified_json(input.allocation_result));
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
        .ok_or_else(|| "allocation_result lacks payload.allocations".to_owned())?;
    require_array_len(allocations, "allocation_result.payload.allocations", 8)?;

    let observation_frontier = canonicalize(&coerce_stringified_json(input.observation_frontier));

    let descent = match (input.parent_allocation, input.selected_node) {
        (Some(parent_allocation), Some(selected_node)) => Some(bind_descent(
            coerce_stringified_json(parent_allocation),
            selected_node,
        )?),
        (None, None) => None,
        _ => {
            return Err("parent_allocation and selected_node must be provided together".to_owned())
        }
    };

    let mut payload = json!({
        "schema": "urn:chatman:allocation-admission-payload:v1",
        "bcinr_revision": BCINR_REVISION,
        "candidates_digest": digest_value(&candidates)?,
        "candidates": candidates,
        "allocation_result_digest": digest_value(&allocation_result)?,
        "allocation_result": allocation_result,
        "observation_frontier_digest": digest_value(&observation_frontier)?,
        "observation_frontier": observation_frontier
    });
    if let Some((parent_receipt, selected_node, parent_candidate)) = descent {
        let object = payload
            .as_object_mut()
            .expect("payload literal is always a JSON object");
        object.insert(
            "parent_allocation_receipt".to_owned(),
            Value::String(parent_receipt),
        );
        object.insert("selected_node".to_owned(), Value::String(selected_node));
        object.insert("selected_node_candidate".to_owned(), parent_candidate);
    }
    let payload = canonicalize(&payload);

    make_envelope("allocation", payload, input.previous_receipt)
}

/// Independently re-verify a claimed parent allocation envelope (recomputing
/// its payload digest and receipt exactly as `verify_receipt` would, not
/// trusting its self-declared fields) and confirm `selected_node` names one
/// of its eight candidates. Returns the parent's verified receipt, the
/// selected node id, and the matched candidate for embedding in the child
/// payload — this is the "parent allocation receipt" and "selected node"
/// half of Checkpoint 9's descent diagram. A tampered or fabricated parent
/// envelope, or a `selected_node` absent from the parent's candidates,
/// refuses here rather than being silently bound.
fn bind_descent(
    parent_allocation: Value,
    selected_node: String,
) -> Result<(String, String, Value), String> {
    let object = parent_allocation
        .as_object()
        .ok_or_else(|| "parent_allocation must be an object".to_owned())?;
    let kind = required_str(object, "kind")?;
    if kind != "allocation" {
        return Err(format!(
            "parent_allocation must be an allocation envelope, found kind `{kind}`"
        ));
    }
    let payload = canonicalize(
        object
            .get("payload")
            .ok_or_else(|| "parent_allocation lacks payload".to_owned())?,
    );
    let previous = object.get("previous_receipt").and_then(Value::as_str);
    validate_digest(previous, "parent_allocation.previous_receipt")?;
    let declared_payload_digest = required_str(object, "payload_digest")?;
    let declared_receipt = required_str(object, "receipt")?;
    validate_digest(
        Some(declared_payload_digest),
        "parent_allocation.payload_digest",
    )?;
    validate_digest(Some(declared_receipt), "parent_allocation.receipt")?;

    let expected_payload_digest = digest_value(&payload)?;
    if declared_payload_digest != expected_payload_digest {
        return Err(
            "parent_allocation.payload_digest does not match its own payload; refusing an unverifiable parent".to_owned(),
        );
    }
    let expected_receipt = receipt_for(kind, &payload, previous)?;
    if declared_receipt != expected_receipt {
        return Err(
            "parent_allocation.receipt does not match its own payload/predecessor; refusing an unverifiable parent".to_owned(),
        );
    }

    let parent_candidates = payload
        .get("candidates")
        .and_then(Value::as_array)
        .ok_or_else(|| {
            "parent_allocation.payload.candidates is missing or not an array".to_owned()
        })?;
    let candidate = parent_candidates
        .iter()
        .find(|candidate| {
            candidate.get("id").and_then(Value::as_str) == Some(selected_node.as_str())
        })
        .cloned()
        .ok_or_else(|| {
            format!("selected_node `{selected_node}` is not a candidate in parent_allocation")
        })?;

    Ok((declared_receipt.to_owned(), selected_node, candidate))
}

fn tool_bind_plan(input: BindPlanInput) -> Result<Value, String> {
    validate_digest(Some(&input.allocation_receipt), "allocation_receipt")?;
    validate_digest(input.previous_receipt.as_deref(), "previous_receipt")?;

    let session_think = canonicalize(&coerce_stringified_json(input.session_think));
    let plan = session_think
        .get("plan")
        .filter(|value| !value.is_null())
        .or_else(|| {
            session_think
                .pointer("/solution/plan")
                .filter(|value| !value.is_null())
        })
        .cloned()
        .ok_or_else(|| "session_think does not contain a solved plan".to_owned())?;
    let session_receipt = session_think
        .get("receipt")
        .and_then(Value::as_str)
        .map(str::to_owned)
        .ok_or_else(|| "session_think lacks a receipt".to_owned())?;
    validate_digest(Some(&session_receipt), "session_think.receipt")?;

    let validator_result = canonicalize(&coerce_stringified_json(input.validator_result));
    let validator_valid = validator_result
        .get("valid")
        .and_then(Value::as_bool)
        .or_else(|| validator_result.get("ok").and_then(Value::as_bool))
        .ok_or_else(|| "validator_result must declare boolean `valid` or `ok`".to_owned())?;
    if !validator_valid {
        return Err("independent validator did not admit the candidate plan".to_owned());
    }

    let plan = canonicalize(&plan);
    let observation_frontier = canonicalize(&coerce_stringified_json(input.observation_frontier));
    let payload = canonicalize(&json!({
        "schema": "urn:chatman:plan-admission-payload:v1",
        "session_receipt": session_receipt,
        "session_think": session_think,
        "plan_digest": digest_value(&plan)?,
        "plan": plan,
        "allocation_receipt": input.allocation_receipt,
        "observation_frontier_digest": digest_value(&observation_frontier)?,
        "observation_frontier": observation_frontier,
        "validator_result_digest": digest_value(&validator_result)?,
        "validator_result": validator_result
    }));

    make_envelope("plan", payload, input.previous_receipt)
}

fn tool_verify(input: VerifyInput) -> Result<Value, String> {
    let envelope = coerce_stringified_json(input.envelope);
    let object = envelope
        .as_object()
        .ok_or_else(|| "envelope must be an object".to_owned())?;
    let kind = required_str(object, "kind")?;
    let payload = canonicalize(
        object
            .get("payload")
            .ok_or_else(|| "envelope lacks payload".to_owned())?,
    );
    let previous = object.get("previous_receipt").and_then(Value::as_str);
    validate_digest(previous, "previous_receipt")?;
    let declared_payload = required_str(object, "payload_digest")?;
    let declared_receipt = required_str(object, "receipt")?;
    validate_digest(Some(declared_payload), "payload_digest")?;
    validate_digest(Some(declared_receipt), "receipt")?;

    let expected_payload = digest_value(&payload)?;
    let expected_receipt = receipt_for(kind, &payload, previous)?;
    let payload_digest_valid = declared_payload == expected_payload;
    let receipt_valid = declared_receipt == expected_receipt;

    Ok(json!({
        "schema": "urn:chatman:receipt-verification:v1",
        "valid": payload_digest_valid && receipt_valid,
        "payload_digest_valid": payload_digest_valid,
        "receipt_valid": receipt_valid,
        "declared_payload_digest": declared_payload,
        "expected_payload_digest": expected_payload,
        "declared_receipt": declared_receipt,
        "expected_receipt": expected_receipt,
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
    let mut hasher = blake3::Hasher::new();
    hasher.update(RECEIPT_DOMAIN);
    update_framed(&mut hasher, kind.as_bytes());
    update_framed(&mut hasher, previous.unwrap_or("").as_bytes());
    update_framed(&mut hasher, &canonical_bytes(payload)?);
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
            let mut result = Map::new();
            for key in keys {
                result.insert(key.clone(), canonicalize(&object[key]));
            }
            Value::Object(result)
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

fn require_array_len(value: &Value, field: &str, length: usize) -> Result<(), String> {
    let actual = value
        .as_array()
        .ok_or_else(|| format!("{field} must be an array"))?
        .len();
    if actual != length {
        return Err(format!(
            "{field} requires exactly {length} items; received {actual}"
        ));
    }
    Ok(())
}

fn required_str<'a>(object: &'a Map<String, Value>, field: &str) -> Result<&'a str, String> {
    object
        .get(field)
        .and_then(Value::as_str)
        .ok_or_else(|| format!("envelope lacks string `{field}`"))
}

fn validate_digest(value: Option<&str>, field: &str) -> Result<(), String> {
    let Some(value) = value else { return Ok(()) };
    if value.len() != 64 || !value.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return Err(format!("{field} must be a 64-character hexadecimal digest"));
    }
    Ok(())
}

#[allow(dead_code)]
fn decode<T: DeserializeOwned>(value: &Value) -> Result<T, String> {
    serde_json::from_value(value.clone()).map_err(|error| error.to_string())
}

fn pretty(value: &Value) -> String {
    serde_json::to_string_pretty(value).unwrap_or_else(|_| value.to_string())
}
