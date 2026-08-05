//! `ferroplan-mcp-plus` — the bounded production MCP+ profile.
//!
//! This binary intentionally has a smaller authority and capability surface
//! than the legacy `ferroplan-mcp` server. It accepts line-delimited JSON-RPC
//! 2.0 over stdio, exposes only planning/evidence operations, isolates every
//! untrusted computational request in a child process, kills that worker at a
//! hard deadline, and never executes caller-selected commands.
//!
//! Protocol profile: `ferroplan-mcp-plus/1.0`. This is an exact ferroplan
//! profile; the binary does not claim conformance to an external MCP revision
//! beyond the JSON-RPC method shapes tested in this crate.

use std::io::{self, BufRead, BufReader, Read, Write};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

use ferroplan::{
    capability_manifest, solve_production, Options, OutcomeClass, Plan, ProductionLimits,
};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use wait_timeout::ChildExt;

const PROFILE: &str = "ferroplan-mcp-plus/1.0";
const MAX_FRAME_BYTES: usize = 8 * 1024 * 1024;
const MAX_RESPONSE_BYTES: usize = 16 * 1024 * 1024;
const MAX_DOMAIN_BYTES: usize = 4 * 1024 * 1024;
const MAX_PROBLEM_BYTES: usize = 4 * 1024 * 1024;
const MAX_PLAN_BYTES: usize = 4 * 1024 * 1024;
const MAX_PDDL_BYTES: usize = 4 * 1024 * 1024;
const DEFAULT_TIMEOUT_MS: u64 = 5_000;
const MAX_TIMEOUT_MS: u64 = 60_000;
const MAX_METHOD_BYTES: usize = 128;

