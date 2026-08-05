//! Fortune 5 capability contracts and bounded production execution.
//!
//! This module does not claim that a capability is production-admitted merely
//! because it is compiled. It provides the canonical contract inventory, an
//! evidence-driven evaluator, and the bounded solve envelope used by public
//! adapters.

use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::api::{Mode, Options, Plan, Solution, SolveError};

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
const MAX_WARNING_BYTES: usize = 1_024;

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
    Internal,
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
    #[error("capability `{0}` grants an unsupported authority class")]
    InvalidAuthority(String),
}

impl CapabilityManifest {
    pub fn validate(&self) -> Result<(), ManifestError> {
        if self.schema_version != CAPABILITY_MANIFEST_SCHEMA {
            return Err(ManifestError::Schema(self.schema_version.clone()));
        }
        let ids: Vec<&str> = self.capabilities.iter().map(|c| c.id.as_str()).collect();
        if ids.windows(2).any(|w| w[0] >= w[1]) {
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
            if !matches!(
                capability.authority,
                AuthorityClass::CandidateOnly
                    | AuthorityClass::EvidenceOnly
                    | AuthorityClass::PresentationOnly
            ) {
                return Err(ManifestError::InvalidAuthority(capability.id.clone()));
            }
        }
        Ok(())
    }

    pub fn fingerprint(&self) -> Result<String, ManifestError> {
        self.validate()?;
        let bytes = serde_json::to_vec(self).map_err(|_| ManifestError::MissingField {
            id: "manifest".to_string(),
            field: "serializable canonical content".to_string(),
        })?;
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
        compatibility: if matches!(interface, InterfaceKind::RustLibrary) {
            CompatibilityClass::Semver
        } else {
            CompatibilityClass::VersionedSchema
        },
        security,
        shipped: true,
        required_evidence: evidence.iter().map(|s| (*s).to_string()).collect(),
    }
}

