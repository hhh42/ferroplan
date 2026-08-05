//! End-to-end proof of the dense persistent-session control plane over real MCP stdio.

mod common;

use common::{Client, DOM, PROB};
use serde_json::{json, Value};

fn open(c: &mut Client, id: &str) -> Value {
    c.call_json(
        "session_open",
        json!({"session_id": id, "domain": DOM, "problem": PROB}),
    )
}

#[test]
fn list_state_atomic_set_fork_compare_checkpoint_restore_and_history_form_one_control_plane() {
    let mut c = Client::start();
    let parent = "dense-parent";
    open(&mut c, parent);

    let listed = c.call_json("session_list", json!({"prefix": "dense-"}));
    assert_eq!(listed["count"], 1);
    assert_eq!(listed["sessions"][0]["session_id"], parent);

    let before = c.call_json(
        "session_state",
        json!({"session_id": parent, "facts": ["(p)", "(q)"], "fluents": []}),
    );
    assert_eq!(before["facts"]["(P)"], true);
    assert_eq!(before["facts"]["(Q)"], false);
    let initial_fingerprint = before["state_fingerprint"].as_str().unwrap().to_owned();

    let (_, refused) = c.call_text(
        "session_set",
        json!({
            "session_id": parent,
            "expected_epoch": 0,
            "facts": [
                {"fact": "(q)", "value": true},
                {"fact": "(not-grounded)", "value": true}
            ]
        }),
    );
    assert!(refused);
    let after_refusal = c.call_json(
        "session_state",
        json!({"session_id": parent, "facts": ["(q)"]}),
    );
    assert_eq!(after_refusal["facts"]["(Q)"], false);
    assert_eq!(after_refusal["state_fingerprint"], initial_fingerprint);

    let changed = c.call_json(
        "session_set",
        json!({
            "session_id": parent,
            "expected_epoch": 0,
            "facts": [
                {"fact": "(p)", "value": false},
                {"fact": "(q)", "value": true}
            ]
        }),
    );
    assert_eq!(changed["epoch"], 1);

    let child = "dense-child";
    let forked = c.call_json(
        "session_fork",
        json!({
            "session_id": parent,
            "child_session_id": child,
            "expected_epoch": 1
        }),
    );
    assert_eq!(forked["forked_from"], parent);
    assert_eq!(forked["session_id"], child);
    assert_eq!(
        forked["shared_world_bytes"],
        listed["sessions"][0]["world_bytes"]
    );

    let equivalent = c.call_json(
        "session_compare",
        json!({"left_session_id": parent, "right_session_id": child}),
    );
    assert_eq!(equivalent["equivalent"], true, "{equivalent}");

    c.call_json(
        "session_set",
        json!({
            "session_id": child,
            "expected_epoch": 1,
            "goal": "(q)"
        }),
    );
    let divergent = c.call_json(
        "session_compare",
        json!({"left_session_id": parent, "right_session_id": child}),
    );
    assert_eq!(divergent["equivalent"], false);
    assert!(divergent["differences"]
        .as_array()
        .unwrap()
        .iter()
        .any(|v| v == "state_fingerprint" || v == "goal_met"));

    let replanned = c.call_json(
        "session_replan",
        json!({
            "session_id": parent,
            "expected_epoch": 1,
            "max_evaluated": 10_000,
            "reason": "forced checkpoint baseline"
        }),
    );
    assert_eq!(replanned["forced"], true);
    assert_eq!(replanned["solved"], true);
    assert_eq!(replanned["solution"]["plan"]["steps"][0]["action"], "B");

    let checkpoint = c.call_json(
        "session_checkpoint",
        json!({
            "session_id": parent,
            "checkpoint_id": "dense-baseline",
            "expected_epoch": 1
        }),
    );
    assert_eq!(checkpoint["checkpoint_id"], "dense-baseline");
    assert_eq!(checkpoint["checkpoint_digest"].as_str().unwrap().len(), 64);

    c.call_json(
        "session_set",
        json!({
            "session_id": parent,
            "expected_epoch": 1,
            "goal": "(r)"
        }),
    );
    let mismatch = c.call_json(
        "session_verify_checkpoint",
        json!({"session_id": parent, "checkpoint_id": "dense-baseline"}),
    );
    assert_eq!(mismatch["matches"], false);

    let restored = c.call_json(
        "session_restore",
        json!({
            "checkpoint_id": "dense-baseline",
            "session_id": parent,
            "expected_epoch": 2,
            "replace": true
        }),
    );
    assert_eq!(restored["session_id"], parent);
    let match_after = c.call_json(
        "session_verify_checkpoint",
        json!({"session_id": parent, "checkpoint_id": "dense-baseline"}),
    );
    assert_eq!(match_after["matches"], true, "{match_after}");

    let history = c.call_json(
        "session_history",
        json!({"session_id": parent, "offset": 0, "limit": 64}),
    );
    assert!(history["total"].as_u64().unwrap() >= 5);
    assert!(history["events"]
        .as_array()
        .unwrap()
        .iter()
        .any(|event| event["event"] == "checkpoint-restored"));
    c.finish();
}

