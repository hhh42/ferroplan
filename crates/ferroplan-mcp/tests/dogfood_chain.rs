//! CE-GALL-40 -- Full 17-Tool Dogfood Chain.
//!
//! No test anywhere in this repository drove all 17 `ferroplan-mcp` tools in
//! one continuous chained flow before this file. `session_protocol.rs` and
//! `session_lifecycle_bookends.rs`/`session_goal_advance.rs` each cover a
//! slice; none of them touch `parse`, `bind_allocation_receipt`,
//! `bind_plan_receipt`, or `verify_receipt` in the same run as the session
//! lifecycle and CMCA tools. This file drives `parse`, `solve`,
//! `session_open`, `session_observe`, `session_set_goal`, `session_think`,
//! `session_advance`, `cmca_allocate`, `cmca_allocate_recursive`,
//! `canonical_digest`, `bind_allocation_receipt`, `bind_plan_receipt`,
//! `verify_receipt`, `validate`, `session_status`, and `session_close` -- 16
//! distinct tool invocations -- as one continuous happy-path chain, plus a
//! negative falsifier calling `session_status` on the closed session_id.
//!
//! `decompose` is the 17th tool in the server's surface and is deliberately
//! NOT called here -- named as a gap, not silently skipped (see
//! CE-GALL-40's receipt `known_anti_patterns`).

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
            "clientInfo": {"name": "dogfood-chain-test", "version": "0"}
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

// A tiny two-action STRIPS domain: at-a -> at-b -> at-c.
const DOM: &str = "(define (domain loc) (:requirements :strips) \
    (:predicates (at-a) (at-b) (at-c)) \
    (:action ab :precondition (at-a) :effect (and (not (at-a)) (at-b))) \
    (:action bc :precondition (at-b) :effect (and (not (at-b)) (at-c))))";
const PROB: &str = "(define (problem locp) (:domain loc) (:init (at-a)) (:goal (at-c)))";

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

/// Eight notional work-frontier candidates, ten factors each -- the shape
/// `cmca_allocate` and `cmca_allocate_recursive`'s root both require.
fn eight_candidates(prefix: &str) -> Value {
    let rows: Vec<[f64; 10]> = vec![
        [0.1, 0.2, 0.3, 0.4, 0.5, 0.6, 0.7, 0.8, 0.9, 1.0],
        [0.2, 0.1, 0.4, 0.3, 0.6, 0.5, 0.8, 0.7, 1.0, 0.9],
        [0.3, 0.4, 0.1, 0.2, 0.7, 0.8, 0.5, 0.6, 0.9, 1.0],
        [0.4, 0.3, 0.2, 0.1, 0.8, 0.7, 0.6, 0.5, 1.0, 0.9],
        [0.5, 0.6, 0.7, 0.8, 0.1, 0.2, 0.3, 0.4, 0.9, 1.0],
        [0.6, 0.5, 0.8, 0.7, 0.2, 0.1, 0.4, 0.3, 1.0, 0.9],
        [0.7, 0.8, 0.5, 0.6, 0.3, 0.4, 0.1, 0.2, 0.9, 1.0],
        [0.8, 0.7, 0.6, 0.5, 0.4, 0.3, 0.2, 0.1, 1.0, 0.9],
    ];
    let candidates: Vec<Value> = rows
        .into_iter()
        .enumerate()
        .map(|(i, factors)| json!({"id": format!("{prefix}{i}"), "factors": factors}))
        .collect();
    json!(candidates)
}

