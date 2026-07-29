//! Drive the built `ferroplan-mcp` binary (the session tools within the
//! merged server) over stdio and check the per-session locking refactor
//! (Fix 1): concurrent tool calls against the *same* session_id queue on
//! that session's own lock rather than racing a remove/reinsert and
//! observing "unknown session".

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
    assert!(
        !is_error(&open),
        "session_open failed: {}",
        tool_text(&open)
    );
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
    assert!(
        !is_error(&think),
        "session_think failed: {}",
        tool_text(&think)
    );
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

/// Eight admitted candidates with one node's id fixed, so a descent step can
/// name it as `selected_parent_node`.
fn eight_candidates(prefix: &str) -> Vec<Value> {
    (0..8)
        .map(|i| {
            json!({
                "id": format!("{prefix}-{i}"),
                "parent": if i == 0 { Value::Null } else { json!(0) },
                "factors": vec![0.5_f64; 10],
                "cost": 1.0
            })
        })
        .collect()
}

/// Falsifier 1: a depth-one-only recursive call (`descents: []`) must match
/// plain `cmca_allocate`'s output exactly -- same candidates in, same
/// allocations/digest out. Proves the shared `run_one_allocation` refactor
/// changed nothing about the existing tool's behavior.
#[test]
fn cmca_recursive_depth_one_matches_plain_cmca_allocate() {
    let (mut child, mut stdout) = spawn_and_handshake();
    let candidates = eight_candidates("root");

    let plain = call(
        &mut child,
        &mut stdout,
        &tool_call(1, "cmca_allocate", json!({"candidates": candidates})),
    );
    assert!(
        !is_error(&plain),
        "cmca_allocate failed: {}",
        tool_text(&plain)
    );

    let recursive = call(
        &mut child,
        &mut stdout,
        &tool_call(
            2,
            "cmca_allocate_recursive",
            json!({"root": candidates, "descents": []}),
        ),
    );
    assert!(
        !is_error(&recursive),
        "cmca_allocate_recursive failed: {}",
        tool_text(&recursive)
    );

    let plain_payload = &plain["result"]["structuredContent"]["payload"];
    let depths = recursive["result"]["structuredContent"]["payload"]["depths"]
        .as_array()
        .expect("depths array");
    assert_eq!(
        depths.len(),
        1,
        "descents: [] must produce exactly one depth"
    );
    assert_eq!(
        &depths[0]["allocation_payload"], plain_payload,
        "depth-one's allocation payload must be byte-identical to plain cmca_allocate's payload"
    );
    assert_eq!(depths[0]["selected_parent_node"], Value::Null);
    assert_eq!(depths[0]["parent_payload_digest"], Value::Null);

    drop(child.stdin.take());
    child.wait().expect("wait");
}

/// Falsifier 2: a real two-level descent. Depth two's `parent_payload_digest`
/// must equal depth one's real `allocation_payload_digest`.
#[test]
fn cmca_recursive_depth_two_binds_the_real_parent_digest() {
    let (mut child, mut stdout) = spawn_and_handshake();
    let root = eight_candidates("root");
    let descend_target = root[0]["id"].as_str().unwrap().to_owned();

    let resp = call(
        &mut child,
        &mut stdout,
        &tool_call(
            1,
            "cmca_allocate_recursive",
            json!({
                "root": root,
                "descents": [{
                    "selected_parent_node": descend_target,
                    "candidates": eight_candidates("child")
                }]
            }),
        ),
    );
    assert!(
        !is_error(&resp),
        "recursive call failed: {}",
        tool_text(&resp)
    );

    let depths = resp["result"]["structuredContent"]["payload"]["depths"]
        .as_array()
        .expect("depths array");
    assert_eq!(depths.len(), 2);
    let depth_one_digest = depths[0]["allocation_payload_digest"].clone();
    assert_eq!(
        depths[1]["parent_payload_digest"], depth_one_digest,
        "depth two's parent_payload_digest must equal depth one's real allocation_payload_digest"
    );

    drop(child.stdin.take());
    child.wait().expect("wait");
}

/// Falsifier 3: `selected_parent_node` names an id that was never admitted
/// at the previous depth -- must refuse, not silently succeed.
#[test]
fn cmca_recursive_refuses_an_unknown_selected_parent_node() {
    let (mut child, mut stdout) = spawn_and_handshake();
    let root = eight_candidates("root");

    let resp = call(
        &mut child,
        &mut stdout,
        &tool_call(
            1,
            "cmca_allocate_recursive",
            json!({
                "root": root,
                "descents": [{
                    "selected_parent_node": "node-that-was-never-admitted",
                    "candidates": eight_candidates("child")
                }]
            }),
        ),
    );
    assert!(
        is_error(&resp),
        "expected refusal for an unknown selected_parent_node, got: {}",
        tool_text(&resp)
    );

    drop(child.stdin.take());
    child.wait().expect("wait");
}

