//! CE-GALL-36: `session_set_goal` and `session_advance` positive/negative
//! witnesses, driven over stdio against the real `ferroplan-mcp` binary
//! (same harness conventions as `session_protocol.rs`).
//!
//! Positive witness: a session is opened against a small three-action
//! sequential domain, planned to its original goal, retargeted mid-plan to a
//! DIFFERENT but still ground-reachable conjunction (no regrounding), and
//! `session_status`/a fresh `session_think` confirm the retarget took. Then
//! `session_advance` moves the cursor with `completed_steps > 0` over a real
//! plan and `session_status` confirms the cursor moved.
//!
//! Negative falsifier: `session_advance` is called with `completed_steps`
//! larger than the plan's actual remaining length. The TRUE observed
//! behavior (a tool-level refusal, per `do_session_advance`'s
//! `next > plan_length` check in crates/ferroplan-mcp/src/session.rs) is
//! asserted, not assumed.

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
            "clientInfo": {"name": "session-goal-advance-test", "version": "0"}
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

/// Three sequential STRIPS actions over three predicates: `a` moves p->q,
/// `b` moves q->r, `c` moves r->s. The original goal is `(s)` (three-step
/// plan). The retarget goal `(q)` is a DIFFERENT ground conjunction over the
/// same already-interned fact space, reachable in one step from the initial
/// state -- exercising a real, non-trivial retarget rather than a no-op.
const DOM: &str = "(define (domain d3) (:requirements :strips) \
    (:predicates (p) (q) (r) (s)) \
    (:action a :precondition (p) :effect (and (not (p)) (q))) \
    (:action b :precondition (q) :effect (and (not (q)) (r))) \
    (:action c :precondition (r) :effect (and (not (r)) (s))))";
const PROB: &str = "(define (problem pr3) (:domain d3) (:init (p)) (:goal (s)))";

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
    read_response_line(&mut stdout);
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

fn plan_len_from_think(think: &Value) -> usize {
    think["result"]["structuredContent"]["plan"]["steps"]
        .as_array()
        .map(std::vec::Vec::len)
        .or_else(|| {
            think["result"]["structuredContent"]["solution"]["plan"]["steps"]
                .as_array()
                .map(std::vec::Vec::len)
        })
        .unwrap_or(0)
}

/// Positive witness: open -> think (original 3-step goal `(s)`) -> set_goal
/// retargets to a DIFFERENT ground conjunction `(q)` mid-session, without
/// regrounding -> a fresh think against the new goal produces a plan that
/// reaches `(q)`, not `(s)` -> session_status confirms epoch advanced past
/// the retarget -> session_advance moves the cursor with completed_steps > 0
/// over the new plan -> session_status confirms the cursor moved.
#[test]
fn set_goal_retargets_and_advance_moves_cursor_on_a_real_plan() {
    let session_id = "goal-advance-positive-session";
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

    // Plan against the original goal (s): expect a 3-step plan a,b,c.
    let think1 = call(
        &mut child,
        &mut stdout,
        &tool_call(
            2,
            "session_think",
            json!({"session_id": session_id, "max_evaluated": 50_000}),
        ),
    );
    assert!(
        !is_error(&think1),
        "session_think (original goal) failed: {}",
        tool_text(&think1)
    );
    let decision1 = think1["result"]["structuredContent"]["decision"]
        .as_str()
        .unwrap_or_default();
    assert_eq!(
        decision1, "replan",
        "expected a fresh plan against the original goal: {think1:?}"
    );
    let plan_len1 = plan_len_from_think(&think1);
    assert_eq!(
        plan_len1, 3,
        "original goal (s) requires the 3-step a,b,c plan: {think1:?}"
    );

    // Retarget mid-session to a DIFFERENT ground conjunction (q), still over
    // the same grounded fact space -- no regrounding.
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
    let set_goal_content = &set_goal["result"]["structuredContent"];
    assert_eq!(
        set_goal_content["goal_met"],
        json!(false),
        "goal (q) should not be met yet from the initial state: {set_goal:?}"
    );

    // session_status must reflect the retarget: epoch advanced, cursor reset.
    let status_after_retarget = call(
        &mut child,
        &mut stdout,
        &tool_call(4, "session_status", json!({"session_id": session_id})),
    );
    assert!(!is_error(&status_after_retarget));
    let status_content = &status_after_retarget["result"]["structuredContent"];
    assert_eq!(
        status_content["cursor"],
        json!(0),
        "set_goal must reset the cursor: {status_after_retarget:?}"
    );
    assert!(
        status_content["epoch"].as_u64().unwrap_or(0) >= 1,
        "set_goal must advance the epoch: {status_after_retarget:?}"
    );

    // A fresh think against the new goal must produce a plan reaching (q),
    // which from the initial state (p) is the single action `a` -- a
    // 1-step plan, different from the original 3-step plan above. This is
    // the real confirmation that the retarget took, not just a status flag.
    let think2 = call(
        &mut child,
        &mut stdout,
        &tool_call(
            5,
            "session_think",
            json!({"session_id": session_id, "max_evaluated": 50_000}),
        ),
    );
    assert!(
        !is_error(&think2),
        "session_think (retargeted goal) failed: {}",
        tool_text(&think2)
    );
    let plan_len2 = plan_len_from_think(&think2);
    assert_eq!(
        plan_len2, 1,
        "retargeted goal (q) is reachable from (p) in exactly one step (action a): {think2:?}"
    );

    // session_advance with completed_steps > 0 over this real, freshly
    // planned 1-step plan.
    let advance = call(
        &mut child,
        &mut stdout,
        &tool_call(
            6,
            "session_advance",
            json!({"session_id": session_id, "completed_steps": plan_len2}),
        ),
    );
    assert!(
        !is_error(&advance),
        "session_advance failed: {}",
        tool_text(&advance)
    );
    assert_eq!(
        advance["result"]["structuredContent"]["cursor"],
        json!(plan_len2),
        "session_advance did not move the cursor to the reported plan length: {advance:?}"
    );

    // session_status confirms the cursor moved and the retargeted goal is
    // now met (the 1-step plan for (q) has been fully executed per the
    // cursor, and (q) is exactly the current goal).
    let status_after_advance = call(
        &mut child,
        &mut stdout,
        &tool_call(7, "session_status", json!({"session_id": session_id})),
    );
    assert!(!is_error(&status_after_advance));
    assert_eq!(
        status_after_advance["result"]["structuredContent"]["cursor"],
        json!(plan_len2),
        "session_status cursor does not reflect the advance: {status_after_advance:?}"
    );

    drop(child.stdin.take());
    let status_code = child.wait().expect("wait");
    assert!(status_code.success(), "server exited with {status_code:?}");
}

