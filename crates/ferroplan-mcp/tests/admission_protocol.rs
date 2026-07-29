//! Drive the built `ferroplan-mcp` binary (the admission tools within the
//! merged server) over stdio and check the JSON-RPC / MCP protocol end to
//! end: initialize, tools/list, each tool's happy path, tamper-detection in
//! `verify_receipt`, resources, and malformed-input rejection. Pure
//! protocol-level determinism: no LLM involved anywhere, only the compiled
//! binary and real BLAKE3 arithmetic.

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
            "clientInfo": {"name": "admission-protocol-test", "version": "0"}
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

fn tool_call(id: i64, name: &str, arguments: Value) -> Value {
    json!({
        "jsonrpc": "2.0", "id": id, "method": "tools/call",
        "params": {"name": name, "arguments": arguments}
    })
}

/// Find the response whose `id` matches, tolerating the async server's freedom
/// to resolve concurrent in-flight requests out of arrival order.
fn find_response(resp: &[Value], id: i64) -> Value {
    resp.iter()
        .find(|v| v["id"] == json!(id))
        .unwrap_or_else(|| panic!("no response with id {id} in {resp:?}"))
        .clone()
}

/// Parse the JSON `structuredContent` a successful tool call returns, panicking
/// with the raw response if the call was actually an error.
fn structured(resp: &Value) -> Value {
    assert_eq!(
        resp["result"]["isError"], false,
        "expected a successful tool call, got: {resp:?}"
    );
    resp["result"]["structuredContent"].clone()
}

const BCINR_REVISION: &str = "fb9321d27882169acc83aaca0639b319cd3b7900";

/// A 64-character hex digest that is well-formed per `^[0-9a-f]{64}$` but is
/// not the digest of anything in particular — good enough wherever the code
/// under test only checks *shape*, not provenance (e.g. `previous_receipt`,
/// `allocation_receipt` as consumed by `bind_plan_receipt`).
fn placeholder_digest() -> String {
    blake3::hash(b"admission-protocol-test-placeholder")
        .to_hex()
        .to_string()
}

fn eight_candidates() -> Value {
    json!([1, 2, 3, 4, 5, 6, 7, 8])
}

/// Eight candidates with real `id`s (unlike `eight_candidates`'s bare
/// integers) so a recursive descent can name one of them as `selected_node`.
fn eight_candidates_with_ids(prefix: &str) -> Value {
    json!((0..8)
        .map(|i| json!({"id": format!("{prefix}-{i}")}))
        .collect::<Vec<Value>>())
}

fn allocation_result_with(revision: &str, n_allocations: usize) -> Value {
    json!({
        "payload": {
            "bcinr_revision": revision,
            "allocations": (0..n_allocations).collect::<Vec<usize>>(),
        }
    })
}

#[test]
fn initialize_advertises_server_and_tools() {
    let resp = raw_drive(&[
        json!({
            "jsonrpc":"2.0","id":1,"method":"initialize",
            "params":{
                "protocolVersion":"2025-06-18",
                "capabilities": {},
                "clientInfo": {"name": "admission-protocol-test", "version": "0"}
            }
        }),
        json!({"jsonrpc":"2.0","method":"notifications/initialized"}),
        json!({"jsonrpc":"2.0","id":2,"method":"tools/list"}),
    ]);
    assert_eq!(resp.len(), 2, "notification must not produce a response");
    assert_eq!(resp[0]["id"], 1);
    // The three original servers are merged into one `ferroplan` binary; see
    // `merged_server.rs` for the full 16-tool assertion.
    assert_eq!(resp[0]["result"]["serverInfo"]["name"], "ferroplan");
    assert_eq!(resp[0]["result"]["protocolVersion"], "2025-06-18");
    let capabilities = &resp[0]["result"]["capabilities"];
    assert!(
        capabilities.get("tools").is_some(),
        "expected tools capability"
    );
    assert!(
        capabilities.get("resources").is_some(),
        "expected resources capability"
    );

    let names: std::collections::BTreeSet<&str> = resp[1]["result"]["tools"]
        .as_array()
        .unwrap()
        .iter()
        .map(|t| t["name"].as_str().unwrap())
        .collect();
    // This server's own admission tools must still be present in the merged
    // tool set (full 16-tool exactness is `merged_server.rs`'s job).
    for expected in [
        "bind_allocation_receipt",
        "bind_plan_receipt",
        "canonical_digest",
        "verify_receipt",
    ] {
        assert!(
            names.contains(expected),
            "missing tool `{expected}`: {names:?}"
        );
    }
}

