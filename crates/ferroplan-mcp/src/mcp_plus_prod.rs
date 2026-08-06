//! Bounded, process-isolated, candidate-only MCP+ production profile.
//!
//! The broad `ferroplan-mcp` compatibility server remains separate. This
//! binary exposes exactly six fixed planning/evidence tools over line-delimited
//! JSON-RPC 2.0. Computational calls execute in a fixed child-worker mode and
//! are killed and reaped at the requested hard deadline.

use std::io::{self, BufRead, BufReader, Read, Write};
use std::process::{Command, Stdio};
use std::thread;
use std::time::{Duration, Instant};

use ferroplan::{
    capability_manifest, explain_production, solve_production, Options, OutcomeClass, Plan,
    ProductionLimits,
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
const DEFAULT_TIMEOUT_MS: u64 = 5_000;
const MAX_TIMEOUT_MS: u64 = 60_000;
const MIN_EXECUTABLE_TIMEOUT_MS: u64 = 2;

#[derive(Debug)]
enum Frame {
    Complete(Vec<u8>),
    TooLarge,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Request {
    jsonrpc: String,
    #[serde(default)]
    id: Option<Value>,
    method: String,
    #[serde(default)]
    params: Value,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct ToolCall {
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
    options: Options,
    #[serde(default)]
    limits: Option<ProductionLimits>,
    #[serde(default)]
    request_id: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct ParseArgs {
    pddl: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct ValidateArgs {
    domain: String,
    problem: String,
    plan: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct ExplainArgs {
    domain: String,
    problem: String,
    plan: Plan,
    #[serde(default)]
    limits: Option<ProductionLimits>,
    #[serde(default)]
    request_id: Option<String>,
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
                message: bounded(&message.into(), 2_048),
                retryable,
            }),
        }
    }
}

fn main() {
    let args = std::env::args().collect::<Vec<_>>();
    let result = match args.get(1).map(String::as_str) {
        None => serve(),
        Some("--worker") => worker(args.get(2).map(String::as_str).unwrap_or("")),
        Some("--self-test") if args.len() == 2 => self_test(),
        _ => Err("unsupported arguments; use no arguments or --self-test".to_string()),
    };
    if let Err(error) = result {
        eprintln!(
            "{}",
            json!({
                "event": "mcp_plus.process.failed",
                "profile": PROFILE,
                "errorCode": "FP_ADAPTER",
                "message": bounded(&error, 2_048)
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

    while let Some(frame) = read_frame(&mut reader, MAX_FRAME_BYTES)
        .map_err(|error| format!("reading JSON-RPC frame: {error}"))?
    {
        let started = Instant::now();
        let response = match frame {
            Frame::TooLarge => Some(rpc_error(
                Value::Null,
                -32_600,
                "request frame exceeds the MCP+ limit",
                "FP_LIMIT_INPUT",
                false,
            )),
            Frame::Complete(bytes) if bytes.iter().all(u8::is_ascii_whitespace) => None,
            Frame::Complete(bytes) => match serde_json::from_slice::<Request>(&bytes) {
                Ok(request) => handle_request(request),
                Err(error) => Some(rpc_error(
                    Value::Null,
                    -32_700,
                    format!("invalid JSON-RPC request: {error}"),
                    "FP_PARSE",
                    false,
                )),
            },
        };
        if let Some(response) = response {
            write_response(&mut writer, &response)?;
            eprintln!(
                "{}",
                json!({
                    "event": "mcp_plus.request.completed",
                    "profile": PROFILE,
                    "outcome": response_outcome(&response),
                    "elapsedMicros": started.elapsed().as_micros().min(u64::MAX as u128) as u64
                })
            );
        }
    }
    Ok(())
}

fn handle_request(request: Request) -> Option<Value> {
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

    let result = match request.method.as_str() {
        "initialize" => Ok(initialize()),
        "tools/list" => Ok(json!({ "tools": tool_definitions() })),
        "tools/call" => call_tool(request.params),
        "ferroplan/version" => Ok(version()),
        "ferroplan/readiness" => readiness(),
        _ => Err(WorkerError {
            code: "FP_UNSUPPORTED".to_string(),
            message: format!("unsupported method `{}`", bounded(&request.method, 128)),
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

fn call_tool(params: Value) -> Result<Value, WorkerError> {
    let call = serde_json::from_value::<ToolCall>(params).map_err(|error| WorkerError {
        code: "FP_INVALID_REQUEST".to_string(),
        message: format!("invalid tools/call parameters: {error}"),
        retryable: false,
    })?;

    match call.name.as_str() {
        "readiness" => return readiness().map(tool_success),
        "version" => return Ok(tool_success(version())),
        "plan" | "parse" | "validate" | "explain" => {}
        _ => {
            return Err(WorkerError {
                code: "FP_UNSUPPORTED".to_string(),
                message: format!("unsupported MCP+ tool `{}`", bounded(&call.name, 128)),
                retryable: false,
            })
        }
    }

    let timeout_ms = requested_timeout(&call.arguments)?;
    if timeout_ms < MIN_EXECUTABLE_TIMEOUT_MS {
        return Err(WorkerError {
            code: "FP_TIMEOUT".to_string(),
            message: format!("requested deadline of {timeout_ms} ms expired before dispatch"),
            retryable: true,
        });
    }

    let mut arguments = call.arguments;
    if let Some(object) = arguments.as_object_mut() {
        object.remove("timeout_ms");
    }
    let reply = run_worker(&call.name, &arguments, timeout_ms)?;
    if let Some(error) = reply.error {
        return Err(error);
    }
    let value = reply.result.unwrap_or(Value::Null);
    Ok(if reply.is_error {
        tool_error(value)
    } else {
        tool_success(value)
    })
}

fn initialize() -> Value {
    json!({
        "protocolProfile": PROFILE,
        "serverInfo": {
            "name": "ferroplan-mcp-plus",
            "version": env!("CARGO_PKG_VERSION")
        },
        "capabilities": { "tools": { "listChanged": false } },
        "authority": "candidate_only",
        "authorityNotice": "Planning grants no actuation authority; BRCE, observed consequence, POWL conformance, OCEL evidence, Truex receipt/refusal, and replay remain downstream obligations.",
        "limits": {
            "maxFrameBytes": MAX_FRAME_BYTES,
            "maxResponseBytes": MAX_RESPONSE_BYTES,
            "defaultTimeoutMs": DEFAULT_TIMEOUT_MS,
            "maxTimeoutMs": MAX_TIMEOUT_MS,
            "maxConcurrentRequests": 1,
            "workerIsolation": true
        }
    })
}

fn version() -> Value {
    json!({
        "profile": PROFILE,
        "productVersion": env!("CARGO_PKG_VERSION"),
        "capabilityManifestSchema": ferroplan::CAPABILITY_MANIFEST_SCHEMA,
        "operationEnvelopeSchema": ferroplan::OPERATION_ENVELOPE_SCHEMA,
        "authority": "candidate_only"
    })
}

fn readiness() -> Result<Value, WorkerError> {
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
        "manifest": manifest
    }))
}

fn tool_definitions() -> Vec<Value> {
    [
        ("plan", "Bounded candidate-only PDDL planning with a hard worker deadline."),
        ("parse", "Bounded PDDL syntax and structure evidence."),
        ("validate", "Independent bounded plan validation."),
        ("explain", "Bounded explanation for an independently validated plan."),
        ("readiness", "Canonical capability contract without self-authored admission."),
        ("version", "Exact product and schema versions."),
    ]
    .into_iter()
    .map(|(name, description)| {
        json!({
            "name": name,
            "description": description,
            "inputSchema": {
                "type": "object",
                "x-ferroplan-maxFrameBytes": MAX_FRAME_BYTES
            }
        })
    })
    .collect()
}

fn requested_timeout(arguments: &Value) -> Result<u64, WorkerError> {
    let timeout = arguments
        .get("timeout_ms")
        .and_then(Value::as_u64)
        .unwrap_or(DEFAULT_TIMEOUT_MS);
    if timeout == 0 || timeout > MAX_TIMEOUT_MS {
        return Err(WorkerError {
            code: "FP_INVALID_REQUEST".to_string(),
            message: format!("timeout_ms must be in 1..={MAX_TIMEOUT_MS}"),
            retryable: false,
        });
    }
    Ok(timeout)
}

fn run_worker(tool: &str, arguments: &Value, timeout_ms: u64) -> Result<WorkerReply, WorkerError> {
    let input = serde_json::to_vec(arguments).map_err(|error| adapter_error(error.to_string()))?;
    if input.len() > MAX_FRAME_BYTES {
        return Err(WorkerError {
            code: "FP_LIMIT_INPUT".to_string(),
            message: "worker request exceeds the MCP+ frame limit".to_string(),
            retryable: false,
        });
    }

    let executable = std::env::current_exe().map_err(|error| adapter_error(error.to_string()))?;
    let mut child = Command::new(executable)
        .arg("--worker")
        .arg(tool)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|error| adapter_error(format!("spawning worker: {error}")))?;

    let stdout = child
        .stdout
        .take()
        .ok_or_else(|| adapter_error("worker stdout was unavailable"))?;
    let reader = thread::spawn(move || {
        let mut output = Vec::new();
        stdout
            .take((MAX_RESPONSE_BYTES + 1) as u64)
            .read_to_end(&mut output)
            .map(|_| output)
    });

    if let Some(mut stdin) = child.stdin.take() {
        if let Err(error) = stdin.write_all(&input) {
            let _ = child.kill();
            let _ = child.wait();
            let _ = reader.join();
            return Err(adapter_error(format!("writing worker request: {error}")));
        }
    }

    let status = child
        .wait_timeout(Duration::from_millis(timeout_ms))
        .map_err(|error| adapter_error(format!("waiting for worker: {error}")))?;
    if status.is_none() {
        let _ = child.kill();
        let _ = child.wait();
        let _ = reader.join();
        return Err(WorkerError {
            code: "FP_TIMEOUT".to_string(),
            message: format!("worker exceeded the {timeout_ms} ms deadline"),
            retryable: true,
        });
    }

    let output = reader
        .join()
        .map_err(|_| adapter_error("worker output reader panicked"))?
        .map_err(|error| adapter_error(format!("reading worker output: {error}")))?;
    if output.len() > MAX_RESPONSE_BYTES {
        return Err(WorkerError {
            code: "FP_LIMIT_OUTPUT".to_string(),
            message: "worker response exceeds the MCP+ output limit".to_string(),
            retryable: true,
        });
    }
    serde_json::from_slice(&output)
        .map_err(|error| adapter_error(format!("decoding worker response: {error}")))
}

fn worker(tool: &str) -> Result<(), String> {
    let mut bytes = Vec::new();
    io::stdin()
        .take((MAX_FRAME_BYTES + 1) as u64)
        .read_to_end(&mut bytes)
        .map_err(|error| format!("reading worker input: {error}"))?;

    let reply = if bytes.len() > MAX_FRAME_BYTES {
        WorkerReply::failure("FP_LIMIT_INPUT", "worker input exceeds the frame limit", false)
    } else {
        match serde_json::from_slice::<Value>(&bytes) {
            Ok(arguments) => execute_worker(tool, arguments),
            Err(error) => WorkerReply::failure(
                "FP_PARSE",
                format!("invalid worker JSON: {error}"),
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

fn execute_worker(tool: &str, arguments: Value) -> WorkerReply {
    match tool {
        "plan" => worker_plan(arguments),
        "parse" => worker_parse(arguments),
        "validate" => worker_validate(arguments),
        "explain" => worker_explain(arguments),
        _ => WorkerReply::failure(
            "FP_UNSUPPORTED",
            format!("unsupported worker tool `{}`", bounded(tool, 128)),
            false,
        ),
    }
}

fn worker_plan(arguments: Value) -> WorkerReply {
    let args = match serde_json::from_value::<PlanArgs>(arguments) {
        Ok(args) => args,
        Err(error) => {
            return WorkerReply::failure(
                "FP_INVALID_REQUEST",
                format!("invalid plan arguments: {error}"),
                false,
            )
        }
    };
    let limits = args.limits.unwrap_or_default();
    let envelope = solve_production(
        &args.domain,
        &args.problem,
        &args.options,
        &limits,
        args.request_id.as_deref(),
    );
    let is_error = matches!(
        envelope.outcome,
        OutcomeClass::Refused | OutcomeClass::Failed | OutcomeClass::LimitExceeded
    );
    match serde_json::to_value(envelope) {
        Ok(value) if is_error => WorkerReply::tool_error(value),
        Ok(value) => WorkerReply::success(value),
        Err(error) => WorkerReply::failure("FP_ADAPTER", error.to_string(), false),
    }
}

fn worker_parse(arguments: Value) -> WorkerReply {
    let args = match serde_json::from_value::<ParseArgs>(arguments) {
        Ok(args) => args,
        Err(error) => {
            return WorkerReply::failure(
                "FP_INVALID_REQUEST",
                format!("invalid parse arguments: {error}"),
                false,
            )
        }
    };
    if args.pddl.len() > MAX_DOMAIN_BYTES {
        return WorkerReply::failure("FP_LIMIT_INPUT", "PDDL exceeds the input limit", false);
    }
    WorkerReply::success(json!({
        "schemaVersion": "ferroplan.parse-evidence.v1",
        "authority": "evidence_only",
        "result": ferroplan::parse(&args.pddl)
    }))
}

fn worker_validate(arguments: Value) -> WorkerReply {
    let args = match serde_json::from_value::<ValidateArgs>(arguments) {
        Ok(args) => args,
        Err(error) => {
            return WorkerReply::failure(
                "FP_INVALID_REQUEST",
                format!("invalid validate arguments: {error}"),
                false,
            )
        }
    };
    if args.domain.len() > MAX_DOMAIN_BYTES
        || args.problem.len() > MAX_PROBLEM_BYTES
        || args.plan.len() > MAX_PLAN_BYTES
    {
        return WorkerReply::failure("FP_LIMIT_INPUT", "validation input exceeds limits", false);
    }
    match ferroplan::plan::validate_plan(&args.domain, &args.problem, &args.plan) {
        Ok(ferroplan::plan::Validity::Valid) => WorkerReply::success(json!({
            "schemaVersion": "ferroplan.plan-validation.v1",
            "authority": "evidence_only",
            "valid": true
        })),
        Ok(ferroplan::plan::Validity::Invalid(reason)) => WorkerReply::success(json!({
            "schemaVersion": "ferroplan.plan-validation.v1",
            "authority": "evidence_only",
            "valid": false,
            "reason": bounded(&reason, 2_048)
        })),
        Err(error) => WorkerReply::failure("FP_VALIDATION", error, false),
    }
}

fn worker_explain(arguments: Value) -> WorkerReply {
    let args = match serde_json::from_value::<ExplainArgs>(arguments) {
        Ok(args) => args,
        Err(error) => {
            return WorkerReply::failure(
                "FP_INVALID_REQUEST",
                format!("invalid explain arguments: {error}"),
                false,
            )
        }
    };
    let envelope = explain_production(
        &args.domain,
        &args.problem,
        &args.plan,
        &args.limits.unwrap_or_default(),
        args.request_id.as_deref(),
    );
    let is_error = envelope.outcome != OutcomeClass::Solved;
    match serde_json::to_value(envelope) {
        Ok(value) if is_error => WorkerReply::tool_error(value),
        Ok(value) => WorkerReply::success(value),
        Err(error) => WorkerReply::failure("FP_ADAPTER", error.to_string(), false),
    }
}

fn tool_success(value: Value) -> Value {
    tool_result(value, false)
}

fn tool_error(value: Value) -> Value {
    tool_result(value, true)
}

fn tool_result(value: Value, is_error: bool) -> Value {
    let text = serde_json::to_string_pretty(&value).unwrap_or_else(|_| "null".to_string());
    json!({
        "content": [{ "type": "text", "text": text }],
        "structuredContent": value,
        "isError": is_error
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
            "message": bounded(&message.into(), 2_048),
            "data": {
                "ferroplanCode": ferroplan_code,
                "retryable": retryable,
                "profile": PROFILE
            }
        }
    })
}

fn response_outcome(response: &Value) -> &'static str {
    if response.get("error").is_some() {
        "refused"
    } else if response
        .pointer("/result/isError")
        .and_then(Value::as_bool)
        .unwrap_or(false)
    {
        "tool_error"
    } else {
        "success"
    }
}

fn adapter_error(message: impl Into<String>) -> WorkerError {
    WorkerError {
        code: "FP_ADAPTER".to_string(),
        message: bounded(&message.into(), 2_048),
        retryable: true,
    }
}

fn write_response(writer: &mut impl Write, response: &Value) -> Result<(), String> {
    let mut bytes = serde_json::to_vec(response)
        .map_err(|error| format!("serializing JSON-RPC response: {error}"))?;
    if bytes.len() > MAX_RESPONSE_BYTES {
        bytes = serde_json::to_vec(&rpc_error(
            response.get("id").cloned().unwrap_or(Value::Null),
            -32_603,
            "response exceeds the MCP+ output limit",
            "FP_LIMIT_OUTPUT",
            true,
        ))
        .map_err(|error| format!("serializing output-limit response: {error}"))?;
    }
    bytes.push(b'\n');
    writer
        .write_all(&bytes)
        .and_then(|_| writer.flush())
        .map_err(|error| format!("writing JSON-RPC response: {error}"))
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
        let length = available.len();
        reader.consume(length);
    }
}

fn bounded(value: &str, max_bytes: usize) -> String {
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
    capability_manifest()
        .validate()
        .map_err(|error| format!("capability manifest: {error}"))?;
    let names = tool_definitions()
        .iter()
        .filter_map(|tool| tool.get("name").and_then(Value::as_str))
        .collect::<Vec<_>>();
    let expected = ["plan", "parse", "validate", "explain", "readiness", "version"];
    if names != expected {
        return Err(format!("unexpected tool inventory: {names:?}"));
    }
    println!(
        "{}",
        json!({
            "profile": PROFILE,
            "status": "ALIVE",
            "authority": "candidate_only",
            "toolCount": names.len(),
            "workerIsolation": true,
            "hardDeadline": true,
            "maxConcurrentRequests": 1
        })
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    #[test]
    fn frame_limit_refuses_and_recovers() {
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
    fn inventory_contains_no_actuation_tool() {
        let names = tool_definitions()
            .into_iter()
            .filter_map(|tool| tool["name"].as_str().map(ToOwned::to_owned))
            .collect::<Vec<_>>();
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
    fn sub_two_millisecond_deadline_is_a_typed_timeout() {
        let error = call_tool(json!({
            "name": "plan",
            "arguments": { "timeout_ms": 1 }
        }))
        .unwrap_err();
        assert_eq!(error.code, "FP_TIMEOUT");
    }
}
