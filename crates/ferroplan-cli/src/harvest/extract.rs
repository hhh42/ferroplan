use std::collections::BTreeMap;

use serde::Serialize;

use super::{
    digest_bytes, format_probability, slug, sort_dedup_evidence, ActuationClass, AdmissionReport,
    AdmittedWork, GallCheckpoint, OperatorOutcome, PlanningOperator,
};

pub fn extract_operators(report: &AdmissionReport) -> Vec<PlanningOperator> {
    extract_operators_with_count(report).0
}

pub(crate) fn extract_operators_with_count(
    report: &AdmissionReport,
) -> (Vec<PlanningOperator>, usize) {
    let mut candidates = Vec::new();
    for admitted in &report.admitted {
        extract_work_operators(admitted, &mut candidates);
    }
    let raw_count = candidates.len();
    (deduplicate(candidates), raw_count)
}

fn extract_work_operators(admitted: &AdmittedWork, candidates: &mut Vec<PlanningOperator>) {
    let subject = admitted
        .work
        .message
        .lines()
        .next()
        .unwrap_or("observed-work")
        .trim();
    let lower = subject.to_ascii_lowercase();
    let mut matched = false;

    push_rule(
        lower.contains("generated")
            && (lower.contains("template")
                || lower.contains("generator")
                || lower.contains("sync")),
        &mut matched,
        candidates,
        admitted,
        "repair-authoritative-source-before-regeneration",
        GallCheckpoint::G5Manufacture,
        &["generated-drift-observed", "authoritative-source-resolved"],
        &["authoritative-source-repaired", "projection-regenerated"],
        &["generated-drift-persists"],
        &["hand-edited-generated-output"],
    );
    push_rule(
        lower.contains("fresh checkout") || lower.contains("clean checkout"),
        &mut matched,
        candidates,
        admitted,
        "prepare-runtime-before-verification",
        GallCheckpoint::G6Verify,
        &["exact-source-materialized", "runtime-state-absent"],
        &["runtime-state-prepared", "verification-self-sufficient"],
        &["warm-checkout-dependency"],
        &["verification-key-silently-generated"],
    );
    push_rule(
        lower.contains("cache") && lower.contains("digest"),
        &mut matched,
        candidates,
        admitted,
        "reuse-verifier-by-exact-input-digest",
        GallCheckpoint::G6Verify,
        &["source-identity-matched", "receipt-digest-matched"],
        &["verified-result-reused"],
        &["cache-identity-mismatch"],
        &["unbound-cache-hit"],
    );
    push_rule(
        lower.contains("timeout") || lower.contains("flaky"),
        &mut matched,
        candidates,
        admitted,
        "replace-wall-clock-test-oracle",
        GallCheckpoint::G6Verify,
        &["load-sensitive-test-observed"],
        &["deterministic-termination-bound-used"],
        &["cycle-guard-regression"],
        &["ambient-load-as-correctness-oracle"],
    );
    push_rule(
        lower.contains("--bin") || lower.contains("which binary"),
        &mut matched,
        candidates,
        admitted,
        "select-explicit-binary-target",
        GallCheckpoint::G4Plan,
        &["package-has-multiple-binaries"],
        &["binary-target-resolved"],
        &["ambiguous-cargo-run"],
        &["implicit-binary-selection"],
    );
    push_rule(
        lower.contains("manifest") && lower.contains("crown"),
        &mut matched,
        candidates,
        admitted,
        "manufacture-prerequisite-before-crown",
        GallCheckpoint::G6Verify,
        &["crown-prerequisite-declared", "prerequisite-artifact-absent"],
        &["prerequisite-artifact-manufactured", "crown-observer-enabled"],
        &["crown-refuses-missing-prerequisite"],
        &["crown-self-manufactures-evidence"],
    );
    push_rule(
        lower.contains("worktree") && lower.contains("scan"),
        &mut matched,
        candidates,
        admitted,
        "exclude-ephemeral-worktrees-from-repository-observation",
        GallCheckpoint::G2Observe,
        &["repository-scan-bounded", "ephemeral-worktrees-present"],
        &["canonical-tree-only-observed"],
        &["duplicate-observation-counts"],
        &["ephemeral-checkout-admitted-as-source"],
    );
    push_rule(
        lower.contains("hardcoded") && lower.contains("count"),
        &mut matched,
        candidates,
        admitted,
        "replace-hardcoded-count-with-authoritative-query",
        GallCheckpoint::G3Admit,
        &["stale-count-observed", "authoritative-enumeration-available"],
        &["count-derived-from-authority"],
        &["prose-count-drifts"],
        &["new-magic-count"],
    );
    push_rule(
        lower.contains("yaml") && (lower.contains("quote") || lower.contains("scalar")),
        &mut matched,
        candidates,
        admitted,
        "quote-ambiguous-workflow-scalar",
        GallCheckpoint::G5Manufacture,
        &["workflow-yaml-refused"],
        &["workflow-parses"],
        &["workflow-parser-failure"],
        &["semantic-command-change"],
    );
    push_rule(
        lower.contains("rustfmt") || lower.contains("format"),
        &mut matched,
        candidates,
        admitted,
        "format-owned-source-boundary",
        GallCheckpoint::G6Verify,
        &["format-drift-observed", "ownership-boundary-known"],
        &["owned-source-formatted"],
        &["format-drift-remains"],
        &["unrelated-format-churn"],
    );

    if !admitted.work.probabilistic_outcomes.is_empty() {
        matched = true;
        let mut candidate = operator_candidate(
            &format!("observe-{}-outcome", slug(subject)),
            GallCheckpoint::G2Observe,
            &["stochastic-edge-admitted", "probability-evidence-bound"],
            &["outcome-observed"],
            &["unobserved-outcome"],
            &["invented-probability"],
            admitted,
        );
        candidate.actuation_class = ActuationClass::Select;
        candidate.probabilistic_outcomes = admitted
            .work
            .probabilistic_outcomes
            .iter()
            .map(|outcome| OperatorOutcome {
                label: outcome.label.clone(),
                probability: outcome.probability,
                success: outcome.success,
                evidence: outcome.evidence.clone(),
            })
            .collect();
        finalize_operator(&mut candidate);
        candidates.push(candidate);
    }

    if !matched {
        candidates.push(operator_candidate(
            "verify-exact-source-change",
            GallCheckpoint::G6Verify,
            &["exact-source-bound", "changed-paths-observed"],
            &["execution-result-admitted"],
            &["verification-failed"],
            &["inspection-promoted-to-execution"],
            admitted,
        ));
    }
}