#[test]
fn canonical_digest_happy_path() {
    let resp = drive(&[tool_call(
        1,
        "canonical_digest",
        json!({"value": {"b": 2, "a": 1}}),
    )]);
    let out = structured(&find_response(&resp, 1));
    assert_eq!(out["schema"], "urn:chatman:canonical-digest:v1");
    assert_eq!(out["algorithm"], "BLAKE3");
    // key-sorted canonicalization: `a` before `b` regardless of input order.
    assert_eq!(out["canonical"], json!({"a": 1, "b": 2}));
    let digest = out["digest"].as_str().expect("digest is a string");
    assert_eq!(digest.len(), 64);
    assert!(digest.bytes().all(|b| b.is_ascii_hexdigit()));
    // Recompute independently to prove this is a real BLAKE3 digest over the
    // canonicalized (key-sorted) JSON bytes, not a placeholder.
    let expected = blake3::hash(&serde_json::to_vec(&json!({"a": 1, "b": 2})).unwrap())
        .to_hex()
        .to_string();
    assert_eq!(digest, expected);
}

/// Regression test for a real client-observed defect: an MCP client can send
/// an `any_json`-schema argument as a JSON-encoded *string* rather than a
/// native value (observed in practice against this exact tool — the
/// stringified form defeated `canonicalize`'s key-sorting because the value
/// never became a `Value::Object`/`Value::Array`). `coerce_stringified_json`
/// must recover the same canonicalization as a native argument.
#[test]
fn canonical_digest_accepts_a_stringified_json_value() {
    let resp = drive(&[tool_call(
        1,
        "canonical_digest",
        json!({"value": "{\"b\": 2, \"a\": 1}"}),
    )]);
    let out = structured(&find_response(&resp, 1));
    // Same key-sorted result as the native-object happy path, proving the
    // string was parsed and canonicalized, not merely echoed back.
    assert_eq!(out["canonical"], json!({"a": 1, "b": 2}));
    let expected = blake3::hash(&serde_json::to_vec(&json!({"a": 1, "b": 2})).unwrap())
        .to_hex()
        .to_string();
    assert_eq!(out["digest"], expected);
}

/// A string that is not itself JSON (or parses to a bare scalar) must be
/// left exactly as-is — `coerce_stringified_json` only recovers structure
/// that transit already lost, it never reinterprets a real string field.
#[test]
fn canonical_digest_leaves_a_non_json_string_untouched() {
    let resp = drive(&[tool_call(
        1,
        "canonical_digest",
        json!({"value": "just text"}),
    )]);
    let out = structured(&find_response(&resp, 1));
    assert_eq!(out["canonical"], json!("just text"));
}

#[test]
fn bind_allocation_receipt_accepts_stringified_candidates_and_allocation_result() {
    // Same inputs as `bind_allocation_receipt_happy_path`, but every
    // `any_json` argument is JSON-encoded as a string first — reproducing
    // what a real client was observed sending.
    let resp = drive(&[tool_call(
        1,
        "bind_allocation_receipt",
        json!({
            "candidates": serde_json::to_string(&eight_candidates()).unwrap(),
            "allocation_result":
                serde_json::to_string(&allocation_result_with(BCINR_REVISION, 8)).unwrap(),
            "observation_frontier": serde_json::to_string(&json!({"frontier": "empty"})).unwrap(),
        }),
    )]);
    let out = structured(&find_response(&resp, 1));
    assert_eq!(out["schema"], "urn:chatman:admission-envelope:v1");
    assert_eq!(out["kind"], "allocation");
    assert_eq!(
        out["payload"]["candidates"],
        eight_candidates(),
        "stringified candidates must be recovered as the real array, not left as text"
    );
    assert_eq!(
        out["payload"]["bcinr_revision"], BCINR_REVISION,
        "payload carries the admitted BCINR revision"
    );
}

