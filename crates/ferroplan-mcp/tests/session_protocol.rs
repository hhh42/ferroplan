//! Drive the built `ferroplan-session-mcp` binary over stdio and check the
//! per-session locking refactor (Fix 1): concurrent tool calls against the
//! *same* session_id queue on that session's own lock rather than racing a
//! remove/reinsert and observing "unknown session".

use serde_json::{json, Value};
use std::io::{BufRead, BufReader, Write};
use std::process::{Command, Stdio};

fn handshake_initialize() -> Value {
    json!({
        "jsonrpc": "2.0",
        "id": "__handshake__",
        "method": "initialize",
        "params": {
            "protocolVersion": "2025-06-18",
            "capabilities": {},
            "clientInfo": {"name": "session-protocol-test", "version": "0"}
        }
    })
}

fn handshake_initialized() -> Value {
    json!({"jsonrpc": "2.0", "method": "notifications/initialized"})
}

/// Read and parse one JSON-RPC response line, skipping blank lines.
fn read_response_line(stdout: &mut BufReader<std::process::ChildStdout>) -> Value {
    loop {
        let mut line = String::new();
        let n = stdout.read_line(&mut line).expect("read line");
        assert!(n > 0, "server closed stdout before responding");
        if line.trim().is_empty() {
            continue;
        }
        return serde_json::from_str(&line).expect("response is one JSON line");
    }
}

fn find_response(resp: &[Value], id: i64) -> Value {
    resp.iter()
        .find(|v| v["id"] == json!(id))
        .unwrap_or_else(|| panic!("no response with id {id} in {resp:?}"))
        .clone()
}

const DOM: &str = "(define (domain d) (:requirements :strips) (:predicates (p) (q)) \
    (:action a :precondition (p) :effect (and (not (p)) (q))))";
const PROB: &str = "(define (problem pr) (:domain d) (:init (p)) (:goal (q)))";

fn tool_call(id: i64, name: &str, arguments: Value) -> Value {
    json!({
        "jsonrpc": "2.0", "id": id, "method": "tools/call",
        "params": {"name": name, "arguments": arguments}
    })
}

/// Send a request, read its response line, and return the parsed value.
/// Uses a fresh handshake against a freshly spawned server so each test gets
/// an isolated process (no shared state with other tests).
fn spawn_and_handshake() -> (std::process::Child, BufReader<std::process::ChildStdout>) {
    let bin = env!("CARGO_BIN_EXE_ferroplan-session-mcp");
    let mut child = Command::new(bin)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .expect("spawn ferroplan-session-mcp");
    let mut stdout = BufReader::new(child.stdout.take().expect("stdout"));
    {
        let stdin = child.stdin.as_mut().expect("stdin");
        writeln!(stdin, "{}", handshake_initialize()).expect("write");
    }
    read_response_line(&mut stdout); // initialize response
    {
        let stdin = child.stdin.as_mut().expect("stdin");
        writeln!(stdin, "{}", handshake_initialized()).expect("write");
    }
    (child, stdout)
}

/// Write one request and read back its response line (request/response are
/// strictly sequential here, so no id-based lookup is needed).
fn call(
    child: &mut std::process::Child,
    stdout: &mut BufReader<std::process::ChildStdout>,
    request: &Value,
) -> Value {
    {
        let stdin = child.stdin.as_mut().expect("stdin");
        writeln!(stdin, "{request}").expect("write");
    }
    read_response_line(stdout)
}

fn tool_text(resp: &Value) -> String {
    resp["result"]["content"][0]["text"]
        .as_str()
        .unwrap_or_default()
        .to_owned()
}

fn is_error(resp: &Value) -> bool {
    resp["result"]["isError"].as_bool().unwrap_or(false)
}

