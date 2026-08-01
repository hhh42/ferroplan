use serde_json::{json, Value};
use std::io::Write;
use std::process::{Command, Stdio};

const BCINR_REVISION: &str = "fb9321d27882169acc83aaca0639b319cd3b7900";

fn raw_drive(messages: &[Value]) -> Vec<Value> {
    let bin = env!("CARGO_BIN_EXE_ferroplan-mcp");
    let mut child = Command::new(bin)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .expect("spawn ferroplan-mcp");
    {
        let stdin = child.stdin.as_mut().expect("stdin");
        for message in messages {
            writeln!(stdin, "{message}").expect("write message");
        }
    }
    let out = child.wait_with_output().expect("wait");
    assert!(out.status.success(), "server exited with {:?}", out.status);
    String::from_utf8(out.stdout)
        .expect("utf8")
        .lines()
        .filter(|line| !line.trim().is_empty())
        .map(|line| serde_json::from_str(line).expect("JSON response line"))
        .collect()
}

fn drive(messages: &[Value]) -> Vec<Value> {
    let mut all = vec![
        json!({
            "jsonrpc": "2.0",
            "id": "__handshake__",
            "method": "initialize",
            "params": {
                "protocolVersion": "2025-06-18",
                "capabilities": {},
                "clientInfo": {"name": "recursive-admission-test", "version": "0"}
            }
        }),
        json!({"jsonrpc": "2.0", "method": "notifications/initialized"}),
    ];
    all.extend_from_slice(messages);
    let mut responses = raw_drive(&all);
    responses.remove(0);
    responses
}

fn tool_call(id: i64, name: &str, arguments: Value) -> Value {
    json!({
        "jsonrpc": "2.0",
        "id": id,
        "method": "tools/call",
        "params": {"name": name, "arguments": arguments}
    })
}

fn find_response(responses: &[Value], id: i64) -> Value {
    responses
        .iter()
        .find(|value| value["id"] == json!(id))
        .unwrap_or_else(|| panic!("no response {id}: {responses:?}"))
        .clone()
}

fn structured(response: &Value) -> Value {
    assert_eq!(response["result"]["isError"], false, "{response:?}");
    response["result"]["structuredContent"].clone()
}

fn eight_candidates_with_ids(prefix: &str) -> Value {
    json!((0..8)
        .map(|index| json!({"id": format!("{prefix}-{index}")}))
        .collect::<Vec<Value>>())
}

fn allocation_result() -> Value {
    json!({
        "payload": {
            "bcinr_revision": BCINR_REVISION,
            "allocations": (0..8).collect::<Vec<usize>>()
        }
    })
}

fn parent_envelope() -> Value {
    let responses = drive(&[tool_call(
        1,
        "bind_allocation_receipt",
        json!({
            "candidates": eight_candidates_with_ids("root"),
            "allocation_result": allocation_result(),
            "observation_frontier": {"frontier": "empty"}
        }),
    )]);
    structured(&find_response(&responses, 1))
}

#[test]
fn recursive_descent_happy_path() {
    let parent = parent_envelope();
    let responses = drive(&[tool_call(
        1,
        "bind_allocation_receipt",
        json!({
            "candidates": eight_candidates_with_ids("child"),
            "allocation_result": allocation_result(),
            "observation_frontier": {"frontier": "root-3 descent"},
            "parent_allocation": parent,
            "selected_node": "root-3"
        }),
    )]);
    let child = structured(&find_response(&responses, 1));
    assert_eq!(child["payload"]["selected_node"], "root-3");
    assert_eq!(
        child["payload"]["selected_node_candidate"],
        json!({"id": "root-3"})
    );
    assert_eq!(
        child["payload"]["parent_allocation_receipt"],
        parent["receipt"]
    );
}

#[test]
fn recursive_descent_rejects_tampered_parent_receipt() {
    let mut parent = parent_envelope();
    parent["receipt"] = json!("0".repeat(64));
    let responses = drive(&[tool_call(
        1,
        "bind_allocation_receipt",
        json!({
            "candidates": eight_candidates_with_ids("child"),
            "allocation_result": allocation_result(),
            "observation_frontier": {},
            "parent_allocation": parent,
            "selected_node": "root-0"
        }),
    )]);
    let response = find_response(&responses, 1);
    assert_eq!(response["result"]["isError"], true);
    assert!(response["result"]["content"][0]["text"]
        .as_str()
        .unwrap()
        .contains("refusing an unverifiable parent"));
}

#[test]
fn recursive_descent_rejects_unknown_selected_node() {
    let responses = drive(&[tool_call(
        1,
        "bind_allocation_receipt",
        json!({
            "candidates": eight_candidates_with_ids("child"),
            "allocation_result": allocation_result(),
            "observation_frontier": {},
            "parent_allocation": parent_envelope(),
            "selected_node": "root-99"
        }),
    )]);
    let response = find_response(&responses, 1);
    assert_eq!(response["result"]["isError"], true);
    assert!(response["result"]["content"][0]["text"]
        .as_str()
        .unwrap()
        .contains("is not a candidate in parent_allocation"));
}

#[test]
fn recursive_descent_requires_paired_fields() {
    let responses = drive(&[tool_call(
        1,
        "bind_allocation_receipt",
        json!({
            "candidates": eight_candidates_with_ids("root"),
            "allocation_result": allocation_result(),
            "observation_frontier": {},
            "selected_node": "root-0"
        }),
    )]);
    let response = find_response(&responses, 1);
    assert_eq!(response["result"]["isError"], true);
    assert!(response["result"]["content"][0]["text"]
        .as_str()
        .unwrap()
        .contains("must be provided together"));
}
