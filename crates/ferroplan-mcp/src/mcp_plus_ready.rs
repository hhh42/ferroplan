//! Compile-safe bounded MCP+ production profile.
//!
//! This binary is deliberately smaller than the compatibility server. It
//! exposes fixed planning/evidence tools, grants no actuation authority, and
//! process-isolates every computational call behind a hard deadline.

use std::io::{self, BufRead, BufReader, Read, Write};
use std::process::{Command, Stdio};
use std::thread;
use std::time::Duration;

use ferroplan::{
    capability_manifest, explain_production, solve_production, Options, OutcomeClass, Plan,
    ProductionLimits,
};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use wait_timeout::ChildExt;

const PROFILE: &str = "ferroplan-mcp-plus/1.0";
const MAX_FRAME: usize = 8 * 1024 * 1024;
const MAX_RESPONSE: usize = 16 * 1024 * 1024;
const MAX_MODEL: usize = 4 * 1024 * 1024;
const MAX_PLAN: usize = 4 * 1024 * 1024;
const DEFAULT_TIMEOUT_MS: u64 = 5_000;
const MAX_TIMEOUT_MS: u64 = 60_000;

#[derive(Debug)]
enum Frame {
    Data(Vec<u8>),
    TooLarge,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Request {
    jsonrpc: String,
    #[serde(default)]
    id: Option<Value>,
    method: String,
    #[serde(default)]
    params: Value,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Call {
    name: String,
    #[serde(default)]
    arguments: Value,
}

#[derive(Deserialize)]
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

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ParseArgs {
    pddl: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ValidateArgs {
    domain: String,
    problem: String,
    plan: String,
}

#[derive(Deserialize)]
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
struct Failure {
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
    error: Option<Failure>,
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
            error: Some(Failure {
                code: code.to_string(),
                message: truncate(&message.into(), 2_048),
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
                "errorCode": "FP_ADAPTER",
                "message": truncate(&error, 2_048)
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

    while let Some(frame) = read_frame(&mut reader, MAX_FRAME)
        .map_err(|error| format!("reading frame: {error}"))?
    {
        let response = match frame {
            Frame::TooLarge => Some(rpc_error(
                Value::Null,
                -32_600,
                "request frame exceeds the MCP+ limit",
                "FP_LIMIT_INPUT",
                false,
            )),
            Frame::Data(bytes) if bytes.iter().all(u8::is_ascii_whitespace) => None,
            Frame::Data(bytes) => match serde_json::from_slice::<Request>(&bytes) {
                Ok(request) => handle(request),
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
                    "outcome": if response.get("error").is_some() { "refused" } else { "success" }
                })
            );
        }
    }
    Ok(())
}

fn handle(request: Request) -> Option<Value> {
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
        _ => Err(Failure {
            code: "FP_UNSUPPORTED".to_string(),
            message: format!("unsupported method `{}`", truncate(&request.method, 128)),
            retryable: false,
        }),
    };

    Some(match result {
        Ok(value) => json!({ "jsonrpc": "2.0", "id": id, "result": value }),
        Err(error) => rpc_error(
            id,
            -32_601,
            error.message,
            &error.code,
            error.retryable,
        ),
    })
}

fn call_tool(params: Value) -> Result<Value, Failure> {
    let call = serde_json::from_value::<Call>(params).map_err(|error| invalid(error.to_string()))?;
    match call.name.as_str() {
        "readiness" => return readiness().map(|value| tool_result(value, false)),
        "version" => return Ok(tool_result(version(), false)),
        "plan" | "parse" | "validate" | "explain" => {}
        _ => {
            return Err(Failure {
                code: "FP_UNSUPPORTED".to_string(),
                message: format!("unsupported MCP+ tool `{}`", truncate(&call.name, 128)),
                retryable: false,
            })
        }
    }

    let timeout = call
        .arguments
        .get("timeout_ms")
        .and_then(Value::as_u64)
        .unwrap_or(DEFAULT_TIMEOUT_MS);
    if timeout == 0 || timeout > MAX_TIMEOUT_MS {
        return Err(invalid(format!(
            "timeout_ms must be in 1..={MAX_TIMEOUT_MS}"
        )));
    }
    if timeout == 1 {
        return Err(Failure {
            code: "FP_TIMEOUT".to_string(),
            message: "deadline expired before worker dispatch".to_string(),
            retryable: true,
        });
    }

    let mut arguments = call.arguments;
    if let Some(object) = arguments.as_object_mut() {
        object.remove("timeout_ms");
    }
    let reply = invoke_worker(&call.name, &arguments, timeout)?;
    if let Some(error) = reply.error {
        return Err(error);
    }
    Ok(tool_result(
        reply.result.unwrap_or(Value::Null),
        reply.is_error,
    ))
}

fn initialize() -> Value {
    json!({
        "protocolProfile": PROFILE,
        "serverInfo": { "name": "ferroplan-mcp-plus", "version": env!("CARGO_PKG_VERSION") },
        "capabilities": { "tools": { "listChanged": false } },
        "authority": "candidate_only",
        "authorityNotice": "Planning grants no actuation authority; BRCE, POWL, OCEL, Truex receipt/refusal, and replay remain downstream.",
        "limits": {
            "maxFrameBytes": MAX_FRAME,
            "maxResponseBytes": MAX_RESPONSE,
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

fn readiness() -> Result<Value, Failure> {
    let manifest = capability_manifest();
    let fingerprint = manifest.fingerprint().map_err(|error| Failure {
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
        "admissionNotice": "Only the exact-source evidence evaluator may derive ADMITTED.",
        "manifest": manifest
    }))
}

fn tool_definitions() -> Vec<Value> {
    ["plan", "parse", "validate", "explain", "readiness", "version"]
        .into_iter()
        .map(|name| {
            json!({
                "name": name,
                "description": format!("ferroplan MCP+ {name} capability"),
                "inputSchema": { "type": "object", "x-ferroplan-maxFrameBytes": MAX_FRAME }
            })
        })
        .collect()
}

fn invoke_worker(tool: &str, arguments: &Value, timeout_ms: u64) -> Result<WorkerReply, Failure> {
    let input = serde_json::to_vec(arguments).map_err(|error| adapter(error.to_string()))?;
    if input.len() > MAX_FRAME {
        return Err(Failure {
            code: "FP_LIMIT_INPUT".to_string(),
            message: "worker request exceeds the frame limit".to_string(),
            retryable: false,
        });
    }

    let executable = std::env::current_exe().map_err(|error| adapter(error.to_string()))?;
    let mut child = Command::new(executable)
        .arg("--worker")
        .arg(tool)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|error| adapter(format!("spawning worker: {error}")))?;

    let stdout = child
        .stdout
        .take()
        .ok_or_else(|| adapter("worker stdout unavailable"))?;
    let reader = thread::spawn(move || {
        let mut output = Vec::new();
        stdout
            .take((MAX_RESPONSE + 1) as u64)
            .read_to_end(&mut output)
            .map(|_| output)
    });

    if let Some(mut stdin) = child.stdin.take() {
        if let Err(error) = stdin.write_all(&input) {
            let _ = child.kill();
            let _ = child.wait();
            let _ = reader.join();
            return Err(adapter(format!("writing worker input: {error}")));
        }
    }

    if child
        .wait_timeout(Duration::from_millis(timeout_ms))
        .map_err(|error| adapter(format!("waiting for worker: {error}")))?
        .is_none()
    {
        let _ = child.kill();
        let _ = child.wait();
        let _ = reader.join();
        return Err(Failure {
            code: "FP_TIMEOUT".to_string(),
            message: format!("worker exceeded the {timeout_ms} ms deadline"),
            retryable: true,
        });
    }

    let output = reader
        .join()
        .map_err(|_| adapter("worker output reader panicked"))?
        .map_err(|error| adapter(format!("reading worker output: {error}")))?;
    if output.len() > MAX_RESPONSE {
        return Err(Failure {
            code: "FP_LIMIT_OUTPUT".to_string(),
            message: "worker response exceeds the output limit".to_string(),
            retryable: true,
        });
    }
    serde_json::from_slice(&output)
        .map_err(|error| adapter(format!("decoding worker output: {error}")))
}

fn worker(tool: &str) -> Result<(), String> {
    let mut bytes = Vec::new();
    io::stdin()
        .take((MAX_FRAME + 1) as u64)
        .read_to_end(&mut bytes)
        .map_err(|error| format!("reading worker input: {error}"))?;

    let reply = if bytes.len() > MAX_FRAME {
        WorkerReply::failure("FP_LIMIT_INPUT", "worker input exceeds the frame limit", false)
    } else {
        match serde_json::from_slice::<Value>(&bytes) {
            Ok(arguments) => execute(tool, arguments),
            Err(error) => WorkerReply::failure("FP_PARSE", error.to_string(), false),
        }
    };
    let output = serde_json::to_vec(&reply)
        .map_err(|error| format!("serializing worker output: {error}"))?;
    if output.len() > MAX_RESPONSE {
        return Err("worker output exceeds the response limit".to_string());
    }
    io::stdout()
        .write_all(&output)
        .map_err(|error| format!("writing worker output: {error}"))
}

fn execute(tool: &str, arguments: Value) -> WorkerReply {
    match tool {
        "plan" => plan_worker(arguments),
        "parse" => parse_worker(arguments),
        "validate" => validate_worker(arguments),
        "explain" => explain_worker(arguments),
        _ => WorkerReply::failure("FP_UNSUPPORTED", "unsupported worker tool", false),
    }
}

fn plan_worker(arguments: Value) -> WorkerReply {
    let args = match serde_json::from_value::<PlanArgs>(arguments) {
        Ok(args) => args,
        Err(error) => return WorkerReply::failure("FP_INVALID_REQUEST", error.to_string(), false),
    };
    let envelope = solve_production(
        &args.domain,
        &args.problem,
        &args.options,
        &args.limits.unwrap_or_default(),
        args.request_id.as_deref(),
    );
    envelope_reply(envelope.outcome, envelope)
}

fn parse_worker(arguments: Value) -> WorkerReply {
    let args = match serde_json::from_value::<ParseArgs>(arguments) {
        Ok(args) => args,
        Err(error) => return WorkerReply::failure("FP_INVALID_REQUEST", error.to_string(), false),
    };
    if args.pddl.len() > MAX_MODEL {
        return WorkerReply::failure("FP_LIMIT_INPUT", "PDDL exceeds the input limit", false);
    }
    WorkerReply::success(json!({
        "schemaVersion": "ferroplan.parse-evidence.v1",
        "authority": "evidence_only",
        "result": ferroplan::parse(&args.pddl)
    }))
}

fn validate_worker(arguments: Value) -> WorkerReply {
    let args = match serde_json::from_value::<ValidateArgs>(arguments) {
        Ok(args) => args,
        Err(error) => return WorkerReply::failure("FP_INVALID_REQUEST", error.to_string(), false),
    };
    if args.domain.len() > MAX_MODEL
        || args.problem.len() > MAX_MODEL
        || args.plan.len() > MAX_PLAN
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
            "reason": truncate(&reason, 2_048)
        })),
        Err(error) => WorkerReply::failure("FP_VALIDATION", error, false),
    }
}

