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
