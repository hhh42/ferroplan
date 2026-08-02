mod compile;
mod extract;
mod gh;
pub mod model;

use std::collections::BTreeSet;
use std::fs;
use std::path::Path;

use anyhow::{anyhow, Context, Result};
use serde::Serialize;

pub use compile::{compile_pack, replay_pack};
pub use extract::extract_operators;
pub use gh::collect_with_gh;
pub use model::*;

pub fn validate_window(window: &ObservationWindow) -> Result<()> {
    validate_utc(&window.start_utc)?;
    validate_utc(&window.end_exclusive_utc)?;
    if window.start_utc >= window.end_exclusive_utc {
        return Err(anyhow!(
            "observation window start must precede end_exclusive"
        ));
    }
    if window.timezone.trim().is_empty() {
        return Err(anyhow!("observation timezone must be named"));
    }
    Ok(())
}

fn validate_utc(value: &str) -> Result<()> {
    let bytes = value.as_bytes();
    let shape = bytes.len() == 20
        && bytes[4] == b'-'
        && bytes[7] == b'-'
        && bytes[10] == b'T'
        && bytes[13] == b':'
        && bytes[16] == b':'
        && bytes[19] == b'Z'
        && bytes.iter().enumerate().all(|(index, byte)| {
            matches!(index, 4 | 7 | 10 | 13 | 16 | 19) || byte.is_ascii_digit()
        });
    if !shape {
        return Err(anyhow!(
            "timestamp must be canonical UTC YYYY-MM-DDTHH:MM:SSZ: {value}"
        ));
    }
    Ok(())
}

pub fn load_observation_pack(path: &Path) -> Result<ObservationPack> {
    let bytes = fs::read(path).with_context(|| format!("reading {}", path.display()))?;
    let pack: ObservationPack =
        serde_json::from_slice(&bytes).with_context(|| format!("decoding {}", path.display()))?;
    validate_pack(&pack)?;
    Ok(pack)
}

pub fn validate_pack(pack: &ObservationPack) -> Result<()> {
    if pack.schema != OBSERVATION_SCHEMA {
        return Err(anyhow!(
            "unsupported observation schema {}; expected {OBSERVATION_SCHEMA}",
            pack.schema
        ));
    }
    validate_window(&pack.window)
}

pub fn admit(pack: &ObservationPack) -> AdmissionReport {
    let mut admitted = Vec::new();
    let mut excluded = Vec::new();

    for work in &pack.work_items {
        let identity = work_identity(work);
        if !is_exact_sha(&work.sha) {
            excluded.push(ExcludedWork {
                identity,
                code: RefusalCode::MissingExactSourceIdentity,
                detail: "work item does not carry a 40-character hexadecimal commit SHA".to_owned(),
            });
            continue;
        }
        if work.committed_at_utc < pack.window.start_utc
            || work.committed_at_utc >= pack.window.end_exclusive_utc
        {
            excluded.push(ExcludedWork {
                identity,
                code: RefusalCode::OutsideObservationWindow,
                detail: "commit timestamp is outside the admitted observation window".to_owned(),
            });
            continue;
        }
        if work.changed_paths.is_empty() {
            excluded.push(ExcludedWork {
                identity,
                code: RefusalCode::MissingChangedPaths,
                detail: "commit existence without changed-path evidence is not executable work"
                    .to_owned(),
            });
            continue;
        }
        if let Some((code, detail)) = probability_refusal(work) {
            excluded.push(ExcludedWork {
                identity,
                code,
                detail,
            });
            continue;
        }

        let successful = work
            .executions
            .iter()
            .filter(|execution| {
                execution.source_sha == work.sha && execution.result == ExecutionResult::Pass
            })
            .collect::<Vec<_>>();
        if successful.is_empty() {
            let mismatched = work
                .executions
                .iter()
                .any(|execution| execution.result == ExecutionResult::Pass);
            excluded.push(ExcludedWork {
                identity,
                code: if mismatched {
                    RefusalCode::WorkflowRunNotBoundToHead
                } else {
                    RefusalCode::ExecutionNotObserved
                },
                detail: "no successful execution was observed against this exact source SHA"
                    .to_owned(),
            });
            continue;
        }

        let mut evidence = vec![EvidenceRef {
            kind: "commit".to_owned(),
            identity: work.sha.clone(),
            location: work.source_url.clone(),
        }];
        evidence.extend(successful.iter().map(|execution| EvidenceRef {
            kind: execution.surface.clone(),
            identity: execution.command.clone(),
            location: execution.evidence_url.clone(),
        }));
        evidence.extend(
            work.artifacts
                .iter()
                .filter(|artifact| artifact.source_sha == work.sha && artifact.digest.is_some())
                .map(|artifact| EvidenceRef {
                    kind: "artifact".to_owned(),
                    identity: artifact
                        .digest
                        .clone()
                        .unwrap_or_else(|| artifact.name.clone()),
                    location: artifact.evidence_url.clone(),
                }),
        );
        sort_dedup_evidence(&mut evidence);
        let corroborated = successful.len() >= 2
            || work
                .artifacts
                .iter()
                .any(|artifact| artifact.source_sha == work.sha && artifact.digest.is_some());
        admitted.push(AdmittedWork {
            identity,
            level: if corroborated {
                AdmissionLevel::ResultCorroborated
            } else {
                AdmissionLevel::ExecutionObserved
            },
            work: work.clone(),
            evidence,
        });
    }

    admitted.sort_by(|left, right| left.identity.cmp(&right.identity));
    excluded.sort_by(|left, right| left.identity.cmp(&right.identity));
    AdmissionReport {
        schema: ADMISSION_SCHEMA.to_owned(),
        admitted,
        excluded,
        unresolved_transport_failures: pack.transport_failures.clone(),
    }
}