/// The canonical shipped capability inventory. Documentation and adapters must
/// project from this list rather than maintain independent capability claims.
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
            &["bevy.native.build", "bevy.wasm.build", "bevy.input.bounds"],
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
            "fp.core.explain",
            "crates/ferroplan",
            InterfaceKind::RustLibrary,
            AuthorityClass::EvidenceOnly,
            DeterminismClass::Exact,
            ReplayClass::Exact,
            SecurityClass::UntrustedInput,
            &["core.explain.unit", "core.explain.negative", "core.explain.replay"],
        ),
        contract(
            "fp.core.fingerprint",
            "crates/ferroplan",
            InterfaceKind::RustLibrary,
            AuthorityClass::EvidenceOnly,
            DeterminismClass::Exact,
            ReplayClass::Exact,
            SecurityClass::UntrustedInput,
            &["core.fingerprint.domain-separated", "core.fingerprint.replay"],
        ),
        contract(
            "fp.core.parallel",
            "crates/ferroplan",
            InterfaceKind::RustLibrary,
            AuthorityClass::CandidateOnly,
            DeterminismClass::Exact,
            ReplayClass::Exact,
            SecurityClass::UntrustedInput,
            &["core.parallel.unit", "core.parallel.thread-parity", "core.parallel.bounds"],
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
                "core.solve.unit",
                "core.solve.independent-validation",
                "core.solve.negative",
                "core.solve.bounds",
                "core.solve.replay",
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
            &["core.stream.unit", "core.stream.bounds", "core.stream.terminal"],
        ),
        contract(
            "fp.core.validate",
            "crates/ferroplan",
            InterfaceKind::RustLibrary,
            AuthorityClass::EvidenceOnly,
            DeterminismClass::Exact,
            ReplayClass::Exact,
            SecurityClass::UntrustedInput,
            &["core.validate.unit", "core.validate.negative", "core.validate.replay"],
        ),
        contract(
            "fp.docs",
            "book",
            InterfaceKind::Documentation,
            AuthorityClass::PresentationOnly,
            DeterminismClass::NotApplicable,
            ReplayClass::BuildReproducible,
            SecurityClass::LocalPresentation,
            &["docs.mdbook", "docs.rustdoc", "docs.capability-truth"],
        ),
        contract(
            "fp.eve.enter",
            "crates/ferroplan",
            InterfaceKind::RustLibrary,
            AuthorityClass::CandidateOnly,
            DeterminismClass::Exact,
            ReplayClass::Exact,
            SecurityClass::UntrustedInput,
            &["eve.unit", "eve.need9-split", "eve.authority", "eve.replay"],
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
                "mcp.protocol",
                "mcp.frame-bounds",
                "mcp.concurrency-bounds",
                "mcp.candidate-authority",
                "mcp.integration",
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
            &["plugin.lint", "plugin.generated-current", "plugin.tests", "plugin.verifier"],
        ),
        contract(
            "fp.python",
            "crates/ferroplan-py",
            InterfaceKind::PythonAbi3,
            AuthorityClass::CandidateOnly,
            DeterminismClass::Exact,
            ReplayClass::Exact,
            SecurityClass::UntrustedInput,
            &["python.wheel", "python.import", "python.bounds", "python.parity"],
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
                "release.full-matrix",
                "release.dependency-audit",
                "release.sbom",
                "release.checksums",
                "release.admission-report",
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
            &["wasm.build", "wasm.bounds", "wasm.parity", "wasm.no-ambient-authority"],
        ),
    ];
    capabilities.sort_by(|a, b| a.id.cmp(&b.id));
    CapabilityManifest {
        schema_version: CAPABILITY_MANIFEST_SCHEMA.to_string(),
        product_version: env!("CARGO_PKG_VERSION").to_string(),
        authority_notice: "All planner outputs are candidate-only; external BRCE/OCEL/Truex receipt closure owns authoritative consequence.".to_string(),
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
    pub overall_state: ReadinessState,
    pub capabilities: Vec<CapabilityEvaluation>,
}

