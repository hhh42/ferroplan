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

    let names: std::collections::BTreeSet<&str> = resp[1]["result"]["tools"]
        .as_array()
        .unwrap()
        .iter()
        .map(|t| t["name"].as_str().unwrap())
        .collect();
    // The merged server exposes 16 tools total; this stateless-planning
    // group's four must be present (full 16-tool exactness is
    // `merged_server.rs`'s job).
    for expected in ["decompose", "parse", "solve", "validate"] {
        assert!(
            names.contains(expected),
            "missing tool `{expected}`: {names:?}"
        );
    }
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

// Two independent numeric deliverables, each from its own producer — the interaction
// partition makes each numeric goal its own contract, so this goal actually splits
// into >= 2 contracts (mirrors ferroplan's own
// `crates/ferroplan/tests/decompose.rs::splits_a_conjunctive_goal_into_contracts`,
// chosen so this test exercises a real decomposition rather than the monolithic
// fallback).
const TEMPORAL_DOM: &str = "
(define (domain mk)
  (:requirements :durative-actions :numeric-fluents)
  (:functions (a) (b))
  (:durative-action make-a :parameters () :duration (= ?duration 2)
    :condition () :effect (at end (increase (a) 1)))
  (:durative-action make-b :parameters () :duration (= ?duration 3)
    :condition () :effect (at end (increase (b) 1))))
";
const TEMPORAL_PROB: &str = "(define (problem p) (:domain mk)
  (:init (= (a) 0) (= (b) 0))
  (:goal (and (>= (a) 1) (>= (b) 1))))";

#[test]
fn decompose_tool_splits_a_temporal_goal() {
    let resp = drive(&[json!({
        "jsonrpc":"2.0","id":1,"method":"tools/call",
        "params":{"name":"decompose","arguments":{"domain":TEMPORAL_DOM,"problem":TEMPORAL_PROB}}
    })]);
    assert_eq!(resp[0]["result"]["isError"], false);
    let text = resp[0]["result"]["content"][0]["text"].as_str().unwrap();
    let dec: Value = serde_json::from_str(text).expect("decompose returns a JSON Decomposition");
    assert_eq!(dec["solved"], true);
    assert_eq!(
        dec["monolithic"], false,
        "two independent deliverables should split"
    );
    let contracts = dec["contracts"].as_array().expect("contracts array");
    assert!(
        contracts.len() >= 2,
        "expected >= 2 contracts, got {}: {dec}",
        contracts.len()
    );
    for c in contracts {
        assert!(
            !c["goal"].as_str().unwrap().is_empty(),
            "contract has a rendered goal"
        );
        assert!(
            !c["steps"].as_array().unwrap().is_empty(),
            "contract has a sub-plan"
        );
    }
    assert!(dec["plan"].is_object(), "a stitched plan is present");
}

#[test]
fn resources_list_and_read_expose_tool_semantics() {
    let resp = drive(&[
        json!({"jsonrpc":"2.0","id":1,"method":"resources/list"}),
        json!({"jsonrpc":"2.0","id":2,"method":"resources/read",
               "params":{"uri":"ferroplan://tools/solve"}}),
    ]);
    let list = find_response(&resp, 1);
    let resources = list["result"]["resources"]
        .as_array()
        .expect("resources array");
    // The merged server exposes 16 resources total; this stateless-planning
    // group's four must be present (full 16-resource exactness is
    // `merged_server.rs`'s job).
    let uris: std::collections::BTreeSet<&str> = resources
        .iter()
        .map(|r| r["uri"].as_str().unwrap())
        .collect();
    for expected in [
        "ferroplan://tools/decompose",
        "ferroplan://tools/parse",
        "ferroplan://tools/solve",
        "ferroplan://tools/validate",
    ] {
        assert!(
            uris.contains(expected),
            "missing resource `{expected}`: {uris:?}"
        );
    }

    let read = find_response(&resp, 2);
    let contents = read["result"]["contents"]
        .as_array()
        .expect("contents array");
    assert_eq!(contents.len(), 1);
    let text = contents[0]["text"].as_str().expect("resource text");
    assert!(!text.is_empty(), "resource body is non-empty");
    let body: Value = serde_json::from_str(text).expect("resource body is JSON");
    assert_eq!(body["tool"], "solve");
    // The ontology-sourced comment is real prose about the tool, not a placeholder.
    let comment = body["rdfs_comment"].as_str().expect("rdfs_comment string");
    assert!(!comment.is_empty(), "ontology comment is non-empty");
    assert!(
        comment.to_lowercase().contains("solution") || comment.to_lowercase().contains("plan"),
        "ontology comment reads like real solve semantics, got: {comment}"
    );
}

#[test]
fn missing_required_field_is_a_tool_error() {
    let resp = drive(&[
        // `validate` requires domain/problem/plan — omit `plan`.
        json!({"jsonrpc":"2.0","id":1,"method":"tools/call",
               "params":{"name":"validate","arguments":{"domain":DOM,"problem":PROB}}}),
        // `decompose` requires domain/problem — omit `problem`.
        json!({"jsonrpc":"2.0","id":2,"method":"tools/call",
               "params":{"name":"decompose","arguments":{"domain":TEMPORAL_DOM}}}),
    ]);
    let validate_resp = find_response(&resp, 1);
    let decompose_resp = find_response(&resp, 2);
    assert_eq!(validate_resp["result"]["isError"], true);
    assert!(validate_resp["result"]["content"][0]["text"]
        .as_str()
        .unwrap()
        .contains("plan"));
    assert_eq!(decompose_resp["result"]["isError"], true);
    assert!(decompose_resp["result"]["content"][0]["text"]
        .as_str()
        .unwrap()
        .contains("problem"));
}

#[test]
fn unknown_field_is_a_tool_error() {
    let resp = drive(&[
        // `SolveRequest` is `#[serde(deny_unknown_fields)]` — an unrecognized field
        // must be rejected, not silently ignored.
        json!({"jsonrpc":"2.0","id":1,"method":"tools/call",
               "params":{"name":"solve",
                         "arguments":{"domain":DOM,"problem":PROB,"bogus_field":true}}}),
        // Same for `parse`.
        json!({"jsonrpc":"2.0","id":2,"method":"tools/call",
               "params":{"name":"parse","arguments":{"pddl":DOM,"bogus_field":true}}}),
    ]);
    let solve_resp = find_response(&resp, 1);
    let parse_resp = find_response(&resp, 2);
    assert_eq!(solve_resp["result"]["isError"], true);
    assert_eq!(parse_resp["result"]["isError"], true);
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