#[test]
fn bind_allocation_receipt_happy_path() {
    let resp = drive(&[tool_call(
        1,
        "bind_allocation_receipt",
        json!({
            "candidates": eight_candidates(),
            "allocation_result": allocation_result_with(BCINR_REVISION, 8),
            "observation_frontier": {"frontier": "empty"},
        }),
    )]);
    let out = structured(&find_response(&resp, 1));
    assert_eq!(out["schema"], "urn:chatman:admission-envelope:v1");
    assert_eq!(out["kind"], "allocation");
    assert_eq!(out["algorithm"], "BLAKE3");
    assert!(out["previous_receipt"].is_null());
    let payload_digest = out["payload_digest"].as_str().expect("payload_digest");
    assert_eq!(payload_digest.len(), 64);
    let receipt = out["receipt"].as_str().expect("receipt");
    assert_eq!(receipt.len(), 64);
    assert_eq!(
        out["payload"]["bcinr_revision"], BCINR_REVISION,
        "payload carries the admitted BCINR revision"
    );
}

#[test]
fn bind_allocation_receipt_rejects_wrong_bcinr_revision() {
    let resp = drive(&[tool_call(
        1,
        "bind_allocation_receipt",
        json!({
            "candidates": eight_candidates(),
            "allocation_result": allocation_result_with("not-the-admitted-revision", 8),
            "observation_frontier": {},
        }),
    )]);
    let r = find_response(&resp, 1);
    assert_eq!(r["result"]["isError"], true);
    let text = r["result"]["content"][0]["text"].as_str().unwrap();
    assert!(text.contains("does not match admitted"), "text was: {text}");
}

fn bind_plan_receipt_args(allocation_receipt: &str, session_receipt: &str) -> Value {
    json!({
        "session_think": {
            "plan": {"steps": ["a", "b"]},
            "receipt": session_receipt,
        },
        "allocation_receipt": allocation_receipt,
        "observation_frontier": {"frontier": "empty"},
        "validator_result": {"valid": true},
    })
}

#[test]
fn bind_plan_receipt_happy_path() {
    let allocation_receipt = placeholder_digest();
    let session_receipt = placeholder_digest();
    let resp = drive(&[tool_call(
        1,
        "bind_plan_receipt",
        bind_plan_receipt_args(&allocation_receipt, &session_receipt),
    )]);
    let out = structured(&find_response(&resp, 1));
    assert_eq!(out["schema"], "urn:chatman:admission-envelope:v1");
    assert_eq!(out["kind"], "plan");
    assert_eq!(out["payload"]["session_receipt"], session_receipt);
    assert_eq!(out["payload"]["allocation_receipt"], allocation_receipt);
    assert_eq!(out["payload"]["plan"], json!({"steps": ["a", "b"]}));
    let receipt = out["receipt"].as_str().expect("receipt");
    assert_eq!(receipt.len(), 64);
}

#[test]
fn bind_plan_receipt_rejects_unadmitted_validator_result() {
    let allocation_receipt = placeholder_digest();
    let session_receipt = placeholder_digest();
    let mut args = bind_plan_receipt_args(&allocation_receipt, &session_receipt);
    args["validator_result"] = json!({"valid": false});
    let resp = drive(&[tool_call(1, "bind_plan_receipt", args)]);
    let r = find_response(&resp, 1);
    assert_eq!(r["result"]["isError"], true);
    let text = r["result"]["content"][0]["text"].as_str().unwrap();
    assert!(
        text.contains("did not admit the candidate plan"),
        "text was: {text}"
    );
}

/// `verify_receipt` round-trips against a real envelope produced by
/// `bind_allocation_receipt` in the same session — proving the recomputation
/// path agrees with the binding path on genuine output, not a canned fixture.
#[test]
fn verify_receipt_happy_path_round_trips_a_real_envelope() {
    let resp = drive(&[
        tool_call(
            1,
            "bind_allocation_receipt",
            json!({
                "candidates": eight_candidates(),
                "allocation_result": allocation_result_with(BCINR_REVISION, 8),
                "observation_frontier": {"frontier": "empty"},
            }),
        ),
        // id 2 is filled in below once we have the envelope from id 1.
    ]);
    let envelope = structured(&find_response(&resp, 1));

    let resp2 = drive(&[tool_call(
        2,
        "verify_receipt",
        json!({"envelope": envelope}),
    )]);
    let out = structured(&find_response(&resp2, 2));
    assert_eq!(out["schema"], "urn:chatman:receipt-verification:v1");
    assert_eq!(out["valid"], true);
    assert_eq!(out["payload_digest_valid"], true);
    assert_eq!(out["receipt_valid"], true);
    assert_eq!(
        out["declared_payload_digest"],
        out["expected_payload_digest"]
    );
    assert_eq!(out["declared_receipt"], out["expected_receipt"]);
    assert_eq!(out["kind"], "allocation");
}