/// Drives the full tool sequence against a single session in order, checking
/// that each response is well-formed and that state (cursor, epoch, goal)
/// flows correctly from one call to the next.
///
/// What this proves: the happy path through session_open -> session_observe
/// -> session_set_goal -> session_think -> session_advance -> session_status
/// -> session_close is wired correctly end to end against a real (tiny)
/// planning problem, with no LLM involved.
///
/// What this does NOT prove: behavior under concurrent access (see the
/// concurrency test above), or correctness of the underlying search/replan
/// algorithms themselves (covered elsewhere).
#[test]
fn full_session_lifecycle() {
    let session_id = "lifecycle-test-session";
    let (mut child, mut stdout) = spawn_and_handshake();

    let open = call(
        &mut child,
        &mut stdout,
        &tool_call(
            1,
            "session_open",
            json!({"session_id": session_id, "domain": DOM, "problem": PROB}),
        ),
    );
    assert!(!is_error(&open), "session_open failed: {}", tool_text(&open));
    assert_eq!(
        open["result"]["structuredContent"]["session_id"], session_id,
        "session_open did not echo session_id: {open:?}"
    );

    let observe = call(
        &mut child,
        &mut stdout,
        &tool_call(
            2,
            "session_observe",
            json!({
                "session_id": session_id,
                "facts": [{"fact": "(P)", "value": true}],
                "fluents": []
            }),
        ),
    );
    assert!(
        !is_error(&observe),
        "session_observe failed: {}",
        tool_text(&observe)
    );

    let set_goal = call(
        &mut child,
        &mut stdout,
        &tool_call(
            3,
            "session_set_goal",
            json!({"session_id": session_id, "goal": "(q)"}),
        ),
    );
    assert!(
        !is_error(&set_goal),
        "session_set_goal failed: {}",
        tool_text(&set_goal)
    );

    let think = call(
        &mut child,
        &mut stdout,
        &tool_call(
            4,
            "session_think",
            json!({"session_id": session_id, "max_evaluated": 50_000}),
        ),
    );
    assert!(!is_error(&think), "session_think failed: {}", tool_text(&think));
    let decision = think["result"]["structuredContent"]["decision"]
        .as_str()
        .unwrap_or_default();
    assert!(
        matches!(decision, "follow" | "replan" | "bounded-refusal"),
        "session_think returned an unrecognized decision: {think:?}"
    );
    // Only "follow" and successful "replan" carry a plan to advance over;
    // "bounded-refusal" means no plan was found within budget.
    let plan_len = think["result"]["structuredContent"]["plan"]["steps"]
        .as_array()
        .map(std::vec::Vec::len)
        .or_else(|| {
            think["result"]["structuredContent"]["solution"]["plan"]["steps"]
                .as_array()
                .map(std::vec::Vec::len)
        })
        .unwrap_or(0);

    let advance = call(
        &mut child,
        &mut stdout,
        &tool_call(
            5,
            "session_advance",
            json!({"session_id": session_id, "completed_steps": plan_len}),
        ),
    );
    assert!(
        !is_error(&advance),
        "session_advance failed: {}",
        tool_text(&advance)
    );
    assert_eq!(
        advance["result"]["structuredContent"]["cursor"],
        json!(plan_len),
        "session_advance did not move the cursor to the reported plan length: {advance:?}"
    );

    let status = call(
        &mut child,
        &mut stdout,
        &tool_call(6, "session_status", json!({"session_id": session_id})),
    );
    assert!(
        !is_error(&status),
        "session_status failed: {}",
        tool_text(&status)
    );
    let status_content = &status["result"]["structuredContent"];
    assert_eq!(
        status_content["cursor"],
        json!(plan_len),
        "session_status cursor does not reflect the advance: {status:?}"
    );
    assert!(
        status_content["epoch"].as_u64().unwrap_or(0) >= 1,
        "session_status epoch should have advanced past set_goal (observe(P=true) is not a \
         surprise since P already holds in the initial state): {status:?}"
    );

    let close = call(
        &mut child,
        &mut stdout,
        &tool_call(7, "session_close", json!({"session_id": session_id})),
    );
    assert!(!is_error(&close), "session_close failed: {}", tool_text(&close));
    assert_eq!(
        close["result"]["structuredContent"]["closed"],
        json!(true),
        "session_close did not report closed: true: {close:?}"
    );

    drop(child.stdin.take());
    let status_code = child.wait().expect("wait");
    assert!(status_code.success(), "server exited with {status_code:?}");
}

