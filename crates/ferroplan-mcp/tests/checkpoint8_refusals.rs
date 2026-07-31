use serde_json::{json, Value};
use std::io::{BufRead, BufReader, Write};
use std::process::{Child, ChildStdout, Command, Stdio};

const BCINR_REVISION: &str = "fb9321d27882169acc83aaca0639b319cd3b7900";

fn spawn_and_handshake() -> (Child, BufReader<ChildStdout>) {
    let bin = env!("CARGO_BIN_EXE_ferroplan-mcp");
    let mut child = Command::new(bin)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .expect("spawn ferroplan-mcp");
    let stdout = child.stdout.take().expect("stdout");
    let mut reader = BufReader::new(stdout);
    let initialize = json!({
        "jsonrpc": "2.0",
        "id": 0,
        "method": "initialize",
        "params": {
            "protocolVersion": "2025-06-18",
            "capabilities": {},
            "clientInfo": {"name": "checkpoint8-refusals", "version": "0"}
        }
    });
    writeln!(child.stdin.as_mut().unwrap(), "{initialize}").unwrap();
    let mut line = String::new();
    reader.read_line(&mut line).unwrap();
    let initialized = json!({"jsonrpc": "2.0", "method": "notifications/initialized"});
    writeln!(child.stdin.as_mut().unwrap(), "{initialized}").unwrap();
    (child, reader)
}

fn tool_call(id: i64, name: &str, arguments: Value) -> Value {
    json!({
        "jsonrpc": "2.0",
        "id": id,
        "method": "tools/call",
        "params": {"name": name, "arguments": arguments}
    })
}

fn call(child: &mut Child, stdout: &mut BufReader<ChildStdout>, request: &Value) -> Value {
    writeln!(child.stdin.as_mut().unwrap(), "{request}").unwrap();
    let mut line = String::new();
    stdout.read_line(&mut line).unwrap();
    serde_json::from_str(&line).unwrap()
}

fn is_error(response: &Value) -> bool {
    response["result"]["isError"].as_bool().unwrap_or(false)
}

fn tool_text(response: &Value) -> &str {
    response["result"]["content"][0]["text"]
        .as_str()
        .unwrap_or("")
}

fn candidates_of_shape(count: usize, factors: usize) -> Vec<Value> {
    (0..count)
        .map(|index| {
            json!({
                "id": format!("node-{index}"),
                "parent": if index == 0 { Value::Null } else { json!(0) },
                "factors": vec![0.5_f64; factors],
                "cost": 1.0
            })
        })
        .collect()
}

fn allocation_result() -> Value {
    json!({
        "payload": {
            "bcinr_revision": BCINR_REVISION,
            "allocations": (0..8).collect::<Vec<usize>>()
        }
    })
}

#[test]
fn cmca_allocate_rejects_seven_candidates() {
    let (mut child, mut stdout) = spawn_and_handshake();
    let response = call(
        &mut child,
        &mut stdout,
        &tool_call(1, "cmca_allocate", json!({"candidates": candidates_of_shape(7, 10)})),
    );
    assert!(is_error(&response), "{response:?}");
    let text = tool_text(&response);
    assert!(text.contains('7') && text.contains('8'), "{text}");
}

#[test]
fn cmca_allocate_rejects_nine_candidates() {
    let (mut child, mut stdout) = spawn_and_handshake();
    let response = call(
        &mut child,
        &mut stdout,
        &tool_call(1, "cmca_allocate", json!({"candidates": candidates_of_shape(9, 10)})),
    );
    assert!(is_error(&response), "{response:?}");
    let text = tool_text(&response);
    assert!(text.contains('9') && text.contains('8'), "{text}");
}

#[test]
fn cmca_allocate_rejects_wrong_factor_count() {
    let (mut child, mut stdout) = spawn_and_handshake();
    let response = call(
        &mut child,
        &mut stdout,
        &tool_call(1, "cmca_allocate", json!({"candidates": candidates_of_shape(8, 9)})),
    );
    assert!(is_error(&response), "{response:?}");
    assert!(tool_text(&response).contains("10 factors"));
}

#[test]
fn cmca_allocate_is_deterministic_across_processes() {
    let candidates = candidates_of_shape(8, 10);
    let (mut child1, mut stdout1) = spawn_and_handshake();
    let first = call(
        &mut child1,
        &mut stdout1,
        &tool_call(1, "cmca_allocate", json!({"candidates": candidates.clone()})),
    );
    let (mut child2, mut stdout2) = spawn_and_handshake();
    let second = call(
        &mut child2,
        &mut stdout2,
        &tool_call(1, "cmca_allocate", json!({"candidates": candidates})),
    );
    let first_content = &first["result"]["structuredContent"];
    let second_content = &second["result"]["structuredContent"];
    assert_eq!(first_content["payload_digest"], second_content["payload_digest"]);
    assert_eq!(first_content["payload"], second_content["payload"]);
}

#[test]
fn verify_rejects_tampered_allocation_payload() {
    let (mut child, mut stdout) = spawn_and_handshake();
    let bound = call(
        &mut child,
        &mut stdout,
        &tool_call(
            1,
            "bind_allocation_receipt",
            json!({
                "candidates": [1,2,3,4,5,6,7,8],
                "allocation_result": allocation_result(),
                "observation_frontier": {"frontier": "empty"}
            }),
        ),
    );
    let mut envelope = bound["result"]["structuredContent"].clone();
    envelope["payload"]["allocation_result"]["payload"]["allocations"] =
        json!([0,0,0,0,0,0,0,0]);
    let verified = call(
        &mut child,
        &mut stdout,
        &tool_call(2, "verify_receipt", json!({"envelope": envelope})),
    );
    let result = &verified["result"]["structuredContent"];
    assert_eq!(result["valid"], false);
    assert_eq!(result["payload_digest_valid"], false);
}
