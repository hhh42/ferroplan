//! Drive the built `ferroplan-mcp` binary over stdio and check the MCP
//! protocol end to end: handshake, the tool catalogue, the four stateless
//! planning tools, and the error conventions.
//!
//! These pin the behaviour that had to SURVIVE the move onto rmcp — the
//! stateless four answer exactly as they did before, tool failures stay
//! `isError` results the agent can read, and an unknown method is still
//! `-32601`.

mod common;

use common::{Client, DOM, PROB};
use serde_json::json;

#[test]
fn initialize_advertises_server_and_both_tool_families() {
    let mut c = Client::start();
    let list = c.request("tools/list", json!({}));
    let names: Vec<&str> = list["result"]["tools"]
        .as_array()
        .expect("tools array")
        .iter()
        .map(|t| t["name"].as_str().unwrap())
        .collect();

    // The stateless four, unchanged since before rmcp.
    for want in ["solve", "parse", "validate", "decompose"] {
        assert!(
            names.contains(&want),
            "missing stateless tool {want}: {names:?}"
        );
    }
    // The session surface this server exists to add.
    for want in [
        "session_open",
        "session_close",
        "session_list",
        "session_fork",
        "session_set",
        "session_observe",
        "session_elapse",
        "session_apply_start",
        "session_replan",
        "session_state",
    ] {
        assert!(
            names.contains(&want),
            "missing session tool {want}: {names:?}"
        );
    }
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
        .find(|t| t["name"] == "solve")
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
    let sol = c.call_json("solve", json!({"domain": DOM, "problem": PROB}));
    assert_eq!(sol["solved"], true);
    assert_eq!(sol["plan"]["steps"][0]["action"], "A");
    assert_eq!(sol["plan"]["steps"][1]["action"], "B");
    c.finish();
}

#[test]
fn validate_tool_checks_a_plan() {
    let mut c = Client::start();
    let (text, err) = c.call_text(
        "validate",
        json!({"domain": DOM, "problem": PROB, "plan": "step 0: (a)\nstep 1: (b)"}),
    );
    assert!(!err, "validate should not be an error result: {text}");
    assert_eq!(text, "Plan valid");

    let (text, err) = c.call_text(
        "validate",
        json!({"domain": DOM, "problem": PROB, "plan": "step 0: (b)"}),
    );
    assert!(!err, "an invalid PLAN is still a successful tool call");
    assert!(text.starts_with("Plan invalid"), "got {text}");
    c.finish();
}

#[test]
fn unsolvable_problem_is_a_normal_answer_not_an_error() {
    let mut c = Client::start();
    let prob = "(define (problem pr) (:domain d) (:init ) (:goal (r)))";
    let sol = c.call_json("solve", json!({"domain": DOM, "problem": prob}));
    assert_eq!(
        sol["solved"], false,
        "solved:false is an answer, not a failure"
    );
    c.finish();
}

#[test]
fn bad_args_are_tool_errors_and_unknown_method_is_an_rpc_error() {
    let mut c = Client::start();
    // A missing required argument is a TOOL error the agent can read and fix,
    // not a protocol error that kills the connection.
    let (text, err) = c.call_text("solve", json!({"problem": PROB}));
    assert!(err, "missing `domain` must be an isError result");
    assert!(
        text.contains("domain"),
        "message should name the field: {text}"
    );

    // Bad PDDL: also a tool error, and the server stays usable afterwards.
    let (_, err) = c.call_text(
        "solve",
        json!({"domain": "(this is not pddl", "problem": PROB}),
    );
    assert!(err, "unparseable PDDL must be an isError result");

    // ...still alive.
    let report = c.call_json("parse", json!({"pddl": DOM}));
    assert_eq!(report["ok"], true);

    let r = c.request("no/such/method", json!({}));
    assert_eq!(r["error"]["code"], -32601);
    c.finish();
}