/// Calls session_status, session_advance, and session_observe against a
/// session_id that was never opened, and checks each returns a proper
/// tool-level error (`isError: true` with an "unknown session" message)
/// rather than crashing the server or hanging.
#[test]
fn session_status_and_advance_reject_unknown_session() {
    let session_id = "never-opened-session";
    let (mut child, mut stdout) = spawn_and_handshake();

    let status = call(
        &mut child,
        &mut stdout,
        &tool_call(1, "session_status", json!({"session_id": session_id})),
    );
    assert!(
        is_error(&status),
        "session_status on an unopened session should be a tool error: {status:?}"
    );
    assert!(
        tool_text(&status).contains("unknown session"),
        "expected an `unknown session` message: {status:?}"
    );

    let advance = call(
        &mut child,
        &mut stdout,
        &tool_call(
            2,
            "session_advance",
            json!({"session_id": session_id, "completed_steps": 1}),
        ),
    );
    assert!(
        is_error(&advance),
        "session_advance on an unopened session should be a tool error: {advance:?}"
    );
    assert!(
        tool_text(&advance).contains("unknown session"),
        "expected an `unknown session` message: {advance:?}"
    );

    let observe = call(
        &mut child,
        &mut stdout,
        &tool_call(
            3,
            "session_observe",
            json!({"session_id": session_id, "facts": [], "fluents": []}),
        ),
    );
    assert!(
        is_error(&observe),
        "session_observe on an unopened session should be a tool error: {observe:?}"
    );
    assert!(
        tool_text(&observe).contains("unknown session"),
        "expected an `unknown session` message: {observe:?}"
    );

    drop(child.stdin.take());
    let status_code = child.wait().expect("wait");
    assert!(status_code.success(), "server exited with {status_code:?}");
}

/// Runs `cmca_allocate` with exactly N=8 candidates x F=10 factors each
/// (the pinned CMCA registry shape, per the tool's own description: "exactly
/// eight admitted nodes and ten factors per node"), arranged as a simple
/// one-root, seven-child forest so `validate_forest` accepts it, and checks
/// a well-formed allocation payload comes back.
#[test]
fn cmca_allocate_returns_an_allocation() {
    let (mut child, mut stdout) = spawn_and_handshake();

    let candidates: Vec<Value> = (0..8)
        .map(|i| {
            json!({
                "id": format!("node-{i}"),
                "parent": if i == 0 { Value::Null } else { json!(0) },
                "factors": vec![0.5_f64; 10],
                "cost": 1.0
            })
        })
        .collect();

    let resp = call(
        &mut child,
        &mut stdout,
        &tool_call(1, "cmca_allocate", json!({"candidates": candidates})),
    );
    assert!(
        !is_error(&resp),
        "cmca_allocate failed: {}",
        tool_text(&resp)
    );
    let content = &resp["result"]["structuredContent"];
    assert!(
        content["payload_digest"].is_string(),
        "cmca_allocate response missing payload_digest: {resp:?}"
    );
    let allocations = content["payload"]["allocations"]
        .as_array()
        .unwrap_or_else(|| panic!("cmca_allocate response missing allocations array: {resp:?}"));
    assert_eq!(
        allocations.len(),
        8,
        "expected one allocation row per candidate: {resp:?}"
    );
    for row in allocations {
        assert!(
            row["share"].is_number(),
            "allocation row missing numeric share: {row:?}"
        );
    }

    drop(child.stdin.take());
    let status_code = child.wait().expect("wait");
    assert!(status_code.success(), "server exited with {status_code:?}");
}

