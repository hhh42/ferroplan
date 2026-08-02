//! Persistent session authority, driven through the real merged MCP stdio
//! server.
//!
//! The authoritative surface grounds one named repository mind, admits
//! observations and goal changes, follows or replans under a declared budget,
//! advances a plan cursor, reports standing, and closes the session. Forking,
//! list-all sessions, ambient state mutation, and implicit handle minting are
//! intentionally not part of this authority.

mod common;

use common::{Client, DOM, PROB};
use serde_json::{json, Value};

fn open(c: &mut Client, session_id: &str) -> Value {
    c.call_json(
        "session_open",
        json!({
            "session_id": session_id,
            "domain": DOM,
            "problem": PROB
        }),
    )
}

fn reported_plan_length(think: &Value) -> usize {
    think["plan"]["steps"]
        .as_array()
        .map(Vec::len)
        .or_else(|| think["solution"]["plan"]["steps"].as_array().map(Vec::len))
        .unwrap_or(0)
}

#[test]
fn open_observe_then_think_replans_from_the_admitted_state() {
    let mut c = Client::start();
    let sid = "stateful-replan";
    let opened = open(&mut c, sid);
    assert_eq!(opened["schema"], "urn:chatman:ferroplan-session-open:v1");
    assert_eq!(opened["session_id"], sid);
    assert_eq!(opened["goal_met"], false);

    let first = c.call_json(
        "session_think",
        json!({"session_id": sid, "max_evaluated": 10_000}),
    );
    assert_eq!(first["decision"], "replan");
    assert_eq!(reported_plan_length(&first), 2, "initial plan is A then B");

    // The external world reports that A already happened. Observation is the
    // only state-mutation path; completed plan steps are not ambiently applied.
    let observed = c.call_json(
        "session_observe",
        json!({
            "session_id": sid,
            "facts": [
                {"fact": "(P)", "value": false},
                {"fact": "(Q)", "value": true}
            ],
            "fluents": []
        }),
    );
    assert_eq!(observed["schema"], "urn:chatman:ferroplan-observation:v1");
    assert_eq!(observed["fact_surprises"].as_array().unwrap().len(), 2);
    assert_eq!(observed["replan_required"], true);

    let second = c.call_json(
        "session_think",
        json!({"session_id": sid, "max_evaluated": 10_000}),
    );
    assert_eq!(second["decision"], "replan");
    assert_eq!(reported_plan_length(&second), 1, "only B remains");
    assert_eq!(second["solution"]["plan"]["steps"][0]["action"], "B");
    c.finish();
}

#[test]
fn session_id_is_caller_owned_and_replace_is_explicit() {
    let mut c = Client::start();
    let sid = "caller-owned-id";
    let first = open(&mut c, sid);
    let first_receipt = first["receipt"]["digest"].as_str().unwrap_or_default();
    assert!(!first_receipt.is_empty());

    let (text, error) = c.call_text(
        "session_open",
        json!({"session_id": sid, "domain": DOM, "problem": PROB}),
    );
    assert!(error, "duplicate open must be refused");
    assert!(text.contains("already exists"), "unexpected refusal: {text}");

    let replacement = c.call_json(
        "session_open",
        json!({
            "session_id": sid,
            "domain": DOM,
            "problem": PROB,
            "replace": true
        }),
    );
    assert_eq!(replacement["session_id"], sid);
    assert_ne!(
        replacement["receipt"]["digest"].as_str().unwrap_or_default(),
        "",
        "replacement must emit a receipt"
    );
    c.finish();
}

#[test]
fn observe_reports_only_contradictions_and_chains_receipts() {
    let mut c = Client::start();
    let sid = "surprise-boundary";
    let opened = open(&mut c, sid);
    let open_digest = opened["receipt"]["digest"]
        .as_str()
        .expect("open receipt")
        .to_owned();

    let quiet = c.call_json(
        "session_observe",
        json!({
            "session_id": sid,
            "facts": [{"fact": "(P)", "value": true}],
            "fluents": []
        }),
    );
    assert!(quiet["fact_surprises"].as_array().unwrap().is_empty());
    assert_eq!(quiet["epoch"], 0);
    assert_eq!(quiet["receipt"]["previous_digest"], open_digest);

    let news = c.call_json(
        "session_observe",
        json!({
            "session_id": sid,
            "facts": [{"fact": "(Q)", "value": true}],
            "fluents": []
        }),
    );
    assert_eq!(news["fact_surprises"].as_array().unwrap().len(), 1);
    assert_eq!(news["epoch"], 1);
    assert_eq!(
        news["receipt"]["previous_digest"],
        quiet["receipt"]["digest"]
    );
    c.finish();
}

#[test]
fn think_budget_is_a_hard_bounded_contract() {
    let mut c = Client::start();
    let sid = "bounded-think";
    open(&mut c, sid);

    let (zero_text, zero_error) = c.call_text(
        "session_think",
        json!({"session_id": sid, "max_evaluated": 0}),
    );
    assert!(zero_error);
    assert!(zero_text.contains("greater than zero"));

    let (ceiling_text, ceiling_error) = c.call_text(
        "session_think",
        json!({"session_id": sid, "max_evaluated": 10_000_001}),
    );
    assert!(ceiling_error);
    assert!(ceiling_text.contains("at most 10000000"));

    let admitted = c.call_json(
        "session_think",
        json!({"session_id": sid, "max_evaluated": 10_000}),
    );
    assert!(matches!(
        admitted["decision"].as_str(),
        Some("follow" | "replan" | "bounded-refusal")
    ));
    c.finish();
}

#[test]
fn status_close_and_unknown_handle_refusals_preserve_the_server() {
    let mut c = Client::start();
    let sid = "close-boundary";
    open(&mut c, sid);

    let status = c.call_json("session_status", json!({"session_id": sid}));
    assert_eq!(status["schema"], "urn:chatman:ferroplan-session-status:v1");
    assert_eq!(status["session_id"], sid);
    assert_eq!(status["cursor"], 0);

    let (unknown_text, unknown_error) =
        c.call_text("session_status", json!({"session_id": "never-opened"}));
    assert!(unknown_error);
    assert!(unknown_text.contains("unknown session `never-opened`"));

    let closed = c.call_json("session_close", json!({"session_id": sid}));
    assert_eq!(closed["closed"], true);
    let closed_again = c.call_json("session_close", json!({"session_id": sid}));
    assert_eq!(closed_again["closed"], false, "close is observable and idempotent");

    let (after_text, after_error) = c.call_text("session_status", json!({"session_id": sid}));
    assert!(after_error);
    assert!(after_text.contains("unknown session"));

    // A refused session lookup does not terminate the merged server.
    let report = c.call_json("parse", json!({"pddl": DOM}));
    assert_eq!(report["ok"], true);
    c.finish();
}

#[test]
fn cursor_advance_refuses_unobserved_execution_beyond_the_plan() {
    let mut c = Client::start();
    let sid = "cursor-boundary";
    open(&mut c, sid);
    let think = c.call_json(
        "session_think",
        json!({"session_id": sid, "max_evaluated": 10_000}),
    );
    let length = reported_plan_length(&think);
    assert_eq!(length, 2);

    let (text, error) = c.call_text(
        "session_advance",
        json!({"session_id": sid, "completed_steps": length + 1}),
    );
    assert!(error);
    assert!(text.contains("beyond plan length"));

    let status = c.call_json("session_status", json!({"session_id": sid}));
    assert_eq!(status["cursor"], 0, "refused advance must not mutate cursor");
    c.finish();
}
