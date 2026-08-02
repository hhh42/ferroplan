//! Drive the built `ferroplan-mcp` binary over stdio and check the MCP
//! protocol end to end: handshake, the complete merged tool catalogue,
//! stateless planning, persistent sessions, admission authority, and error
//! conventions.
//!
//! These pin the behaviour that must survive the merged-router composition:
//! tool failures stay `isError` results an agent can read, structured
//! validation remains machine-readable, and an unknown method remains
//! JSON-RPC `-32601`.

mod common;

use common::{Client, DOM, PROB};
use serde_json::json;

#[test]
fn initialize_advertises_the_exact_merged_authority_surface() {
    let mut c = Client::start();
    let list = c.request("tools/list", json!({}));
    let mut names: Vec<&str> = list["result"]["tools"]
        .as_array()
        .expect("tools array")
        .iter()
        .map(|tool| tool["name"].as_str().expect("tool name"))
        .collect();
    names.sort_unstable();

    let mut expected = vec![
        "bind_allocation_receipt",
        "bind_plan_receipt",
        "canonical_digest",
        "cmca_allocate",
        "cmca_allocate_recursive",
        "decompose",
        "parse",
        "session_advance",
        "session_close",
        "session_observe",
        "session_open",
        "session_set_goal",
        "session_status",
        "session_think",
        "solve",
        "validate",
        "verify_receipt",
    ];
    expected.sort_unstable();

    assert_eq!(names, expected, "merged MCP authority surface drifted");
    c.finish();
}

/// The `schema` cargo feature earns its keep here: `solve`'s `options` must be
/// a TYPED object with the real knobs, not an opaque blob an agent has to
/// guess at. This is the end-to-end proof of the uptake that added it.
#[test]
fn solve_advertises_a_typed_options_schema() {
    let mut c = Client::start();
    let list = c.request("tools/list", json!({}));
    let solve = list["result"]["tools"]
        .as_array()
        .unwrap()
        .iter()
        .find(|tool| tool["name"] == "solve")
        .expect("solve is advertised")
        .clone();
    let schema = solve["inputSchema"].to_string();
    for knob in ["domain", "problem", "options", "mode", "search", "threads"] {
        assert!(
            schema.contains(knob),
            "solve inputSchema should mention `{knob}`: {schema}"
        );
    }
    c.finish();
}

#[test]
fn parse_tool_summarizes_a_domain() {
    let mut c = Client::start();
    let report = c.call_json("parse", json!({"pddl": DOM}));
    assert_eq!(report["ok"], true);
    assert_eq!(report["kind"], "domain");
    assert_eq!(report["name"], "d");
    c.finish();
}

#[test]
fn solve_tool_returns_a_plan() {
    let mut c = Client::start();
    let solution = c.call_json("solve", json!({"domain": DOM, "problem": PROB}));
    assert_eq!(solution["solved"], true);
    assert_eq!(solution["plan"]["steps"][0]["action"], "A");
    assert_eq!(solution["plan"]["steps"][1]["action"], "B");
    c.finish();
}

#[test]
fn validate_tool_returns_the_structured_validation_contract() {
    let mut c = Client::start();
    let valid = c.call_json(
        "validate",
        json!({"domain": DOM, "problem": PROB, "plan": "step 0: (a)\nstep 1: (b)"}),
    );
    assert_eq!(valid["schema"], "urn:ferroplan:plan-validation:v1");
    assert_eq!(valid["valid"], true);
    assert!(valid["reason"].is_null());

    let invalid = c.call_json(
        "validate",
        json!({"domain": DOM, "problem": PROB, "plan": "step 0: (b)"}),
    );
    assert_eq!(invalid["schema"], "urn:ferroplan:plan-validation:v1");
    assert_eq!(invalid["valid"], false);
    assert!(
        invalid["reason"]
            .as_str()
            .is_some_and(|reason| !reason.is_empty()),
        "invalid plans must carry a reason: {invalid}"
    );
    c.finish();
}

#[test]
fn unsolvable_problem_is_a_normal_answer_not_an_error() {
    let mut c = Client::start();
    let problem = "(define (problem pr) (:domain d) (:init ) (:goal (r)))";
    let solution = c.call_json("solve", json!({"domain": DOM, "problem": problem}));
    assert_eq!(
        solution["solved"], false,
        "solved:false is an answer, not a failure"
    );
    c.finish();
}

#[test]
fn bad_args_are_tool_errors_and_unknown_method_is_an_rpc_error() {
    let mut c = Client::start();
    // A missing required argument is a TOOL error the agent can read and fix,
    // not a protocol error that kills the connection.
    let (text, error) = c.call_text("solve", json!({"problem": PROB}));
    assert!(error, "missing `domain` must be an isError result");
    assert!(
        text.contains("domain"),
        "message should name the field: {text}"
    );

    // Bad PDDL: also a tool error, and the server stays usable afterwards.
    let (_, error) = c.call_text(
        "solve",
        json!({"domain": "(this is not pddl", "problem": PROB}),
    );
    assert!(error, "unparseable PDDL must be an isError result");

    // ...still alive.
    let report = c.call_json("parse", json!({"pddl": DOM}));
    assert_eq!(report["ok"], true);

    let response = c.request("no/such/method", json!({}));
    assert_eq!(response["error"]["code"], -32601);
    c.finish();
}
