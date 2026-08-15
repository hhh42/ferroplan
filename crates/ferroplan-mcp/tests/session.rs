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

// ---- 0.24 Phase 5: the budget-stamped think contract on the wire -----------

/// The farm from the library's session tests: three steps of work, so a
/// 1-eval think honestly fails and a fed one solves.
const FARM_DOM: &str = "
(define (domain farm) (:requirements :strips :typing :numeric-fluents)
  (:types agent place)
  (:predicates (at ?a - agent ?p - place) (road ?x ?y - place) (fertile ?p - place))
  (:functions (grain))
  (:action walk :parameters (?a - agent ?from ?to - place)
    :precondition (and (at ?a ?from) (road ?from ?to))
    :effect (and (not (at ?a ?from)) (at ?a ?to)))
  (:action harvest :parameters (?a - agent ?p - place)
    :precondition (and (at ?a ?p) (fertile ?p))
    :effect (increase (grain) 1)))";
const FARM_PRB: &str = "
(define (problem p) (:domain farm)
  (:objects v1 - agent hut field - place)
  (:init (at v1 hut) (road hut field) (road field hut) (fertile field) (= (grain) 0))
  (:goal (>= (grain) 2)))";

/// Two interchangeable balls — the orbit-aware replan's wire witness (the
/// unary-goal SOLO1 shape the library fixture pins).
const ORB_DOM: &str = "
(define (domain rollers) (:requirements :strips :typing)
  (:types ball room)
  (:predicates (at ?b - ball ?r - room) (link ?x ?y - room)
               (goal-room ?r - room) (home ?b - ball))
  (:action roll :parameters (?b - ball ?from ?to - room)
    :precondition (and (at ?b ?from) (link ?from ?to))
    :effect (and (not (at ?b ?from)) (at ?b ?to)))
  (:action park :parameters (?b - ball ?r - room)
    :precondition (and (at ?b ?r) (goal-room ?r))
    :effect (home ?b)))";
const ORB_PRB: &str = "
(define (problem p) (:domain rollers)
  (:objects b1 b2 - ball ra rb - room)
  (:init (at b1 ra) (at b2 ra) (link ra rb) (link rb ra) (goal-room rb))
  (:goal (and (home b1) (home b2))))";

/// 0.24: every replan is budget-stamped — capped, spent_ms, spent_evals,
/// verdict ride alongside the unchanged Solution fields, and the memory
/// split stays honest across a stamped think.
#[test]
fn a_replan_is_budget_stamped_on_the_wire() {
    let mut c = Client::start();
    let s = c.call_json("session_open", json!({"domain": DOM, "problem": PROB}));
    let sid = s["session_id"].as_str().unwrap().to_string();
    let before = c.call_json("session_state", json!({"session_id": sid}));

    let sol = c.call_json(
        "session_replan",
        json!({"session_id": sid, "max_evaluated": 10000, "wall_ms": 60000, "memory_mb": 64}),
    );
    assert_eq!(sol["solved"], true);
    assert_eq!(sol["plan"]["length"], 2, "the Solution shape is unchanged");
    assert_eq!(sol["capped"], false);
    assert_eq!(sol["verdict"], "solved");
    assert!(sol["spent_evals"].as_u64().unwrap() >= 1, "{sol}");
    assert!(sol["spent_ms"].is_u64(), "{sol}");

    let after = c.call_json("session_state", json!({"session_id": sid}));
    assert_eq!(before["world_bytes"], after["world_bytes"]);
    assert_eq!(before["mind_bytes"], after["mind_bytes"]);
    assert!(after["world_bytes"].as_u64().unwrap() > 0);
    c.finish();
}

/// The capped-search honesty, verbatim on the wire: a budget-starved think
/// says `capped`, never anything an agent could read as "unsolvable".
#[test]
fn a_capped_think_never_reads_unsolvable_on_the_wire() {
    let mut c = Client::start();
    let s = c.call_json(
        "session_open",
        json!({"domain": FARM_DOM, "problem": FARM_PRB}),
    );
    let sid = s["session_id"].as_str().unwrap().to_string();

    let (text, err) = c.call_text(
        "session_replan",
        json!({"session_id": sid, "max_evaluated": 1, "memory_mb": 1}),
    );
    assert!(!err, "a capped think is an answer, not an error: {text}");
    let sol: serde_json::Value = serde_json::from_str(&text).expect("stamped JSON");
    assert_eq!(sol["solved"], false);
    assert_eq!(sol["capped"], true);
    assert_eq!(sol["verdict"], "capped");
    assert!(
        !text.to_lowercase().contains("unsolvable"),
        "the cap honesty must reach the wire verbatim: {text}"
    );

    // Round trip: the same session, properly fed, solves.
    let sol = c.call_json(
        "session_replan",
        json!({"session_id": sid, "max_evaluated": 100000, "wall_ms": 60000}),
    );
    assert_eq!(sol["solved"], true);
    assert_eq!(sol["verdict"], "solved");
    c.finish();
}

/// Orbit-aware replans reach the wire: a symmetric world's think narrates
/// the re-detected orbit in its notes.
#[test]
fn an_orbit_aware_replan_narrates_itself() {
    let mut c = Client::start();
    let s = c.call_json(
        "session_open",
        json!({"domain": ORB_DOM, "problem": ORB_PRB}),
    );
    let sid = s["session_id"].as_str().unwrap().to_string();
    let sol = c.call_json(
        "session_replan",
        json!({"session_id": sid, "max_evaluated": 10000}),
    );
    assert_eq!(sol["solved"], true);
    let notes = sol["notes"].as_array().unwrap();
    assert!(
        notes.iter().any(|n| n.as_str().unwrap().contains("orbit")),
        "{notes:?}"
    );
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
