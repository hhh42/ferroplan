//! CE-GALL-35 — Session Lifecycle Bookends.
//!
//! Targets the three tools that had no dedicated checkpoint coverage:
//! `session_open`, `session_status`, `session_close`. `session_protocol.rs`
//! exercises them as steps inside a longer happy-path chain and inside an
//! "unopened session" refusal check (`session_status`/`session_advance`/
//! `session_observe` on an id that was never opened). Neither file checks
//! what `session_status` does to a session *after* it has been closed --
//! that is this file's negative falsifier.

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
            "clientInfo": {"name": "session-lifecycle-bookends-test", "version": "0"}
        }
    })
}

fn handshake_initialized() -> Value {
    json!({"jsonrpc": "2.0", "method": "notifications/initialized"})
}

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

const DOM: &str = "(define (domain d) (:requirements :strips) (:predicates (p) (q)) \
    (:action a :precondition (p) :effect (and (not (p)) (q))))";
const PROB: &str = "(define (problem pr) (:domain d) (:init (p)) (:goal (q)))";

fn tool_call(id: i64, name: &str, arguments: Value) -> Value {
    json!({
        "jsonrpc": "2.0", "id": id, "method": "tools/call",
        "params": {"name": name, "arguments": arguments}
    })
}

fn spawn_and_handshake() -> (std::process::Child, BufReader<std::process::ChildStdout>) {
    let bin = env!("CARGO_BIN_EXE_ferroplan-mcp");
    let mut child = Command::new(bin)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .expect("spawn ferroplan-mcp");
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

/// Positive witness: `session_open` grounds a session against a small valid
/// PDDL domain+problem, `session_status` reflects the session_id, goal, and
/// epoch that `session_open` established, and `session_close` reports
/// `closed: true`. Proves the three bookend tools are wired to the same
/// underlying session state, not just individually reachable.
#[test]
fn open_status_close_bookends_agree_on_session_state() {
    let session_id = "bookends-positive-session";
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
    assert!(
        !is_error(&open),
        "session_open failed: {}",
        tool_text(&open)
    );
    assert_eq!(
        open["result"]["structuredContent"]["session_id"], session_id,
        "session_open did not echo session_id: {open:?}"
    );

    let status = call(
        &mut child,
        &mut stdout,
        &tool_call(2, "session_status", json!({"session_id": session_id})),
    );
    assert!(
        !is_error(&status),
        "session_status failed: {}",
        tool_text(&status)
    );
    let status_content = &status["result"]["structuredContent"];
    assert_eq!(
        status_content["session_id"], session_id,
        "session_status did not echo the session_id opened by session_open: {status:?}"
    );
    // session_status has no "goal" field (checked by running this against
    // the live server -- the schema is
    // urn:chatman:ferroplan-session-status:v1 with cursor, epoch, goal_met,
    // domain_digest, problem_digest, plan_length, remaining_plan_valid,
    // receipt_chain_head). Assert on what it actually reports: the digests
    // of the exact domain/problem session_open grounded, and goal_met
    // false for an unsolved fresh session.
    assert!(
        status_content["domain_digest"].is_string(),
        "session_status did not report a domain_digest grounded by session_open: {status:?}"
    );
    assert!(
        status_content["problem_digest"].is_string(),
        "session_status did not report a problem_digest grounded by session_open: {status:?}"
    );
    assert_eq!(
        status_content["goal_met"],
        json!(false),
        "a freshly opened, unsolved session should report goal_met: false: {status:?}"
    );
    assert!(
        status_content["epoch"].is_u64() || status_content["epoch"].is_i64(),
        "session_status did not report a numeric epoch: {status:?}"
    );
    assert_eq!(
        status_content["cursor"],
        json!(0),
        "a freshly opened session should have cursor 0: {status:?}"
    );

    let close = call(
        &mut child,
        &mut stdout,
        &tool_call(3, "session_close", json!({"session_id": session_id})),
    );
    assert!(
        !is_error(&close),
        "session_close failed: {}",
        tool_text(&close)
    );
    assert_eq!(
        close["result"]["structuredContent"]["closed"],
        json!(true),
        "session_close did not report closed: true: {close:?}"
    );

    drop(child.stdin.take());
    let status_code = child.wait().expect("wait");
    assert!(status_code.success(), "server exited with {status_code:?}");
}

/// Negative falsifier: after `session_close` has dropped a session,
/// `session_status` against that same, now-closed session_id must fail
/// lawfully (a tool-level `isError: true` naming the unknown session) rather
/// than crash the server, hang, or silently return stale state. This is the
/// actual observed behavior of the server -- close removes the session from
/// the live map, so post-close status hits the same "unknown session" path
/// as a session_id that was never opened.
#[test]
fn session_status_after_close_refuses_lawfully() {
    let session_id = "bookends-negative-session";
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
    assert!(
        !is_error(&open),
        "session_open failed: {}",
        tool_text(&open)
    );

    let close = call(
        &mut child,
        &mut stdout,
        &tool_call(2, "session_close", json!({"session_id": session_id})),
    );
    assert!(
        !is_error(&close),
        "session_close failed: {}",
        tool_text(&close)
    );
    assert_eq!(close["result"]["structuredContent"]["closed"], json!(true));

    let status_after_close = call(
        &mut child,
        &mut stdout,
        &tool_call(3, "session_status", json!({"session_id": session_id})),
    );
    assert!(
        is_error(&status_after_close),
        "session_status on a closed session must be a tool-level error, not a crash or stale \
         success: {status_after_close:?}"
    );
    assert!(
        tool_text(&status_after_close).contains("unknown session"),
        "expected an `unknown session` message on post-close status: {status_after_close:?}"
    );

    // A second close on the same (already-closed) session_id, observed
    // against the live server: this is NOT a tool-level error. It returns a
    // normal (isError: false) response with `closed: false`, i.e. "there
    // was nothing here to close" rather than a crash, a hang, or a second
    // `closed: true`. Recorded as the actual behavior rather than the
    // originally assumed "unknown session" refusal, per the no-fabrication
    // rule for negative falsifiers.
    let second_close = call(
        &mut child,
        &mut stdout,
        &tool_call(4, "session_close", json!({"session_id": session_id})),
    );
    assert!(
        !is_error(&second_close),
        "a second session_close on an already-closed session should be a normal (non-error) \
         idempotent response: {second_close:?}"
    );
    assert_eq!(
        second_close["result"]["structuredContent"]["closed"],
        json!(false),
        "double-close should report closed: false (nothing was there to close), not true: \
         {second_close:?}"
    );

    drop(child.stdin.take());
    let status_code = child.wait().expect("wait");
    assert!(status_code.success(), "server exited with {status_code:?}");
}
