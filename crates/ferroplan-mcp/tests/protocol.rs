//! Drive the built `ferroplan-mcp` binary over stdio and check the JSON-RPC / MCP
//! protocol end to end: initialize, tools/list, a solve call, and the error paths.

use serde_json::{json, Value};
use std::io::Write;
use std::process::{Command, Stdio};

/// Send a batch of JSON-RPC messages (one per line), close stdin, and collect every
/// response line as parsed JSON.
fn drive(messages: &[Value]) -> Vec<Value> {
    let bin = env!("CARGO_BIN_EXE_ferroplan-mcp");
    let mut child = Command::new(bin)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .expect("spawn ferroplan-mcp");
    {
        let stdin = child.stdin.as_mut().expect("stdin");
        for m in messages {
            writeln!(stdin, "{m}").expect("write message");
        }
    } // drop stdin → EOF → server drains and exits
    let out = child.wait_with_output().expect("wait");
    assert!(out.status.success(), "server exited with {:?}", out.status);
    String::from_utf8(out.stdout)
        .expect("utf8")
        .lines()
        .filter(|l| !l.trim().is_empty())
        .map(|l| serde_json::from_str(l).expect("each response is one JSON line"))
        .collect()
}

const DOM: &str = "(define (domain d) (:requirements :strips) (:predicates (p) (q)) \
    (:action a :precondition (p) :effect (and (not (p)) (q))))";
const PROB: &str = "(define (problem pr) (:domain d) (:init (p)) (:goal (q)))";

#[test]
fn initialize_advertises_server_and_tools() {
    let resp = drive(&[
        json!({"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-06-18"}}),
        json!({"jsonrpc":"2.0","method":"notifications/initialized"}),
        json!({"jsonrpc":"2.0","id":2,"method":"tools/list"}),
    ]);
    // The notification gets no reply: two requests → two responses.
    assert_eq!(resp.len(), 2, "notification must not produce a response");
    assert_eq!(resp[0]["id"], 1);
    assert_eq!(resp[0]["result"]["serverInfo"]["name"], "ferroplan");
    // protocolVersion is echoed from the client.
    assert_eq!(resp[0]["result"]["protocolVersion"], "2025-06-18");

    let names: Vec<&str> = resp[1]["result"]["tools"]
        .as_array()
        .unwrap()
        .iter()
        .map(|t| t["name"].as_str().unwrap())
        .collect();
    assert_eq!(names, ["solve", "parse", "validate", "decompose"]);
}

#[test]
fn parse_tool_summarizes_a_domain() {
    let resp = drive(&[json!({
        "jsonrpc":"2.0","id":1,"method":"tools/call",
        "params":{"name":"parse","arguments":{"pddl":DOM}}
    })]);
    let text = resp[0]["result"]["content"][0]["text"].as_str().unwrap();
    let report: Value = serde_json::from_str(text).expect("parse returns a JSON report");
    assert_eq!(report["ok"], true);
    assert_eq!(report["kind"], "domain");
    assert_eq!(report["name"], "d");
}

#[test]
fn solve_tool_returns_a_plan() {
    let resp = drive(&[json!({
        "jsonrpc":"2.0","id":1,"method":"tools/call",
        "params":{"name":"solve","arguments":{"domain":DOM,"problem":PROB}}
    })]);
    let text = resp[0]["result"]["content"][0]["text"].as_str().unwrap();
    let sol: Value = serde_json::from_str(text).expect("solve returns a JSON Solution");
    assert_eq!(sol["solved"], true);
    assert_eq!(sol["plan"]["steps"][0]["action"], "A");
    assert!(resp[0]["result"].get("isError").is_none());
}

#[test]
fn validate_tool_checks_a_plan() {
    let resp = drive(&[json!({
        "jsonrpc":"2.0","id":1,"method":"tools/call",
        "params":{"name":"validate","arguments":{"domain":DOM,"problem":PROB,"plan":"step 0: (a)"}}
    })]);
    let text = resp[0]["result"]["content"][0]["text"].as_str().unwrap();
    assert_eq!(text, "Plan valid");
}

#[test]
fn bad_args_are_tool_errors_unknown_method_is_rpc_error() {
    let resp = drive(&[
        // missing `domain` → isError tool result (not an RPC error)
        json!({"jsonrpc":"2.0","id":1,"method":"tools/call",
               "params":{"name":"solve","arguments":{"problem":PROB}}}),
        // unknown JSON-RPC method → -32601
        json!({"jsonrpc":"2.0","id":2,"method":"no/such/method"}),
    ]);
    assert_eq!(resp[0]["result"]["isError"], true);
    assert!(resp[0]["result"]["content"][0]["text"]
        .as_str()
        .unwrap()
        .contains("domain"));
    assert_eq!(resp[1]["error"]["code"], -32601);
}