#[allow(clippy::too_many_arguments)]
fn push_rule(
    condition: bool,
    matched: &mut bool,
    candidates: &mut Vec<PlanningOperator>,
    admitted: &AdmittedWork,
    name: &str,
    checkpoint: GallCheckpoint,
    preconditions: &[&str],
    effects: &[&str],
    failures: &[&str],
    refusals: &[&str],
) {
    if condition {
        *matched = true;
        candidates.push(operator_candidate(
            name,
            checkpoint,
            preconditions,
            effects,
            failures,
            refusals,
            admitted,
        ));
    }
}

fn operator_candidate(
    name: &str,
    checkpoint: GallCheckpoint,
    preconditions: &[&str],
    effects: &[&str],
    failures: &[&str],
    refusals: &[&str],
    admitted: &AdmittedWork,
) -> PlanningOperator {
    let mut operator = PlanningOperator {
        id: String::new(),
        name: name.to_owned(),
        signature: String::new(),
        checkpoint,
        actuation_class: ActuationClass::Construct,
        preconditions: preconditions.iter().map(|value| (*value).to_owned()).collect(),
        effects: effects.iter().map(|value| (*value).to_owned()).collect(),
        invariants: vec![
            "zero-unreceipted-actuation".to_owned(),
            "observation-is-not-admission".to_owned(),
            "generated-output-is-not-authority".to_owned(),
        ],
        failures: failures.iter().map(|value| (*value).to_owned()).collect(),
        refusals: refusals.iter().map(|value| (*value).to_owned()).collect(),
        receipt_hook: true,
        replay_hook: true,
        probabilistic_outcomes: Vec::new(),
        evidence: admitted.evidence.clone(),
        source_work: vec![admitted.identity.clone()],
    };
    finalize_operator(&mut operator);
    operator
}

fn finalize_operator(operator: &mut PlanningOperator) {
    normalize_operator(operator);
    operator.signature = operator_signature(operator);
    operator.id = format!("method-{}", &operator.signature[..16]);
}

fn deduplicate(candidates: Vec<PlanningOperator>) -> Vec<PlanningOperator> {
    let mut by_signature: BTreeMap<String, PlanningOperator> = BTreeMap::new();
    for mut candidate in candidates {
        let entry = by_signature
            .entry(candidate.signature.clone())
            .or_insert_with(|| candidate.clone());
        entry.evidence.append(&mut candidate.evidence);
        entry.source_work.append(&mut candidate.source_work);
        sort_dedup_evidence(&mut entry.evidence);
        entry.source_work.sort();
        entry.source_work.dedup();
        if candidate.name < entry.name {
            entry.name = candidate.name;
        }
    }
    by_signature.into_values().collect()
}

fn normalize_operator(operator: &mut PlanningOperator) {
    operator.preconditions.sort();
    operator.preconditions.dedup();
    operator.effects.sort();
    operator.effects.dedup();
    operator.invariants.sort();
    operator.invariants.dedup();
    operator.failures.sort();
    operator.failures.dedup();
    operator.refusals.sort();
    operator.refusals.dedup();
    operator
        .probabilistic_outcomes
        .sort_by(|left, right| left.label.cmp(&right.label));
    for outcome in &mut operator.probabilistic_outcomes {
        sort_dedup_evidence(&mut outcome.evidence);
    }
    sort_dedup_evidence(&mut operator.evidence);
    operator.source_work.sort();
    operator.source_work.dedup();
}

#[derive(Serialize)]
struct SignatureView<'a> {
    checkpoint: GallCheckpoint,
    actuation_class: ActuationClass,
    preconditions: &'a [String],
    effects: &'a [String],
    invariants: &'a [String],
    failures: &'a [String],
    refusals: &'a [String],
    receipt_hook: bool,
    replay_hook: bool,
    outcomes: Vec<(&'a str, String, bool)>,
}

fn operator_signature(operator: &PlanningOperator) -> String {
    let view = SignatureView {
        checkpoint: operator.checkpoint,
        actuation_class: operator.actuation_class,
        preconditions: &operator.preconditions,
        effects: &operator.effects,
        invariants: &operator.invariants,
        failures: &operator.failures,
        refusals: &operator.refusals,
        receipt_hook: operator.receipt_hook,
        replay_hook: operator.replay_hook,
        outcomes: operator
            .probabilistic_outcomes
            .iter()
            .map(|outcome| {
                (
                    outcome.label.as_str(),
                    format_probability(outcome.probability),
                    outcome.success,
                )
            })
            .collect(),
    };
    digest_bytes(&serde_json::to_vec(&view).expect("signature serialization"))
}