#[derive(Debug)]
enum Frame {
    Complete(Vec<u8>),
    TooLarge,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct JsonRpcRequest {
    jsonrpc: String,
    #[serde(default)]
    id: Option<Value>,
    method: String,
    #[serde(default)]
    params: Value,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct ToolCallParams {
    name: String,
    #[serde(default)]
    arguments: Value,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct PlanArgs {
    domain: String,
    problem: String,
    #[serde(default)]
    options: Option<Options>,
    #[serde(default)]
    limits: Option<McpLimits>,
    #[serde(default)]
    request_id: Option<String>,
    #[serde(default)]
    timeout_ms: Option<u64>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
struct McpLimits {
    #[serde(default)]
    max_domain_bytes: Option<usize>,
    #[serde(default)]
    max_problem_bytes: Option<usize>,
    #[serde(default)]
    max_evaluated: Option<usize>,
    #[serde(default)]
    max_plan_steps: Option<usize>,
    #[serde(default)]
    max_output_bytes: Option<usize>,
    #[serde(default)]
    max_workers: Option<usize>,
}

impl McpLimits {
    fn into_production(self) -> ProductionLimits {
        let defaults = ProductionLimits::default();
        ProductionLimits {
            max_domain_bytes: self.max_domain_bytes.unwrap_or(defaults.max_domain_bytes),
            max_problem_bytes: self.max_problem_bytes.unwrap_or(defaults.max_problem_bytes),
            max_evaluated: self.max_evaluated.unwrap_or(defaults.max_evaluated),
            max_plan_steps: self.max_plan_steps.unwrap_or(defaults.max_plan_steps),
            max_output_bytes: self.max_output_bytes.unwrap_or(defaults.max_output_bytes),
            max_workers: self.max_workers.unwrap_or(defaults.max_workers),
        }
    }
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct ParseArgs {
    pddl: String,
    #[serde(default)]
    timeout_ms: Option<u64>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct ValidateArgs {
    domain: String,
    problem: String,
    plan: String,
    #[serde(default)]
    timeout_ms: Option<u64>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct ExplainArgs {
    domain: String,
    problem: String,
    plan: Plan,
    #[serde(default)]
    timeout_ms: Option<u64>,
}

#[derive(Debug, Serialize, Deserialize)]
struct WorkerError {
    code: String,
    message: String,
    retryable: bool,
}

#[derive(Debug, Serialize, Deserialize)]
struct WorkerReply {
    is_error: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    result: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<WorkerError>,
}

impl WorkerReply {
    fn success(result: Value) -> Self {
        Self {
            is_error: false,
            result: Some(result),
            error: None,
        }
    }

    fn tool_error(result: Value) -> Self {
        Self {
            is_error: true,
            result: Some(result),
            error: None,
        }
    }

    fn failure(code: &str, message: impl Into<String>, retryable: bool) -> Self {
        Self {
            is_error: true,
            result: None,
            error: Some(WorkerError {
                code: code.to_string(),
                message: bounded_text(&message.into(), 2_048),
                retryable,
            }),
        }
    }
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let result = if args.get(1).map(String::as_str) == Some("--worker") {
        let tool = args.get(2).map(String::as_str).unwrap_or("");
        worker_entry(tool)
    } else if args.get(1).map(String::as_str) == Some("--self-test") {
        self_test()
    } else if args.len() == 1 {
        serve()
    } else {
        Err(format!(
            "unsupported arguments; use no arguments, --self-test, or internal --worker"
        ))
    };

    if let Err(error) = result {
        eprintln!(
            "{}",
            json!({
                "event": "mcp_plus.process.failed",
                "profile": PROFILE,
                "error_code": "FP_ADAPTER",
                "message": bounded_text(&error, 2_048),
            })
        );
        std::process::exit(70);
    }
}

fn serve() -> Result<(), String> {
    let stdin = io::stdin();
    let stdout = io::stdout();
    let mut reader = BufReader::new(stdin.lock());
    let mut writer = stdout.lock();

    loop {
        let frame = match read_frame(&mut reader, MAX_FRAME_BYTES)
            .map_err(|error| format!("reading protocol frame: {error}"))?
        {
            None => return Ok(()),
            Some(frame) => frame,
        };

        match frame {
            Frame::TooLarge => {
                let response = rpc_error(
                    Value::Null,
                    -32_600,
                    "request frame exceeds the MCP+ limit",
                    "FP_LIMIT_INPUT",
                    false,
                );
                write_response(&mut writer, &response)?;
            }
            Frame::Complete(bytes) if bytes.iter().all(u8::is_ascii_whitespace) => {}
            Frame::Complete(bytes) => {
                let started = Instant::now();
                let parsed = serde_json::from_slice::<JsonRpcRequest>(&bytes);
                let (response, method, outcome) = match parsed {
                    Ok(request) => {
                        let method = bounded_text(&request.method, MAX_METHOD_BYTES);
                        let id = request.id.clone();
                        match handle_request(request) {
                            Some(response) => {
                                let outcome = response_outcome(&response);
                                (Some(response), method, outcome)
                            }
                            None => (None, method, "notification".to_string()),
                        }
                    }
                    Err(error) => (
                        Some(rpc_error(
                            Value::Null,
                            -32_700,
                            format!("invalid JSON-RPC request: {error}"),
                            "FP_PARSE",
                            false,
                        )),
                        "unparsed".to_string(),
                        "refused".to_string(),
                    ),
                };
                if let Some(response) = response {
                    write_response(&mut writer, &response)?;
                }
                eprintln!(
                    "{}",
                    json!({
                        "event": "mcp_plus.request.completed",
                        "profile": PROFILE,
                        "method": method,
                        "outcome": outcome,
                        "elapsed_micros": started.elapsed().as_micros().min(u64::MAX as u128) as u64,
                    })
                );
            }
        }
    }
}

fn handle_request(request: JsonRpcRequest) -> Option<Value> {
    let id = request.id?;
    if request.jsonrpc != "2.0" {
        return Some(rpc_error(
            id,
            -32_600,
            "jsonrpc must equal 2.0",
            "FP_INVALID_REQUEST",
            false,
        ));
    }
    if request.method.len() > MAX_METHOD_BYTES {
        return Some(rpc_error(
            id,
            -32_600,
            "method name exceeds the MCP+ limit",
            "FP_LIMIT_INPUT",
            false,
        ));
    }

    let result = match request.method.as_str() {
        "initialize" => Ok(initialize_result()),
        "tools/list" => Ok(json!({ "tools": tool_definitions() })),
        "tools/call" => handle_tool_call(request.params),
        "ferroplan/version" => Ok(version_result()),
        "ferroplan/readiness" => readiness_result(),
        _ => Err(WorkerError {
            code: "FP_UNSUPPORTED".to_string(),
            message: format!("unsupported method `{}`", request.method),
            retryable: false,
        }),
    };

    Some(match result {
        Ok(value) => rpc_success(id, value),
        Err(error) => rpc_error(
            id,
            -32_601,
            error.message,
            &error.code,
            error.retryable,
        ),
    })
}

fn handle_tool_call(params: Value) -> Result<Value, WorkerError> {
    let call: ToolCallParams = serde_json::from_value(params).map_err(|error| WorkerError {
        code: "FP_INVALID_REQUEST".to_string(),
        message: format!("invalid tools/call parameters: {error}"),
        retryable: false,
    })?;
    if !matches!(
        call.name.as_str(),
        "plan" | "parse" | "validate" | "explain" | "readiness" | "version"
    ) {
        return Err(WorkerError {
            code: "FP_UNSUPPORTED".to_string(),
            message: format!("unsupported MCP+ tool `{}`", call.name),
            retryable: false,
        });
    }
    if call.name == "readiness" {
        return readiness_result().map(tool_result_success);
    }
    if call.name == "version" {
        return Ok(tool_result_success(version_result()));
    }

    let timeout_ms = requested_timeout(&call.arguments)?;
    let reply = run_worker(&call.name, &call.arguments, timeout_ms)?;
    if let Some(error) = reply.error {
        return Err(error);
    }
    let result = reply.result.unwrap_or(Value::Null);
    Ok(if reply.is_error {
        tool_result_error(result)
    } else {
        tool_result_success(result)
    })
}

fn initialize_result() -> Value {
    json!({
        "protocolProfile": PROFILE,
        "serverInfo": {
            "name": "ferroplan-mcp-plus",
            "version": env!("CARGO_PKG_VERSION"),
        },
        "capabilities": {
            "tools": { "listChanged": false },
            "resources": false,
            "prompts": false,
        },
        "authority": "candidate_only",
        "authorityNotice": "Tool discovery and planning grant no actuation authority. BRCE, observed consequence, POWL conformance, OCEL evidence, Truex receipt/refusal, and replay remain downstream obligations.",
        "limits": {
            "maxFrameBytes": MAX_FRAME_BYTES,
            "maxResponseBytes": MAX_RESPONSE_BYTES,
            "defaultTimeoutMs": DEFAULT_TIMEOUT_MS,
            "maxTimeoutMs": MAX_TIMEOUT_MS,
            "maxConcurrentRequests": 1,
        }
    })
}

fn version_result() -> Value {
    json!({
        "profile": PROFILE,
        "productVersion": env!("CARGO_PKG_VERSION"),
        "capabilityManifestSchema": ferroplan::CAPABILITY_MANIFEST_SCHEMA,
        "operationEnvelopeSchema": ferroplan::OPERATION_ENVELOPE_SCHEMA,
        "authority": "candidate_only",
    })
}

fn readiness_result() -> Result<Value, WorkerError> {
    let manifest = capability_manifest();
    let fingerprint = manifest.fingerprint().map_err(|error| WorkerError {
        code: "FP_INVARIANT".to_string(),
        message: error.to_string(),
        retryable: false,
    })?;
    Ok(json!({
        "schemaVersion": "ferroplan.readiness-contract.v1",
        "profile": PROFILE,
        "productVersion": env!("CARGO_PKG_VERSION"),
        "manifestFingerprint": fingerprint,
        "contractValid": true,
        "admissionState": "declared",
        "admissionNotice": "Admission is independently derived from exact-source evidence; this endpoint cannot self-crown the build.",
        "manifest": manifest,
    }))
}

fn tool_definitions() -> Vec<Value> {
    vec![
        json!({
            "name": "plan",
            "description": "Bounded candidate-only PDDL planning with independent validation and a hard worker-process deadline.",
            "inputSchema": {
                "type": "object",
                "additionalProperties": false,
                "required": ["domain", "problem"],
                "properties": {
                    "domain": { "type": "string", "maxLength": MAX_DOMAIN_BYTES },
                    "problem": { "type": "string", "maxLength": MAX_PROBLEM_BYTES },
                    "options": { "type": "object" },
                    "limits": { "type": "object" },
                    "request_id": { "type": "string", "maxLength": 128 },
                    "timeout_ms": { "type": "integer", "minimum": 1, "maximum": MAX_TIMEOUT_MS }
                }
            }
        }),
        json!({
            "name": "parse",
            "description": "Bounded PDDL syntax and structure inspection; evidence-only, not execution authority.",
            "inputSchema": {
                "type": "object",
                "additionalProperties": false,
                "required": ["pddl"],
                "properties": {
                    "pddl": { "type": "string", "maxLength": MAX_PDDL_BYTES },
                    "timeout_ms": { "type": "integer", "minimum": 1, "maximum": MAX_TIMEOUT_MS }
                }
            }
        }),
        json!({
            "name": "validate",
            "description": "Independently validate a bounded plan against its domain and problem.",
            "inputSchema": {
                "type": "object",
                "additionalProperties": false,
                "required": ["domain", "problem", "plan"],
                "properties": {
                    "domain": { "type": "string", "maxLength": MAX_DOMAIN_BYTES },
                    "problem": { "type": "string", "maxLength": MAX_PROBLEM_BYTES },
                    "plan": { "type": "string", "maxLength": MAX_PLAN_BYTES },
                    "timeout_ms": { "type": "integer", "minimum": 1, "maximum": MAX_TIMEOUT_MS }
                }
            }
        }),
        json!({
            "name": "explain",
            "description": "Produce bounded explanation evidence for a typed candidate plan.",
            "inputSchema": {
                "type": "object",
                "additionalProperties": false,
                "required": ["domain", "problem", "plan"],
                "properties": {
                    "domain": { "type": "string", "maxLength": MAX_DOMAIN_BYTES },
                    "problem": { "type": "string", "maxLength": MAX_PROBLEM_BYTES },
                    "plan": { "type": "object" },
                    "timeout_ms": { "type": "integer", "minimum": 1, "maximum": MAX_TIMEOUT_MS }
                }
            }
        }),
        json!({
            "name": "readiness",
            "description": "Return the canonical capability contract and manifest fingerprint without self-authoring admission.",
            "inputSchema": { "type": "object", "additionalProperties": false }
        }),
        json!({
            "name": "version",
            "description": "Return exact product and schema versions for this MCP+ profile.",
            "inputSchema": { "type": "object", "additionalProperties": false }
        }),
    ]
}

fn requested_timeout(arguments: &Value) -> Result<u64, WorkerError> {
    let requested = arguments
        .get("timeout_ms")
        .and_then(Value::as_u64)
        .unwrap_or(DEFAULT_TIMEOUT_MS);
    if requested == 0 || requested > MAX_TIMEOUT_MS {
        return Err(WorkerError {
            code: "FP_INVALID_REQUEST".to_string(),
            message: format!("timeout_ms must be in 1..={MAX_TIMEOUT_MS}"),
            retryable: false,
        });
    }
    Ok(requested)
}

fn run_worker(tool: &str, arguments: &Value, timeout_ms: u64) -> Result<WorkerReply, WorkerError> {
    let request = serde_json::to_vec(arguments).map_err(|error| WorkerError {
        code: "FP_ADAPTER".to_string(),
        message: format!("serializing worker request: {error}"),
        retryable: false,
    })?;
    if request.len() > MAX_FRAME_BYTES {
        return Err(WorkerError {
            code: "FP_LIMIT_INPUT".to_string(),
            message: "worker request exceeds the MCP+ frame limit".to_string(),
            retryable: false,
        });
    }

    let executable = std::env::current_exe().map_err(|error| WorkerError {
        code: "FP_ADAPTER".to_string(),
        message: format!("resolving current executable: {error}"),
        retryable: false,
    })?;
    let mut child = Command::new(executable)
        .arg("--worker")
        .arg(tool)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|error| WorkerError {
            code: "FP_ADAPTER".to_string(),
            message: format!("spawning isolated worker: {error}"),
            retryable: true,
        })?;

    if let Some(mut stdin) = child.stdin.take() {
        stdin.write_all(&request).map_err(|error| WorkerError {
            code: "FP_ADAPTER".to_string(),
            message: format!("writing isolated worker request: {error}"),
            retryable: true,
        })?;
    }

    let status = child
        .wait_timeout(Duration::from_millis(timeout_ms))
        .map_err(|error| WorkerError {
            code: "FP_ADAPTER".to_string(),
            message: format!("waiting for isolated worker: {error}"),
            retryable: true,
        })?;
    if status.is_none() {
        let _ = child.kill();
        let _ = child.wait();
        return Err(WorkerError {
            code: "FP_TIMEOUT".to_string(),
            message: format!("isolated worker exceeded the {timeout_ms} ms deadline"),
            retryable: true,
        });
    }

    let mut output = Vec::new();
    if let Some(stdout) = child.stdout.take() {
        stdout
            .take((MAX_RESPONSE_BYTES + 1) as u64)
            .read_to_end(&mut output)
            .map_err(|error| WorkerError {
                code: "FP_ADAPTER".to_string(),
                message: format!("reading isolated worker response: {error}"),
                retryable: true,
            })?;
    }
    if output.len() > MAX_RESPONSE_BYTES {
        return Err(WorkerError {
            code: "FP_LIMIT_OUTPUT".to_string(),
            message: "isolated worker response exceeds the MCP+ output limit".to_string(),
            retryable: true,
        });
    }
    serde_json::from_slice(&output).map_err(|error| WorkerError {
        code: "FP_ADAPTER".to_string(),
        message: format!("decoding isolated worker response: {error}"),
        retryable: true,
    })
}

fn worker_entry(tool: &str) -> Result<(), String> {
    let mut bytes = Vec::new();
    io::stdin()
        .take((MAX_FRAME_BYTES + 1) as u64)
        .read_to_end(&mut bytes)
        .map_err(|error| format!("reading worker input: {error}"))?;
    let reply = if bytes.len() > MAX_FRAME_BYTES {
        WorkerReply::failure("FP_LIMIT_INPUT", "worker input exceeds the frame limit", false)
    } else {
        match serde_json::from_slice::<Value>(&bytes) {
            Ok(arguments) => execute_worker_tool(tool, arguments),
            Err(error) => WorkerReply::failure(
                "FP_PARSE",
                format!("invalid worker request JSON: {error}"),
                false,
            ),
        }
    };
    let encoded = serde_json::to_vec(&reply)
        .map_err(|error| format!("serializing worker response: {error}"))?;
    if encoded.len() > MAX_RESPONSE_BYTES {
        return Err("worker response exceeds the output limit".to_string());
    }
    io::stdout()
        .write_all(&encoded)
        .map_err(|error| format!("writing worker response: {error}"))
}

fn execute_worker_tool(tool: &str, arguments: Value) -> WorkerReply {
    match tool {
        "plan" => worker_plan(arguments),
        "parse" => worker_parse(arguments),
        "validate" => worker_validate(arguments),
        "explain" => worker_explain(arguments),
        _ => WorkerReply::failure(
            "FP_UNSUPPORTED",
            format!("unsupported isolated worker tool `{tool}`"),
            false,
        ),
    }
}

fn worker_plan(arguments: Value) -> WorkerReply {
    let args: PlanArgs = match serde_json::from_value(arguments) {
        Ok(args) => args,
        Err(error) => {
            return WorkerReply::failure(
                "FP_INVALID_REQUEST",
                format!("invalid plan arguments: {error}"),
                false,
            )
        }
    };
    let limits = args.limits.unwrap_or_default().into_production();
    let mut options = args.options.unwrap_or_default();
    if options.threads == 0 {
        options.threads = 1;
    }
    let envelope = solve_production(
        &args.domain,
        &args.problem,
        &options,
        &limits,
        args.request_id.as_deref(),
    );
    let is_error = matches!(
        envelope.outcome,
        OutcomeClass::Refused | OutcomeClass::Failed | OutcomeClass::LimitExceeded
    );
    let value = match serde_json::to_value(envelope) {
        Ok(value) => value,
        Err(error) => {
            return WorkerReply::failure(
                "FP_ADAPTER",
                format!("serializing planning envelope: {error}"),
                false,
            )
        }
    };
    if is_error {
        WorkerReply::tool_error(value)
    } else {
        WorkerReply::success(value)
    }
}

fn worker_parse(arguments: Value) -> WorkerReply {
    let args: ParseArgs = match serde_json::from_value(arguments) {
        Ok(args) => args,
        Err(error) => {
            return WorkerReply::failure(
                "FP_INVALID_REQUEST",
                format!("invalid parse arguments: {error}"),
                false,
            )
        }
    };
    if args.pddl.len() > MAX_PDDL_BYTES {
        return WorkerReply::failure("FP_LIMIT_INPUT", "PDDL exceeds the parse limit", false);
    }
    WorkerReply::success(json!({
        "schemaVersion": "ferroplan.parse-evidence.v1",
        "capabilityId": "fp.core.validate",
        "authority": "evidence_only",
        "result": ferroplan::parse(&args.pddl),
    }))
}

fn worker_validate(arguments: Value) -> WorkerReply {
    let args: ValidateArgs = match serde_json::from_value(arguments) {
        Ok(args) => args,
        Err(error) => {
            return WorkerReply::failure(
                "FP_INVALID_REQUEST",
                format!("invalid validate arguments: {error}"),
                false,
            )
        }
    };
    if let Some(error) = validate_text_bounds(&args.domain, &args.problem, Some(&args.plan)) {
        return error;
    }
    match ferroplan::plan::validate_plan(&args.domain, &args.problem, &args.plan) {
        Ok(ferroplan::plan::Validity::Valid) => WorkerReply::success(json!({
            "schemaVersion": "ferroplan.plan-validation.v1",
            "capabilityId": "fp.core.validate",
            "authority": "evidence_only",
            "valid": true,
            "reason": null,
        })),
        Ok(ferroplan::plan::Validity::Invalid(reason)) => WorkerReply::success(json!({
            "schemaVersion": "ferroplan.plan-validation.v1",
            "capabilityId": "fp.core.validate",
            "authority": "evidence_only",
            "valid": false,
            "reason": bounded_text(&reason, 2_048),
        })),
        Err(error) => WorkerReply::failure("FP_VALIDATION", error, false),
    }
}

fn worker_explain(arguments: Value) -> WorkerReply {
    let args: ExplainArgs = match serde_json::from_value(arguments) {
        Ok(args) => args,
        Err(error) => {
            return WorkerReply::failure(
                "FP_INVALID_REQUEST",
                format!("invalid explain arguments: {error}"),
                false,
            )
        }
    };
    if let Some(error) = validate_text_bounds(&args.domain, &args.problem, None) {
        return error;
    }
    match ferroplan::introspect::explain(&args.domain, &args.problem, &args.plan) {
        Ok(explanation) => match serde_json::to_value(explanation) {
            Ok(value) => WorkerReply::success(json!({
                "schemaVersion": "ferroplan.explanation-evidence.v1",
                "capabilityId": "fp.core.explain",
                "authority": "evidence_only",
                "result": value,
            })),
            Err(error) => WorkerReply::failure(
                "FP_ADAPTER",
                format!("serializing explanation: {error}"),
                false,
            ),
        },
        Err(error) => WorkerReply::failure("FP_VALIDATION", error, false),
    }
}

fn validate_text_bounds(domain: &str, problem: &str, plan: Option<&str>) -> Option<WorkerReply> {
    if domain.len() > MAX_DOMAIN_BYTES {
        return Some(WorkerReply::failure(
            "FP_LIMIT_INPUT",
            "domain exceeds the MCP+ input limit",
            false,
        ));
    }
    if problem.len() > MAX_PROBLEM_BYTES {
        return Some(WorkerReply::failure(
            "FP_LIMIT_INPUT",
            "problem exceeds the MCP+ input limit",
            false,
        ));
    }
    if plan.is_some_and(|plan| plan.len() > MAX_PLAN_BYTES) {
        return Some(WorkerReply::failure(
            "FP_LIMIT_INPUT",
            "plan exceeds the MCP+ input limit",
            false,
        ));
    }
    None
}

fn tool_result_success(value: Value) -> Value {
    let text = serde_json::to_string_pretty(&value).unwrap_or_else(|_| "null".to_string());
    json!({
        "content": [{ "type": "text", "text": text }],
        "structuredContent": value,
        "isError": false,
    })
}

fn tool_result_error(value: Value) -> Value {
    let text = serde_json::to_string_pretty(&value).unwrap_or_else(|_| "null".to_string());
    json!({
        "content": [{ "type": "text", "text": text }],
        "structuredContent": value,
        "isError": true,
    })
}

fn rpc_success(id: Value, result: Value) -> Value {
    json!({ "jsonrpc": "2.0", "id": id, "result": result })
}

fn rpc_error(
    id: Value,
    code: i64,
    message: impl Into<String>,
    ferroplan_code: &str,
    retryable: bool,
) -> Value {
    json!({
        "jsonrpc": "2.0",
        "id": id,
        "error": {
            "code": code,
            "message": bounded_text(&message.into(), 2_048),
            "data": {
                "ferroplanCode": ferroplan_code,
                "retryable": retryable,
                "profile": PROFILE,
            }
        }
    })
}

fn response_outcome(response: &Value) -> String {
    if response.get("error").is_some() {
        "refused".to_string()
    } else if response
        .pointer("/result/isError")
        .and_then(Value::as_bool)
        .unwrap_or(false)
    {
        "tool_error".to_string()
    } else {
        "success".to_string()
    }
}

fn write_response(writer: &mut impl Write, response: &Value) -> Result<(), String> {
    let mut encoded = serde_json::to_vec(response)
        .map_err(|error| format!("serializing protocol response: {error}"))?;
    if encoded.len() > MAX_RESPONSE_BYTES {
        encoded = serde_json::to_vec(&rpc_error(
            response.get("id").cloned().unwrap_or(Value::Null),
            -32_603,
            "response exceeds the MCP+ output limit",
            "FP_LIMIT_OUTPUT",
            true,
        ))
        .map_err(|error| format!("serializing output-limit response: {error}"))?;
    }
    encoded.push(b'\n');
    writer
        .write_all(&encoded)
        .and_then(|_| writer.flush())
        .map_err(|error| format!("writing protocol response: {error}"))
}

fn read_frame(reader: &mut impl BufRead, max_bytes: usize) -> io::Result<Option<Frame>> {
    let mut frame = Vec::new();
    loop {
        let available = reader.fill_buf()?;
        if available.is_empty() {
            return if frame.is_empty() {
                Ok(None)
            } else {
                Ok(Some(Frame::Complete(frame)))
            };
        }
        let newline = available.iter().position(|byte| *byte == b'\n');
        let take = newline.map_or(available.len(), |position| position + 1);
        if frame.len().saturating_add(take) > max_bytes {
            reader.consume(take);
            if newline.is_none() {
                discard_until_newline(reader)?;
            }
            return Ok(Some(Frame::TooLarge));
        }
        frame.extend_from_slice(&available[..take]);
        reader.consume(take);
        if newline.is_some() {
            while matches!(frame.last(), Some(b'\n' | b'\r')) {
                frame.pop();
            }
            return Ok(Some(Frame::Complete(frame)));
        }
    }
}

fn discard_until_newline(reader: &mut impl BufRead) -> io::Result<()> {
    loop {
        let available = reader.fill_buf()?;
        if available.is_empty() {
            return Ok(());
        }
        if let Some(position) = available.iter().position(|byte| *byte == b'\n') {
            reader.consume(position + 1);
            return Ok(());
        }
        let len = available.len();
        reader.consume(len);
    }
}

fn bounded_text(value: &str, max_bytes: usize) -> String {
    if value.len() <= max_bytes {
        return value.to_string();
    }
    let mut end = max_bytes;
    while end > 0 && !value.is_char_boundary(end) {
        end -= 1;
    }
    value[..end].to_string()
}

fn self_test() -> Result<(), String> {
    let manifest = capability_manifest();
    manifest
        .validate()
        .map_err(|error| format!("capability manifest: {error}"))?;
    let tools = tool_definitions();
    let names: Vec<_> = tools
        .iter()
        .filter_map(|tool| tool.get("name").and_then(Value::as_str))
        .collect();
    if names != ["plan", "parse", "validate", "explain", "readiness", "version"] {
        return Err(format!("unexpected tool inventory: {names:?}"));
    }
    println!(
        "{}",
        json!({
            "profile": PROFILE,
            "status": "ALIVE",
            "authority": "candidate_only",
            "maxFrameBytes": MAX_FRAME_BYTES,
            "maxConcurrentRequests": 1,
            "workerIsolation": true,
            "hardDeadline": true,
            "toolCount": names.len(),
        })
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    const DOMAIN: &str = "(define (domain smoke) (:requirements :strips) \
        (:predicates (done)) (:action finish :parameters () \
        :precondition (and) :effect (done)))";
    const PROBLEM: &str = "(define (problem smoke-p) (:domain smoke) \
        (:init) (:goal (done)))";

    #[test]
    fn frame_reader_refuses_oversized_input_and_recovers_at_newline() {
        let mut input = vec![b'x'; 9];
        input.extend_from_slice(b"\n{}\n");
        let mut cursor = Cursor::new(input);
        assert!(matches!(read_frame(&mut cursor, 8).unwrap(), Some(Frame::TooLarge)));
        assert!(matches!(
            read_frame(&mut cursor, 8).unwrap(),
            Some(Frame::Complete(bytes)) if bytes == b"{}"
        ));
    }

    #[test]
    fn notifications_emit_no_response() {
        let request = JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            id: None,
            method: "initialized".to_string(),
            params: Value::Null,
        };
        assert!(handle_request(request).is_none());
    }

    #[test]
    fn unknown_methods_are_typed_refusals() {
        let response = handle_request(JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            id: Some(json!(1)),
            method: "shell/exec".to_string(),
            params: Value::Null,
        })
        .unwrap();
        assert_eq!(
            response.pointer("/error/data/ferroplanCode"),
            Some(&json!("FP_UNSUPPORTED"))
        );
    }

    #[test]
    fn plan_worker_is_candidate_only_and_independently_validated() {
        let reply = worker_plan(json!({
            "domain": DOMAIN,
            "problem": PROBLEM,
            "timeout_ms": 1000
        }));
        assert!(!reply.is_error, "{reply:?}");
        let result = reply.result.unwrap();
        assert_eq!(result["authority"], "candidate_only");
        assert_eq!(result["outcome"], "solved");
        assert_eq!(result["validation"], "valid");
    }

    #[test]
    fn tool_inventory_has_no_actuation_primitive() {
        let names: Vec<_> = tool_definitions()
            .into_iter()
            .filter_map(|tool| tool["name"].as_str().map(ToOwned::to_owned))
            .collect();
        assert_eq!(
            names,
            ["plan", "parse", "validate", "explain", "readiness", "version"]
        );
        assert!(!names.iter().any(|name| {
            name.contains("exec")
                || name.contains("shell")
                || name.contains("write")
                || name.contains("network")
        }));
    }

    #[test]
    fn timeout_is_strictly_bounded() {
        assert_eq!(requested_timeout(&json!({})).unwrap(), DEFAULT_TIMEOUT_MS);
        assert!(requested_timeout(&json!({ "timeout_ms": 0 })).is_err());
        assert!(requested_timeout(&json!({ "timeout_ms": MAX_TIMEOUT_MS + 1 })).is_err());
    }
}
