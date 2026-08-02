use std::fs;

use ferroplan::{solve_ppddl, ProbabilisticOptions};
use ferroplan_cli::harvest::{
    admit, compile_pack, extract_operators, replay_pack, AdmissionLevel, ArtifactEvidence,
    EvidenceRef, ExecutionEvidence, ExecutionResult, FinalState, ObservationPack,
    ObservationWindow, ObservedOutcome, ObservedWorkItem, RefusalCode, ReplayState,
    OBSERVATION_SCHEMA,
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

fn assert_blake3_hex(value: &str) {
    assert_eq!(value.len(), 64, "BLAKE3 identity must contain 64 hex characters");
    assert!(
        value
            .bytes()
            .all(|byte| matches!(byte, b'0'..=b'9' | b'a'..=b'f')),
        "BLAKE3 identity must be canonical lowercase hex: {value}"
    );
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

    let domain = fs::read_to_string(first.join("domain.ppddl")).expect("domain");
    let problem = fs::read_to_string(first.join("problem.ppddl")).expect("problem");
    let policy = solve_ppddl(
        &domain,
        &problem,
        &ProbabilisticOptions {
            horizon: Some(7),
            ..Default::default()
        },
    )
    .expect("policy");
    assert!(policy
        .states
        .iter()
        .all(|state| state.facts.windows(2).all(|pair| pair[0] <= pair[1])));

    let replay = replay_pack(&input, &receipt, &second).expect("replay");
    assert_eq!(replay.replay, ReplayState::ReplayMatch);
    assert_eq!(replay.final_state, FinalState::Alive);
    assert_eq!(
        fs::read(first.join("domain.ppddl")).unwrap(),
        fs::read(second.join("domain.ppddl")).unwrap()
    );
    let _ = fs::remove_dir_all(&root);
}

/// Chicago/classicist TDD validation of the bounded replacement claim.
///
/// No interaction mocks are used. Real domain objects, real files, the real
/// PPDDL parser/solver/validator, receipt generation, semantic deduplication,
/// and clean replay must leave the observable system in the claimed state.
#[test]
fn chicago_tdd_replaces_the_bounded_human_continuity_function() {
    let first = executed_work("fix: cache verifier by receipt digest");
    let mut second = executed_work("perf: cache subsystem by source digest");
    second.sha = sha('c');
    second.parent_sha = Some(sha('d'));
    second.source_url = format!("https://github.com/commit/{}", second.sha);
    second.executions[0].source_sha = second.sha.clone();
    second.executions[0].command = "workflow:harvest#2".to_owned();
    second.executions[0].evidence_url = "https://github.com/run/2".to_owned();
    second.artifacts[0].source_sha = second.sha.clone();
    second.artifacts[0].evidence_url = "https://github.com/artifact/2".to_owned();

    let input = pack(vec![first, second]);
    let root = std::env::temp_dir().join(format!(
        "ferroplan-chicago-replacement-{}",
        std::process::id()
    ));
    let manufactured = root.join("manufactured");
    let replayed = root.join("replayed");
    let _ = fs::remove_dir_all(&root);

    let receipt = compile_pack(&input, &manufactured).expect("manufacture bounded workflow");

    // Human status collection and evidence assembly are replaced by state.
    assert_eq!(receipt.admitted_work.len(), 2);
    assert!(receipt.excluded_work.is_empty());
    assert!(receipt.transport_failures.is_empty());
    assert!(receipt.failures.is_empty());

    // Human tribal memory and manual procedure authoring are replaced by one
    // reusable semantic operator derived from two differently named events.
    assert_eq!(receipt.operators_added.len(), 1);
    assert_eq!(receipt.operators_deduplicated, 1);

    // Manual PPDDL authoring, test coordination, and audit-packet assembly are
    // replaced by the observable manufactured artifacts and verifier state.
    assert_eq!(receipt.final_state, FinalState::PartialAlive);
    assert!(receipt.validation.parse_ok);
    assert_eq!(receipt.validation.solved, Some(true));
    assert_eq!(receipt.validation.policy_valid, Some(true));
    assert_blake3_hex(&receipt.receipt_digest);
    assert!(manufactured.join("observation-pack.json").is_file());
    assert!(manufactured.join("admission-report.json").is_file());
    assert!(manufactured.join("method-catalog.json").is_file());
    assert!(manufactured.join("domain.ppddl").is_file());
    assert!(manufactured.join("problem.ppddl").is_file());

    // A fresh execution reproduces the same state without a human remembering
    // the command sequence or interpreting the implementation.
    let replay = replay_pack(&input, &receipt, &replayed).expect("clean replay");
    assert_eq!(replay.replay, ReplayState::ReplayMatch);
    assert_eq!(replay.final_state, FinalState::Alive);
    assert_eq!(receipt.source_pack_digest, replay.source_pack_digest);
    assert_eq!(receipt.catalog_digest, replay.catalog_digest);
    assert_eq!(receipt.outputs, replay.outputs);

    let _ = fs::remove_dir_all(&root);
}

/// Counter-test for the part of the thesis that must remain human-owned.
///
/// The system may replace repetitive continuity work, but it may not invent
/// execution, authority, probability, or standing merely to complete a run.
#[test]
fn chicago_tdd_preserves_the_human_authority_boundary() {
    let mut unexecuted = executed_work("feat: declare an unverified capability");
    unexecuted.executions.clear();
    unexecuted.artifacts.clear();
    let input = pack(vec![unexecuted]);
    let root = std::env::temp_dir().join(format!(
        "ferroplan-chicago-authority-{}",
        std::process::id()
    ));
    let _ = fs::remove_dir_all(&root);

    let receipt = compile_pack(&input, &root).expect("bounded refusal receipt");

    assert_eq!(receipt.final_state, FinalState::Unknown);
    assert!(receipt.admitted_work.is_empty());
    assert!(receipt.operators_added.is_empty());
    assert_eq!(receipt.excluded_work.len(), 1);
    assert_eq!(
        receipt.excluded_work[0].code,
        RefusalCode::ExecutionNotObserved
    );
    assert!(receipt
        .failures
        .iter()
        .any(|failure| failure == "NO_ADMITTED_EXECUTED_WORK"));
    assert!(!root.join("domain.ppddl").exists());
    assert!(!root.join("problem.ppddl").exists());
    assert_blake3_hex(&receipt.receipt_digest);

    let _ = fs::remove_dir_all(&root);
}
