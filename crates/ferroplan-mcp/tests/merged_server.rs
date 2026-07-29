//! Proof that merging the three former binaries (`ferroplan-mcp`,
//! `ferroplan-session-mcp`, `chatman-admission-mcp`) into one `ferroplan-mcp`
//! binary did not silently drop or duplicate any tool or resource: an exact
//! 17-tool, 17-resource assertion against the single merged server.

use serde_json::{json, Value};
use std::io::Write;
use std::process::{Command, Stdio};

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
            "clientInfo": {"name": "merged-server-test", "version": "0"}
        }
    })
}

fn handshake_initialized() -> Value {
    json!({"jsonrpc": "2.0", "method": "notifications/initialized"})
}

fn drive(messages: &[Value]) -> Vec<Value> {
    let mut all = vec![handshake_initialize(), handshake_initialized()];
    all.extend_from_slice(messages);
    let mut resp = raw_drive(&all);
    assert!(!resp.is_empty(), "expected at least the handshake response");
    resp.remove(0); // drop the initialize response
    resp
}

fn find_response(resp: &[Value], id: i64) -> Value {
    resp.iter()
        .find(|v| v["id"] == json!(id))
        .unwrap_or_else(|| panic!("no response with id {id} in {resp:?}"))
        .clone()
}

const ALL_17_TOOLS: &[&str] = &[
    // stateless planning
    "solve",
    "parse",
    "validate",
    "decompose",
    // persistent sessions + CMCA
    "session_open",
    "session_observe",
    "session_set_goal",
    "session_think",
    "session_advance",
    "session_status",
    "session_close",
    "cmca_allocate",
    "cmca_allocate_recursive",
    // canonical evidence admission
    "canonical_digest",
    "bind_allocation_receipt",
    "bind_plan_receipt",
    "verify_receipt",
];

#[test]
fn initialize_advertises_all_17_tools() {
    let resp = raw_drive(&[
        json!({
            "jsonrpc":"2.0","id":1,"method":"initialize",
            "params":{
                "protocolVersion":"2025-06-18",
                "capabilities": {},
                "clientInfo": {"name": "merged-server-test", "version": "0"}
            }
        }),
        json!({"jsonrpc":"2.0","method":"notifications/initialized"}),
        json!({"jsonrpc":"2.0","id":2,"method":"tools/list"}),
    ]);
    assert_eq!(resp.len(), 2, "notification must not produce a response");
    assert_eq!(resp[0]["result"]["serverInfo"]["name"], "ferroplan");

    let mut names: Vec<&str> = resp[1]["result"]["tools"]
        .as_array()
        .expect("tools array")
        .iter()
        .map(|t| t["name"].as_str().unwrap())
        .collect();
    names.sort_unstable();

    let mut expected: Vec<&str> = ALL_17_TOOLS.to_vec();
    expected.sort_unstable();

    assert_eq!(
        names.len(),
        17,
        "expected exactly 17 tools, got {}: {names:?}",
        names.len()
    );
    assert_eq!(
        names, expected,
        "merged server tool set does not match expected 17"
    );
}

#[test]
fn resources_list_exposes_exactly_17_under_the_unified_scheme() {
    let resp = drive(&[
        json!({"jsonrpc":"2.0","id":1,"method":"resources/list"}),
        json!({"jsonrpc":"2.0","id":2,"method":"resources/read",
               "params":{"uri":"ferroplan://tools/session_think"}}),
    ]);

    let list = find_response(&resp, 1);
    let resources = list["result"]["resources"]
        .as_array()
        .expect("resources array");
    assert_eq!(
        resources.len(),
        17,
        "expected exactly 17 resources, got {}: {resources:?}",
        resources.len()
    );

    let mut uris: Vec<String> = resources
        .iter()
        .map(|r| r["uri"].as_str().unwrap().to_owned())
        .collect();
    uris.sort_unstable();
    let mut expected: Vec<String> = ALL_17_TOOLS
        .iter()
        .map(|name| format!("ferroplan://tools/{name}"))
        .collect();
    expected.sort_unstable();
    assert_eq!(
        uris, expected,
        "every resource must live under the unified `ferroplan://tools/*` scheme"
    );

    // One resources/read still returns real ontology prose, not a stub.
    let read = find_response(&resp, 2);
    let contents = read["result"]["contents"]
        .as_array()
        .expect("contents array");
    assert_eq!(contents.len(), 1);
    let text = contents[0]["text"].as_str().expect("resource text");
    let body: Value = serde_json::from_str(text).expect("resource body is JSON");
    assert_eq!(body["tool"], "session_think");
    let comment = body["rdfs_comment"].as_str().expect("rdfs_comment string");
    assert!(
        comment.len() > 10,
        "ontology comment is real prose, not a stub: {comment:?}"
    );
}

/// Every `inputSchema` property must be an *object* subschema, never the
/// boolean schema `true`.
///
/// Regression guard for the defect that broke the whole client surface without
/// failing a single test: `schemars` renders an unconstrained
/// `serde_json::Value` field as `true`. That is legal JSON Schema, but MCP
/// clients validate `properties.*` as an object and reject the entire
/// `tools/list` response on the first violation — so all 17 tools vanished at
/// once while the server itself remained perfectly well-formed. Boolean
/// subschemas must therefore be spelled `{}`.
#[test]
fn no_tool_input_schema_uses_a_boolean_subschema() {
    let resp = drive(&[json!({"jsonrpc": "2.0", "id": 1, "method": "tools/list"})]);
    let tools = find_response(&resp, 1)["result"]["tools"]
        .as_array()
        .expect("tools array")
        .clone();
    assert_eq!(tools.len(), 17, "expected all 17 tools");

    let offenders: Vec<String> = tools
        .iter()
        .flat_map(|t| {
            let name = t["name"].as_str().unwrap_or("<unnamed>").to_owned();
            t["inputSchema"]["properties"]
                .as_object()
                .into_iter()
                .flatten()
                .filter(|(_, v)| v.is_boolean())
                .map(move |(k, v)| format!("{name}.{k} = {v}"))
                .collect::<Vec<_>>()
        })
        .collect();

    assert!(
        offenders.is_empty(),
        "boolean subschemas are rejected by MCP clients; use `{{}}` instead: {offenders:#?}"
    );
}