/// Mutates one hex character of a genuine envelope's `receipt` and proves
/// `verify_receipt` reports it invalid — the specific tamper-detection
/// property called out in review: the server recomputes both digests
/// independently rather than trusting the envelope's self-reported values.
///
/// This proves receipt-tamper detection end to end (independent
/// recomputation catches a corrupted `receipt` field on a real, freshly
/// bound envelope). It does NOT separately exercise payload-digest tamper
/// detection or payload-content tamper detection (e.g. mutating
/// `payload.candidates` without touching either digest field) — those are
/// PARTIAL coverage left for a follow-up test.
#[test]
fn verify_rejects_tampered_digest() {
    let resp = drive(&[tool_call(
        1,
        "bind_allocation_receipt",
        json!({
            "candidates": eight_candidates(),
            "allocation_result": allocation_result_with(BCINR_REVISION, 8),
            "observation_frontier": {"frontier": "empty"},
        }),
    )]);
    let mut envelope = structured(&find_response(&resp, 1));

    // Confirm the untampered envelope verifies clean first, so the failure
    // below is attributable to the mutation, not to some other defect.
    let baseline = drive(&[tool_call(
        1,
        "verify_receipt",
        json!({"envelope": envelope.clone()}),
    )]);
    assert_eq!(structured(&find_response(&baseline, 1))["valid"], true);

    // Flip one hex character of the receipt (a valid 64-hex-char string is
    // still required to pass the `deny_unknown_fields`/regex-shaped input
    // validation, so we mutate in place rather than truncate or corrupt the
    // format).
    let receipt = envelope["receipt"].as_str().unwrap().to_owned();
    let mut chars: Vec<char> = receipt.chars().collect();
    let flip_at = 0;
    chars[flip_at] = if chars[flip_at] == 'a' { 'b' } else { 'a' };
    let tampered_receipt: String = chars.into_iter().collect();
    assert_ne!(tampered_receipt, receipt);
    envelope["receipt"] = json!(tampered_receipt);

    let resp2 = drive(&[tool_call(
        2,
        "verify_receipt",
        json!({"envelope": envelope}),
    )]);
    let out = structured(&find_response(&resp2, 2));
    assert_eq!(out["valid"], false);
    assert_eq!(
        out["payload_digest_valid"], true,
        "payload was untouched, so its digest should still verify"
    );
    assert_eq!(
        out["receipt_valid"], false,
        "the tampered receipt must not match the independently recomputed one"
    );
    assert_eq!(out["declared_receipt"], tampered_receipt);
    assert_ne!(out["expected_receipt"], json!(tampered_receipt));
}

#[test]
fn resources_list_and_read_expose_tool_semantics() {
    let resp = drive(&[
        json!({"jsonrpc":"2.0","id":1,"method":"resources/list"}),
        json!({
            "jsonrpc":"2.0","id":2,"method":"resources/read",
            "params": {"uri": "ferroplan://tools/canonical_digest"}
        }),
    ]);
    let list = find_response(&resp, 1);
    let uris: std::collections::BTreeSet<&str> = list["result"]["resources"]
        .as_array()
        .unwrap()
        .iter()
        .map(|r| r["uri"].as_str().unwrap())
        .collect();
    // The merged server exposes 16 resources total; this admission group's
    // four must be present (full 16-resource exactness is
    // `merged_server.rs`'s job).
    for expected in [
        "ferroplan://tools/bind_allocation_receipt",
        "ferroplan://tools/bind_plan_receipt",
        "ferroplan://tools/canonical_digest",
        "ferroplan://tools/verify_receipt",
    ] {
        assert!(
            uris.contains(expected),
            "missing resource `{expected}`: {uris:?}"
        );
    }

    let read = find_response(&resp, 2);
    let text = read["result"]["contents"][0]["text"]
        .as_str()
        .expect("resource content is text");
    let body: Value = serde_json::from_str(text).expect("resource body is JSON");
    assert_eq!(body["tool"], "canonical_digest");
    assert_eq!(
        body["source"],
        "plugins/chatman-ecosystem/ontology/ferroplan-domain.ttl"
    );
    let comment = body["rdfs_comment"]
        .as_str()
        .expect("rdfs_comment is real ontology prose");
    assert!(
        !comment.trim().is_empty(),
        "expected non-empty ontology-sourced prose, got: {comment:?}"
    );
}