fn explain_worker(arguments: Value) -> WorkerReply {
    let args = match serde_json::from_value::<ExplainArgs>(arguments) {
        Ok(args) => args,
        Err(error) => return WorkerReply::failure("FP_INVALID_REQUEST", error.to_string(), false),
    };
    let envelope = explain_production(
        &args.domain,
        &args.problem,
        &args.plan,
        &args.limits.unwrap_or_default(),
        args.request_id.as_deref(),
    );
    envelope_reply(envelope.outcome, envelope)
}

fn envelope_reply<T: Serialize>(outcome: OutcomeClass, envelope: T) -> WorkerReply {
    match serde_json::to_value(envelope) {
        Ok(value)
            if matches!(
                outcome,
                OutcomeClass::Refused | OutcomeClass::Failed | OutcomeClass::LimitExceeded
            ) => WorkerReply::tool_error(value),
        Ok(value) => WorkerReply::success(value),
        Err(error) => WorkerReply::failure("FP_ADAPTER", error.to_string(), false),
    }
}

fn tool_result(value: Value, is_error: bool) -> Value {
    let text = serde_json::to_string_pretty(&value).unwrap_or_else(|_| "null".to_string());
    json!({
        "content": [{ "type": "text", "text": text }],
        "structuredContent": value,
        "isError": is_error
    })
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
            "message": truncate(&message.into(), 2_048),
            "data": {
                "ferroplanCode": ferroplan_code,
                "retryable": retryable,
                "profile": PROFILE
            }
        }
    })
}