#[test]
fn optimistic_concurrency_refuses_stale_mutation_without_state_drift() {
    let mut c = Client::start();
    let sid = "epoch-guard";
    open(&mut c, sid);
    c.call_json(
        "session_set",
        json!({
            "session_id": sid,
            "expected_epoch": 0,
            "facts": [{"fact": "(q)", "value": true}]
        }),
    );
    let before = c.call_json(
        "session_state",
        json!({"session_id": sid, "facts": ["(p)", "(q)"]}),
    );
    let (_, stale) = c.call_text(
        "session_set",
        json!({
            "session_id": sid,
            "expected_epoch": 0,
            "facts": [{"fact": "(p)", "value": false}]
        }),
    );
    assert!(stale);
    let after = c.call_json(
        "session_state",
        json!({"session_id": sid, "facts": ["(p)", "(q)"]}),
    );
    assert_eq!(after["state_fingerprint"], before["state_fingerprint"]);
    assert_eq!(after["facts"], before["facts"]);
    c.finish();
}

#[test]
fn operator_scope_is_planner_authority_not_post_hoc_filtering() {
    let mut c = Client::start();
    let sid = "operator-authority";
    open(&mut c, sid);
    let restricted = c.call_json(
        "session_restrict_ops",
        json!({
            "session_id": sid,
            "expected_epoch": 0,
            "allowed_prefixes": ["B"]
        }),
    );
    assert_eq!(restricted["epoch"], 1);
    let refused_plan = c.call_json(
        "session_replan",
        json!({"session_id": sid, "expected_epoch": 1, "max_evaluated": 10_000}),
    );
    assert_eq!(refused_plan["solved"], false);

    c.call_json(
        "session_restrict_ops",
        json!({
            "session_id": sid,
            "expected_epoch": 1,
            "allowed_prefixes": []
        }),
    );
    let admitted_plan = c.call_json(
        "session_replan",
        json!({"session_id": sid, "expected_epoch": 2, "max_evaluated": 10_000}),
    );
    assert_eq!(admitted_plan["solved"], true);
    c.finish();
}

const TEMPORAL_DOMAIN: &str = r#"
(define (domain clockwork)
  (:requirements :strips :durative-actions)
  (:predicates (ready) (done))
  (:durative-action work
    :parameters ()
    :duration (= ?duration 2)
    :condition (and (at start (ready)))
    :effect (and (at start (not (ready))) (at end (done))))
  (:durative-action reset
    :parameters ()
    :duration (= ?duration 1)
    :condition (and (at start (done)))
    :effect (and (at end (ready))))
)
"#;

const TEMPORAL_PROBLEM: &str = r#"
(define (problem clockwork-p)
  (:domain clockwork)
  (:init (ready))
  (:goal (done)))
"#;

#[test]
fn temporal_schedule_inflight_action_and_elapse_are_one_replayable_state_machine() {
    let mut c = Client::start();
    let sid = "temporal-control";
    c.call_json(
        "session_open",
        json!({
            "session_id": sid,
            "domain": TEMPORAL_DOMAIN,
            "problem": TEMPORAL_PROBLEM
        }),
    );
    c.call_json(
        "session_schedule_fact",
        json!({
            "session_id": sid,
            "expected_epoch": 0,
            "delay": 4.0,
            "fact": "(ready)",
            "value": true
        }),
    );
    c.call_json(
        "session_apply_start",
        json!({
            "session_id": sid,
            "expected_epoch": 1,
            "action": "(work)"
        }),
    );
    let before = c.call_json(
        "session_state",
        json!({"session_id": sid, "facts": ["(ready)", "(done)"]}),
    );
    assert_eq!(before["facts"]["(READY)"], false);
    assert_eq!(before["facts"]["(DONE)"], false);

    let elapsed = c.call_json(
        "session_elapse",
        json!({"session_id": sid, "expected_epoch": 2, "delta": 2.0}),
    );
    assert_eq!(elapsed["broken_intervals"].as_array().unwrap().len(), 0);
    assert_eq!(elapsed["goal_met"], true);
    let after = c.call_json(
        "session_state",
        json!({"session_id": sid, "facts": ["(done)"]}),
    );
    assert_eq!(after["facts"]["(DONE)"], true);
    c.finish();
}
