//! Drive the built `ferroplan-mcp` binary over stdio and check the JSON-RPC / MCP
//! protocol end to end: initialize, tools/list, a solve call, and the error paths.

use serde_json::{json, Value};
use std::io::Write;
use std::process::{Command, Stdio};

/// Send a batch of JSON-RPC messages verbatim (one per line), close stdin, and collect
/// every response line as parsed JSON. Callers are responsible for a spec-conformant
/// `initialize`/`notifications/initialized` handshake if the server requires one.
fn raw_drive(messages: &[Value]) -> Vec<Value> {
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

fn handshake_initialize() -> Value {
    json!({
        "jsonrpc": "2.0",
        "id": "__handshake__",
        "method": "initialize",
        "params": {
            "protocolVersion": "2025-06-18",
            "capabilities": {},
            "clientInfo": {"name": "protocol-test", "version": "0"}
        }
    })
}

fn handshake_initialized() -> Value {
    json!({"jsonrpc": "2.0", "method": "notifications/initialized"})
}

/// Like `raw_drive`, but performs the real MCP handshake rmcp requires
/// (`initialize` with `capabilities`/`clientInfo`, then
/// `notifications/initialized`) before sending `messages`, and strips the
/// handshake's own response so callers see only responses to `messages`.
fn drive(messages: &[Value]) -> Vec<Value> {
    let mut all = vec![handshake_initialize(), handshake_initialized()];
    all.extend_from_slice(messages);
    let mut resp = raw_drive(&all);
    assert!(!resp.is_empty(), "expected at least the handshake response");
    resp.remove(0); // drop the initialize response
    resp
}

const DOM: &str = "(define (domain d) (:requirements :strips) (:predicates (p) (q)) \
    (:action a :precondition (p) :effect (and (not (p)) (q))))";
const PROB: &str = "(define (problem pr) (:domain d) (:init (p)) (:goal (q)))";

#[test]
fn initialize_advertises_server_and_tools() {
    let resp = raw_drive(&[
        json!({
            "jsonrpc":"2.0","id":1,"method":"initialize",
            "params":{
                "protocolVersion":"2025-06-18",
                "capabilities": {},
                "clientInfo": {"name": "protocol-test", "version": "0"}
            }
        }),
        json!({"jsonrpc":"2.0","method":"notifications/initialized"}),
        json!({"jsonrpc":"2.0","id":2,"method":"tools/list"}),
    ]);
    // The notification gets no reply: two requests → two responses.
    assert_eq!(resp.len(), 2, "notification must not produce a response");
    assert_eq!(resp[0]["id"], 1);
    assert_eq!(resp[0]["result"]["serverInfo"]["name"], "ferroplan");
    // protocolVersion is echoed from the client.
    assert_eq!(resp[0]["result"]["protocolVersion"], "2025-06-18");

    let mut names: Vec<&str> = resp[1]["result"]["tools"]
        .as_array()
        .unwrap()
        .iter()
        .map(|t| t["name"].as_str().unwrap())
        .collect();
    names.sort_unstable();
    assert_eq!(names, ["decompose", "parse", "solve", "validate"]);
}

/// Find the response whose `id` matches, tolerating the async server's freedom
/// to resolve concurrent in-flight requests out of arrival order.
fn find_response(resp: &[Value], id: i64) -> Value {
    resp.iter()
        .find(|v| v["id"] == json!(id))
        .unwrap_or_else(|| panic!("no response with id {id} in {resp:?}"))
        .clone()
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
    // rmcp always sets `isError` explicitly (`false` on success), unlike the prior
    // hand-rolled server which omitted the field entirely on success.
    assert_eq!(resp[0]["result"]["isError"], false);
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
    // The server may resolve concurrent in-flight requests out of order, so match by id.
    let tool_call = find_response(&resp, 1);
    let unknown_method = find_response(&resp, 2);
    assert_eq!(tool_call["result"]["isError"], true);
    assert!(tool_call["result"]["content"][0]["text"]
        .as_str()
        .unwrap()
        .contains("domain"));
    assert_eq!(unknown_method["error"]["code"], -32601);
}