fn probability_refusal(work: &ObservedWorkItem) -> Option<(RefusalCode, String)> {
    let mut mass = 0.0;
    let mut labels = BTreeSet::new();
    for outcome in &work.probabilistic_outcomes {
        if outcome.evidence.is_empty() {
            return Some((
                RefusalCode::ProbabilityEvidenceMissing,
                format!("probability for outcome {} has no evidence", outcome.label),
            ));
        }
        if outcome.label.trim().is_empty() || !labels.insert(outcome.label.clone()) {
            return Some((
                RefusalCode::InvalidProbability,
                format!("outcome label is empty or duplicated: {}", outcome.label),
            ));
        }
        if !outcome.probability.is_finite()
            || outcome.probability <= 0.0
            || outcome.probability > 1.0
        {
            return Some((
                RefusalCode::InvalidProbability,
                format!(
                    "outcome {} carries invalid probability {}",
                    outcome.label, outcome.probability
                ),
            ));
        }
        mass += outcome.probability;
    }
    if mass > 1.0 + 1e-12 {
        return Some((
            RefusalCode::ProbabilityMassExceeded,
            format!("probability mass {mass} exceeds one"),
        ));
    }
    None
}

pub(crate) fn canonical_json<T: Serialize>(value: &T) -> Result<Vec<u8>> {
    let mut bytes = serde_json::to_vec_pretty(value)?;
    bytes.push(b'\n');
    Ok(bytes)
}

pub(crate) fn write_json<T: Serialize>(path: &Path, value: &T) -> Result<()> {
    write_atomic(path, &canonical_json(value)?)
}

pub(crate) fn write_atomic(path: &Path, bytes: &[u8]) -> Result<()> {
    let parent = path
        .parent()
        .filter(|candidate| !candidate.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."));
    fs::create_dir_all(parent)?;
    let temporary = path.with_extension("tmp-ferroplan-harvest");
    fs::write(&temporary, bytes)?;
    if path.exists() {
        fs::remove_file(path)?;
    }
    fs::rename(&temporary, path)?;
    Ok(())
}

pub fn digest_bytes(bytes: &[u8]) -> String {
    blake3::hash(bytes).to_hex().to_string()
}

fn is_exact_sha(value: &str) -> bool {
    value.len() == 40 && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}

pub(crate) fn work_identity(work: &ObservedWorkItem) -> String {
    format!("{}@{}", work.repository, work.sha)
}

pub(crate) fn sort_dedup_evidence(evidence: &mut Vec<EvidenceRef>) {
    evidence.sort();
    evidence.dedup();
}

pub(crate) fn slug(value: &str) -> String {
    let mut output = String::new();
    let mut dash = false;
    for ch in value.chars().flat_map(char::to_lowercase) {
        if ch.is_ascii_alphanumeric() {
            output.push(ch);
            dash = false;
        } else if !dash && !output.is_empty() {
            output.push('-');
            dash = true;
        }
        if output.len() >= 72 {
            break;
        }
    }
    while output.ends_with('-') {
        output.pop();
    }
    if output.is_empty() {
        "observed-work".to_owned()
    } else {
        output
    }
}

pub(crate) fn format_probability(value: f64) -> String {
    let mut rendered = format!("{value:.12}");
    while rendered.contains('.') && rendered.ends_with('0') {
        rendered.pop();
    }
    if rendered.ends_with('.') {
        rendered.pop();
    }
    rendered
}

pub(crate) fn seal_receipt(receipt: &mut HarvestReceipt) -> Result<()> {
    receipt.receipt_digest.clear();
    receipt.receipt_digest = digest_bytes(&canonical_json(receipt)?);
    Ok(())
}

pub(crate) fn verify_receipt_digest(receipt: &HarvestReceipt) -> Result<()> {
    let expected = receipt.receipt_digest.clone();
    let mut basis = receipt.clone();
    basis.receipt_digest.clear();
    let observed = digest_bytes(&canonical_json(&basis)?);
    if expected != observed {
        return Err(anyhow!(
            "receipt digest mismatch: expected {expected}, observed {observed}"
        ));
    }
    Ok(())
}

pub fn save_observation_pack(path: &Path, pack: &ObservationPack) -> Result<()> {
    validate_pack(pack)?;
    write_json(path, pack)
}

pub fn load_receipt(path: &Path) -> Result<HarvestReceipt> {
    let bytes = fs::read(path).with_context(|| format!("reading {}", path.display()))?;
    let receipt: HarvestReceipt =
        serde_json::from_slice(&bytes).with_context(|| format!("decoding {}", path.display()))?;
    if receipt.schema != RECEIPT_SCHEMA {
        return Err(anyhow!(
            "unsupported receipt schema {}; expected {RECEIPT_SCHEMA}",
            receipt.schema
        ));
    }
    verify_receipt_digest(&receipt)?;
    Ok(receipt)
}

pub fn receipt_exit_code(receipt: &HarvestReceipt) -> i32 {
    match receipt.final_state {
        FinalState::Alive | FinalState::PartialAlive => 0,
        FinalState::Blocked
        | FinalState::BuildBroken
        | FinalState::Unknown
        | FinalState::Unsupported => 1,
    }
}
