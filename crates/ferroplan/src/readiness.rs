//! Canonical capability contracts, bounded production execution, and
//! evidence-derived readiness.
//!
//! Nothing in this module grants execution authority. A successful planning
//! envelope is a candidate consequence. `ADMITTED` is computed only from an
//! exact-source evidence set; it is never stored in a producer-authored
//! capability declaration.

use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::api::{Options, Plan, Solution, SolveError};

pub const CAPABILITY_MANIFEST_SCHEMA: &str = "ferroplan.capabilities.v1";
pub const OPERATION_ENVELOPE_SCHEMA: &str = "ferroplan.operation.v1";
pub const CANDIDATE_AUTHORITY: &str = "candidate_only";

const MANIFEST_HASH_DOMAIN: &[u8] = b"ferroplan.capability-manifest.v1\0";
const INPUT_HASH_DOMAIN: &[u8] = b"ferroplan.production-input.v1\0";
const HARD_MAX_INPUT_BYTES: usize = 64 * 1024 * 1024;
const HARD_MAX_OUTPUT_BYTES: usize = 64 * 1024 * 1024;
const HARD_MAX_EVALUATED: usize = 50_000_000;
const HARD_MAX_PLAN_STEPS: usize = 100_000;
const HARD_MAX_WORKERS: usize = 64;
const MAX_REQUEST_ID_BYTES: usize = 128;
const MAX_WARNING_COUNT: usize = 32;
const MAX_DIAGNOSTIC_BYTES: usize = 2_048;

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "snake_case")]
pub enum InterfaceKind {
    RustLibrary,
    NativeCli,
    PythonAbi3,
    BrowserWasm,
    BevyGui,
    McpPlus,
    Plugin,
    Documentation,
    ReleasePipeline,
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "snake_case")]
pub enum AuthorityClass {
    CandidateOnly,
    EvidenceOnly,
    PresentationOnly,
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "snake_case")]
pub enum DeterminismClass {
    Exact,
    OutcomeEquivalent,
    NotApplicable,
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "snake_case")]
pub enum ReplayClass {
    Exact,
    Outcome,
    BuildReproducible,
    NotApplicable,
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "snake_case")]
pub enum CompatibilityClass {
    Semver,
    VersionedSchema,
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "snake_case")]
pub enum SecurityClass {
    UntrustedInput,
    LocalPresentation,
    BuildControl,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct CapabilityContract {
    pub id: String,
    pub version: String,
    pub owner: String,
    pub component: String,
    pub interface: InterfaceKind,
    pub authority: AuthorityClass,
    pub determinism: DeterminismClass,
    pub replay: ReplayClass,
    pub input_schema: String,
    pub output_schema: String,
    pub resource_profile: String,
    pub failure_contract: String,
    pub telemetry_contract: String,
    pub compatibility: CompatibilityClass,
    pub security: SecurityClass,
    pub shipped: bool,
    pub required_evidence: Vec<String>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct CapabilityManifest {
    pub schema_version: String,
    pub product_version: String,
    pub authority_notice: String,
    pub capabilities: Vec<CapabilityContract>,
}

#[derive(thiserror::Error, Clone, Debug, PartialEq, Eq)]
pub enum ManifestError {
    #[error("unsupported capability manifest schema `{0}`")]
    Schema(String),
    #[error("capability identifiers are not in canonical ascending order")]
    NonCanonicalOrder,
    #[error("duplicate capability id `{0}`")]
    DuplicateId(String),
    #[error("capability `{id}` is missing required field `{field}`")]
    MissingField { id: String, field: String },
    #[error("capability `{0}` has no executable evidence requirements")]
    MissingEvidence(String),
    #[error("capability `{0}` has duplicate or non-canonical evidence identifiers")]
    NonCanonicalEvidence(String),
    #[error("source identity must be non-empty")]
    MissingSourceIdentity,
    #[error("canonical manifest serialization failed")]
    Serialization,
}

impl CapabilityManifest {
    pub fn validate(&self) -> Result<(), ManifestError> {
        if self.schema_version != CAPABILITY_MANIFEST_SCHEMA {
            return Err(ManifestError::Schema(self.schema_version.clone()));
        }
        let ids: Vec<&str> = self.capabilities.iter().map(|item| item.id.as_str()).collect();
        if ids.windows(2).any(|window| window[0] >= window[1]) {
            return Err(ManifestError::NonCanonicalOrder);
        }
        let mut seen = BTreeSet::new();
        for capability in &self.capabilities {
            if !seen.insert(capability.id.clone()) {
                return Err(ManifestError::DuplicateId(capability.id.clone()));
            }
            for (field, value) in [
                ("id", capability.id.as_str()),
                ("version", capability.version.as_str()),
                ("owner", capability.owner.as_str()),
                ("component", capability.component.as_str()),
                ("input_schema", capability.input_schema.as_str()),
                ("output_schema", capability.output_schema.as_str()),
                ("resource_profile", capability.resource_profile.as_str()),
                ("failure_contract", capability.failure_contract.as_str()),
                ("telemetry_contract", capability.telemetry_contract.as_str()),
            ] {
                if value.trim().is_empty() {
                    return Err(ManifestError::MissingField {
                        id: capability.id.clone(),
                        field: field.to_string(),
                    });
                }
            }
            if capability.required_evidence.is_empty() {
                return Err(ManifestError::MissingEvidence(capability.id.clone()));
            }
            if capability
                .required_evidence
                .windows(2)
                .any(|window| window[0] >= window[1])
            {
                return Err(ManifestError::NonCanonicalEvidence(capability.id.clone()));
            }
        }
        Ok(())
    }

    pub fn fingerprint(&self) -> Result<String, ManifestError> {
        self.validate()?;
        let bytes = serde_json::to_vec(self).map_err(|_| ManifestError::Serialization)?;
        Ok(sha256_hex(MANIFEST_HASH_DOMAIN, &[&bytes]))
    }
}

fn contract(
    id: &str,
    component: &str,
    interface: InterfaceKind,
    authority: AuthorityClass,
    determinism: DeterminismClass,
    replay: ReplayClass,
    security: SecurityClass,
    evidence: &[&str],
) -> CapabilityContract {
    let mut evidence: Vec<String> = evidence.iter().map(|value| (*value).to_string()).collect();
    evidence.sort();
    CapabilityContract {
        id: id.to_string(),
        version: env!("CARGO_PKG_VERSION").to_string(),
        owner: "ferroplan-maintainers".to_string(),
        component: component.to_string(),
        interface,
        authority,
        determinism,
        replay,
        input_schema: format!("{id}.input.v1"),
        output_schema: format!("{id}.output.v1"),
        resource_profile: "ferroplan.bounded-production.v1".to_string(),
        failure_contract: "ferroplan.public-errors.v1".to_string(),
        telemetry_contract: "ferroplan.redacted-events.v1".to_string(),
        compatibility: if interface == InterfaceKind::RustLibrary {
            CompatibilityClass::Semver
        } else {
            CompatibilityClass::VersionedSchema
        },
        security,
        shipped: true,
        required_evidence: evidence,
    }
}

/// Canonical inventory for every default production capability family.
/// Experimental internals and the broad legacy MCP compatibility binary are
/// intentionally not advertised as production capabilities.
pub fn capability_manifest() -> CapabilityManifest {
    let mut capabilities = vec![
        contract(
            "fp.bevy",
            "crates/ferroplan-bevy",
            InterfaceKind::BevyGui,
            AuthorityClass::PresentationOnly,
            DeterminismClass::OutcomeEquivalent,
            ReplayClass::Outcome,
            SecurityClass::LocalPresentation,
            &["bevy.input.bounds", "bevy.native.build", "bevy.wasm.build"],
        ),
        contract(
            "fp.cli",
            "crates/ferroplan-cli",
            InterfaceKind::NativeCli,
            AuthorityClass::CandidateOnly,
            DeterminismClass::Exact,
            ReplayClass::Exact,
            SecurityClass::UntrustedInput,
            &["cli.contract", "cli.exit-codes", "cli.input.bounds", "cli.replay"],
        ),
        contract(
            "fp.core.decompose",
            "crates/ferroplan",
            InterfaceKind::RustLibrary,
            AuthorityClass::CandidateOnly,
            DeterminismClass::Exact,
            ReplayClass::Exact,
            SecurityClass::UntrustedInput,
            &[
                "core.decompose.bounds",
                "core.decompose.unit",
                "core.decompose.validation",
            ],
        ),
        contract(
            "fp.core.explain",
            "crates/ferroplan",
            InterfaceKind::RustLibrary,
            AuthorityClass::EvidenceOnly,
            DeterminismClass::Exact,
            ReplayClass::Exact,
            SecurityClass::UntrustedInput,
            &[
                "core.explain.negative",
                "core.explain.replay",
                "core.explain.unit",
            ],
        ),
        contract(
            "fp.core.fingerprint",
            "crates/ferroplan",
            InterfaceKind::RustLibrary,
            AuthorityClass::EvidenceOnly,
            DeterminismClass::Exact,
            ReplayClass::Exact,
            SecurityClass::UntrustedInput,
            &[
                "core.fingerprint.domain-separated",
                "core.fingerprint.replay",
            ],
        ),
        contract(
            "fp.core.parallel",
            "crates/ferroplan",
            InterfaceKind::RustLibrary,
            AuthorityClass::CandidateOnly,
            DeterminismClass::Exact,
            ReplayClass::Exact,
            SecurityClass::UntrustedInput,
            &[
                "core.parallel.bounds",
                "core.parallel.thread-parity",
                "core.parallel.unit",
            ],
        ),
        contract(
            "fp.core.parse",
            "crates/ferroplan",
            InterfaceKind::RustLibrary,
            AuthorityClass::EvidenceOnly,
            DeterminismClass::Exact,
            ReplayClass::Exact,
            SecurityClass::UntrustedInput,
            &["core.parse.bounds", "core.parse.negative", "core.parse.unit"],
        ),
        contract(
            "fp.core.ppddl",
            "crates/ferroplan",
            InterfaceKind::RustLibrary,
            AuthorityClass::CandidateOnly,
            DeterminismClass::Exact,
            ReplayClass::Exact,
            SecurityClass::UntrustedInput,
            &[
                "core.ppddl.policy-validation",
                "core.ppddl.replay",
                "core.ppddl.unit",
            ],
        ),
        contract(
            "fp.core.session",
            "crates/ferroplan",
            InterfaceKind::RustLibrary,
            AuthorityClass::CandidateOnly,
            DeterminismClass::OutcomeEquivalent,
            ReplayClass::Outcome,
            SecurityClass::UntrustedInput,
            &[
                "core.session.budget",
                "core.session.replay",
                "core.session.unit",
            ],
        ),
        contract(
            "fp.core.solve",
            "crates/ferroplan",
            InterfaceKind::RustLibrary,
            AuthorityClass::CandidateOnly,
            DeterminismClass::Exact,
            ReplayClass::Exact,
            SecurityClass::UntrustedInput,
            &[
                "core.solve.bounds",
                "core.solve.independent-validation",
                "core.solve.negative",
                "core.solve.replay",
                "core.solve.unit",
            ],
        ),
        contract(
            "fp.core.stream",
            "crates/ferroplan",
            InterfaceKind::RustLibrary,
            AuthorityClass::CandidateOnly,
            DeterminismClass::OutcomeEquivalent,
            ReplayClass::Outcome,
            SecurityClass::UntrustedInput,
            &["core.stream.bounds", "core.stream.terminal", "core.stream.unit"],
        ),
        contract(
            "fp.core.trace",
            "crates/ferroplan",
            InterfaceKind::RustLibrary,
            AuthorityClass::EvidenceOnly,
            DeterminismClass::Exact,
            ReplayClass::Exact,
            SecurityClass::UntrustedInput,
            &["core.trace.negative", "core.trace.replay", "core.trace.unit"],
        ),
        contract(
            "fp.core.validate",
            "crates/ferroplan",
            InterfaceKind::RustLibrary,
            AuthorityClass::EvidenceOnly,
            DeterminismClass::Exact,
            ReplayClass::Exact,
            SecurityClass::UntrustedInput,
            &[
                "core.validate.negative",
                "core.validate.replay",
                "core.validate.unit",
            ],
        ),
        contract(
            "fp.docs",
            "book",
            InterfaceKind::Documentation,
            AuthorityClass::PresentationOnly,
            DeterminismClass::NotApplicable,
            ReplayClass::BuildReproducible,
            SecurityClass::LocalPresentation,
            &["docs.capability-truth", "docs.mdbook", "docs.rustdoc"],
        ),
        contract(
            "fp.eve.enter",
            "crates/ferroplan",
            InterfaceKind::RustLibrary,
            AuthorityClass::CandidateOnly,
            DeterminismClass::Exact,
            ReplayClass::Exact,
            SecurityClass::UntrustedInput,
            &["eve.authority", "eve.need9-split", "eve.replay", "eve.unit"],
        ),
        contract(
            "fp.mcpplus",
            "crates/ferroplan-mcp",
            InterfaceKind::McpPlus,
            AuthorityClass::CandidateOnly,
            DeterminismClass::OutcomeEquivalent,
            ReplayClass::Outcome,
            SecurityClass::UntrustedInput,
            &[
                "mcp.candidate-authority",
                "mcp.concurrency-bounds",
                "mcp.frame-bounds",
                "mcp.integration",
                "mcp.protocol",
            ],
        ),
        contract(
            "fp.plugin.chatman",
            "plugins/chatman-ecosystem",
            InterfaceKind::Plugin,
            AuthorityClass::CandidateOnly,
            DeterminismClass::OutcomeEquivalent,
            ReplayClass::Outcome,
            SecurityClass::UntrustedInput,
            &[
                "plugin.generated-current",
                "plugin.lint",
                "plugin.tests",
                "plugin.verifier",
            ],
        ),
        contract(
            "fp.python",
            "crates/ferroplan-py",
            InterfaceKind::PythonAbi3,
            AuthorityClass::CandidateOnly,
            DeterminismClass::Exact,
            ReplayClass::Exact,
            SecurityClass::UntrustedInput,
            &["python.bounds", "python.import", "python.parity", "python.wheel"],
        ),
        contract(
            "fp.release",
            ".github/workflows",
            InterfaceKind::ReleasePipeline,
            AuthorityClass::EvidenceOnly,
            DeterminismClass::NotApplicable,
            ReplayClass::BuildReproducible,
            SecurityClass::BuildControl,
            &[
                "release.admission-report",
                "release.checksums",
                "release.dependency-audit",
                "release.full-matrix",
                "release.license-inventory",
                "release.sbom",
                "release.source-identity",
            ],
        ),
        contract(
            "fp.wasm",
            "crates/ferroplan-wasm",
            InterfaceKind::BrowserWasm,
            AuthorityClass::CandidateOnly,
            DeterminismClass::Exact,
            ReplayClass::Exact,
            SecurityClass::UntrustedInput,
            &[
                "wasm.bounds",
                "wasm.build",
                "wasm.no-ambient-authority",
                "wasm.parity",
            ],
        ),
    ];
    capabilities.sort_by(|left, right| left.id.cmp(&right.id));
    CapabilityManifest {
        schema_version: CAPABILITY_MANIFEST_SCHEMA.to_string(),
        product_version: env!("CARGO_PKG_VERSION").to_string(),
        authority_notice: "All planner outputs are candidate-only; external BRCE/POWL/OCEL/Truex receipt closure owns authoritative consequence.".to_string(),
        capabilities,
    }
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "snake_case")]
pub enum ReadinessState {
    Unknown,
    Declared,
    Partial,
    Admitted,
    Blocked,
    Unsupported,
    Refused,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct CapabilityEvaluation {
    pub capability_id: String,
    pub state: ReadinessState,
    pub satisfied_evidence: Vec<String>,
    pub missing_evidence: Vec<String>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct ReadinessReport {
    pub schema_version: String,
    pub product_version: String,
    pub source_identity: String,
    pub manifest_fingerprint: String,
    pub evaluator_version: String,
    pub overall_state: ReadinessState,
    pub capabilities: Vec<CapabilityEvaluation>,
}

pub fn evaluate_readiness<I, S>(
    source_identity: impl Into<String>,
    evidence: I,
) -> Result<ReadinessReport, ManifestError>
where
    I: IntoIterator<Item = S>,
    S: Into<String>,
{
    let source_identity = source_identity.into();
    if source_identity.trim().is_empty() {
        return Err(ManifestError::MissingSourceIdentity);
    }
    let manifest = capability_manifest();
    manifest.validate()?;
    let evidence: BTreeSet<String> = evidence.into_iter().map(Into::into).collect();
    let capabilities = manifest
        .capabilities
        .iter()
        .map(|contract| {
            let (satisfied_evidence, missing_evidence): (Vec<_>, Vec<_>) = contract
                .required_evidence
                .iter()
                .cloned()
                .partition(|requirement| evidence.contains(requirement));
            let state = if !contract.shipped {
                ReadinessState::Blocked
            } else if missing_evidence.is_empty() {
                ReadinessState::Admitted
            } else if satisfied_evidence.is_empty() {
                ReadinessState::Declared
            } else {
                ReadinessState::Partial
            };
            CapabilityEvaluation {
                capability_id: contract.id.clone(),
                state,
                satisfied_evidence,
                missing_evidence,
            }
        })
        .collect::<Vec<_>>();
    let overall_state = if capabilities
        .iter()
        .all(|item| item.state == ReadinessState::Admitted)
    {
        ReadinessState::Admitted
    } else if capabilities.iter().any(|item| {
        matches!(item.state, ReadinessState::Blocked | ReadinessState::Refused)
    }) {
        ReadinessState::Blocked
    } else if capabilities
        .iter()
        .any(|item| item.state == ReadinessState::Partial)
    {
        ReadinessState::Partial
    } else {
        ReadinessState::Declared
    };
    Ok(ReadinessReport {
        schema_version: "ferroplan.readiness-report.v1".to_string(),
        product_version: manifest.product_version.clone(),
        source_identity,
        manifest_fingerprint: manifest.fingerprint()?,
        evaluator_version: env!("CARGO_PKG_VERSION").to_string(),
        overall_state,
        capabilities,
    })
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct ProductionLimits {
    pub max_domain_bytes: usize,
    pub max_problem_bytes: usize,
    pub max_evaluated: usize,
    pub max_plan_steps: usize,
    pub max_output_bytes: usize,
    pub max_workers: usize,
}

impl Default for ProductionLimits {
    fn default() -> Self {
        Self {
            max_domain_bytes: 4 * 1024 * 1024,
            max_problem_bytes: 4 * 1024 * 1024,
            max_evaluated: 1_000_000,
            max_plan_steps: 10_000,
            max_output_bytes: 16 * 1024 * 1024,
            max_workers: 8,
        }
    }
}

impl ProductionLimits {
    pub fn validate(&self) -> Result<(), PublicError> {
        for (name, value, hard) in [
            ("max_domain_bytes", self.max_domain_bytes, HARD_MAX_INPUT_BYTES),
            ("max_problem_bytes", self.max_problem_bytes, HARD_MAX_INPUT_BYTES),
            ("max_evaluated", self.max_evaluated, HARD_MAX_EVALUATED),
            ("max_plan_steps", self.max_plan_steps, HARD_MAX_PLAN_STEPS),
            ("max_output_bytes", self.max_output_bytes, HARD_MAX_OUTPUT_BYTES),
            ("max_workers", self.max_workers, HARD_MAX_WORKERS),
        ] {
            if value == 0 || value > hard {
                return Err(PublicError::new(
                    "FP_INVALID_REQUEST",
                    format!("{name} must be in 1..={hard}"),
                    false,
                ));
            }
        }
        Ok(())
    }
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum OutcomeClass {
    Solved,
    NoPlan,
    LimitExceeded,
    Refused,
    Failed,
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ValidationStatus {
    Valid,
    NotApplicable,
    Failed,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct PublicError {
    pub code: String,
    pub message: String,
    pub retryable: bool,
}

impl PublicError {
    pub fn new(code: impl Into<String>, message: impl Into<String>, retryable: bool) -> Self {
        Self {
            code: code.into(),
            message: truncate_utf8(&message.into(), MAX_DIAGNOSTIC_BYTES),
            retryable,
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct BuildIdentity {
    pub product_version: String,
    pub source_revision: Option<String>,
    pub manifest_fingerprint: Option<String>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct OperationEnvelope<T> {
    pub schema_version: String,
    pub request_id: String,
    pub capability_id: String,
    pub capability_version: String,
    pub build_identity: BuildIdentity,
    pub input_fingerprint: String,
    pub authority: String,
    pub outcome: OutcomeClass,
    pub validation: ValidationStatus,
    pub elapsed_micros: u64,
    pub counters: BTreeMap<String, u64>,
    pub warnings: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub payload: Option<T>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<PublicError>,
}

/// Bounded deterministic planning with typed failure semantics and independent
/// validation. Hard wall-clock isolation belongs to service adapters such as
/// `ferroplan-mcp-plus`; this in-process API is bounded by deterministic work,
/// worker, plan, input, and output limits.
pub fn solve_production(
    domain: &str,
    problem: &str,
    options: &Options,
    limits: &ProductionLimits,
    request_id: Option<&str>,
) -> OperationEnvelope<Solution> {
    let clock = crate::clock::Clock::now();
    let input_fingerprint = production_input_fingerprint(domain, problem, options);
    let request_id = normalize_request_id(request_id, &input_fingerprint);
    let manifest_fingerprint = capability_manifest().fingerprint();
    let mut envelope = base_envelope(
        request_id,
        input_fingerprint,
        elapsed(&clock),
        manifest_fingerprint.clone().ok(),
    );
    if let Err(error) = manifest_fingerprint {
        envelope.error = Some(PublicError::new("FP_INVARIANT", error.to_string(), false));
        return envelope;
    }
    if let Err(error) = limits.validate() {
        envelope.outcome = OutcomeClass::Refused;
        envelope.error = Some(error);
        return envelope;
    }
    if let Some(error) = validate_request(domain, problem, options, limits) {
        envelope.outcome = OutcomeClass::Refused;
        envelope.error = Some(error);
        return envelope;
    }

    let effective_max = options
        .max_evaluated
        .unwrap_or(limits.max_evaluated)
        .min(limits.max_evaluated);
    let mut bounded_options = options.clone();
    bounded_options.threads = bounded_options.threads.max(1);
    bounded_options.max_evaluated = Some(effective_max);

    match crate::api::solve(domain, problem, &bounded_options) {
        Err(error) => {
            envelope.outcome = OutcomeClass::Refused;
            envelope.error = Some(map_solve_error(error));
        }
        Ok(solution) => {
            envelope.counters.insert(
                "grounded_facts".to_string(),
                saturating_u64(solution.statistics.grounded_facts),
            );
            envelope.counters.insert(
                "grounded_actions".to_string(),
                saturating_u64(solution.statistics.grounded_actions),
            );
            envelope.counters.insert(
                "evaluated_states".to_string(),
                saturating_u64(solution.statistics.evaluated_states),
            );
            envelope.counters.insert(
                "threads".to_string(),
                saturating_u64(solution.statistics.threads),
            );
            envelope.warnings = bounded_warnings(&solution.notes);

            if !solution.solved {
                let capped = solution.statistics.evaluated_states >= effective_max;
                envelope.outcome = if capped {
                    OutcomeClass::LimitExceeded
                } else {
                    OutcomeClass::NoPlan
                };
                if capped {
                    envelope.error = Some(PublicError::new(
                        "FP_LIMIT_SEARCH",
                        format!("search reached max_evaluated={effective_max}"),
                        true,
                    ));
                }
                envelope.payload = Some(solution);
            } else {
                admit_solved_candidate(domain, problem, solution, limits, &mut envelope);
            }
        }
    }
    envelope.elapsed_micros = elapsed(&clock);
    enforce_output_limit(&mut envelope, limits.max_output_bytes);
    envelope
}

fn admit_solved_candidate(
    domain: &str,
    problem: &str,
    solution: Solution,
    limits: &ProductionLimits,
    envelope: &mut OperationEnvelope<Solution>,
) {
    let Some(plan) = solution.plan.as_ref() else {
        envelope.outcome = OutcomeClass::Failed;
        envelope.validation = ValidationStatus::Failed;
        envelope.error = Some(PublicError::new(
            "FP_INVARIANT",
            "solver reported solved=true without a plan",
            false,
        ));
        return;
    };
    if plan.length != plan.steps.len() {
        envelope.outcome = OutcomeClass::Failed;
        envelope.validation = ValidationStatus::Failed;
        envelope.error = Some(PublicError::new(
            "FP_INVARIANT",
            "plan length does not match emitted step count",
            false,
        ));
        return;
    }
    if plan.length > limits.max_plan_steps {
        envelope.outcome = OutcomeClass::LimitExceeded;
        envelope.error = Some(PublicError::new(
            "FP_LIMIT_PLAN",
            format!(
                "plan contains {} steps; maximum is {}",
                plan.length, limits.max_plan_steps
            ),
            true,
        ));
        return;
    }
    match independently_validate(domain, problem, plan) {
        Ok(ValidationStatus::Valid) => {
            envelope.outcome = OutcomeClass::Solved;
            envelope.validation = ValidationStatus::Valid;
            envelope.payload = Some(solution);
        }
        Ok(ValidationStatus::NotApplicable) => {
            envelope.outcome = OutcomeClass::Failed;
            envelope.validation = ValidationStatus::Failed;
            envelope.error = Some(PublicError::new(
                "FP_VALIDATION",
                "independent validation was unexpectedly unavailable",
                false,
            ));
        }
        Ok(ValidationStatus::Failed) | Err(_) => {
            envelope.outcome = OutcomeClass::Failed;
            envelope.validation = ValidationStatus::Failed;
            envelope.error = Some(PublicError::new(
                "FP_VALIDATION",
                "emitted plan failed independent validation",
                false,
            ));
        }
    }
}

fn validate_request(
    domain: &str,
    problem: &str,
    options: &Options,
    limits: &ProductionLimits,
) -> Option<PublicError> {
    if domain.is_empty() || problem.is_empty() {
        return Some(PublicError::new(
            "FP_INVALID_REQUEST",
            "domain and problem must both be non-empty",
            false,
        ));
    }
    if domain.len() > limits.max_domain_bytes {
        return Some(PublicError::new(
            "FP_LIMIT_INPUT",
            format!(
                "domain is {} bytes; maximum is {}",
                domain.len(), limits.max_domain_bytes
            ),
            false,
        ));
    }
    if problem.len() > limits.max_problem_bytes {
        return Some(PublicError::new(
            "FP_LIMIT_INPUT",
            format!(
                "problem is {} bytes; maximum is {}",
                problem.len(), limits.max_problem_bytes
            ),
            false,
        ));
    }
    if !options.weight_g.is_finite()
        || !options.weight_h.is_finite()
        || options.weight_g < 0.0
        || options.weight_h < 0.0
    {
        return Some(PublicError::new(
            "FP_INVALID_REQUEST",
            "search weights must be finite and non-negative",
            false,
        ));
    }
    if options.threads > limits.max_workers {
        return Some(PublicError::new(
            "FP_LIMIT_WORKERS",
            format!(
                "requested {} workers; maximum is {}",
                options.threads, limits.max_workers
            ),
            false,
        ));
    }
    if options
        .max_evaluated
        .is_some_and(|requested| requested == 0 || requested > limits.max_evaluated)
    {
        return Some(PublicError::new(
            "FP_LIMIT_SEARCH",
            format!("max_evaluated must be in 1..={}", limits.max_evaluated),
            false,
        ));
    }
    None
}

fn map_solve_error(error: SolveError) -> PublicError {
    match error {
        SolveError::DomainParse(error) => {
            PublicError::new("FP_PARSE", format!("domain parse error: {error}"), false)
        }
        SolveError::ProblemParse(error) => {
            PublicError::new("FP_PARSE", format!("problem parse error: {error}"), false)
        }
        SolveError::Unsupported(message) => PublicError::new("FP_UNSUPPORTED", message, false),
        SolveError::EmptyType { kind, pred, ty } => PublicError::new(
            "FP_MODEL",
            format!("{kind} {pred} uses an unknown or empty type {ty}"),
            false,
        ),
        SolveError::Derived(message) => PublicError::new("FP_MODEL", message, false),
    }
}

fn independently_validate(
    domain: &str,
    problem: &str,
    plan: &Plan,
) -> Result<ValidationStatus, String> {
    let plan_text = render_plan(plan);
    match crate::plan::validate_plan(domain, problem, &plan_text)? {
        crate::plan::Validity::Valid => Ok(ValidationStatus::Valid),
        crate::plan::Validity::Invalid(_) => Ok(ValidationStatus::Failed),
    }
}

fn render_plan(plan: &Plan) -> String {
    let temporal = plan.steps.iter().any(|step| step.time.is_some());
    let mut output = String::new();
    for step in &plan.steps {
        let args = if step.args.is_empty() {
            String::new()
        } else {
            format!(" {}", step.args.join(" "))
        };
        if temporal {
            output.push_str(&format!(
                "{:.6}: ({}{args}) [{:.6}]\n",
                step.time.unwrap_or(0.0),
                step.action,
                step.duration.unwrap_or(0.0)
            ));
        } else {
            output.push_str(&format!("step {}: {}{args}\n", step.index, step.action));
        }
    }
    output
}

pub fn production_input_fingerprint(domain: &str, problem: &str, options: &Options) -> String {
    let options_bytes = serde_json::to_vec(options)
        .unwrap_or_else(|_| format!("{options:?}").into_bytes());
    sha256_hex(
        INPUT_HASH_DOMAIN,
        &[domain.as_bytes(), problem.as_bytes(), &options_bytes],
    )
}

fn build_identity(manifest_fingerprint: Option<String>) -> BuildIdentity {
    BuildIdentity {
        product_version: env!("CARGO_PKG_VERSION").to_string(),
        source_revision: option_env!("FERROPLAN_BUILD_SHA")
            .filter(|value| !value.trim().is_empty())
            .map(ToOwned::to_owned),
        manifest_fingerprint,
    }
}

fn base_envelope<T>(
    request_id: String,
    input_fingerprint: String,
    elapsed_micros: u64,
    manifest_fingerprint: Option<String>,
) -> OperationEnvelope<T> {
    OperationEnvelope {
        schema_version: OPERATION_ENVELOPE_SCHEMA.to_string(),
        request_id,
        capability_id: "fp.core.solve".to_string(),
        capability_version: env!("CARGO_PKG_VERSION").to_string(),
        build_identity: build_identity(manifest_fingerprint),
        input_fingerprint,
        authority: CANDIDATE_AUTHORITY.to_string(),
        outcome: OutcomeClass::Failed,
        validation: ValidationStatus::NotApplicable,
        elapsed_micros,
        counters: BTreeMap::new(),
        warnings: Vec::new(),
        payload: None,
        error: None,
    }
}

fn enforce_output_limit<T: Serialize>(envelope: &mut OperationEnvelope<T>, max_bytes: usize) {
    match serde_json::to_vec(envelope) {
        Ok(bytes) if bytes.len() <= max_bytes => {}
        Ok(bytes) => {
            envelope.payload = None;
            envelope.outcome = OutcomeClass::LimitExceeded;
            envelope.validation = ValidationStatus::NotApplicable;
            envelope.error = Some(PublicError::new(
                "FP_LIMIT_OUTPUT",
                format!(
                    "operation envelope is {} bytes; maximum is {max_bytes}",
                    bytes.len()
                ),
                true,
            ));
        }
        Err(_) => {
            envelope.payload = None;
            envelope.outcome = OutcomeClass::Failed;
            envelope.validation = ValidationStatus::Failed;
            envelope.error = Some(PublicError::new(
                "FP_ADAPTER",
                "operation envelope could not be serialized",
                false,
            ));
        }
    }
}

fn sha256_hex(domain: &[u8], parts: &[&[u8]]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(domain);
    for part in parts {
        hasher.update((part.len() as u64).to_be_bytes());
        hasher.update(part);
    }
    format!("{:x}", hasher.finalize())
}

fn normalize_request_id(request_id: Option<&str>, fingerprint: &str) -> String {
    request_id
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(|value| truncate_utf8(value, MAX_REQUEST_ID_BYTES))
        .unwrap_or_else(|| format!("req-{}", &fingerprint[..16.min(fingerprint.len())]))
}

fn bounded_warnings(warnings: &[String]) -> Vec<String> {
    warnings
        .iter()
        .take(MAX_WARNING_COUNT)
        .map(|warning| truncate_utf8(warning, MAX_DIAGNOSTIC_BYTES))
        .collect()
}

fn truncate_utf8(value: &str, max_bytes: usize) -> String {
    if value.len() <= max_bytes {
        return value.to_string();
    }
    let mut end = max_bytes;
    while end > 0 && !value.is_char_boundary(end) {
        end -= 1;
    }
    value[..end].to_string()
}

fn elapsed(clock: &crate::clock::Clock) -> u64 {
    clock.elapsed_us().min(u64::MAX as u128) as u64
}

fn saturating_u64(value: usize) -> u64 {
    value.try_into().unwrap_or(u64::MAX)
}

#[cfg(test)]
mod tests {
    use super::*;

    const DOMAIN: &str = "(define (domain smoke) (:requirements :strips) \
        (:predicates (done)) (:action finish :parameters () \
        :precondition (and) :effect (done)))";
    const PROBLEM: &str = "(define (problem smoke-p) (:domain smoke) \
        (:init) (:goal (done)))";

    #[test]
    fn manifest_is_canonical_complete_and_domain_separated() {
        let manifest = capability_manifest();
        manifest.validate().unwrap();
        let fingerprint = manifest.fingerprint().unwrap();
        assert_eq!(fingerprint, capability_manifest().fingerprint().unwrap());
        assert_eq!(fingerprint.len(), 64);
        assert_ne!(
            fingerprint,
            production_input_fingerprint(DOMAIN, PROBLEM, &Options::default())
        );
        let ids: BTreeSet<_> = manifest
            .capabilities
            .into_iter()
            .map(|item| item.id)
            .collect();
        for expected in [
            "fp.core.solve",
            "fp.core.parse",
            "fp.core.ppddl",
            "fp.core.session",
            "fp.core.decompose",
            "fp.eve.enter",
            "fp.cli",
            "fp.python",
            "fp.wasm",
            "fp.bevy",
            "fp.mcpplus",
            "fp.plugin.chatman",
            "fp.docs",
            "fp.release",
        ] {
            assert!(ids.contains(expected), "missing {expected}");
        }
    }

    #[test]
    fn admission_is_computed_from_complete_evidence() {
        let declared = evaluate_readiness("test-source", Vec::<String>::new()).unwrap();
        assert_eq!(declared.overall_state, ReadinessState::Declared);
        let all = capability_manifest()
            .capabilities
            .iter()
            .flat_map(|contract| contract.required_evidence.clone())
            .collect::<Vec<_>>();
        let admitted = evaluate_readiness("test-source", all).unwrap();
        assert_eq!(admitted.overall_state, ReadinessState::Admitted);
        assert!(admitted
            .capabilities
            .iter()
            .all(|item| item.state == ReadinessState::Admitted));
    }

    #[test]
    fn partial_evidence_cannot_be_crowned() {
        let report = evaluate_readiness("test-source", ["core.solve.unit"]).unwrap();
        assert_eq!(report.overall_state, ReadinessState::Partial);
        let solve = report
            .capabilities
            .iter()
            .find(|item| item.capability_id == "fp.core.solve")
            .unwrap();
        assert_eq!(solve.state, ReadinessState::Partial);
        assert!(!solve.missing_evidence.is_empty());
    }

    #[test]
    fn production_solve_is_bounded_validated_and_candidate_only() {
        let envelope = solve_production(
            DOMAIN,
            PROBLEM,
            &Options::default(),
            &ProductionLimits::default(),
            Some("smoke-request"),
        );
        assert_eq!(envelope.outcome, OutcomeClass::Solved);
        assert_eq!(envelope.validation, ValidationStatus::Valid);
        assert_eq!(envelope.authority, CANDIDATE_AUTHORITY);
        assert_eq!(envelope.request_id, "smoke-request");
        assert!(envelope.payload.as_ref().is_some_and(|value| value.solved));
        assert!(envelope.build_identity.manifest_fingerprint.is_some());
    }

    #[test]
    fn empty_plan_is_independently_validated() {
        let problem = "(define (problem smoke-p) (:domain smoke) \
            (:init (done)) (:goal (done)))";
        let envelope = solve_production(
            DOMAIN,
            problem,
            &Options::default(),
            &ProductionLimits::default(),
            None,
        );
        assert_eq!(envelope.outcome, OutcomeClass::Solved);
        assert_eq!(envelope.validation, ValidationStatus::Valid);
        assert_eq!(
            envelope.payload.as_ref().and_then(|value| value.plan.as_ref()).map(|plan| plan.length),
            Some(0)
        );
    }

    #[test]
    fn production_solve_refuses_input_before_parse() {
        let limits = ProductionLimits {
            max_domain_bytes: 8,
            ..ProductionLimits::default()
        };
        let envelope = solve_production(DOMAIN, PROBLEM, &Options::default(), &limits, None);
        assert_eq!(envelope.outcome, OutcomeClass::Refused);
        assert_eq!(
            envelope.error.as_ref().map(|error| error.code.as_str()),
            Some("FP_LIMIT_INPUT")
        );
        assert!(envelope.payload.is_none());
    }

    #[test]
    fn invalid_floats_are_refused_and_remain_fingerprint_distinct() {
        let valid = production_input_fingerprint(DOMAIN, PROBLEM, &Options::default());
        let mut invalid = Options::default();
        invalid.weight_h = f64::NAN;
        let invalid_fingerprint = production_input_fingerprint(DOMAIN, PROBLEM, &invalid);
        assert_ne!(valid, invalid_fingerprint);
        let envelope = solve_production(
            DOMAIN,
            PROBLEM,
            &invalid,
            &ProductionLimits::default(),
            None,
        );
        assert_eq!(envelope.outcome, OutcomeClass::Refused);
        assert_eq!(
            envelope.error.as_ref().map(|error| error.code.as_str()),
            Some("FP_INVALID_REQUEST")
        );
    }

    #[test]
    fn option_changes_are_bound_into_input_identity() {
        let auto = production_input_fingerprint(DOMAIN, PROBLEM, &Options::default());
        let mut ff = Options::default();
        ff.mode = crate::Mode::Ff;
        assert_ne!(auto, production_input_fingerprint(DOMAIN, PROBLEM, &ff));
    }
}
