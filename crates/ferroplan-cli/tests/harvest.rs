use std::fs;

use ferroplan_cli::harvest::{
    admit, compile_pack, extract_operators, replay_pack, AdmissionLevel, ArtifactEvidence,
    EvidenceRef, ExecutionEvidence, ExecutionResult, FinalState, ObservationPack,
    ObservationWindow, ObservedOutcome, ObservedWorkItem, ReplayState, OBSERVATION_SCHEMA,
};

fn sha(ch: char) -> String {
    ch.to_string().repeat(40)
}

fn executed_work(message: &str) -> ObservedWorkItem {
    let head = sha('a');
    ObservedWorkItem {
        repository: "seanchatmangpt/ferroplan".to_owned(),
        sha: head.clone(),
        parent_sha: Some(sha('b')),
        message: message.to_owned(),
        committed_at_utc: "2026-08-01T12:00:00Z".to_owned(),
        source_url: format!("https://github.com/seanchatmangpt/ferroplan/commit/{head}"),
        changed_paths: vec!["crates/ferroplan/src/lib.rs".to_owned()],
        executions: vec![ExecutionEvidence {
            surface: "github-actions".to_owned(),
            command: "workflow:harvest#1".to_owned(),
            source_sha: head,
            result: ExecutionResult::Pass,
            exit_code: Some(0),
            observed_at_utc: "2026-08-01T12:05:00Z".to_owned(),
            evidence_url: "https://github.com/run/1".to_owned(),
        }],
        artifacts: vec![ArtifactEvidence {
            name: "receipt".to_owned(),
            source_sha: sha('a'),
            evidence_url: "https://github.com/artifact/1".to_owned(),
            digest: Some("blake3:abc".to_owned()),
            size_bytes: Some(12),
        }],
        probabilistic_outcomes: Vec::new(),
    }
}

fn pack(work_items: Vec<ObservedWorkItem>) -> ObservationPack {
    ObservationPack {
        schema: OBSERVATION_SCHEMA.to_owned(),
        run_id: "test-run".to_owned(),
        window: ObservationWindow {
            start_utc: "2026-08-01T00:00:00Z".to_owned(),
            end_exclusive_utc: "2026-08-02T00:00:00Z".to_owned(),
            timezone: "America/Los_Angeles".to_owned(),
        },
        repositories: vec!["seanchatmangpt/ferroplan".to_owned()],
        work_items,
        transport_failures: Vec::new(),
    }
}

#[test]
fn exact_head_success_is_admitted_and_corroborated() {
    let report = admit(&pack(vec![executed_work("fix: cache receipt digest")]));
    assert_eq!(report.admitted.len(), 1);
    assert_eq!(report.admitted[0].level, AdmissionLevel::ResultCorroborated);
    assert!(report.excluded.is_empty());
}

#[test]
fn workflow_success_bound_to_another_sha_is_refused() {
    let mut work = executed_work("fix: verify exact source");
    work.executions[0].source_sha = sha('c');
    let report = admit(&pack(vec![work]));
    assert!(report.admitted.is_empty());
    assert_eq!(report.excluded.len(), 1);
}

#[test]
fn equivalent_patterns_deduplicate_by_semantics_not_commit_name() {
    let mut first = executed_work("fix: cache verifier by receipt digest");
    let mut second = executed_work("perf: cache subsystem by source digest");
    second.sha = sha('c');
    second.parent_sha = Some(sha('d'));
    second.source_url = format!("https://github.com/commit/{}", second.sha);
    second.executions[0].source_sha = second.sha.clone();
    second.artifacts[0].source_sha = second.sha.clone();
    first.changed_paths.push("tools/verifier.rs".to_owned());
    let report = admit(&pack(vec![first, second]));
    let operators = extract_operators(&report);
    assert_eq!(operators.len(), 1);
    assert_eq!(operators[0].source_work.len(), 2);
}

#[test]
fn probability_without_evidence_is_refused() {
    let mut work = executed_work("feat: observe stochastic external edge");
    work.probabilistic_outcomes.push(ObservedOutcome {
        label: "success".to_owned(),
        probability: 0.5,
        success: true,
        evidence: Vec::new(),
    });
    let report = admit(&pack(vec![work]));
    assert!(report.admitted.is_empty());
}

#[test]
fn compile_and_external_replay_cross_real_filesystem_and_ppddl_boundaries() {
    let mut work = executed_work("fix: cache verifier by receipt digest");
    work.probabilistic_outcomes = vec![
        ObservedOutcome {
            label: "success".to_owned(),
            probability: 0.75,
            success: true,
            evidence: vec![EvidenceRef {
                kind: "measurement".to_owned(),
                identity: "sample-3-of-4".to_owned(),
                location: "file://measurement.json".to_owned(),
            }],
        },
        ObservedOutcome {
            label: "failure".to_owned(),
            probability: 0.25,
            success: false,
            evidence: vec![EvidenceRef {
                kind: "measurement".to_owned(),
                identity: "sample-1-of-4".to_owned(),
                location: "file://measurement.json".to_owned(),
            }],
        },
    ];
    let input = pack(vec![work]);
    let root = std::env::temp_dir().join(format!("ferroplan-harvest-test-{}", std::process::id()));
    let first = root.join("first");
    let second = root.join("second");
    let _ = fs::remove_dir_all(&root);

    let receipt = compile_pack(&input, &first).expect("compile");
    let receipt_json = serde_json::to_string_pretty(&receipt).expect("receipt JSON");
    assert_eq!(
        receipt.final_state,
        FinalState::PartialAlive,
        "{receipt_json}"
    );
    assert!(receipt.validation.parse_ok, "{receipt_json}");
    assert_eq!(
        receipt.validation.policy_valid,
        Some(true),
        "{receipt_json}"
    );
    assert!(first.join("domain.ppddl").is_file());
    assert!(first.join("problem.ppddl").is_file());
    assert!(first.join("method-catalog.json").is_file());

    let replay = replay_pack(&input, &receipt, &second).expect("replay");
    assert_eq!(replay.replay, ReplayState::ReplayMatch);
    assert_eq!(replay.final_state, FinalState::Alive);
    assert_eq!(
        fs::read(first.join("domain.ppddl")).unwrap(),
        fs::read(second.join("domain.ppddl")).unwrap()
    );
    let _ = fs::remove_dir_all(&root);
}