/// Falsifier 4: cyclic ancestry -- a depth-three descent re-selects the same
/// node id used to enter depth two. Must refuse.
#[test]
fn cmca_recursive_refuses_cyclic_ancestry() {
    let (mut child, mut stdout) = spawn_and_handshake();
    let root = eight_candidates("root");
    let root_target = root[0]["id"].as_str().unwrap().to_owned();
    let depth_two_candidates = eight_candidates("depth2");
    let depth_two_target = depth_two_candidates[0]["id"].as_str().unwrap().to_owned();

    let resp = call(
        &mut child,
        &mut stdout,
        &tool_call(
            1,
            "cmca_allocate_recursive",
            json!({
                "root": root,
                "descents": [
                    {
                        "selected_parent_node": root_target,
                        "candidates": depth_two_candidates
                    },
                    {
                        // Reuses root_target's id as the depth-three entry
                        // point -- the same id already used to enter depth
                        // two above -- which must refuse as cyclic ancestry
                        // even though depth_two_target is otherwise a real,
                        // admitted depth-two candidate.
                        "selected_parent_node": depth_two_target,
                        "candidates": eight_candidates("depth3")
                    }
                ]
            }),
        ),
    );
    // This first construction is NOT cyclic (root_target != depth_two_target
    // by construction, different prefixes) -- confirm it succeeds, then
    // build the actually-cyclic case below.
    assert!(
        !is_error(&resp),
        "non-cyclic 3-depth chain unexpectedly refused: {}",
        tool_text(&resp)
    );

    let cyclic_resp = call(
        &mut child,
        &mut stdout,
        &tool_call(
            2,
            "cmca_allocate_recursive",
            json!({
                "root": root,
                "descents": [
                    {
                        "selected_parent_node": root_target,
                        "candidates": depth_two_candidates_for_cycle(&root_target)
                    },
                    {
                        // Re-selects root_target -- already used to enter
                        // depth two -- a genuine cycle.
                        "selected_parent_node": root_target,
                        "candidates": eight_candidates("depth3-cyclic")
                    }
                ]
            }),
        ),
    );
    assert!(
        is_error(&cyclic_resp),
        "expected cyclic-ancestry refusal, got: {}",
        tool_text(&cyclic_resp)
    );

    drop(child.stdin.take());
    child.wait().expect("wait");
}

/// Depth-two candidates for the cyclic-ancestry falsifier: includes
/// `root_target` as one of its own admitted ids so depth three can validly
/// re-select it by id (the id is a legitimate depth-two candidate) while
/// still being a real cycle (that same id string already entered depth two).
fn depth_two_candidates_for_cycle(root_target: &str) -> Vec<Value> {
    let mut candidates = eight_candidates("depth2-cyc");
    candidates[0]["id"] = json!(root_target);
    candidates
}

/// Falsifier 5: a descent step's candidate list has the wrong count (7, not
/// N=8) -- the WHOLE call must refuse, including depth one's otherwise-valid
/// allocation (no partial chain returned).
#[test]
fn cmca_recursive_refuses_the_whole_chain_on_a_bad_depth() {
    let (mut child, mut stdout) = spawn_and_handshake();
    let root = eight_candidates("root");
    let root_target = root[0]["id"].as_str().unwrap().to_owned();
    let mut bad_candidates = eight_candidates("child");
    bad_candidates.pop(); // 7 entries, not the required 8

    let resp = call(
        &mut child,
        &mut stdout,
        &tool_call(
            1,
            "cmca_allocate_recursive",
            json!({
                "root": root,
                "descents": [{
                    "selected_parent_node": root_target,
                    "candidates": bad_candidates
                }]
            }),
        ),
    );
    assert!(
        is_error(&resp),
        "expected the whole call to refuse on a malformed depth-two candidate list, got: {}",
        tool_text(&resp)
    );
    assert!(
        resp["result"]["structuredContent"].is_null(),
        "a refused call must not return any partial allocation chain: {resp:?}"
    );

    drop(child.stdin.take());
    child.wait().expect("wait");
}

/// Falsifier 6: determinism -- two calls with the identical
/// `CmcaRecursiveInput` must produce byte-identical output at every depth.
#[test]
fn cmca_recursive_is_deterministic_across_repeated_calls() {
    let (mut child, mut stdout) = spawn_and_handshake();
    let root = eight_candidates("root");
    let root_target = root[0]["id"].as_str().unwrap().to_owned();
    let input = json!({
        "root": root,
        "descents": [{
            "selected_parent_node": root_target,
            "candidates": eight_candidates("child")
        }]
    });

    let first = call(
        &mut child,
        &mut stdout,
        &tool_call(1, "cmca_allocate_recursive", input.clone()),
    );
    let second = call(
        &mut child,
        &mut stdout,
        &tool_call(2, "cmca_allocate_recursive", input),
    );
    assert!(!is_error(&first) && !is_error(&second));
    assert_eq!(
        first["result"]["structuredContent"], second["result"]["structuredContent"],
        "identical input must produce byte-identical output at every depth"
    );

    drop(child.stdin.take());
    child.wait().expect("wait");
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
    // The merged server exposes 16 resources total; this session group's
    // eight must be present (full 16-resource exactness is
    // `merged_server.rs`'s job).
    let names: std::collections::BTreeSet<&str> = resources
        .iter()
        .filter_map(|r| r["uri"].as_str())
        .filter_map(|uri| uri.strip_prefix("ferroplan://tools/"))
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
    assert!(
        expected.is_subset(&names),
        "expected session tool resources missing: {:?}",
        expected.difference(&names).collect::<Vec<_>>()
    );

    let read = call(
        &mut child,
        &mut stdout,
        &json!({
            "jsonrpc": "2.0",
            "id": 2,
            "method": "resources/read",
            "params": {"uri": "ferroplan://tools/session_think"}
        }),
    );
    let contents = read["result"]["contents"]
        .as_array()
        .unwrap_or_else(|| panic!("resources/read returned no contents array: {read:?}"));
    assert_eq!(
        contents.len(),
        1,
        "expected exactly one content block: {read:?}"
    );
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
    let bin = env!("CARGO_BIN_EXE_ferroplan-mcp");
    let mut child = Command::new(bin)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .expect("spawn ferroplan-mcp");
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
    let resp = vec![
        read_response_line(&mut stdout),
        read_response_line(&mut stdout),
    ];
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
