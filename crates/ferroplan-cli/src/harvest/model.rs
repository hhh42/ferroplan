use serde::{Deserialize, Serialize};

pub const OBSERVATION_SCHEMA: &str = "ferroplan-observation-pack/v1";
pub const ADMISSION_SCHEMA: &str = "ferroplan-harvest-admission/v1";
pub const CATALOG_SCHEMA: &str = "ferroplan-method-catalog/v1";
pub const RECEIPT_SCHEMA: &str = "ferroplan-harvest-receipt/v1";

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct ObservationWindow {
    pub start_utc: String,
    pub end_exclusive_utc: String,
    pub timezone: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct ObservationPack {
    pub schema: String,
    pub run_id: String,
    pub window: ObservationWindow,
    pub repositories: Vec<String>,
    #[serde(default)]
    pub work_items: Vec<ObservedWorkItem>,
    #[serde(default)]
    pub transport_failures: Vec<TransportFailure>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct ObservedWorkItem {
    pub repository: String,
    pub sha: String,
    #[serde(default)]
    pub parent_sha: Option<String>,
    pub message: String,
    pub committed_at_utc: String,
    pub source_url: String,
    #[serde(default)]
    pub changed_paths: Vec<String>,
    #[serde(default)]
    pub executions: Vec<ExecutionEvidence>,
    #[serde(default)]
    pub artifacts: Vec<ArtifactEvidence>,
    #[serde(default)]
    pub probabilistic_outcomes: Vec<ObservedOutcome>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ExecutionResult {
    Pass,
    Fail,
    Cancelled,
    Pending,
    Unknown,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct ExecutionEvidence {
    pub surface: String,
    pub command: String,
    pub source_sha: String,
    pub result: ExecutionResult,
    #[serde(default)]
    pub exit_code: Option<i32>,
    pub observed_at_utc: String,
    pub evidence_url: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct ArtifactEvidence {
    pub name: String,
    pub source_sha: String,
    pub evidence_url: String,
    #[serde(default)]
    pub digest: Option<String>,
    #[serde(default)]
    pub size_bytes: Option<u64>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct ObservedOutcome {
    pub label: String,
    pub probability: f64,
    pub success: bool,
    #[serde(default)]
    pub evidence: Vec<EvidenceRef>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub struct EvidenceRef {
    pub kind: String,
    pub identity: String,
    pub location: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct TransportFailure {
    pub repository: String,
    pub operation: String,
    pub state: String,
    pub detail: String,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum AdmissionLevel {
    Observed,
    IdentityResolved,
    ExecutionObserved,
    ResultCorroborated,
    ReceiptVerified,
    ReplayVerified,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum RefusalCode {
    MissingExactSourceIdentity,
    OutsideObservationWindow,
    MissingChangedPaths,
    ExecutionNotObserved,
    WorkflowRunNotBoundToHead,
    ProbabilityEvidenceMissing,
    InvalidProbability,
    ProbabilityMassExceeded,
    OperatorBoundExceeded,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct AdmittedWork {
    pub identity: String,
    pub level: AdmissionLevel,
    pub work: ObservedWorkItem,
    pub evidence: Vec<EvidenceRef>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct ExcludedWork {
    pub identity: String,
    pub code: RefusalCode,
    pub detail: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct AdmissionReport {
    pub schema: String,
    pub admitted: Vec<AdmittedWork>,
    pub excluded: Vec<ExcludedWork>,
    pub unresolved_transport_failures: Vec<TransportFailure>,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum GallCheckpoint {
    G0Orient,
    G1Fence,
    G2Observe,
    G3Admit,
    G4Plan,
    G5Manufacture,
    G6Verify,
    G7Replay,
    G8ReleaseAdmission,
    G9SunsetAdmission,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ActuationClass {
    Select,
    Construct,
    Do,
    HookIntent,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct OperatorOutcome {
    pub label: String,
    pub probability: f64,
    pub success: bool,
    pub evidence: Vec<EvidenceRef>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct PlanningOperator {
    pub id: String,
    pub name: String,
    pub signature: String,
    pub checkpoint: GallCheckpoint,
    pub actuation_class: ActuationClass,
    pub preconditions: Vec<String>,
    pub effects: Vec<String>,
    pub invariants: Vec<String>,
    pub failures: Vec<String>,
    pub refusals: Vec<String>,
    pub receipt_hook: bool,
    pub replay_hook: bool,
    #[serde(default)]
    pub probabilistic_outcomes: Vec<OperatorOutcome>,
    pub evidence: Vec<EvidenceRef>,
    pub source_work: Vec<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct MethodCatalog {
    pub schema: String,
    pub run_id: String,
    pub source_pack_digest: String,
    pub raw_operator_count: usize,
    pub operator_count: usize,
    pub operators: Vec<PlanningOperator>,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum FinalState {
    PartialAlive,
    Alive,
    Blocked,
    BuildBroken,
    Unknown,
    Unsupported,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ReplayState {
    NotExecuted,
    ReplayMatch,
    ReplayMismatch,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct ValidationRecord {
    pub command: String,
    pub result: String,
    #[serde(default)]
    pub detail: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct ValidationSummary {
    pub parse_ok: bool,
    #[serde(default)]
    pub parse_error: Option<String>,
    pub solve_attempted: bool,
    #[serde(default)]
    pub solved: Option<bool>,
    #[serde(default)]
    pub initial_value: Option<f64>,
    #[serde(default)]
    pub policy_valid: Option<bool>,
    #[serde(default)]
    pub policy_errors: Vec<String>,
    pub records: Vec<ValidationRecord>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct OutputArtifact {
    pub path: String,
    pub bytes: usize,
    pub blake3: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct SourceRevision {
    pub repository: String,
    #[serde(default)]
    pub base_sha: Option<String>,
    pub head_sha: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct HarvestReceipt {
    pub schema: String,
    pub run_id: String,
    pub receipt_digest: String,
    pub source_pack_digest: String,
    pub catalog_digest: String,
    pub source_revisions: Vec<SourceRevision>,
    pub source_work: Vec<String>,
    pub admitted_work: Vec<String>,
    pub excluded_work: Vec<ExcludedWork>,
    pub operators_added: Vec<String>,
    pub operators_deduplicated: usize,
    pub probabilistic_operators: usize,
    pub outputs: Vec<OutputArtifact>,
    pub validation: ValidationSummary,
    pub replay: ReplayState,
    pub generated_outputs_hand_edited: bool,
    pub transport_failures: Vec<TransportFailure>,
    pub failures: Vec<String>,
    pub exclusions: Vec<String>,
    pub final_state: FinalState,
}