/// resources/list must expose exactly one resource per tool (8 tools:
/// session_open, session_observe, session_set_goal, session_think,
/// session_advance, session_status, session_close, cmca_allocate), and
/// resources/read on one of them must return real ontology-sourced prose
/// (not an empty stub).
#[test]
fn resources_list_and_read_expose_tool_semantics() {
    let (mut child, mut stdout) = spawn_and_handshake();

    let list = call(
        &mut child,
        &mut stdout,
        &json!({"jsonrpc": "2.0", "id": 1, "method": "resources/list", "params": {}}),
    );
    let resources = list["result"]["resources"]
        .as_array()
        .unwrap_or_else(|| panic!("resources/list returned no resources array: {list:?}"));
    assert_eq!(
        resources.len(),
        8,
        "expected exactly 8 tool resources: {list:?}"
    );
    let names: std::collections::BTreeSet<&str> = resources
        .iter()
        .filter_map(|r| r["uri"].as_str())
        .filter_map(|uri| uri.strip_prefix("ferroplan-session://tools/"))
        .collect();
    let expected: std::collections::BTreeSet<&str> = [
        "session_open",
        "session_observe",
        "session_set_goal",
        "session_think",
        "session_advance",
        "session_status",
        "session_close",
        "cmca_allocate",
    ]
    .into_iter()
    .collect();
    assert_eq!(names, expected, "resource URIs did not match the tool set");

    let read = call(
        &mut child,
        &mut stdout,
        &json!({
            "jsonrpc": "2.0",
            "id": 2,
            "method": "resources/read",
            "params": {"uri": "ferroplan-session://tools/session_think"}
        }),
    );
    let contents = read["result"]["contents"]
        .as_array()
        .unwrap_or_else(|| panic!("resources/read returned no contents array: {read:?}"));
    assert_eq!(contents.len(), 1, "expected exactly one content block: {read:?}");
    let text = contents[0]["text"].as_str().unwrap_or_default();
    assert!(
        text.contains("session_think") && text.contains("rdfs_comment"),
        "resources/read did not return ontology-sourced prose: {text}"
    );
    let parsed: Value = serde_json::from_str(text).expect("resource body is JSON");
    let comment = parsed["rdfs_comment"].as_str().unwrap_or_default();
    assert!(
        comment.len() > 10,
        "rdfs_comment should be real prose, not an empty stub: {parsed:?}"
    );

    drop(child.stdin.take());
    let status_code = child.wait().expect("wait");
    assert!(status_code.success(), "server exited with {status_code:?}");
}

/// `OpenInput` is `#[serde(deny_unknown_fields)]`; sending an unrecognized
/// field must surface as a proper tool-call error rather than crashing the
/// server or being silently ignored.
#[test]
fn malformed_input_with_unknown_field_is_rejected() {
    let (mut child, mut stdout) = spawn_and_handshake();

    let resp = call(
        &mut child,
        &mut stdout,
        &tool_call(
            1,
            "session_open",
            json!({
                "session_id": "malformed-test-session",
                "domain": DOM,
                "problem": PROB,
                "this_field_does_not_exist": true
            }),
        ),
    );
    assert!(
        resp.get("error").is_some() || is_error(&resp),
        "an unknown field on OpenInput should be rejected, not silently accepted: {resp:?}"
    );

    // Confirm the server is still alive and responsive after the rejection
    // (i.e. it didn't crash on the malformed request).
    let status = call(
        &mut child,
        &mut stdout,
        &tool_call(
            2,
            "session_status",
            json!({"session_id": "malformed-test-session"}),
        ),
    );
    assert!(
        is_error(&status),
        "server should still be alive and reporting `unknown session` after the malformed \
         request, not crashed: {status:?}"
    );

    drop(child.stdin.take());
    let status_code = child.wait().expect("wait");
    assert!(status_code.success(), "server exited with {status_code:?}");
}