/// Negative falsifier: after planning a real (short) plan, call
/// `session_advance` with `completed_steps` far beyond the plan's actual
/// length. Asserts the TRUE observed behavior -- per
/// `do_session_advance`'s `next > plan_length` check in
/// crates/ferroplan-mcp/src/session.rs, this must come back as a tool-level
/// error naming the plan-length bound, and the cursor must be left
/// unmodified by the rejected call (confirmed via session_status).
#[test]
fn advance_beyond_plan_length_is_refused_and_cursor_is_unchanged() {
    let session_id = "goal-advance-negative-session";
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

    let think = call(
        &mut child,
        &mut stdout,
        &tool_call(
            2,
            "session_think",
            json!({"session_id": session_id, "max_evaluated": 50_000}),
        ),
    );
    assert!(
        !is_error(&think),
        "session_think failed: {}",
        tool_text(&think)
    );
    let plan_len = plan_len_from_think(&think);
    assert_eq!(
        plan_len, 3,
        "original goal (s) requires a 3-step plan: {think:?}"
    );

    // completed_steps is far beyond the real plan length (3) -- an
    // out-of-range advance that must be refused, not silently clamped or
    // silently accepted.
    let bogus_completed_steps = plan_len + 1000;
    let advance = call(
        &mut child,
        &mut stdout,
        &tool_call(
            3,
            "session_advance",
            json!({"session_id": session_id, "completed_steps": bogus_completed_steps}),
        ),
    );
    let observed_is_error = is_error(&advance);
    let observed_text = tool_text(&advance);
    assert!(
        observed_is_error,
        "TRUE OBSERVED BEHAVIOR CHECK: session_advance with completed_steps={bogus_completed_steps} \
         (plan length {plan_len}) was expected to be refused per do_session_advance's \
         `next > plan_length` guard, but the call did NOT come back as isError. Full response: \
         {advance:?}"
    );
    assert!(
        observed_text.contains("beyond plan length"),
        "expected the refusal message to name the plan-length bound (matching \
         do_session_advance's exact wording), got: {observed_text}"
    );

    // Confirm the rejected call left the cursor untouched: a subsequent
    // legitimate advance to the full real plan length must still succeed
    // and land exactly at plan_len, not at some partially-applied value.
    let status = call(
        &mut child,
        &mut stdout,
        &tool_call(4, "session_status", json!({"session_id": session_id})),
    );
    assert!(!is_error(&status));
    assert_eq!(
        status["result"]["structuredContent"]["cursor"],
        json!(0),
        "the refused out-of-range advance must not have moved the cursor: {status:?}"
    );

    let valid_advance = call(
        &mut child,
        &mut stdout,
        &tool_call(
            5,
            "session_advance",
            json!({"session_id": session_id, "completed_steps": plan_len}),
        ),
    );
    assert!(
        !is_error(&valid_advance),
        "a legitimate in-range advance after the refused one should still succeed: {}",
        tool_text(&valid_advance)
    );
    assert_eq!(
        valid_advance["result"]["structuredContent"]["cursor"],
        json!(plan_len)
    );

    drop(child.stdin.take());
    let status_code = child.wait().expect("wait");
    assert!(status_code.success(), "server exited with {status_code:?}");
}
