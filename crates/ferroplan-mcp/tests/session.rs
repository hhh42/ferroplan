//! The session surface, driven over the real stdio protocol.
//!
//! What these pin is the thing the server exists for: a world grounded ONCE,
//! then told what changed and asked to rethink — and `fork`, which gives a
//! second mind its own beliefs and goal over the SAME shared grounded world.

mod common;

use common::{Client, DOM, PROB};
use serde_json::json;

/// The loop an agent actually runs: open, look, think, tell it the world
/// moved, think again — and the second plan must be shorter because the
/// session kept the state instead of re-grounding from scratch.
#[test]
fn open_then_tell_then_rethink_replans_from_the_new_state() {
    let mut c = Client::start();
    let opened = c.call_json("session_open", json!({"domain": DOM, "problem": PROB}));
    let sid = opened["session_id"].as_str().expect("a session handle");
    assert_eq!(opened["goal_met"], false);

    let state = c.call_json(
        "session_state",
        json!({"session_id": sid, "facts": ["(p)", "(q)", "(r)"]}),
    );
    assert_eq!(state["facts"]["(p)"], true);
    assert_eq!(state["facts"]["(r)"], false);

    let first = c.call_json("session_replan", json!({"session_id": sid}));
    assert_eq!(first["solved"], true);
    assert_eq!(
        first["plan"]["length"], 2,
        "from the initial state: A then B"
    );

    // The world moved without us: `a` fired out there.
    let applied = c.call_json(
        "session_set",
        json!({"session_id": sid, "facts": [["(p)", false], ["(q)", true]]}),
    );
    assert_eq!(applied["facts"], 2);

    let second = c.call_json("session_replan", json!({"session_id": sid}));
    assert_eq!(second["solved"], true);
    assert_eq!(
        second["plan"]["length"], 1,
        "the session kept its state — only B is left to do"
    );
    assert_eq!(second["plan"]["steps"][0]["action"], "B");
    c.finish();
}

/// The many-minds primitive. A fork shares the grounded world (same
/// `world_bytes`) but owns its beliefs and goal, so the two can disagree
/// about whether they are done.
#[test]
fn fork_shares_the_world_and_owns_its_goal() {
    let mut c = Client::start();
    let a = c.call_json("session_open", json!({"domain": DOM, "problem": PROB}));
    let a_id = a["session_id"].as_str().unwrap().to_string();

    let b = c.call_json("session_fork", json!({"session_id": a_id}));
    let b_id = b["session_id"].as_str().unwrap().to_string();
    assert_ne!(a_id, b_id, "a fork is a distinct handle");
    assert_eq!(b["forked_from"], a_id.as_str());

    // Move the shared-looking world only in the FORK, and give it its own goal.
    c.call_json(
        "session_set",
        json!({"session_id": b_id, "facts": [["(p)", false], ["(q)", true]], "goal": "(q)"}),
    );

    let a_state = c.call_json(
        "session_state",
        json!({"session_id": a_id, "facts": ["(q)"]}),
    );
    let b_state = c.call_json(
        "session_state",
        json!({"session_id": b_id, "facts": ["(q)"]}),
    );

    // Beliefs are private...
    assert_eq!(a_state["facts"]["(q)"], false, "the parent did not move");
    assert_eq!(b_state["facts"]["(q)"], true);
    // ...goals are private...
    assert_eq!(a_state["goal_met"], false);
    assert_eq!(
        b_state["goal_met"], true,
        "the fork's own goal is satisfied"
    );
    // ...but the grounded world is ONE copy, which is the whole point.
    assert_eq!(
        a_state["world_bytes"], b_state["world_bytes"],
        "forks share the grounded payload"
    );
    c.finish();
}

#[test]
fn observe_reports_only_the_surprises() {
    let mut c = Client::start();
    let s = c.call_json("session_open", json!({"domain": DOM, "problem": PROB}));
    let sid = s["session_id"].as_str().unwrap().to_string();

    // It already believes (p); seeing (p) true is no news.
    let quiet = c.call_json(
        "session_observe",
        json!({"session_id": sid, "sight": [["(p)", true]]}),
    );
    assert_eq!(
        quiet["surprises"].as_array().unwrap().len(),
        0,
        "a sighting that matches belief is not a surprise"
    );

    // Seeing (q) true contradicts it.
    let news = c.call_json(
        "session_observe",
        json!({"session_id": sid, "sight": [["(q)", true]]}),
    );
    assert_eq!(
        news["surprises"].as_array().unwrap().len(),
        1,
        "a contradicted belief must be reported: {news}"
    );
    c.finish();
}

#[test]
fn budgeted_replan_is_a_contract() {
    let mut c = Client::start();
    let s = c.call_json("session_open", json!({"domain": DOM, "problem": PROB}));
    let sid = s["session_id"].as_str().unwrap().to_string();
    // A generous budget still solves this two-step task.
    let sol = c.call_json(
        "session_replan",
        json!({"session_id": sid, "max_evaluated": 10000}),
    );
    assert_eq!(sol["solved"], true);
    c.finish();
}

#[test]
fn sessions_are_listed_closed_and_missing_handles_are_tool_errors() {
    let mut c = Client::start();
    let s = c.call_json("session_open", json!({"domain": DOM, "problem": PROB}));
    let sid = s["session_id"].as_str().unwrap().to_string();

    let listed = c.call_json("session_list", json!({}));
    assert_eq!(listed["sessions"].as_array().unwrap().len(), 1);

    // An unknown handle is a tool error the agent can recover from, and it
    // must NOT take the connection down.
    let (text, err) = c.call_text("session_replan", json!({"session_id": "nope"}));
    assert!(err, "unknown handle must be an isError result");
    assert!(
        text.contains("nope"),
        "message should name the handle: {text}"
    );

    let (_, err) = c.call_text("session_close", json!({"session_id": sid}));
    assert!(!err, "closing a live session succeeds");
    let after = c.call_json("session_list", json!({}));
    assert_eq!(after["sessions"].as_array().unwrap().len(), 0);

    // Still serving.
    let report = c.call_json("parse", json!({"pddl": DOM}));
    assert_eq!(report["ok"], true);
    c.finish();
}

/// A rejected edit must say what it managed to apply first — a half-applied
/// world the agent cannot see is worse than one it can.
#[test]
fn a_rejected_edit_reports_what_it_applied_first() {
    let mut c = Client::start();
    let s = c.call_json("session_open", json!({"domain": DOM, "problem": PROB}));
    let sid = s["session_id"].as_str().unwrap().to_string();
    let (text, err) = c.call_text(
        "session_set",
        json!({"session_id": sid, "facts": [["(q)", true], ["(nonexistent-fact)", true]]}),
    );
    assert!(err, "an unknown fact must be refused: {text}");
    assert!(
        text.contains("applied before the failure"),
        "the partial application must be reported: {text}"
    );
    c.finish();
}
