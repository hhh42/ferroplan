//! End-to-end proof of the operator-experience plane over the real MCP stdio server.

mod common;

use common::{Client, DOM, PROB};
use serde_json::{json, Value};

#[test]
fn manifest_composition_and_lattice_make_the_authority_surface_computable() {
    let mut client = Client::start();
    let manifest = client.call_json("dx_manifest", json!({"include_examples": true}));
    assert_eq!(manifest["advertised_tool_count"], 42);
    assert_eq!(manifest["modeled_tool_count"], 42);

    let resource = client.request(
        "resources/read",
        json!({"uri": "ferroplan://tools/dx_manifest"}),
    );
    let resource_text = resource["result"]["contents"][0]["text"]
        .as_str()
        .expect("experience resource text");
    let resource_body: Value =
        serde_json::from_str(resource_text).expect("experience resource JSON");
    assert_eq!(
        resource_body["source"],
        "plugins/chatman-ecosystem/ontology/ferroplan-experience.ttl"
    );
    assert!(resource_body["rdfs_comment"]
        .as_str()
        .is_some_and(|comment| comment.contains("self-describing")));
    assert!(manifest["categories"]
        .as_array()
        .unwrap()
        .iter()
        .any(|category| category == "doctor"));

    let composition = client.call_json(
        "dx_compose",
        json!({
            "have": ["domain_source", "problem_source", "digest"],
            "want": ["verified_receipt"],
            "max_steps": 6
        }),
    );
    assert_eq!(composition["solved"], true, "{composition}");
    assert_eq!(
        composition["steps"],
        json!(["solve", "bind_plan_receipt", "verify_receipt"])
    );

    let lattice = client.call_json(
        "vision_lattice",
        json!({
            "seeds": ["domain_source", "problem_source", "digest"],
            "max_depth": 5,
            "max_states": 4096
        }),
    );
    assert!(matches!(
        lattice["standing"].as_str(),
        Some("ALIVE" | "PARTIAL_ALIVE")
    ));
    assert!(lattice["reachable_tools"]
        .as_array()
        .unwrap()
        .iter()
        .any(|tool| tool == "solve"));
    assert!(lattice["reachable_atoms"]
        .as_array()
        .unwrap()
        .iter()
        .any(|atom| atom == "verified_receipt"));
    client.finish();
}

#[test]
fn wizard_bootstrap_snapshot_and_doctor_collapse_setup_and_operability() {
    let mut client = Client::start();
    let session_id = "vision-bootstrap";
    let bootstrap = client.call_json(
        "wizard_bootstrap",
        json!({
            "session_id": session_id,
            "domain": DOM,
            "problem": PROB,
            "plan": true,
            "max_evaluated": 10_000
        }),
    );
    assert_eq!(bootstrap["session_id"], session_id);
    assert_eq!(bootstrap["solution"]["solved"], true);
    assert_eq!(bootstrap["solution"]["plan"]["steps"][0]["action"], "A");
    assert_eq!(bootstrap["report"]["standing"], "ALIVE", "{bootstrap}");

    let snapshot = client.call_json(
        "qol_snapshot",
        json!({
            "session_id": session_id,
            "facts": ["(p)", "(q)", "(r)"],
            "history_tail": 16
        }),
    );
    assert_eq!(snapshot["state"]["facts"]["(P)"], true);
    assert_eq!(snapshot["plan"]["length"], 2);
    assert_eq!(snapshot["doctor"]["standing"], "ALIVE");
    assert_eq!(snapshot["history_tail"].as_array().unwrap().len(), 1);

    let doctor = client.call_json(
        "doctor_scan",
        json!({"session_id": session_id, "history_tail": 4}),
    );
    assert_eq!(doctor["report"]["standing"], "ALIVE");
    assert!(doctor["report"]["health_score"].as_u64().unwrap() >= 90);
    client.finish();
}