/// Compute readiness from independently supplied evidence identifiers. The
/// capability producer cannot set `Admitted` in the manifest.
pub fn evaluate_readiness<I, S>(
    source_identity: impl Into<String>,
    evidence: I,
) -> Result<ReadinessReport, ManifestError>
where
    I: IntoIterator<Item = S>,
    S: Into<String>,
{
    let manifest = capability_manifest();
    manifest.validate()?;
    let evidence: BTreeSet<String> = evidence.into_iter().map(Into::into).collect();
    let mut evaluations = Vec::with_capacity(manifest.capabilities.len());
    for contract in &manifest.capabilities {
        let mut satisfied = Vec::new();
        let mut missing = Vec::new();
        for requirement in &contract.required_evidence {
            if evidence.contains(requirement) {
                satisfied.push(requirement.clone());
            } else {
                missing.push(requirement.clone());
            }
        }
        let state = if !contract.shipped {
            ReadinessState::Blocked
        } else if missing.is_empty() {
            ReadinessState::Admitted
        } else if satisfied.is_empty() {
            ReadinessState::Declared
        } else {
            ReadinessState::Partial
        };
        evaluations.push(CapabilityEvaluation {
            capability_id: contract.id.clone(),
            state,
            satisfied_evidence: satisfied,
            missing_evidence: missing,
        });
    }
    let overall_state = if evaluations
        .iter()
        .all(|evaluation| evaluation.state == ReadinessState::Admitted)
    {
        ReadinessState::Admitted
    } else if evaluations.iter().any(|evaluation| {
        matches!(
            evaluation.state,
            ReadinessState::Blocked | ReadinessState::Refused
        )
    }) {
        ReadinessState::Blocked
    } else if evaluations
        .iter()
        .any(|evaluation| evaluation.state == ReadinessState::Partial)
    {
        ReadinessState::Partial
    } else {
        ReadinessState::Declared
    };
    Ok(ReadinessReport {
        schema_version: "ferroplan.readiness-report.v1".to_string(),
        product_version: manifest.product_version.clone(),
        source_identity: source_identity.into(),
        manifest_fingerprint: manifest.fingerprint()?,
        overall_state,
        capabilities: evaluations,
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
            message: truncate_utf8(&message.into(), MAX_WARNING_BYTES),
            retryable,
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct BuildIdentity {
    pub product_version: String,
    pub source_revision: String,
    pub manifest_fingerprint: String,
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

fn build_identity() -> BuildIdentity {
    let manifest = capability_manifest();
    BuildIdentity {
        product_version: env!("CARGO_PKG_VERSION").to_string(),
        source_revision: option_env!("FERROPLAN_BUILD_SHA")
            .unwrap_or("source-revision-not-embedded")
            .to_string(),
        manifest_fingerprint: manifest
            .fingerprint()
            .unwrap_or_else(|_| "invalid-manifest".to_string()),
    }
}

fn base_envelope<T>(
    request_id: String,
    input_fingerprint: String,
    elapsed_micros: u64,
) -> OperationEnvelope<T> {
    OperationEnvelope {
        schema_version: OPERATION_ENVELOPE_SCHEMA.to_string(),
        request_id,
        capability_id: "fp.core.solve".to_string(),
        capability_version: env!("CARGO_PKG_VERSION").to_string(),
        build_identity: build_identity(),
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

/// Execute the deterministic planner through bounded, independently validated,
/// candidate-only production semantics.
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

    if let Err(error) = limits.validate() {
        let mut envelope = base_envelope(request_id, input_fingerprint, elapsed(&clock));
        envelope.outcome = OutcomeClass::Refused;
        envelope.error = Some(error);
        return envelope;
    }
    if let Some(error) = validate_request(domain, problem, options, limits) {
        let mut envelope = base_envelope(request_id, input_fingerprint, elapsed(&clock));
        envelope.outcome = OutcomeClass::Refused;
        envelope.error = Some(error);
        return envelope;
    }

    let mut bounded_options = options.clone();
    if bounded_options.threads == 0 {
        bounded_options.threads = 1;
    }
    bounded_options.max_evaluated = Some(
        bounded_options
            .max_evaluated
            .unwrap_or(limits.max_evaluated)
            .min(limits.max_evaluated),
    );

    let result = crate::api::solve(domain, problem, &bounded_options);
    let mut envelope = base_envelope(request_id, input_fingerprint, elapsed(&clock));
    match result {
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
                let capped = solution.statistics.evaluated_states >= limits.max_evaluated;
                envelope.outcome = if capped {
                    OutcomeClass::LimitExceeded
                } else {
                    OutcomeClass::NoPlan
                };
                if capped {
                    envelope.error = Some(PublicError::new(
                        "FP_LIMIT_SEARCH",
                        format!(
                            "search reached the configured max_evaluated limit ({})",
                            limits.max_evaluated
                        ),
                        true,
                    ));
                }
                envelope.payload = Some(solution);
                envelope.elapsed_micros = elapsed(&clock);
                return envelope;
            }

            let plan = match solution.plan.as_ref() {
                Some(plan) => plan,
                None => {
                    envelope.outcome = OutcomeClass::Failed;
                    envelope.validation = ValidationStatus::Failed;
                    envelope.error = Some(PublicError::new(
                        "FP_INVARIANT",
                        "solver reported solved=true without a plan",
                        false,
                    ));
                    envelope.elapsed_micros = elapsed(&clock);
                    return envelope;
                }
            };
            if plan.length != plan.steps.len() {
                envelope.outcome = OutcomeClass::Failed;
                envelope.validation = ValidationStatus::Failed;
                envelope.error = Some(PublicError::new(
                    "FP_INVARIANT",
                    "plan length does not match the number of emitted steps",
                    false,
                ));
                envelope.elapsed_micros = elapsed(&clock);
                return envelope;
            }
            if plan.length > limits.max_plan_steps {
                envelope.outcome = OutcomeClass::LimitExceeded;
                envelope.error = Some(PublicError::new(
                    "FP_LIMIT_PLAN",
                    format!(
                        "plan contains {} steps; configured maximum is {}",
                        plan.length, limits.max_plan_steps
                    ),
                    true,
                ));
                envelope.elapsed_micros = elapsed(&clock);
                return envelope;
            }

            match independently_validate(domain, problem, plan) {
                Ok(ValidationStatus::Valid) => {
                    envelope.validation = ValidationStatus::Valid;
                }
                Ok(ValidationStatus::NotApplicable) => {
                    envelope.validation = ValidationStatus::NotApplicable;
                    envelope.warnings.push(
                        "independent text-plan validation is not applicable to the empty plan"
                            .to_string(),
                    );
                }
                Ok(ValidationStatus::Failed) | Err(_) => {
                    envelope.outcome = OutcomeClass::Failed;
                    envelope.validation = ValidationStatus::Failed;
                    envelope.error = Some(PublicError::new(
                        "FP_VALIDATION",
                        "the emitted plan failed independent validation",
                        false,
                    ));
                    envelope.elapsed_micros = elapsed(&clock);
                    return envelope;
                }
            }

            match serde_json::to_vec(&solution) {
                Ok(bytes) if bytes.len() <= limits.max_output_bytes => {
                    envelope.outcome = OutcomeClass::Solved;
                    envelope.payload = Some(solution);
                }
                Ok(bytes) => {
                    envelope.outcome = OutcomeClass::LimitExceeded;
                    envelope.error = Some(PublicError::new(
                        "FP_LIMIT_OUTPUT",
                        format!(
                            "serialized solution is {} bytes; configured maximum is {}",
                            bytes.len(), limits.max_output_bytes
                        ),
                        true,
                    ));
                }
                Err(_) => {
                    envelope.outcome = OutcomeClass::Failed;
                    envelope.error = Some(PublicError::new(
                        "FP_ADAPTER",
                        "solution could not be serialized",
                        false,
                    ));
                }
            }
        }
    }
    envelope.elapsed_micros = elapsed(&clock);
    envelope
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
                "domain is {} bytes; configured maximum is {}",
                domain.len(), limits.max_domain_bytes
            ),
            false,
        ));
    }
    if problem.len() > limits.max_problem_bytes {
        return Some(PublicError::new(
            "FP_LIMIT_INPUT",
            format!(
                "problem is {} bytes; configured maximum is {}",
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
                "requested {} workers; configured maximum is {}",
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
            format!(
                "max_evaluated must be in 1..={} for this surface",
                limits.max_evaluated
            ),
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
    if plan.steps.is_empty() {
        return Ok(ValidationStatus::NotApplicable);
    }
    let plan_text = render_plan(plan);
    match crate::plan::validate_plan(domain, problem, &plan_text)? {
        crate::plan::Validity::Valid => Ok(ValidationStatus::Valid),
        crate::plan::Validity::Invalid(_) => Ok(ValidationStatus::Failed),
    }
}

fn render_plan(plan: &Plan) -> String {
    let temporal = plan.steps.iter().any(|step| step.time.is_some());
    let mut out = String::new();
    for step in &plan.steps {
        let args = if step.args.is_empty() {
            String::new()
        } else {
            format!(" {}", step.args.join(" "))
        };
        if temporal {
            let time = step.time.unwrap_or(0.0);
            let duration = step.duration.unwrap_or(0.0);
            out.push_str(&format!(
                "{time:.6}: ({}{args}) [{duration:.6}]\n",
                step.action
            ));
        } else {
            out.push_str(&format!("step {}: {}{args}\n", step.index, step.action));
        }
    }
    out
}

pub fn production_input_fingerprint(domain: &str, problem: &str, options: &Options) -> String {
    let options = serde_json::to_vec(options).unwrap_or_default();
    sha256_hex(
        INPUT_HASH_DOMAIN,
        &[domain.as_bytes(), problem.as_bytes(), &options],
    )
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
    match request_id.map(str::trim).filter(|id| !id.is_empty()) {
        Some(id) => truncate_utf8(id, MAX_REQUEST_ID_BYTES),
        None => format!("req-{}", &fingerprint[..16.min(fingerprint.len())]),
    }
}

fn bounded_warnings(warnings: &[String]) -> Vec<String> {
    warnings
        .iter()
        .take(MAX_WARNING_COUNT)
        .map(|warning| truncate_utf8(warning, MAX_WARNING_BYTES))
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
    fn manifest_is_canonical_and_domain_separated() {
        let manifest = capability_manifest();
        manifest.validate().unwrap();
        let first = manifest.fingerprint().unwrap();
        let second = capability_manifest().fingerprint().unwrap();
        assert_eq!(first, second);
        assert_eq!(first.len(), 64);
        assert_ne!(first, production_input_fingerprint(DOMAIN, PROBLEM, &Options::default()));
    }

    #[test]
    fn admission_is_evidence_derived() {
        let declared = evaluate_readiness("test-source", Vec::<String>::new()).unwrap();
        assert_eq!(declared.overall_state, ReadinessState::Declared);
        assert!(declared
            .capabilities
            .iter()
            .all(|evaluation| evaluation.state == ReadinessState::Declared));

        let all = capability_manifest()
            .capabilities
            .iter()
            .flat_map(|contract| contract.required_evidence.clone())
            .collect::<Vec<_>>();
        let admitted = evaluate_readiness("test-source", all).unwrap();
        assert_eq!(admitted.overall_state, ReadinessState::Admitted);
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
        assert!(envelope.payload.as_ref().is_some_and(|solution| solution.solved));
    }

    #[test]
    fn production_solve_refuses_oversized_input_before_parse() {
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
    fn invalid_weights_are_refused_not_sanitized_at_public_boundary() {
        let mut options = Options::default();
        options.weight_h = f64::NAN;
        let envelope = solve_production(
            DOMAIN,
            PROBLEM,
            &options,
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
    fn partial_evidence_cannot_be_crowned() {
        let report = evaluate_readiness("test-source", ["core.solve.unit"]).unwrap();
        assert_eq!(report.overall_state, ReadinessState::Partial);
        let solve = report
            .capabilities
            .iter()
            .find(|evaluation| evaluation.capability_id == "fp.core.solve")
            .unwrap();
        assert_eq!(solve.state, ReadinessState::Partial);
        assert!(!solve.missing_evidence.is_empty());
    }

    #[test]
    fn empty_plan_is_not_falsely_claimed_independently_validated() {
        let already_true = "(define (problem smoke-p) (:domain smoke) \
            (:init (done)) (:goal (done)))";
        let envelope = solve_production(
            DOMAIN,
            already_true,
            &Options::default(),
            &ProductionLimits::default(),
            None,
        );
        assert_eq!(envelope.outcome, OutcomeClass::Solved);
        assert_eq!(envelope.validation, ValidationStatus::NotApplicable);
    }

    #[test]
    fn canonical_manifest_contains_every_advertised_surface() {
        let ids: BTreeSet<_> = capability_manifest()
            .capabilities
            .into_iter()
            .map(|contract| contract.id)
            .collect();
        for expected in [
            "fp.core.solve",
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
    fn mode_is_bound_into_input_identity() {
        let auto = production_input_fingerprint(DOMAIN, PROBLEM, &Options::default());
        let mut ff = Options::default();
        ff.mode = Mode::Ff;
        assert_ne!(auto, production_input_fingerprint(DOMAIN, PROBLEM, &ff));
    }
}