#[test]
fn resources_read_unknown_uri_is_not_found() {
    let resp = drive(&[json!({
        "jsonrpc":"2.0","id":1,"method":"resources/read",
        "params": {"uri": "ferroplan://tools/no_such_tool"}
    })]);
    let r = find_response(&resp, 1);
    assert!(
        r.get("error").is_some(),
        "expected a JSON-RPC error for an unknown resource URI, got: {r:?}"
    );
}

/// `DigestInput` is `#[serde(deny_unknown_fields)]`; sending an extra field
/// must surface as a tool-error response, not a server crash or a silently
/// ignored field.
#[test]
fn malformed_input_unknown_field_is_rejected_not_a_crash() {
    let resp = drive(&[tool_call(
        1,
        "canonical_digest",
        json!({"value": {"a": 1}, "unexpected_extra_field": true}),
    )]);
    let r = find_response(&resp, 1);
    assert_eq!(
        r["result"]["isError"], true,
        "unknown field must be a tool error, got: {r:?}"
    );
    let text = r["result"]["content"][0]["text"].as_str().unwrap();
    assert!(
        text.contains("unexpected_extra_field") || text.to_lowercase().contains("unknown field"),
        "expected the error to name the rejected field, text was: {text}"
    );
}

/// Same property on `BindAllocationInput`, which has more required fields —
/// confirms `deny_unknown_fields` rejection isn't special-cased to the
/// simplest struct.
#[test]
fn malformed_input_unknown_field_on_bind_allocation_is_rejected() {
    let resp = drive(&[tool_call(
        1,
        "bind_allocation_receipt",
        json!({
            "candidates": eight_candidates(),
            "allocation_result": allocation_result_with(BCINR_REVISION, 8),
            "observation_frontier": {},
            "not_a_real_field": 1,
        }),
    )]);
    let r = find_response(&resp, 1);
    assert_eq!(
        r["result"]["isError"], true,
        "unknown field must be a tool error, got: {r:?}"
    );
}

/// Checkpoint 9 ("Recursive Multifractal Allocation"): a local eight-node
/// frontier can bind as the descent of one selected node inside a prior,
/// independently re-verified parent allocation envelope — not just chain
/// sequentially via `previous_receipt`. Depth one and depth two are both
/// exercised against the real tool, on the real compiled binary, in one
/// server process.
#[test]
fn bind_allocation_receipt_recursive_descent_happy_path() {
    let resp = drive(&[
        tool_call(
            1,
            "bind_allocation_receipt",
            json!({
                "candidates": eight_candidates_with_ids("root"),
                "allocation_result": allocation_result_with(BCINR_REVISION, 8),
                "observation_frontier": {"frontier": "empty"},
            }),
        ),
        tool_call(
            2,
            "bind_allocation_receipt",
            json!({
                "candidates": eight_candidates_with_ids("root"),
                "allocation_result": allocation_result_with(BCINR_REVISION, 8),
                "observation_frontier": {"frontier": "empty"},
            }),
        ),
    ]);
    let parent = structured(&find_response(&resp, 1));
    assert_eq!(parent, structured(&find_response(&resp, 2)));

    let resp = drive(&[tool_call(
        1,
        "bind_allocation_receipt",
        json!({
            "candidates": eight_candidates_with_ids("root-3-child"),
            "allocation_result": allocation_result_with(BCINR_REVISION, 8),
            "observation_frontier": {"frontier": "root-3 descent"},
            "parent_allocation": parent,
            "selected_node": "root-3",
        }),
    )]);
    let child = structured(&find_response(&resp, 1));
    assert_eq!(child["schema"], "urn:chatman:admission-envelope:v1");
    assert_eq!(
        child["payload"]["parent_allocation_receipt"], parent["receipt"],
        "child binds the parent's real, independently re-verified receipt"
    );
    assert_eq!(child["payload"]["selected_node"], "root-3");
    assert_eq!(
        child["payload"]["selected_node_candidate"],
        json!({"id": "root-3"}),
        "the exact parent candidate the descent expands is embedded, not just its id"
    );
}