/// Happy path: all 16 tool calls chained in order, each one's output feeding
/// the next. Proves the full 17-tool surface (minus `decompose`, named as a
/// gap in the module doc comment) composes end to end against a real
/// two-action domain, not just individually.
#[test]
fn full_seventeen_tool_dogfood_chain() {
    let session_id = "dogfood-chain-ce-gall-40";
    let (mut child, mut stdout) = spawn_and_handshake();
    let mut id = 0i64;
    let mut next_id = || {
        id += 1;
        id
    };

    // 1. parse (domain)
    let parse_domain = call(
        &mut child,
        &mut stdout,
        &tool_call(next_id(), "parse", json!({"pddl": DOM})),
    );
    assert!(
        !is_error(&parse_domain),
        "parse(domain) failed: {}",
        tool_text(&parse_domain)
    );
    assert_eq!(
        parse_domain["result"]["structuredContent"]["ok"],
        json!(true),
        "domain did not parse ok: {parse_domain:?}"
    );

    // parse (problem)
    let parse_problem = call(
        &mut child,
        &mut stdout,
        &tool_call(next_id(), "parse", json!({"pddl": PROB})),
    );
    assert!(
        !is_error(&parse_problem),
        "parse(problem) failed: {}",
        tool_text(&parse_problem)
    );
    assert_eq!(
        parse_problem["result"]["structuredContent"]["ok"],
        json!(true),
        "problem did not parse ok: {parse_problem:?}"
    );

    // 2. solve -> monolithic 2-step plan (ab, bc)
    let solve = call(
        &mut child,
        &mut stdout,
        &tool_call(
            next_id(),
            "solve",
            json!({"domain": DOM, "problem": PROB}),
        ),
    );
    assert!(!is_error(&solve), "solve failed: {}", tool_text(&solve));
    let solve_content = &solve["result"]["structuredContent"];
    assert_eq!(solve_content["solved"], json!(true), "solve: {solve:?}");
    assert_eq!(
        solve_content["plan"]["length"],
        json!(2),
        "expected a 2-step monolithic plan: {solve:?}"
    );

    // 3. session_open on the same domain/problem
    let open = call(
        &mut child,
        &mut stdout,
        &tool_call(
            next_id(),
            "session_open",
            json!({"session_id": session_id, "domain": DOM, "problem": PROB}),
        ),
    );
    assert!(!is_error(&open), "session_open failed: {}", tool_text(&open));
    assert_eq!(
        open["result"]["structuredContent"]["session_id"],
        session_id,
        "session_open: {open:?}"
    );

    // 4. session_observe with a minimal facts payload
    let observe = call(
        &mut child,
        &mut stdout,
        &tool_call(
            next_id(),
            "session_observe",
            json!({
                "session_id": session_id,
                "facts": [{"fact": "(at-a)", "value": true}]
            }),
        ),
    );
    assert!(
        !is_error(&observe),
        "session_observe failed: {}",
        tool_text(&observe)
    );

    // 5. session_set_goal retargeting to a reachable ground conjunction
    let set_goal = call(
        &mut child,
        &mut stdout,
        &tool_call(
            next_id(),
            "session_set_goal",
            json!({"session_id": session_id, "goal": "(at-b)"}),
        ),
    );
    assert!(
        !is_error(&set_goal),
        "session_set_goal failed: {}",
        tool_text(&set_goal)
    );

    // 6. session_think -> candidate plan for the retargeted goal (1 step: ab)
    let think = call(
        &mut child,
        &mut stdout,
        &tool_call(next_id(), "session_think", json!({"session_id": session_id})),
    );
    assert!(!is_error(&think), "session_think failed: {}", tool_text(&think));
    let think_content = think["result"]["structuredContent"].clone();
    assert_eq!(
        think_content["solution"]["solved"],
        json!(true),
        "session_think: {think_content:?}"
    );
    assert_eq!(
        think_content["solution"]["plan"]["length"],
        json!(1),
        "retargeted goal (at-b) should plan in exactly 1 step: {think_content:?}"
    );

    // 7. session_advance past 1 completed step
    let advance = call(
        &mut child,
        &mut stdout,
        &tool_call(
            next_id(),
            "session_advance",
            json!({"session_id": session_id, "completed_steps": 1}),
        ),
    );
    assert!(
        !is_error(&advance),
        "session_advance failed: {}",
        tool_text(&advance)
    );
    assert_eq!(
        advance["result"]["structuredContent"]["cursor"],
        json!(1),
        "session_advance: {advance:?}"
    );

    // 8. cmca_allocate with 8 candidates, 10 factors each
    let cmca = call(
        &mut child,
        &mut stdout,
        &tool_call(
            next_id(),
            "cmca_allocate",
            json!({"candidates": eight_candidates("w")}),
        ),
    );
    assert!(!is_error(&cmca), "cmca_allocate failed: {}", tool_text(&cmca));
    let cmca_content = cmca["result"]["structuredContent"].clone();
    let allocations = cmca_content["payload"]["allocations"]
        .as_array()
        .expect("cmca_allocate: allocations array");
    assert_eq!(allocations.len(), 8, "cmca_allocate: {cmca_content:?}");

    // 9. cmca_allocate_recursive: root + 1 descent (2 depths total)
    let recursive = call(
        &mut child,
        &mut stdout,
        &tool_call(
            next_id(),
            "cmca_allocate_recursive",
            json!({
                "root": eight_candidates("w"),
                "descents": [
                    {"selected_parent_node": "w0", "candidates": eight_candidates("d")}
                ]
            }),
        ),
    );
    assert!(
        !is_error(&recursive),
        "cmca_allocate_recursive failed: {}",
        tool_text(&recursive)
    );
    let recursive_content = recursive["result"]["structuredContent"].clone();
    let depths = recursive_content["payload"]["depths"]
        .as_array()
        .expect("cmca_allocate_recursive: depths array");
    assert_eq!(
        depths.len(),
        2,
        "expected root + 1 descent = 2 depths: {recursive_content:?}"
    );
    let depth1_digest = depths[0]["allocation_payload_digest"]
        .as_str()
        .expect("depth 1 allocation_payload_digest")
        .to_owned();
    let depth2_parent_digest = depths[1]["parent_payload_digest"]
        .as_str()
        .expect("depth 2 parent_payload_digest")
        .to_owned();
    assert_eq!(
        depth1_digest, depth2_parent_digest,
        "depth 2's parent_payload_digest must equal depth 1's real digest, not a placeholder"
    );

    // 10. canonical_digest over the session_think result
    let digest = call(
        &mut child,
        &mut stdout,
        &tool_call(
            next_id(),
            "canonical_digest",
            json!({"value": think_content}),
        ),
    );
    assert!(
        !is_error(&digest),
        "canonical_digest failed: {}",
        tool_text(&digest)
    );
    assert!(
        digest["result"]["structuredContent"]["digest"]
            .as_str()
            .is_some_and(|s| !s.is_empty()),
        "canonical_digest: {digest:?}"
    );

    // 11. bind_allocation_receipt binding the cmca_allocate_recursive result.
    // bind_allocation_receipt's schema expects the flat cmca_allocate shape
    // (payload.bcinr_revision at the top level), not the recursive `depths`
    // shape -- so the root depth's own payload is bound as
    // `allocation_result`, with the recursive extension folded in as an
    // explicit extra field so the depth-2 chain is still recorded in the
    // bound receipt rather than silently dropped.
    let allocation_result = json!({
        "payload": cmca_content["payload"].clone(),
        "payload_digest": cmca_content["payload_digest"].clone(),
        "recursive_extension": {
            "depth_count": recursive_content["payload"]["depth_count"].clone(),
            "depth2_selected_parent_node": depths[1]["selected_parent_node"].clone(),
            "depth2_parent_payload_digest": depth2_parent_digest.clone(),
            "depth2_allocation_payload_digest": depths[1]["allocation_payload_digest"].clone(),
            "recursive_payload_digest": recursive_content["payload_digest"].clone(),
        }
    });
    let observation_frontier = json!({
        "session_id": session_id,
        "facts": [{"fact": "(at-a)", "value": true}],
        "epoch": 1
    });
    let bind_alloc = call(
        &mut child,
        &mut stdout,
        &tool_call(
            next_id(),
            "bind_allocation_receipt",
            json!({
                "candidates": eight_candidates("w"),
                "allocation_result": allocation_result,
                "observation_frontier": observation_frontier
            }),
        ),
    );
    assert!(
        !is_error(&bind_alloc),
        "bind_allocation_receipt failed: {}",
        tool_text(&bind_alloc)
    );
    let allocation_receipt = bind_alloc["result"]["structuredContent"]["receipt"]
        .as_str()
        .expect("bind_allocation_receipt: receipt")
        .to_owned();
    assert!(
        !allocation_receipt.is_empty(),
        "bind_allocation_receipt returned an empty receipt: {bind_alloc:?}"
    );

    // 14 (called here, before bind_plan_receipt, since its result feeds
    // validator_result): validate the plan session_think produced,
    // independently, against the retargeted (at-a)->(at-b) goal.
    let validate = call(
        &mut child,
        &mut stdout,
        &tool_call(
            next_id(),
            "validate",
            json!({
                "domain": DOM,
                "problem": "(define (problem locp) (:domain loc) (:init (at-a)) (:goal (at-b)))",
                "plan": "step 1: (ab)"
            }),
        ),
    );
    assert!(
        !is_error(&validate),
        "validate failed: {}",
        tool_text(&validate)
    );
    let validator_result = validate["result"]["structuredContent"].clone();
    assert_eq!(
        validator_result["valid"],
        json!(true),
        "validate: {validator_result:?}"
    );

    // 12. bind_plan_receipt binding session_think + allocation receipt +
    // an observation frontier + the validator result above.
    let bind_plan = call(
        &mut child,
        &mut stdout,
        &tool_call(
            next_id(),
            "bind_plan_receipt",
            json!({
                "session_think": think_content,
                "allocation_receipt": allocation_receipt,
                "observation_frontier": observation_frontier,
                "validator_result": validator_result
            }),
        ),
    );
    assert!(
        !is_error(&bind_plan),
        "bind_plan_receipt failed: {}",
        tool_text(&bind_plan)
    );
    let plan_envelope = bind_plan["result"]["structuredContent"].clone();
    assert!(
        plan_envelope["receipt"]
            .as_str()
            .is_some_and(|s| !s.is_empty()),
        "bind_plan_receipt: {plan_envelope:?}"
    );

    // 13. verify_receipt on the plan envelope from step 12
    let verify = call(
        &mut child,
        &mut stdout,
        &tool_call(
            next_id(),
            "verify_receipt",
            json!({"envelope": plan_envelope}),
        ),
    );
    assert!(
        !is_error(&verify),
        "verify_receipt failed: {}",
        tool_text(&verify)
    );
    assert_eq!(
        verify["result"]["structuredContent"]["valid"],
        json!(true),
        "verify_receipt should confirm the freshly-bound plan envelope recomputes cleanly: \
         {verify:?}"
    );

    // 15. session_status for final session state
    let status = call(
        &mut child,
        &mut stdout,
        &tool_call(next_id(), "session_status", json!({"session_id": session_id})),
    );
    assert!(
        !is_error(&status),
        "session_status failed: {}",
        tool_text(&status)
    );
    assert_eq!(
        status["result"]["structuredContent"]["cursor"],
        json!(1),
        "session_status should reflect the step 7 advance: {status:?}"
    );

    // 16. session_close
    let close = call(
        &mut child,
        &mut stdout,
        &tool_call(next_id(), "session_close", json!({"session_id": session_id})),
    );
    assert!(
        !is_error(&close),
        "session_close failed: {}",
        tool_text(&close)
    );
    assert_eq!(
        close["result"]["structuredContent"]["closed"],
        json!(true),
        "session_close: {close:?}"
    );

    drop(child.stdin.take());
    let status_code = child.wait().expect("wait");
    assert!(status_code.success(), "server exited with {status_code:?}");
}

/// Negative falsifier: `session_status` on the same session_id AFTER
/// `session_close` must refuse with an unknown-session error, consistent
/// with CE-GALL-35's finding for the bookend tools.
#[test]
fn session_status_after_close_refuses_unknown_session() {
    let session_id = "dogfood-chain-ce-gall-40-falsifier";
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
        "session_status on a closed session must refuse, not succeed or crash: \
         {status_after_close:?}"
    );
    assert!(
        tool_text(&status_after_close).contains("unknown session"),
        "expected an `unknown session` refusal message: {status_after_close:?}"
    );

    drop(child.stdin.take());
    let status_code = child.wait().expect("wait");
    assert!(status_code.success(), "server exited with {status_code:?}");
}