#[test]
fn qol_batch_is_atomic_epoch_guarded_and_can_finish_with_a_real_replan() {
    let mut client = Client::start();
    let session_id = "vision-batch";
    client.call_json(
        "wizard_bootstrap",
        json!({
            "session_id": session_id,
            "domain": DOM,
            "problem": PROB,
            "plan": false
        }),
    );
    let before = client.call_json(
        "qol_snapshot",
        json!({"session_id": session_id, "facts": ["(p)", "(q)"]}),
    );
    let before_fingerprint = before["identity"]["state_fingerprint"]
        .as_str()
        .unwrap()
        .to_owned();

    let (_, refused) = client.call_text(
        "qol_batch",
        json!({
            "session_id": session_id,
            "expected_epoch": 0,
            "operations": [
                {"op": "set_fact", "fact": "(q)", "value": true},
                {"op": "set_fact", "fact": "(not-grounded)", "value": true}
            ]
        }),
    );
    assert!(refused);
    let after_refusal = client.call_json(
        "qol_snapshot",
        json!({"session_id": session_id, "facts": ["(p)", "(q)"]}),
    );
    assert_eq!(
        after_refusal["identity"]["state_fingerprint"],
        before_fingerprint
    );
    assert_eq!(after_refusal["state"]["facts"]["(P)"], true);
    assert_eq!(after_refusal["state"]["facts"]["(Q)"], false);

    let committed = client.call_json(
        "qol_batch",
        json!({
            "session_id": session_id,
            "expected_epoch": 0,
            "operations": [
                {"op": "set_fact", "fact": "(p)", "value": false},
                {"op": "set_fact", "fact": "(q)", "value": true},
                {"op": "set_goal", "goal": "(r)"},
                {"op": "replan", "max_evaluated": 10_000}
            ]
        }),
    );
    assert_eq!(committed["epoch"], 1);
    assert_ne!(committed["after_fingerprint"], before_fingerprint);
    assert_eq!(committed["results"][3]["solution"]["solved"], true);
    assert_eq!(
        committed["results"][3]["solution"]["plan"]["steps"][0]["action"],
        "B"
    );

    let stable = client.call_json(
        "qol_snapshot",
        json!({"session_id": session_id, "facts": ["(p)", "(q)"]}),
    );
    let stable_fingerprint = stable["identity"]["state_fingerprint"].clone();
    let (_, stale) = client.call_text(
        "qol_batch",
        json!({
            "session_id": session_id,
            "expected_epoch": 0,
            "operations": [{"op": "set_fact", "fact": "(p)", "value": true}]
        }),
    );
    assert!(stale);
    let after_stale = client.call_json(
        "qol_snapshot",
        json!({"session_id": session_id, "facts": ["(p)", "(q)"]}),
    );
    assert_eq!(
        after_stale["identity"]["state_fingerprint"],
        stable_fingerprint
    );
    client.finish();
}

#[test]
fn doctor_and_wizard_turn_failures_and_intents_into_machine_actions() {
    let mut client = Client::start();
    let diagnosis = client.call_json(
        "doctor_explain",
        json!({
            "message": "stale session epoch: expected 2, observed 3",
            "context": {"session_id": "alpha"}
        }),
    );
    assert_eq!(diagnosis["code"], "STALE_EPOCH");
    assert!(diagnosis["confidence"].as_f64().unwrap() > 0.9);
    assert!(diagnosis["recommended_tools"]
        .as_array()
        .unwrap()
        .iter()
        .any(|tool| tool == "qol_snapshot"));

    let recipe = client.call_json(
        "wizard_recipe",
        json!({
            "intent": "remote_handoff",
            "parameters": {"recipient": "edge-agent"}
        }),
    );
    assert_eq!(recipe["intent"], "remote_handoff");
    assert_eq!(recipe["steps"][0]["tool"], "qol_snapshot");
    assert_eq!(recipe["steps"][1]["tool"], "telco_envelope");
    assert_eq!(recipe["steps"][2]["tool"], "telco_verify");
    client.finish();
}

#[test]
fn telco_envelopes_are_deterministic_expiring_and_tamper_evident() {
    let mut client = Client::start();
    let envelope = client.call_json(
        "telco_envelope",
        json!({
            "sender": "ferroplan/operator",
            "recipient": "edge-agent-7",
            "channel": "mcp.handoff",
            "issued_at_ms": 1_000,
            "ttl_ms": 5_000,
            "correlation_id": "corr-001",
            "payload": {"standing": "ALIVE", "epoch": 1}
        }),
    );
    assert_eq!(envelope["authentication"], "UNSUPPORTED");
    assert_eq!(envelope["payload_digest"].as_str().unwrap().len(), 64);
    assert_eq!(envelope["envelope_digest"].as_str().unwrap().len(), 64);

    let verified = client.call_json(
        "telco_verify",
        json!({
            "envelope": envelope,
            "observed_at_ms": 2_000,
            "expected_recipient": "edge-agent-7"
        }),
    );
    assert_eq!(verified["valid"], true, "{verified}");

    let mut tampered: Value = client.call_json(
        "telco_envelope",
        json!({
            "sender": "ferroplan/operator",
            "recipient": "edge-agent-7",
            "channel": "mcp.handoff",
            "issued_at_ms": 1_000,
            "ttl_ms": 5_000,
            "correlation_id": "corr-002",
            "payload": {"standing": "ALIVE"}
        }),
    );
    tampered["payload"]["standing"] = json!("BLOCKED");
    let refused = client.call_json(
        "telco_verify",
        json!({"envelope": tampered, "observed_at_ms": 2_000}),
    );
    assert_eq!(refused["valid"], false);
    assert!(refused["failures"]
        .as_array()
        .unwrap()
        .iter()
        .any(|failure| failure == "PAYLOAD_DIGEST_MISMATCH"));

    let expired = client.call_json(
        "telco_envelope",
        json!({
            "sender": "ferroplan/operator",
            "recipient": "edge-agent-7",
            "channel": "mcp.handoff",
            "issued_at_ms": 1_000,
            "ttl_ms": 1_000,
            "correlation_id": "corr-003",
            "payload": {"standing": "ALIVE"}
        }),
    );
    let refused = client.call_json(
        "telco_verify",
        json!({"envelope": expired, "observed_at_ms": 2_001}),
    );
    assert_eq!(refused["standing"], "REFUSED");
    assert!(refused["failures"]
        .as_array()
        .unwrap()
        .iter()
        .any(|failure| failure == "ENVELOPE_EXPIRED"));
    client.finish();
}