/// A `parent_allocation` whose `receipt` field was hand-edited after the
/// fact must be refused, not trusted — this is the "parent receipt
/// mismatch refusal" required-proof line for Checkpoint 9.
#[test]
fn bind_allocation_receipt_recursive_descent_rejects_tampered_parent_receipt() {
    let resp = drive(&[tool_call(
        1,
        "bind_allocation_receipt",
        json!({
            "candidates": eight_candidates_with_ids("root"),
            "allocation_result": allocation_result_with(BCINR_REVISION, 8),
            "observation_frontier": {},
        }),
    )]);
    let mut parent = structured(&find_response(&resp, 1));
    let real_receipt = parent["receipt"].as_str().unwrap().to_owned();
    let mut tampered_receipt = real_receipt.clone();
    let first_char = tampered_receipt.remove(0);
    tampered_receipt.insert(0, if first_char == '0' { '1' } else { '0' });
    parent["receipt"] = json!(tampered_receipt);

    let resp = drive(&[tool_call(
        1,
        "bind_allocation_receipt",
        json!({
            "candidates": eight_candidates_with_ids("child"),
            "allocation_result": allocation_result_with(BCINR_REVISION, 8),
            "observation_frontier": {},
            "parent_allocation": parent,
            "selected_node": "root-0",
        }),
    )]);
    let r = find_response(&resp, 1);
    assert_eq!(r["result"]["isError"], true);
    let text = r["result"]["content"][0]["text"].as_str().unwrap();
    assert!(
        text.contains("refusing an unverifiable parent"),
        "text was: {text}"
    );
}

/// `selected_node` must actually name one of the parent's eight candidates —
/// a fabricated node id is refused, not silently accepted.
#[test]
fn bind_allocation_receipt_recursive_descent_rejects_unknown_selected_node() {
    let resp = drive(&[tool_call(
        1,
        "bind_allocation_receipt",
        json!({
            "candidates": eight_candidates_with_ids("root"),
            "allocation_result": allocation_result_with(BCINR_REVISION, 8),
            "observation_frontier": {},
        }),
    )]);
    let parent = structured(&find_response(&resp, 1));

    let resp = drive(&[tool_call(
        1,
        "bind_allocation_receipt",
        json!({
            "candidates": eight_candidates_with_ids("child"),
            "allocation_result": allocation_result_with(BCINR_REVISION, 8),
            "observation_frontier": {},
            "parent_allocation": parent,
            "selected_node": "root-99-does-not-exist",
        }),
    )]);
    let r = find_response(&resp, 1);
    assert_eq!(r["result"]["isError"], true);
    let text = r["result"]["content"][0]["text"].as_str().unwrap();
    assert!(
        text.contains("is not a candidate in parent_allocation"),
        "text was: {text}"
    );
}

/// `parent_allocation` and `selected_node` are a paired descent claim —
/// providing only one is refused rather than silently binding a rootless or
/// nodeless descent.
#[test]
fn bind_allocation_receipt_recursive_descent_requires_both_fields_together() {
    let resp = drive(&[tool_call(
        1,
        "bind_allocation_receipt",
        json!({
            "candidates": eight_candidates_with_ids("root"),
            "allocation_result": allocation_result_with(BCINR_REVISION, 8),
            "observation_frontier": {},
            "selected_node": "root-0",
        }),
    )]);
    let r = find_response(&resp, 1);
    assert_eq!(r["result"]["isError"], true);
    let text = r["result"]["content"][0]["text"].as_str().unwrap();
    assert!(
        text.contains("must be provided together"),
        "text was: {text}"
    );
}