/// Opens a session, then fires `session_think` and `session_status` against
/// the SAME session_id back-to-back in one batch (both written to stdin
/// before either response is read, so rmcp dispatches them as concurrent
/// tokio tasks rather than strictly sequentially).
///
/// What this proves: the per-session `Arc<AsyncMutex<ManagedSession>>`
/// lookup (`ServerState::get`) succeeds for a call issued while another
/// call against the same session is in flight — i.e. the queuing path does
/// NOT return "unknown session", which is what the old remove/spawn_blocking/
/// reinsert scheme could do under this exact race.
///
/// What this does NOT prove: true concurrency-safety under sustained load,
/// absence of deadlocks in longer call chains, or that block_in_place scales
/// under many concurrent searches — only that the specific "second caller
/// sees unknown session" failure mode from the old scheme is closed.
#[test]
fn concurrent_calls_against_same_session_do_not_see_unknown_session() {
    let session_id = "concurrent-test-session";
    let bin = env!("CARGO_BIN_EXE_ferroplan-session-mcp");
    let mut child = Command::new(bin)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .expect("spawn ferroplan-session-mcp");
    let mut stdout = BufReader::new(child.stdout.take().expect("stdout"));

    // Handshake, then session_open, reading each response before sending the
    // next request — this guarantees session_open has actually completed
    // (the session exists in the map) before the concurrent pair below is
    // dispatched, so any "unknown session" from *them* is attributable only
    // to the locking scheme under test, not to a request-ordering race.
    {
        let stdin = child.stdin.as_mut().expect("stdin");
        writeln!(stdin, "{}", handshake_initialize()).expect("write");
    }
    read_response_line(&mut stdout); // initialize response
    {
        let stdin = child.stdin.as_mut().expect("stdin");
        writeln!(stdin, "{}", handshake_initialized()).expect("write");
        writeln!(
            stdin,
            "{}",
            tool_call(
                1,
                "session_open",
                json!({"session_id": session_id, "domain": DOM, "problem": PROB}),
            )
        )
        .expect("write");
    }
    let open = read_response_line(&mut stdout);
    assert_eq!(
        open["result"]["isError"], false,
        "session_open failed: {open:?}"
    );

    // Now fire session_think and session_status against the SAME session_id
    // back to back, both written before either response is read, so rmcp
    // dispatches them as concurrent tokio tasks racing for the per-session
    // lock rather than strictly sequentially.
    {
        let stdin = child.stdin.as_mut().expect("stdin");
        writeln!(
            stdin,
            "{}",
            tool_call(
                2,
                "session_think",
                json!({"session_id": session_id, "max_evaluated": 50_000}),
            )
        )
        .expect("write");
        writeln!(
            stdin,
            "{}",
            tool_call(3, "session_status", json!({"session_id": session_id}))
        )
        .expect("write");
    } // drop stdin → EOF → server drains remaining requests and exits
    drop(child.stdin.take());

    // `child.stdout` was already taken into `stdout` above, so read the
    // remaining two response lines from that same handle rather than via
    // `wait_with_output` (which would see an empty stdout pipe here).
    let resp = vec![read_response_line(&mut stdout), read_response_line(&mut stdout)];
    let status_code = child.wait().expect("wait");
    assert!(status_code.success(), "server exited with {status_code:?}");

    let think = find_response(&resp, 2);
    let status = find_response(&resp, 3);

    for (label, r) in [("session_think", &think), ("session_status", &status)] {
        let is_error = r["result"]["isError"].as_bool().unwrap_or(false);
        if is_error {
            let text = r["result"]["content"][0]["text"].as_str().unwrap_or("");
            assert!(
                !text.contains("unknown session"),
                "{label} saw `unknown session` under concurrent access against the same \
                 session_id — the per-session lock lookup should never fail this way once \
                 session_open has returned successfully: {text}"
            );
        }
    }
}
