//! Canonical evidence admission for the Claude Code self-hosting loop.
//!
//! This server does not plan, allocate, validate, or actuate. It binds the
//! exact outputs of those independent authorities into replayable BLAKE3
//! envelopes with explicit predecessor commitments.

#![forbid(unsafe_code)]

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

const BCINR_REVISION: &str = "fb9321d27882169acc83aaca0639b319cd3b7900";
const RECEIPT_DOMAIN: &[u8] = b"urn:chatman:claude-code-admission:v1\0";

// Static per-tool semantic descriptions sourced from
// `plugins/chatman-ecosystem/ontology/ferroplan-domain.ttl`'s `rdfs:comment`
// annotations. The ontology flags this server's tool schemas as
// UNVERIFIED/lower-fidelity relative to session-mcp's, so field shapes here
// follow the actual Rust source (this file), not the ontology — only the
// prose semantic summary below is drawn from the ontology. Generated at
// compile time by `build.rs` — see that file for the extraction logic.
include!(concat!(env!("OUT_DIR"), "/admission_ontology.rs"));

#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct DigestInput {
    value: Value,
}

#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct BindAllocationInput {
    candidates: Value,
    allocation_result: Value,
    observation_frontier: Value,
    #[serde(default)]
    previous_receipt: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct BindPlanInput {
    session_think: Value,
    allocation_receipt: String,
    observation_frontier: Value,
    validator_result: Value,
    #[serde(default)]
    previous_receipt: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct VerifyInput {
    envelope: Value,
}

#[derive(Debug, Clone)]
struct ChatmanAdmission {
    tool_router: ToolRouter<Self>,
}

impl ChatmanAdmission {
    fn new() -> Self {
        Self {
            tool_router: Self::tool_router(),
        }
    }
}

#[tool_router]
impl ChatmanAdmission {
    #[tool(description = "Compute a BLAKE3 digest over recursively key-sorted canonical JSON.")]
    fn canonical_digest(
        &self,
        Parameters(input): Parameters<DigestInput>,
    ) -> Result<CallToolResult, McpError> {
        to_result(tool_canonical_digest(input))
    }

    #[tool(
        description = "Bind exactly eight CMCA candidates, the allocation result, the \
            observation frontier, the admitted BCINR revision, and an optional predecessor."
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

#[tool_handler(router = self.tool_router)]
impl ServerHandler for ChatmanAdmission {
    fn get_info(&self) -> ServerInfo {
        ServerInfo::new(
            ServerCapabilities::builder()
                .enable_tools()
                .enable_resources()
                .build(),
        )
        .with_server_info(rmcp::model::Implementation::new(
            "chatman-admission",
            env!("CARGO_PKG_VERSION"),
        ))
        .with_instructions(
            "Bind canonical observation, allocation, plan, validation, and predecessor \
             commitments. This server admits evidence; it does not plan or actuate. Read \
             `chatman-admission://tools/<name>` resources for ontology-sourced semantics.",
        )
    }

    async fn list_resources(
        &self,
        _request: Option<rmcp::model::PaginatedRequestParams>,
        _context: RequestContext<RoleServer>,
    ) -> Result<ListResourcesResult, McpError> {
        let resources = [
            "canonical_digest",
            "bind_allocation_receipt",
            "bind_plan_receipt",
            "verify_receipt",
        ]
        .into_iter()
        .map(|name| {
            Resource::new(
                format!("chatman-admission://tools/{name}"),
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
            .strip_prefix("chatman-admission://tools/")
            .ok_or_else(|| McpError::resource_not_found(request.uri.clone(), None))?;
        let ontology_comment = match name {
            "canonical_digest" => DIGEST_ONTOLOGY,
            "bind_allocation_receipt" => BIND_ALLOC_ONTOLOGY,
            "bind_plan_receipt" => BIND_PLAN_ONTOLOGY,
            "verify_receipt" => VERIFY_ONTOLOGY,
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
    let canonical = canonicalize(&input.value);
    Ok(json!({
        "schema": "urn:chatman:canonical-digest:v1",
        "algorithm": "BLAKE3",
        "digest": digest_value(&canonical)?,
        "canonical": canonical
    }))
}

fn tool_bind_allocation(input: BindAllocationInput) -> Result<Value, String> {
    validate_digest(input.previous_receipt.as_deref(), "previous_receipt")?;

    let candidates = canonicalize(&input.candidates);
    require_array_len(&candidates, "candidates", 8)?;

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
        .ok_or_else(|| "allocation_result lacks payload.allocations".to_owned())?;
    require_array_len(allocations, "allocation_result.payload.allocations", 8)?;

    let observation_frontier = canonicalize(&input.observation_frontier);
    let payload = canonicalize(&json!({
        "schema": "urn:chatman:allocation-admission-payload:v1",
        "bcinr_revision": BCINR_REVISION,
        "candidates_digest": digest_value(&candidates)?,
        "candidates": candidates,
        "allocation_result_digest": digest_value(&allocation_result)?,
        "allocation_result": allocation_result,
        "observation_frontier_digest": digest_value(&observation_frontier)?,
        "observation_frontier": observation_frontier
    }));

    make_envelope("allocation", payload, input.previous_receipt)
}

fn tool_bind_plan(input: BindPlanInput) -> Result<Value, String> {
    validate_digest(Some(&input.allocation_receipt), "allocation_receipt")?;
    validate_digest(input.previous_receipt.as_deref(), "previous_receipt")?;

    let session_think = canonicalize(&input.session_think);
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

    let validator_result = canonicalize(&input.validator_result);
    let validator_valid = validator_result
        .get("valid")
        .and_then(Value::as_bool)
        .or_else(|| validator_result.get("ok").and_then(Value::as_bool))
        .ok_or_else(|| "validator_result must declare boolean `valid` or `ok`".to_owned())?;
    if !validator_valid {
        return Err("independent validator did not admit the candidate plan".to_owned());
    }

    let plan = canonicalize(&plan);
    let observation_frontier = canonicalize(&input.observation_frontier);
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
    let object = input
        .envelope
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

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let service = ChatmanAdmission::new()
        .serve(rmcp::transport::stdio())
        .await
        .map_err(|e| {
            eprintln!("serving error: {e}");
            e
        })?;
    service.waiting().await?;
    Ok(())
}