fn invalid(message: impl Into<String>) -> Failure {
    Failure {
        code: "FP_INVALID_REQUEST".to_string(),
        message: truncate(&message.into(), 2_048),
        retryable: false,
    }
}

fn adapter(message: impl Into<String>) -> Failure {
    Failure {
        code: "FP_ADAPTER".to_string(),
        message: truncate(&message.into(), 2_048),
        retryable: true,
    }
}

fn write_response(writer: &mut impl Write, response: &Value) -> Result<(), String> {
    let mut bytes = serde_json::to_vec(response)
        .map_err(|error| format!("serializing response: {error}"))?;
    if bytes.len() > MAX_RESPONSE {
        bytes = serde_json::to_vec(&rpc_error(
            response.get("id").cloned().unwrap_or(Value::Null),
            -32_603,
            "response exceeds the output limit",
            "FP_LIMIT_OUTPUT",
            true,
        ))
        .map_err(|error| format!("serializing refusal: {error}"))?;
    }
    bytes.push(b'\n');
    writer
        .write_all(&bytes)
        .and_then(|_| writer.flush())
        .map_err(|error| format!("writing response: {error}"))
}

fn read_frame(reader: &mut impl BufRead, max: usize) -> io::Result<Option<Frame>> {
    let mut frame = Vec::new();
    loop {
        let available = reader.fill_buf()?;
        if available.is_empty() {
            return if frame.is_empty() {
                Ok(None)
            } else {
                Ok(Some(Frame::Data(frame)))
            };
        }
        let newline = available.iter().position(|byte| *byte == b'\n');
        let take = newline.map_or(available.len(), |position| position + 1);
        if frame.len().saturating_add(take) > max {
            reader.consume(take);
            if newline.is_none() {
                discard_line(reader)?;
            }
            return Ok(Some(Frame::TooLarge));
        }
        frame.extend_from_slice(&available[..take]);
        reader.consume(take);
        if newline.is_some() {
            while matches!(frame.last(), Some(b'\n' | b'\r')) {
                frame.pop();
            }
            return Ok(Some(Frame::Data(frame)));
        }
    }
}

fn discard_line(reader: &mut impl BufRead) -> io::Result<()> {
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

fn truncate(value: &str, max: usize) -> String {
    if value.len() <= max {
        return value.to_string();
    }
    let mut end = max;
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
        .into_iter()
        .filter_map(|tool| tool.get("name").and_then(Value::as_str).map(str::to_string))
        .collect::<Vec<_>>();
    let expected = ["plan", "parse", "validate", "explain", "readiness", "version"];
    if names.iter().map(String::as_str).collect::<Vec<_>>() != expected {
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
    fn oversized_frame_is_refused_and_reader_recovers() {
        let mut bytes = vec![b'x'; 9];
        bytes.extend_from_slice(b"\n{}\n");
        let mut cursor = Cursor::new(bytes);
        assert!(matches!(read_frame(&mut cursor, 8).unwrap(), Some(Frame::TooLarge)));
        assert!(matches!(
            read_frame(&mut cursor, 8).unwrap(),
            Some(Frame::Data(value)) if value == b"{}"
        ));
    }

    #[test]
    fn fixed_inventory_contains_no_actuation_primitive() {
        let names = tool_definitions()
            .into_iter()
            .filter_map(|tool| tool.get("name").and_then(Value::as_str).map(str::to_string))
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
}
