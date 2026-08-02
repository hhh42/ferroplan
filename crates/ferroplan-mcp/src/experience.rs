//! Vision 2030 operator-experience plane.
//!
//! The planner and persistent-session layers already own execution. This
//! module makes those authorities discoverable, composable, diagnosable,
//! guided, batchable, and transportable without introducing network access or
//! bypassing the existing receipt and session laws.

use std::collections::{BTreeMap, BTreeSet, VecDeque};
use std::sync::Arc;

use ferroplan::{Options, Plan, Session};
use rmcp::handler::server::wrapper::Parameters;
use rmcp::model::{CallToolResult, ErrorData as McpError};
use rmcp::{tool, tool_router};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::{json, Map, Value};
use tokio::sync::Mutex as AsyncMutex;

use crate::result::to_result;
use crate::session::{
    chain_receipt, current_plan_valid, digest_value, validate_session_id, ManagedSession,
};
use crate::Ferroplan;

const DEFAULT_SEARCH_BUDGET: usize = 50_000;
const MAX_SEARCH_BUDGET: usize = 10_000_000;
const MAX_MEMORY_MB: usize = 16_384;
const MAX_BATCH_OPS: usize = 64;
const MAX_HISTORY_TAIL: usize = 256;
const MAX_LATTICE_STATES: usize = 16_384;
const MAX_LATTICE_DEPTH: usize = 16;
const MAX_TELCO_TTL_MS: u64 = 86_400_000;
const MAX_TELCO_PAYLOAD_BYTES: usize = 1_048_576;

pub(crate) const RESOURCE_TOOLS: &[&str] = &[
    "dx_manifest",
    "dx_compose",
    "doctor_scan",
    "doctor_explain",
    "wizard_bootstrap",
    "wizard_recipe",
    "qol_snapshot",
    "qol_batch",
    "telco_envelope",
    "telco_verify",
    "vision_lattice",
];

pub(crate) fn ontology_comment(name: &str) -> Option<&'static str> {
    Some(match name {
        "dx_manifest" => "Return the complete self-describing Ferroplan capability manifest, including authority categories, contracts, effects, reversibility, receipt behavior, and composition examples.",
        "dx_compose" => "Search the bounded capability graph for a minimal deterministic tool sequence from admitted starting atoms to requested outcome atoms.",
        "doctor_scan" => "Diagnose global or per-session health, assign typed findings, calculate standing, and emit executable remediation hints without mutating state.",
        "doctor_explain" => "Classify a tool or protocol failure into a typed cause with bounded confidence, corrective actions, and refusal-preserving recovery guidance.",
        "wizard_bootstrap" => "Atomically manufacture a ready persistent planning mind from domain, problem, goal, authority scope, and bounded search settings.",
        "wizard_recipe" => "Compile a high-level operator intent into an ordered, inspectable Ferroplan tool recipe with preflight, rollback, and receipt checkpoints.",
        "qol_snapshot" => "Read session state, selected facts and fluents, plan standing, diagnostics, memory, lineage, and recent history in one round trip.",
        "qol_batch" => "Apply a bounded heterogeneous session transaction on a staged fork and commit exactly once, or refuse without partial mutation.",
        "telco_envelope" => "Manufacture a deterministic transport-neutral BLAKE3 integrity envelope with correlation, causation, idempotency, predecessor, and expiry fields; it performs no network operation.",
        "telco_verify" => "Verify a transport envelope's schema, payload identity, envelope identity, routing expectations, predecessor, and expiry without treating integrity as authentication.",
        "vision_lattice" => "Enumerate a bounded combinatorial capability lattice, minimal reachability depths, dependency edges, blocked frontiers, and theoretical composition capacity.",
        _ => return None,
    })
}

#[derive(Clone, Copy)]
struct CapabilitySpec {
    tool: &'static str,
    category: &'static str,
    requires: &'static [&'static str],
    provides: &'static [&'static str],
    mutates: bool,
    reversible: bool,
    receipt: bool,
    latency: &'static str,
    summary: &'static str,
}

const CAPABILITIES: &[CapabilitySpec] = &[
    CapabilitySpec {
        tool: "solve",
        category: "planning",
        requires: &["domain_source", "problem_source"],
        provides: &["solution", "plan"],
        mutates: false,
        reversible: true,
        receipt: false,
        latency: "search",
        summary: "Solve a grounded planning problem.",
    },
    CapabilitySpec {
        tool: "parse",
        category: "planning",
        requires: &["pddl_source"],
        provides: &["pddl_ast", "syntax_report"],
        mutates: false,
        reversible: true,
        receipt: false,
        latency: "instant",
        summary: "Parse and summarize PDDL.",
    },
    CapabilitySpec {
        tool: "validate",
        category: "planning",
        requires: &["domain_source", "problem_source", "plan"],
        provides: &["valid_plan"],
        mutates: false,
        reversible: true,
        receipt: false,
        latency: "bounded",
        summary: "Validate a plan independently.",
    },
    CapabilitySpec {
        tool: "decompose",
        category: "planning",
        requires: &["domain_source", "problem_source"],
        provides: &["decomposition", "plan"],
        mutates: false,
        reversible: true,
        receipt: false,
        latency: "search",
        summary: "Decompose and solve a temporal goal.",
    },
    CapabilitySpec {
        tool: "session_open",
        category: "session",
        requires: &["domain_source", "problem_source"],
        provides: &["session_id", "grounded_session"],
        mutates: true,
        reversible: true,
        receipt: true,
        latency: "grounding",
        summary: "Create a persistent grounded mind.",
    },
    CapabilitySpec {
        tool: "session_observe",
        category: "session",
        requires: &["grounded_session", "observation"],
        provides: &["session_state", "surprise_report"],
        mutates: true,
        reversible: false,
        receipt: true,
        latency: "instant",
        summary: "Admit visible state changes.",
    },
    CapabilitySpec {
        tool: "session_set_goal",
        category: "session",
        requires: &["grounded_session", "goal_expression"],
        provides: &["goal_bound"],
        mutates: true,
        reversible: true,
        receipt: true,
        latency: "instant",
        summary: "Retarget a grounded mind.",
    },
    CapabilitySpec {
        tool: "session_think",
        category: "session",
        requires: &["grounded_session", "goal_bound"],
        provides: &["plan"],
        mutates: true,
        reversible: true,
        receipt: true,
        latency: "search",
        summary: "Retain a valid suffix or replan.",
    },
    CapabilitySpec {
        tool: "session_advance",
        category: "session",
        requires: &["grounded_session", "plan", "completed_steps"],
        provides: &["plan_cursor"],
        mutates: true,
        reversible: false,
        receipt: true,
        latency: "instant",
        summary: "Advance the admitted plan cursor.",
    },
    CapabilitySpec {
        tool: "session_status",
        category: "session",
        requires: &["grounded_session"],
        provides: &["session_state"],
        mutates: false,
        reversible: true,
        receipt: false,
        latency: "instant",
        summary: "Inspect one persistent mind.",
    },
    CapabilitySpec {
        tool: "session_close",
        category: "session",
        requires: &["grounded_session"],
        provides: &["closed_session"],
        mutates: true,
        reversible: false,
        receipt: false,
        latency: "instant",
        summary: "Drop one persistent mind.",
    },
    CapabilitySpec {
        tool: "session_list",
        category: "control",
        requires: &[],
        provides: &["session_catalog"],
        mutates: false,
        reversible: true,
        receipt: false,
        latency: "bounded",
        summary: "Discover live sessions deterministically.",
    },
    CapabilitySpec {
        tool: "session_state",
        category: "control",
        requires: &["grounded_session"],
        provides: &["session_state", "state_fingerprint"],
        mutates: false,
        reversible: true,
        receipt: false,
        latency: "instant",
        summary: "Read selected semantic state.",
    },
    CapabilitySpec {
        tool: "session_set",
        category: "control",
        requires: &["grounded_session"],
        provides: &["session_state"],
        mutates: true,
        reversible: true,
        receipt: true,
        latency: "instant",
        summary: "Atomically set facts, fluents, and goal.",
    },
    CapabilitySpec {
        tool: "session_fork",
        category: "control",
        requires: &["grounded_session"],
        provides: &["forked_session", "grounded_session"],
        mutates: true,
        reversible: true,
        receipt: true,
        latency: "instant",
        summary: "Fork an independent mind.",
    },
    CapabilitySpec {
        tool: "session_replan",
        category: "control",
        requires: &["grounded_session", "goal_bound"],
        provides: &["plan"],
        mutates: true,
        reversible: true,
        receipt: true,
        latency: "search",
        summary: "Force bounded replanning.",
    },
    CapabilitySpec {
        tool: "session_checkpoint",
        category: "control",
        requires: &["grounded_session"],
        provides: &["checkpoint"],
        mutates: true,
        reversible: true,
        receipt: true,
        latency: "instant",
        summary: "Capture an immutable checkpoint.",
    },
    CapabilitySpec {
        tool: "session_restore",
        category: "control",
        requires: &["checkpoint"],
        provides: &["grounded_session", "restored_session"],
        mutates: true,
        reversible: true,
        receipt: true,
        latency: "instant",
        summary: "Restore a checkpoint explicitly.",
    },
    CapabilitySpec {
        tool: "session_verify_checkpoint",
        category: "control",
        requires: &["checkpoint", "grounded_session"],
        provides: &["checkpoint_verified"],
        mutates: false,
        reversible: true,
        receipt: false,
        latency: "instant",
        summary: "Verify live state against a checkpoint.",
    },
    CapabilitySpec {
        tool: "session_history",
        category: "control",
        requires: &["grounded_session"],
        provides: &["event_history"],
        mutates: false,
        reversible: true,
        receipt: false,
        latency: "bounded",
        summary: "Read canonical session history.",
    },
    CapabilitySpec {
        tool: "session_compare",
        category: "control",
        requires: &["grounded_session", "forked_session"],
        provides: &["comparison"],
        mutates: false,
        reversible: true,
        receipt: false,
        latency: "instant",
        summary: "Compare two live minds.",
    },
    CapabilitySpec {
        tool: "session_restrict_ops",
        category: "control",
        requires: &["grounded_session"],
        provides: &["operator_scope"],
        mutates: true,
        reversible: true,
        receipt: true,
        latency: "instant",
        summary: "Install planner-level operator authority.",
    },
    CapabilitySpec {
        tool: "session_schedule_fact",
        category: "control",
        requires: &["grounded_session", "temporal_fact"],
        provides: &["temporal_schedule"],
        mutates: true,
        reversible: true,
        receipt: true,
        latency: "instant",
        summary: "Schedule an exogenous fact.",
    },
    CapabilitySpec {
        tool: "session_apply_start",
        category: "control",
        requires: &["grounded_session", "durative_action"],
        provides: &["in_flight_action"],
        mutates: true,
        reversible: false,
        receipt: true,
        latency: "instant",
        summary: "Apply a durative action start.",
    },
    CapabilitySpec {
        tool: "session_elapse",
        category: "control",
        requires: &["grounded_session"],
        provides: &["elapsed_state"],
        mutates: true,
        reversible: false,
        receipt: true,
        latency: "bounded",
        summary: "Advance temporal state.",
    },
    CapabilitySpec {
        tool: "cmca_allocate",
        category: "allocation",
        requires: &["cmca_frontier"],
        provides: &["allocation"],
        mutates: false,
        reversible: true,
        receipt: true,
        latency: "bounded",
        summary: "Allocate one admitted frontier.",
    },
    CapabilitySpec {
        tool: "cmca_allocate_recursive",
        category: "allocation",
        requires: &["cmca_frontier"],
        provides: &["recursive_allocation"],
        mutates: false,
        reversible: true,
        receipt: true,
        latency: "bounded",
        summary: "Allocate a recursive admitted cascade.",
    },
    CapabilitySpec {
        tool: "canonical_digest",
        category: "admission",
        requires: &["canonical_value"],
        provides: &["digest"],
        mutates: false,
        reversible: true,
        receipt: true,
        latency: "instant",
        summary: "Create canonical BLAKE3 identity.",
    },
    CapabilitySpec {
        tool: "bind_allocation_receipt",
        category: "admission",
        requires: &["allocation", "digest"],
        provides: &["allocation_receipt"],
        mutates: false,
        reversible: true,
        receipt: true,
        latency: "instant",
        summary: "Bind allocation evidence.",
    },
    CapabilitySpec {
        tool: "bind_plan_receipt",
        category: "admission",
        requires: &["plan", "digest"],
        provides: &["plan_receipt"],
        mutates: false,
        reversible: true,
        receipt: true,
        latency: "instant",
        summary: "Bind planning evidence.",
    },
    CapabilitySpec {
        tool: "verify_receipt",
        category: "admission",
        requires: &["plan_receipt"],
        provides: &["verified_receipt"],
        mutates: false,
        reversible: true,
        receipt: false,
        latency: "instant",
        summary: "Verify a canonical receipt envelope.",
    },
    CapabilitySpec {
        tool: "dx_manifest",
        category: "dx",
        requires: &[],
        provides: &["capability_manifest"],
        mutates: false,
        reversible: true,
        receipt: false,
        latency: "instant",
        summary: "Describe the entire authority surface.",
    },
    CapabilitySpec {
        tool: "dx_compose",
        category: "dx",
        requires: &["desired_outcome"],
        provides: &["tool_composition"],
        mutates: false,
        reversible: true,
        receipt: false,
        latency: "bounded",
        summary: "Find a minimal tool composition.",
    },
    CapabilitySpec {
        tool: "doctor_scan",
        category: "doctor",
        requires: &[],
        provides: &["diagnosis"],
        mutates: false,
        reversible: true,
        receipt: false,
        latency: "bounded",
        summary: "Diagnose server or session health.",
    },
    CapabilitySpec {
        tool: "doctor_explain",
        category: "doctor",
        requires: &["failure_message"],
        provides: &["diagnosis", "recovery_recipe"],
        mutates: false,
        reversible: true,
        receipt: false,
        latency: "instant",
        summary: "Explain and remediate a failure.",
    },
    CapabilitySpec {
        tool: "wizard_bootstrap",
        category: "wizard",
        requires: &["domain_source", "problem_source"],
        provides: &[
            "grounded_session",
            "goal_bound",
            "plan",
            "bootstrapped_session",
        ],
        mutates: true,
        reversible: true,
        receipt: true,
        latency: "search",
        summary: "Manufacture a ready planning mind.",
    },
    CapabilitySpec {
        tool: "wizard_recipe",
        category: "wizard",
        requires: &["operator_intent"],
        provides: &["tool_recipe"],
        mutates: false,
        reversible: true,
        receipt: false,
        latency: "instant",
        summary: "Compile intent into an inspectable recipe.",
    },
    CapabilitySpec {
        tool: "qol_snapshot",
        category: "qol",
        requires: &["grounded_session"],
        provides: &["session_snapshot", "diagnosis"],
        mutates: false,
        reversible: true,
        receipt: false,
        latency: "bounded",
        summary: "Collapse many reads into one snapshot.",
    },
    CapabilitySpec {
        tool: "qol_batch",
        category: "qol",
        requires: &["grounded_session", "batch_operations"],
        provides: &["session_state", "atomic_batch"],
        mutates: true,
        reversible: true,
        receipt: true,
        latency: "bounded",
        summary: "Commit a heterogeneous transaction once.",
    },
    CapabilitySpec {
        tool: "telco_envelope",
        category: "telco",
        requires: &["message_payload"],
        provides: &["transport_envelope"],
        mutates: false,
        reversible: true,
        receipt: true,
        latency: "instant",
        summary: "Manufacture a transport-neutral envelope.",
    },
    CapabilitySpec {
        tool: "telco_verify",
        category: "telco",
        requires: &["transport_envelope"],
        provides: &["verified_envelope"],
        mutates: false,
        reversible: true,
        receipt: false,
        latency: "instant",
        summary: "Verify envelope integrity and expiry.",
    },
    CapabilitySpec {
        tool: "vision_lattice",
        category: "vision",
        requires: &[],
        provides: &["capability_lattice", "blue_ocean_frontier"],
        mutates: false,
        reversible: true,
        receipt: false,
        latency: "bounded",
        summary: "Enumerate bounded combinatorial reachability.",
    },
];

fn default_true() -> bool {
    true
}

fn default_compose_steps() -> usize {
    8
}

fn default_history_tail() -> usize {
    8
}

fn default_lattice_depth() -> usize {
    6
}

fn default_lattice_states() -> usize {
    4096
}

fn default_budget() -> usize {
    DEFAULT_SEARCH_BUDGET
}

fn default_ttl_ms() -> u64 {
    300_000
}

#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct ManifestInput {
    #[serde(default)]
    category: Option<String>,
    #[serde(default = "default_true")]
    include_examples: bool,
}

#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct ComposeInput {
    #[serde(default)]
    have: Vec<String>,
    want: Vec<String>,
    #[serde(default = "default_compose_steps")]
    max_steps: usize,
    #[serde(default)]
    allowed_tools: Vec<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct VisionInput {
    #[serde(default)]
    seeds: Vec<String>,
    #[serde(default = "default_lattice_depth")]
    max_depth: usize,
    #[serde(default = "default_lattice_states")]
    max_states: usize,
    #[serde(default)]
    category: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct DoctorScanInput {
    #[serde(default)]
    session_id: Option<String>,
    #[serde(default = "default_history_tail")]
    history_tail: usize,
}

#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct DoctorExplainInput {
    message: String,
    #[serde(default)]
    context: BTreeMap<String, String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct WizardRecipeInput {
    intent: String,
    #[serde(default)]
    parameters: BTreeMap<String, String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct WizardBootstrapInput {
    session_id: String,
    domain: String,
    problem: String,
    #[serde(default)]
    options: Option<Options>,
    #[serde(default)]
    goal: Option<String>,
    #[serde(default)]
    allowed_prefixes: Vec<String>,
    #[serde(default)]
    denied_prefixes: Vec<String>,
    #[serde(default = "default_true")]
    plan: bool,
    #[serde(default = "default_budget")]
    max_evaluated: usize,
    #[serde(default)]
    memory_mb: Option<usize>,
    #[serde(default)]
    replace: bool,
}

#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct SnapshotInput {
    session_id: String,
    #[serde(default)]
    facts: Vec<String>,
    #[serde(default)]
    fluents: Vec<String>,
    #[serde(default = "default_history_tail")]
    history_tail: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "op", rename_all = "snake_case")]
enum BatchOperation {
    SetFact {
        fact: String,
        value: bool,
    },
    SetFluent {
        fluent: String,
        value: f64,
    },
    SetGoal {
        goal: String,
    },
    RestrictOps {
        #[serde(default)]
        allowed_prefixes: Vec<String>,
        #[serde(default)]
        denied_prefixes: Vec<String>,
    },
    ScheduleFact {
        delay: f64,
        fact: String,
        value: bool,
    },
    ApplyStart {
        action: String,
    },
    Elapse {
        delta: f64,
    },
    Replan {
        #[serde(default = "default_budget")]
        max_evaluated: usize,
        #[serde(default)]
        memory_mb: Option<usize>,
    },
}

#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct BatchInput {
    session_id: String,
    #[serde(default)]
    expected_epoch: Option<u64>,
    operations: Vec<BatchOperation>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct TelcoEnvelope {
    schema: String,
    version: u32,
    sender: String,
    recipient: String,
    channel: String,
    issued_at_ms: u64,
    expires_at_ms: u64,
    correlation_id: String,
    #[serde(default)]
    causation_id: Option<String>,
    #[serde(default)]
    predecessor_digest: Option<String>,
    idempotency_key: String,
    payload: BTreeMap<String, Value>,
    payload_digest: String,
    envelope_digest: String,
    integrity: String,
    authentication: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct TelcoEnvelopeInput {
    sender: String,
    recipient: String,
    channel: String,
    issued_at_ms: u64,
    #[serde(default = "default_ttl_ms")]
    ttl_ms: u64,
    correlation_id: String,
    #[serde(default)]
    causation_id: Option<String>,
    #[serde(default)]
    predecessor_digest: Option<String>,
    #[serde(default)]
    idempotency_key: Option<String>,
    payload: BTreeMap<String, Value>,
}

#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct TelcoVerifyInput {
    envelope: TelcoEnvelope,
    observed_at_ms: u64,
    #[serde(default)]
    expected_recipient: Option<String>,
    #[serde(default)]
    expected_predecessor_digest: Option<String>,
}

#[tool_router(router = experience_router, vis = "pub")]
impl Ferroplan {
    #[tool(
        description = "Return the self-describing capability manifest and composition contracts."
    )]
    fn dx_manifest(
        &self,
        Parameters(input): Parameters<ManifestInput>,
    ) -> Result<CallToolResult, McpError> {
        to_result(do_manifest(input))
    }

    #[tool(description = "Find a bounded minimal deterministic composition of Ferroplan tools.")]
    fn dx_compose(
        &self,
        Parameters(input): Parameters<ComposeInput>,
    ) -> Result<CallToolResult, McpError> {
        to_result(do_compose(input))
    }

    #[tool(
        description = "Diagnose global or per-session health and return typed remediation hints."
    )]
    async fn doctor_scan(
        &self,
        Parameters(input): Parameters<DoctorScanInput>,
    ) -> Result<CallToolResult, McpError> {
        to_result(self.do_doctor_scan(input).await)
    }

    #[tool(description = "Explain a failure or refusal and return a bounded recovery recipe.")]
    fn doctor_explain(
        &self,
        Parameters(input): Parameters<DoctorExplainInput>,
    ) -> Result<CallToolResult, McpError> {
        to_result(do_doctor_explain(input))
    }

    #[tool(description = "Atomically manufacture a ready persistent planning mind in one call.")]
    async fn wizard_bootstrap(
        &self,
        Parameters(input): Parameters<WizardBootstrapInput>,
    ) -> Result<CallToolResult, McpError> {
        to_result(self.do_wizard_bootstrap(input).await)
    }

    #[tool(
        description = "Compile a high-level operator intent into an inspectable Ferroplan recipe."
    )]
    fn wizard_recipe(
        &self,
        Parameters(input): Parameters<WizardRecipeInput>,
    ) -> Result<CallToolResult, McpError> {
        to_result(do_wizard_recipe(input))
    }

    #[tool(
        description = "Collapse session state, plan, diagnostics, memory, and history into one read."
    )]
    async fn qol_snapshot(
        &self,
        Parameters(input): Parameters<SnapshotInput>,
    ) -> Result<CallToolResult, McpError> {
        to_result(self.do_qol_snapshot(input).await)
    }

    #[tool(
        description = "Apply a bounded heterogeneous session transaction atomically on a staged fork."
    )]
    async fn qol_batch(
        &self,
        Parameters(input): Parameters<BatchInput>,
    ) -> Result<CallToolResult, McpError> {
        to_result(self.do_qol_batch(input).await)
    }

    #[tool(
        description = "Manufacture a deterministic transport-neutral integrity envelope; no network operation occurs."
    )]
    fn telco_envelope(
        &self,
        Parameters(input): Parameters<TelcoEnvelopeInput>,
    ) -> Result<CallToolResult, McpError> {
        to_result(do_telco_envelope(input))
    }

    #[tool(
        description = "Verify transport-envelope integrity, routing expectations, predecessor, and expiry."
    )]
    fn telco_verify(
        &self,
        Parameters(input): Parameters<TelcoVerifyInput>,
    ) -> Result<CallToolResult, McpError> {
        to_result(do_telco_verify(input))
    }

    #[tool(
        description = "Enumerate a bounded combinatorial capability lattice and blocked frontier."
    )]
    fn vision_lattice(
        &self,
        Parameters(input): Parameters<VisionInput>,
    ) -> Result<CallToolResult, McpError> {
        to_result(do_vision_lattice(input))
    }
}

fn do_manifest(input: ManifestInput) -> Result<Value, String> {
    let category = input.category.map(|value| normalize_atom(&value));
    let tools: Vec<Value> = CAPABILITIES
        .iter()
        .filter(|spec| {
            category
                .as_deref()
                .is_none_or(|wanted| spec.category == wanted)
        })
        .map(capability_json)
        .collect();
    if tools.is_empty() && category.is_some() {
        return Err(format!(
            "unknown or empty capability category `{}`",
            category.unwrap_or_default()
        ));
    }

    let categories: BTreeSet<&str> = CAPABILITIES.iter().map(|spec| spec.category).collect();
    let examples = if input.include_examples {
        json!([
            {
                "name": "author-prove-receipt",
                "have": ["domain_source", "problem_source", "digest"],
                "want": ["verified_receipt"],
                "expected": ["solve", "bind_plan_receipt", "verify_receipt"]
            },
            {
                "name": "persistent-experiment",
                "have": ["domain_source", "problem_source", "goal_expression"],
                "want": ["comparison"],
                "expected": ["session_open", "session_set_goal", "session_fork", "session_compare"]
            },
            {
                "name": "remote-handoff",
                "have": ["message_payload"],
                "want": ["verified_envelope"],
                "expected": ["telco_envelope", "telco_verify"]
            }
        ])
    } else {
        json!([])
    };

    Ok(json!({
        "schema": "urn:chatman:ferroplan-dx-manifest:v1",
        "server_version": env!("CARGO_PKG_VERSION"),
        "advertised_tool_count": crate::all_tool_names().len(),
        "modeled_tool_count": CAPABILITIES.len(),
        "categories": categories,
        "tools": tools,
        "composition_atoms": all_atoms(),
        "examples": examples,
        "hard_invariants": [
            "zero implicit network actuation",
            "no partial qol_batch mutation",
            "stale epochs refuse before mutation",
            "search and lattice expansion are mechanically bounded",
            "integrity digests are not represented as authentication",
            "all session mutations preserve existing receipt-chain law"
        ]
    }))
}

fn do_compose(input: ComposeInput) -> Result<Value, String> {
    if input.want.is_empty() {
        return Err("want must contain at least one outcome atom".into());
    }
    if input.max_steps == 0 || input.max_steps > MAX_LATTICE_DEPTH {
        return Err(format!("max_steps must be within 1..={MAX_LATTICE_DEPTH}"));
    }
    let have = normalize_atoms(input.have)?;
    let want = normalize_atoms(input.want)?;
    let allowed = normalize_atoms(input.allowed_tools)?;
    let filter = |spec: &&CapabilitySpec| allowed.is_empty() || allowed.contains(spec.tool);

    let mut queue = VecDeque::from([(have.clone(), Vec::<String>::new())]);
    let mut seen = BTreeSet::from([state_key(&have)]);
    let mut expanded = 0usize;
    let mut solved = None;

    while let Some((state, path)) = queue.pop_front() {
        if want.iter().all(|atom| state.contains(atom)) {
            solved = Some((state, path));
            break;
        }
        if path.len() >= input.max_steps || expanded >= MAX_LATTICE_STATES {
            continue;
        }
        expanded += 1;
        for spec in CAPABILITIES.iter().filter(filter) {
            if !requirements_met(spec, &state) {
                continue;
            }
            let mut next = state.clone();
            for effect in spec.provides {
                next.insert((*effect).to_owned());
            }
            if next == state {
                continue;
            }
            let key = state_key(&next);
            if seen.insert(key) {
                let mut next_path = path.clone();
                next_path.push(spec.tool.to_owned());
                queue.push_back((next, next_path));
            }
        }
    }

    let (final_atoms, steps, is_solved) = match solved {
        Some((state, path)) => (state, path, true),
        None => (have.clone(), Vec::new(), false),
    };
    let missing: Vec<String> = want
        .iter()
        .filter(|atom| !final_atoms.contains(*atom))
        .cloned()
        .collect();
    let step_contracts: Vec<Value> = steps
        .iter()
        .filter_map(|name| CAPABILITIES.iter().find(|spec| spec.tool == name))
        .map(capability_json)
        .collect();

    Ok(json!({
        "schema": "urn:chatman:ferroplan-dx-composition:v1",
        "solved": is_solved,
        "have": have,
        "want": want,
        "steps": steps,
        "step_contracts": step_contracts,
        "final_atoms": final_atoms,
        "missing_atoms": missing,
        "expanded_states": expanded,
        "bounded_by": {"max_steps": input.max_steps, "max_states": MAX_LATTICE_STATES}
    }))
}

async fn global_session_entries(
    server: &Ferroplan,
) -> Vec<(String, Arc<AsyncMutex<ManagedSession>>)> {
    let sessions = server.session_state.sessions.lock().await;
    sessions
        .iter()
        .map(|(id, session)| (id.clone(), Arc::clone(session)))
        .collect()
}

impl Ferroplan {
    async fn do_doctor_scan(&self, input: DoctorScanInput) -> Result<Value, String> {
        validate_history_tail(input.history_tail)?;
        if let Some(session_id) = input.session_id {
            let lock = self.session_state.get(&session_id).await?;
            let managed = lock.lock().await;
            return Ok(json!({
                "schema": "urn:chatman:ferroplan-doctor:v1",
                "scope": "session",
                "session_id": session_id,
                "report": doctor_report(&managed, input.history_tail)
            }));
        }

        let entries = global_session_entries(self).await;
        let mut reports = Vec::with_capacity(entries.len());
        let mut total_score = 0u64;
        for (session_id, lock) in entries {
            let managed = lock.lock().await;
            let report = doctor_report(&managed, input.history_tail);
            total_score += report["health_score"].as_u64().unwrap_or(0);
            reports.push(json!({"session_id": session_id, "report": report}));
        }
        let count = reports.len();
        let score = if count == 0 {
            100
        } else {
            total_score / count as u64
        };
        Ok(json!({
            "schema": "urn:chatman:ferroplan-doctor:v1",
            "scope": "server",
            "standing": if score >= 90 { "ALIVE" } else if score >= 60 { "PARTIAL_ALIVE" } else { "BLOCKED" },
            "health_score": score,
            "session_count": count,
            "tool_count": crate::all_tool_names().len(),
            "modeled_capability_count": CAPABILITIES.len(),
            "sessions": reports,
            "next_actions": if count == 0 {
                json!([{"tool": "wizard_bootstrap", "reason": "no persistent sessions exist"}])
            } else {
                json!([])
            }
        }))
    }

    async fn do_wizard_bootstrap(&self, input: WizardBootstrapInput) -> Result<Value, String> {
        validate_session_id(&input.session_id)?;
        validate_budget(input.max_evaluated, input.memory_mb)?;
        let allowed = normalize_prefixes(input.allowed_prefixes)?;
        let denied = normalize_prefixes(input.denied_prefixes)?;
        let options = input.options.unwrap_or_default();
        let mut session = Session::new(&input.domain, &input.problem, &options)?;
        if let Some(goal) = &input.goal {
            session.set_goal(goal)?;
        }
        if !allowed.is_empty() || !denied.is_empty() {
            session.restrict_ops(|display| operator_admitted(display, &allowed, &denied));
        }

        let (solution_value, last_plan): (Option<Value>, Option<Plan>) = if input.plan {
            let solution = tokio::task::block_in_place(|| {
                session.replan_budgeted(input.max_evaluated, input.memory_mb)
            });
            let value = serde_json::to_value(&solution).map_err(|error| error.to_string())?;
            (Some(value), solution.plan.clone())
        } else {
            (None, None)
        };

        let mut sessions = self.session_state.sessions.lock().await;
        if sessions.contains_key(&input.session_id) && !input.replace {
            return Err(format!(
                "session `{}` already exists; set replace=true to discard it",
                input.session_id
            ));
        }
        let predecessor = if let Some(existing) = sessions.get(&input.session_id) {
            existing.lock().await.receipt_head.clone()
        } else {
            None
        };
        let mut managed = ManagedSession {
            session,
            last_plan,
            cursor: 0,
            epoch: 0,
            domain_digest: digest_bytes(input.domain.as_bytes()),
            problem_digest: digest_bytes(input.problem.as_bytes()),
            receipt_head: predecessor,
            event_log: Vec::new(),
            parent_session_id: None,
            generation: 0,
            allowed_ops: allowed,
            denied_ops: denied,
        };
        let event = json!({
            "schema": "urn:chatman:ferroplan-session-event:v2",
            "event": "wizard-bootstrap",
            "session_id": input.session_id,
            "domain_digest": managed.domain_digest,
            "problem_digest": managed.problem_digest,
            "goal_changed": input.goal.is_some(),
            "search_requested": input.plan,
            "solution_digest": solution_value.as_ref().map(digest_value).transpose()?,
            "state_fingerprint": managed.session.state_fingerprint(),
            "allowed_prefixes": managed.allowed_ops,
            "denied_prefixes": managed.denied_ops
        });
        let receipt = chain_receipt(&mut managed, &event)?;
        let report = doctor_report(&managed, 8);
        let response = json!({
            "schema": "urn:chatman:ferroplan-wizard-bootstrap:v1",
            "session_id": input.session_id,
            "epoch": managed.epoch,
            "domain_digest": managed.domain_digest,
            "problem_digest": managed.problem_digest,
            "state_fingerprint": managed.session.state_fingerprint(),
            "goal_met": managed.session.goal_met(),
            "solution": solution_value,
            "report": report,
            "receipt": receipt
        });
        sessions.insert(input.session_id, Arc::new(AsyncMutex::new(managed)));
        Ok(response)
    }

    async fn do_qol_snapshot(&self, input: SnapshotInput) -> Result<Value, String> {
        validate_history_tail(input.history_tail)?;
        let lock = self.session_state.get(&input.session_id).await?;
        let managed = lock.lock().await;
        let mut facts = Map::new();
        let mut unknown_facts = Vec::new();
        for fact in input.facts {
            match managed.session.fact(&fact) {
                Some(value) => {
                    facts.insert(fact.to_ascii_uppercase(), Value::Bool(value));
                }
                None => unknown_facts.push(fact.to_ascii_uppercase()),
            }
        }
        let mut fluents = Map::new();
        let mut unknown_fluents = Vec::new();
        for fluent in input.fluents {
            match managed.session.fluent(&fluent) {
                Some(value) => {
                    fluents.insert(fluent.to_ascii_uppercase(), json!(value));
                }
                None => unknown_fluents.push(fluent.to_ascii_uppercase()),
            }
        }
        let history_start = managed.event_log.len().saturating_sub(input.history_tail);
        Ok(json!({
            "schema": "urn:chatman:ferroplan-qol-snapshot:v1",
            "session_id": input.session_id,
            "epoch": managed.epoch,
            "lineage": {"parent_session_id": managed.parent_session_id, "generation": managed.generation},
            "identity": {
                "domain_digest": managed.domain_digest,
                "problem_digest": managed.problem_digest,
                "state_fingerprint": managed.session.state_fingerprint(),
                "receipt_chain_head": managed.receipt_head
            },
            "state": {
                "goal_met": managed.session.goal_met(),
                "facts": facts,
                "fluents": fluents,
                "unknown_facts": unknown_facts,
                "unknown_fluents": unknown_fluents
            },
            "plan": {
                "cursor": managed.cursor,
                "length": managed.last_plan.as_ref().map(|plan| plan.steps.len()),
                "remaining_valid": current_plan_valid(&managed),
                "value": managed.last_plan.as_ref()
            },
            "authority": {"allowed_prefixes": managed.allowed_ops, "denied_prefixes": managed.denied_ops},
            "memory": {"world_bytes": managed.session.world_bytes(), "mind_bytes": managed.session.mind_bytes()},
            "history_tail": &managed.event_log[history_start..],
            "doctor": doctor_report(&managed, input.history_tail)
        }))
    }

    async fn do_qol_batch(&self, input: BatchInput) -> Result<Value, String> {
        if input.operations.is_empty() || input.operations.len() > MAX_BATCH_OPS {
            return Err(format!(
                "operations must contain 1..={MAX_BATCH_OPS} entries"
            ));
        }
        let replan_positions: Vec<usize> = input
            .operations
            .iter()
            .enumerate()
            .filter_map(|(index, operation)| {
                matches!(operation, BatchOperation::Replan { .. }).then_some(index)
            })
            .collect();
        if replan_positions.len() > 1 {
            return Err("qol_batch admits at most one replan operation".into());
        }
        if replan_positions
            .first()
            .is_some_and(|index| *index + 1 != input.operations.len())
        {
            return Err("replan must be the final qol_batch operation".into());
        }

        let lock = self.session_state.get(&input.session_id).await?;
        let mut managed = lock.lock().await;
        require_epoch(&managed, input.expected_epoch)?;
        let before_fingerprint = managed.session.state_fingerprint();
        let mut staged = managed.session.fork();
        let mut staged_plan = managed.last_plan.clone();
        let mut staged_cursor = managed.cursor;
        let mut allowed = managed.allowed_ops.clone();
        let mut denied = managed.denied_ops.clone();
        let mut results = Vec::with_capacity(input.operations.len());

        for operation in &input.operations {
            let result = match operation {
                BatchOperation::SetFact { fact, value } => {
                    staged.set_fact(fact, *value)?;
                    staged_plan = None;
                    staged_cursor = 0;
                    json!({"op": "set_fact", "fact": fact, "value": value})
                }
                BatchOperation::SetFluent { fluent, value } => {
                    if !value.is_finite() {
                        return Err(format!("fluent `{fluent}` must be finite"));
                    }
                    staged.set_fluent(fluent, *value)?;
                    staged_plan = None;
                    staged_cursor = 0;
                    json!({"op": "set_fluent", "fluent": fluent, "value": value})
                }
                BatchOperation::SetGoal { goal } => {
                    staged.set_goal(goal)?;
                    staged_plan = None;
                    staged_cursor = 0;
                    json!({"op": "set_goal", "goal": goal})
                }
                BatchOperation::RestrictOps {
                    allowed_prefixes,
                    denied_prefixes,
                } => {
                    allowed = normalize_prefixes(allowed_prefixes.clone())?;
                    denied = normalize_prefixes(denied_prefixes.clone())?;
                    staged.restrict_ops(|display| operator_admitted(display, &allowed, &denied));
                    staged_plan = None;
                    staged_cursor = 0;
                    json!({"op": "restrict_ops", "allowed_prefixes": allowed, "denied_prefixes": denied})
                }
                BatchOperation::ScheduleFact { delay, fact, value } => {
                    staged.set_timed_fact(*delay, fact, *value)?;
                    json!({"op": "schedule_fact", "delay": delay, "fact": fact, "value": value})
                }
                BatchOperation::ApplyStart { action } => {
                    staged.apply_start(action)?;
                    staged_plan = None;
                    staged_cursor = 0;
                    json!({"op": "apply_start", "action": action})
                }
                BatchOperation::Elapse { delta } => {
                    let broken_intervals = staged.elapse(*delta)?;
                    staged_plan = None;
                    staged_cursor = 0;
                    json!({"op": "elapse", "delta": delta, "broken_intervals": broken_intervals})
                }
                BatchOperation::Replan {
                    max_evaluated,
                    memory_mb,
                } => {
                    validate_budget(*max_evaluated, *memory_mb)?;
                    let solution = tokio::task::block_in_place(|| {
                        staged.replan_budgeted(*max_evaluated, *memory_mb)
                    });
                    staged_plan = solution.plan.clone();
                    staged_cursor = 0;
                    json!({"op": "replan", "solution": solution})
                }
            };
            results.push(result);
        }

        managed.session = staged;
        managed.last_plan = staged_plan;
        managed.cursor = staged_cursor;
        managed.allowed_ops = allowed;
        managed.denied_ops = denied;
        managed.epoch = managed.epoch.saturating_add(1);
        let after_fingerprint = managed.session.state_fingerprint();
        let event = json!({
            "schema": "urn:chatman:ferroplan-session-event:v2",
            "event": "qol-batch",
            "session_id": input.session_id,
            "epoch": managed.epoch,
            "operation_count": input.operations.len(),
            "operations": input.operations,
            "results": results,
            "before_fingerprint": before_fingerprint,
            "after_fingerprint": after_fingerprint
        });
        let receipt = chain_receipt(&mut managed, &event)?;
        Ok(json!({
            "schema": "urn:chatman:ferroplan-qol-batch:v1",
            "session_id": input.session_id,
            "epoch": managed.epoch,
            "operation_count": input.operations.len(),
            "results": results,
            "before_fingerprint": before_fingerprint,
            "after_fingerprint": after_fingerprint,
            "goal_met": managed.session.goal_met(),
            "remaining_plan_valid": current_plan_valid(&managed),
            "receipt": receipt
        }))
    }
}

fn do_doctor_explain(input: DoctorExplainInput) -> Result<Value, String> {
    let message = input.message.trim();
    if message.is_empty() {
        return Err("message must not be empty".into());
    }
    let lower = message.to_ascii_lowercase();
    let (code, confidence, cause, tools): (&str, f64, &str, Value) = if lower
        .contains("unknown session")
    {
        (
            "SESSION_NOT_FOUND",
            0.99,
            "The requested persistent session id is not live in this server process.",
            json!(["session_list", "wizard_bootstrap", "session_open"]),
        )
    } else if lower.contains("stale session epoch") || lower.contains("expected_epoch") {
        (
            "STALE_EPOCH",
            0.99,
            "The caller attempted optimistic mutation against an older observed epoch.",
            json!(["qol_snapshot", "session_state", "qol_batch"]),
        )
    } else if lower.contains("already exists") {
        (
            "IDENTITY_COLLISION",
            0.98,
            "The requested session or checkpoint identity is already owned.",
            json!(["session_list", "session_fork", "wizard_bootstrap"]),
        )
    } else if lower.contains("max_evaluated")
        || lower.contains("memory_mb")
        || lower.contains("budget")
    {
        ("BOUNDARY_REFUSED", 0.96, "A mechanical search or memory ceiling refused the request before unbounded work began.", json!(["doctor_scan", "session_replan", "wizard_bootstrap"]))
    } else if lower.contains("finite") || lower.contains("nan") || lower.contains("infinite") {
        (
            "NON_FINITE_INPUT",
            0.97,
            "A numeric or temporal input violated the finite-value boundary.",
            json!(["doctor_explain", "qol_batch"]),
        )
    } else if lower.contains("checkpoint")
        && (lower.contains("unknown") || lower.contains("not found"))
    {
        (
            "CHECKPOINT_NOT_FOUND",
            0.96,
            "The checkpoint id is not live in the current server process.",
            json!(["session_checkpoint", "session_restore"]),
        )
    } else if lower.contains("plan")
        && (lower.contains("invalid") || lower.contains("no plan") || lower.contains("unsolved"))
    {
        ("REPLAN_REQUIRED", 0.90, "The current plan is absent, exhausted, invalid, or no solution was found within the admitted boundary.", json!(["doctor_scan", "session_replan", "dx_compose"]))
    } else if lower.contains("fact") && (lower.contains("unknown") || lower.contains("ground")) {
        (
            "FACT_NOT_GROUNDED",
            0.91,
            "The requested fact is not part of the admitted grounded problem state.",
            json!(["parse", "qol_snapshot", "wizard_bootstrap"]),
        )
    } else if lower.contains("expired") || lower.contains("ttl") {
        (
            "ENVELOPE_EXPIRED",
            0.95,
            "The transport envelope was observed outside its admitted lifetime.",
            json!(["telco_envelope", "telco_verify"]),
        )
    } else if lower.contains("digest") || lower.contains("tamper") || lower.contains("integrity") {
        (
            "INTEGRITY_MISMATCH",
            0.88,
            "Canonical content no longer matches the bound digest.",
            json!(["canonical_digest", "verify_receipt", "telco_verify"]),
        )
    } else {
        (
            "UNKNOWN_DIAGNOSIS",
            0.25,
            "No high-confidence deterministic diagnostic rule matched the supplied message.",
            json!(["doctor_scan", "dx_manifest"]),
        )
    };

    Ok(json!({
        "schema": "urn:chatman:ferroplan-doctor-explanation:v1",
        "code": code,
        "confidence": confidence,
        "cause": cause,
        "original_message": message,
        "context": input.context,
        "recommended_tools": tools,
        "recovery_law": "observe current standing before retry; do not reinterpret refusal as success"
    }))
}

fn do_wizard_recipe(input: WizardRecipeInput) -> Result<Value, String> {
    let intent = normalize_atom(&input.intent);
    let steps = match intent.as_str() {
        "author_and_prove" | "author_prove" => json!([
            {"tool": "parse", "purpose": "syntax feedback"},
            {"tool": "solve", "purpose": "manufacture a plan"},
            {"tool": "validate", "purpose": "independent execution semantics"},
            {"tool": "canonical_digest", "purpose": "bind admitted inputs"},
            {"tool": "bind_plan_receipt", "purpose": "bind plan evidence"},
            {"tool": "verify_receipt", "purpose": "verify the evidence envelope"}
        ]),
        "bootstrap_session" | "bootstrap" => json!([
            {"tool": "wizard_bootstrap", "purpose": "ground, scope, goal, and plan atomically"},
            {"tool": "qol_snapshot", "purpose": "read the complete operating state"},
            {"tool": "session_checkpoint", "purpose": "capture a recovery point"}
        ]),
        "diagnose_and_recover" | "recover" => json!([
            {"tool": "doctor_scan", "purpose": "establish current standing"},
            {"tool": "qol_snapshot", "purpose": "collect one-round-trip evidence"},
            {"tool": "session_replan", "purpose": "refresh planning only when diagnosed"},
            {"tool": "session_checkpoint", "purpose": "seal recovered standing"}
        ]),
        "branch_experiment" | "experiment" => json!([
            {"tool": "session_checkpoint", "purpose": "seal parent baseline"},
            {"tool": "session_fork", "purpose": "create an independent mind"},
            {"tool": "qol_batch", "purpose": "apply the experimental transaction"},
            {"tool": "session_compare", "purpose": "measure divergence"},
            {"tool": "session_verify_checkpoint", "purpose": "prove parent baseline remains intact"}
        ]),
        "temporal_execute" | "temporal" => json!([
            {"tool": "session_schedule_fact", "purpose": "admit exogenous timing"},
            {"tool": "session_apply_start", "purpose": "admit a real durative start"},
            {"tool": "session_elapse", "purpose": "fire due events and ends"},
            {"tool": "doctor_scan", "purpose": "diagnose interval and goal standing"}
        ]),
        "remote_handoff" | "telco" => json!([
            {"tool": "qol_snapshot", "purpose": "materialize the handoff state"},
            {"tool": "telco_envelope", "purpose": "bind routing and integrity metadata"},
            {"tool": "telco_verify", "purpose": "verify before transport actuation"}
        ]),
        "combinatorial_discovery" | "blue_ocean" | "vision_2030" => json!([
            {"tool": "dx_manifest", "purpose": "load the capability vocabulary"},
            {"tool": "vision_lattice", "purpose": "enumerate bounded reachability and gaps"},
            {"tool": "dx_compose", "purpose": "compile a chosen outcome into minimal tools"}
        ]),
        _ => {
            return Err(format!(
                "unsupported intent `{}`; supported intents: author_and_prove, bootstrap_session, diagnose_and_recover, branch_experiment, temporal_execute, remote_handoff, combinatorial_discovery",
                input.intent
            ))
        }
    };
    Ok(json!({
        "schema": "urn:chatman:ferroplan-wizard-recipe:v1",
        "intent": intent,
        "parameters": input.parameters,
        "preflight": ["read tool schemas", "admit exact identifiers", "set mechanical budgets"],
        "steps": steps,
        "rollback": ["checkpoint before irreversible temporal or observation changes", "restore only through explicit session_restore"],
        "receipt_checkpoints": ["after bootstrap", "after mutation", "after validation", "before external transport"]
    }))
}

fn do_telco_envelope(input: TelcoEnvelopeInput) -> Result<Value, String> {
    validate_telco_name("sender", &input.sender)?;
    validate_telco_name("recipient", &input.recipient)?;
    validate_telco_name("channel", &input.channel)?;
    validate_telco_name("correlation_id", &input.correlation_id)?;
    if input.ttl_ms == 0 || input.ttl_ms > MAX_TELCO_TTL_MS {
        return Err(format!("ttl_ms must be within 1..={MAX_TELCO_TTL_MS}"));
    }
    let payload_value = serde_json::to_value(&input.payload).map_err(|error| error.to_string())?;
    let payload_bytes = serde_json::to_vec(&payload_value).map_err(|error| error.to_string())?;
    if payload_bytes.len() > MAX_TELCO_PAYLOAD_BYTES {
        return Err(format!(
            "payload exceeds {MAX_TELCO_PAYLOAD_BYTES} canonical JSON bytes"
        ));
    }
    let expires_at_ms = input
        .issued_at_ms
        .checked_add(input.ttl_ms)
        .ok_or_else(|| "issued_at_ms + ttl_ms overflowed u64".to_owned())?;
    let payload_digest = digest_value(&payload_value)?;
    let idempotency_key = match input.idempotency_key {
        Some(value) => {
            validate_telco_name("idempotency_key", &value)?;
            value
        }
        None => digest_value(&json!({
            "sender": input.sender,
            "recipient": input.recipient,
            "channel": input.channel,
            "correlation_id": input.correlation_id,
            "payload_digest": payload_digest
        }))?,
    };
    let mut envelope = TelcoEnvelope {
        schema: "urn:chatman:ferroplan-telco-envelope:v1".into(),
        version: 1,
        sender: input.sender,
        recipient: input.recipient,
        channel: input.channel,
        issued_at_ms: input.issued_at_ms,
        expires_at_ms,
        correlation_id: input.correlation_id,
        causation_id: input.causation_id,
        predecessor_digest: input.predecessor_digest,
        idempotency_key,
        payload: input.payload,
        payload_digest,
        envelope_digest: String::new(),
        integrity: "BLAKE3".into(),
        authentication: "UNSUPPORTED".into(),
    };
    envelope.envelope_digest = digest_value(&telco_basis(&envelope)?)?;
    serde_json::to_value(envelope).map_err(|error| error.to_string())
}

fn do_telco_verify(input: TelcoVerifyInput) -> Result<Value, String> {
    let envelope = input.envelope;
    let mut failures = Vec::new();
    if envelope.schema != "urn:chatman:ferroplan-telco-envelope:v1" || envelope.version != 1 {
        failures.push("SCHEMA_MISMATCH".to_owned());
    }
    let payload_value =
        serde_json::to_value(&envelope.payload).map_err(|error| error.to_string())?;
    let observed_payload_digest = digest_value(&payload_value)?;
    if observed_payload_digest != envelope.payload_digest {
        failures.push("PAYLOAD_DIGEST_MISMATCH".to_owned());
    }
    let observed_envelope_digest = digest_value(&telco_basis(&envelope)?)?;
    if observed_envelope_digest != envelope.envelope_digest {
        failures.push("ENVELOPE_DIGEST_MISMATCH".to_owned());
    }
    if input.observed_at_ms > envelope.expires_at_ms {
        failures.push("ENVELOPE_EXPIRED".to_owned());
    }
    if input.observed_at_ms < envelope.issued_at_ms {
        failures.push("OBSERVED_BEFORE_ISSUE".to_owned());
    }
    if input
        .expected_recipient
        .as_ref()
        .is_some_and(|expected| expected != &envelope.recipient)
    {
        failures.push("RECIPIENT_MISMATCH".to_owned());
    }
    if input
        .expected_predecessor_digest
        .as_ref()
        .is_some_and(|expected| envelope.predecessor_digest.as_ref() != Some(expected))
    {
        failures.push("PREDECESSOR_MISMATCH".to_owned());
    }
    Ok(json!({
        "schema": "urn:chatman:ferroplan-telco-verification:v1",
        "valid": failures.is_empty(),
        "standing": if failures.is_empty() { "ALIVE" } else { "REFUSED" },
        "failures": failures,
        "payload_digest": observed_payload_digest,
        "envelope_digest": observed_envelope_digest,
        "authentication": "UNSUPPORTED",
        "warning": "BLAKE3 proves integrity and canonical identity, not sender authentication or delivery"
    }))
}

fn do_vision_lattice(input: VisionInput) -> Result<Value, String> {
    if input.max_depth == 0 || input.max_depth > MAX_LATTICE_DEPTH {
        return Err(format!("max_depth must be within 1..={MAX_LATTICE_DEPTH}"));
    }
    if input.max_states == 0 || input.max_states > MAX_LATTICE_STATES {
        return Err(format!(
            "max_states must be within 1..={MAX_LATTICE_STATES}"
        ));
    }
    let seeds = normalize_atoms(input.seeds)?;
    let category = input.category.map(|value| normalize_atom(&value));
    let specs: Vec<&CapabilitySpec> = CAPABILITIES
        .iter()
        .filter(|spec| {
            category
                .as_deref()
                .is_none_or(|wanted| spec.category == wanted)
        })
        .collect();
    if specs.is_empty() {
        return Err("category selected no capabilities".into());
    }

    let mut queue = VecDeque::from([(seeds.clone(), 0usize)]);
    let mut seen = BTreeSet::from([state_key(&seeds)]);
    let mut reachable_atoms = seeds.clone();
    let mut reachable_tools = BTreeSet::new();
    let mut minimal_depth: BTreeMap<String, usize> =
        seeds.iter().cloned().map(|atom| (atom, 0)).collect();
    let mut max_depth_reached = 0usize;

    while let Some((state, depth)) = queue.pop_front() {
        max_depth_reached = max_depth_reached.max(depth);
        if depth >= input.max_depth || seen.len() >= input.max_states {
            continue;
        }
        for spec in &specs {
            if !requirements_met(spec, &state) {
                continue;
            }
            reachable_tools.insert(spec.tool.to_owned());
            let mut next = state.clone();
            for effect in spec.provides {
                let effect = (*effect).to_owned();
                next.insert(effect.clone());
                reachable_atoms.insert(effect.clone());
                minimal_depth.entry(effect).or_insert(depth + 1);
            }
            if next != state {
                let key = state_key(&next);
                if seen.insert(key) {
                    queue.push_back((next, depth + 1));
                }
            }
        }
    }

    let blocked: Vec<Value> = specs
        .iter()
        .filter(|spec| !reachable_tools.contains(spec.tool))
        .map(|spec| {
            let missing: Vec<&str> = spec
                .requires
                .iter()
                .copied()
                .filter(|atom| !reachable_atoms.contains(*atom))
                .collect();
            json!({"tool": spec.tool, "missing": missing})
        })
        .collect();
    let edges: Vec<Value> = specs
        .iter()
        .flat_map(|left| {
            specs.iter().filter_map(move |right| {
                let shared: Vec<&str> = left
                    .provides
                    .iter()
                    .copied()
                    .filter(|effect| right.requires.contains(effect))
                    .collect();
                (!shared.is_empty())
                    .then(|| json!({"from": left.tool, "to": right.tool, "atoms": shared}))
            })
        })
        .collect();
    let atom_count = reachable_atoms.len();
    let theoretical = if atom_count < 128 {
        (1u128 << atom_count).to_string()
    } else {
        format!("2^{atom_count}")
    };

    Ok(json!({
        "schema": "urn:chatman:ferroplan-vision-lattice:v1",
        "seed_atoms": seeds,
        "category": category,
        "reachable_state_count": seen.len(),
        "reachable_atom_count": atom_count,
        "reachable_atoms": reachable_atoms,
        "reachable_tools": reachable_tools,
        "minimal_depth_by_atom": minimal_depth,
        "dependency_edges": edges,
        "blocked_frontier": blocked,
        "max_depth_reached": max_depth_reached,
        "theoretical_atom_subset_capacity": theoretical,
        "bounded_by": {"max_depth": input.max_depth, "max_states": input.max_states},
        "standing": if seen.len() >= input.max_states { "PARTIAL_ALIVE" } else { "ALIVE" }
    }))
}

fn capability_json(spec: &CapabilitySpec) -> Value {
    json!({
        "tool": spec.tool,
        "category": spec.category,
        "requires": spec.requires,
        "provides": spec.provides,
        "mutates": spec.mutates,
        "reversible": spec.reversible,
        "receipt": spec.receipt,
        "latency": spec.latency,
        "summary": spec.summary
    })
}

fn all_atoms() -> BTreeSet<&'static str> {
    CAPABILITIES
        .iter()
        .flat_map(|spec| spec.requires.iter().chain(spec.provides.iter()))
        .copied()
        .collect()
}

fn normalize_atom(value: &str) -> String {
    value
        .trim()
        .to_ascii_lowercase()
        .chars()
        .map(|character| {
            if character.is_ascii_alphanumeric() || matches!(character, '_' | ':' | '.') {
                character
            } else {
                '_'
            }
        })
        .collect::<String>()
        .trim_matches('_')
        .to_owned()
}

fn normalize_atoms(values: Vec<String>) -> Result<BTreeSet<String>, String> {
    let mut out = BTreeSet::new();
    for value in values {
        let normalized = normalize_atom(&value);
        if normalized.is_empty() {
            return Err(format!(
                "capability atom is empty after normalization: `{value}`"
            ));
        }
        out.insert(normalized);
    }
    Ok(out)
}

fn state_key(state: &BTreeSet<String>) -> String {
    state.iter().cloned().collect::<Vec<_>>().join("\u{0}")
}

fn requirements_met(spec: &CapabilitySpec, state: &BTreeSet<String>) -> bool {
    spec.requires.iter().all(|atom| state.contains(*atom))
}

fn doctor_report(managed: &ManagedSession, history_tail: usize) -> Value {
    let mut diagnostics = Vec::new();
    let mut score = 100i64;
    let goal_met = managed.session.goal_met();
    let plan_valid = current_plan_valid(managed);
    let plan_length = managed.last_plan.as_ref().map(|plan| plan.steps.len());

    if managed.receipt_head.is_none() {
        score -= 35;
        diagnostics.push(json!({
            "severity": "error",
            "code": "RECEIPT_CHAIN_MISSING",
            "message": "The live mind has no receipt-chain head.",
            "remediation": [{"tool": "session_close"}, {"tool": "wizard_bootstrap"}]
        }));
    }
    if managed.event_log.is_empty() {
        score -= 10;
        diagnostics.push(json!({
            "severity": "warning",
            "code": "EVENT_HISTORY_EMPTY",
            "message": "No canonical session events are available.",
            "remediation": [{"tool": "qol_snapshot"}]
        }));
    }
    if goal_met {
        diagnostics.push(json!({
            "severity": "info",
            "code": "GOAL_MET",
            "message": "The current grounded goal already stands.",
            "remediation": [{"tool": "session_checkpoint"}, {"tool": "bind_plan_receipt"}]
        }));
    } else if managed.last_plan.is_none() {
        score -= 15;
        diagnostics.push(json!({
            "severity": "warning",
            "code": "NO_PLAN",
            "message": "The goal is not met and no plan is retained.",
            "remediation": [{"tool": "session_replan"}, {"tool": "session_think"}]
        }));
    } else if managed.cursor >= plan_length.unwrap_or(0) {
        score -= 20;
        diagnostics.push(json!({
            "severity": "warning",
            "code": "PLAN_EXHAUSTED",
            "message": "The plan cursor is at or beyond the retained plan while the goal is not met.",
            "remediation": [{"tool": "session_observe"}, {"tool": "session_replan"}]
        }));
    }
    if plan_valid == Some(false) {
        score -= 30;
        diagnostics.push(json!({
            "severity": "error",
            "code": "PLAN_INVALID",
            "message": "The remaining retained plan does not stand in the current state.",
            "remediation": [{"tool": "qol_snapshot"}, {"tool": "session_replan"}]
        }));
    }
    if !managed.allowed_ops.is_empty() || !managed.denied_ops.is_empty() {
        diagnostics.push(json!({
            "severity": "info",
            "code": "OPERATOR_SCOPE_ACTIVE",
            "message": "Planner-level operator authority is active.",
            "allowed_prefixes": managed.allowed_ops,
            "denied_prefixes": managed.denied_ops
        }));
    }
    let world_bytes = managed.session.world_bytes();
    let mind_bytes = managed.session.mind_bytes();
    if world_bytes > 0 && mind_bytes > world_bytes.saturating_mul(4) {
        score -= 5;
        diagnostics.push(json!({
            "severity": "info",
            "code": "MIND_MEMORY_HEAVY",
            "message": "Private mutable mind memory exceeds four times the shared grounding size.",
            "world_bytes": world_bytes,
            "mind_bytes": mind_bytes
        }));
    }
    score = score.clamp(0, 100);
    let standing = if diagnostics.iter().any(|item| item["severity"] == "error") {
        "BLOCKED"
    } else if diagnostics.iter().any(|item| item["severity"] == "warning") {
        "PARTIAL_ALIVE"
    } else {
        "ALIVE"
    };
    let history_start = managed.event_log.len().saturating_sub(history_tail);
    json!({
        "standing": standing,
        "health_score": score,
        "epoch": managed.epoch,
        "state_fingerprint": managed.session.state_fingerprint(),
        "goal_met": goal_met,
        "plan_length": plan_length,
        "cursor": managed.cursor,
        "remaining_plan_valid": plan_valid,
        "lineage": {"parent_session_id": managed.parent_session_id, "generation": managed.generation},
        "memory": {"world_bytes": world_bytes, "mind_bytes": mind_bytes},
        "event_count": managed.event_log.len(),
        "history_tail": &managed.event_log[history_start..],
        "receipt_chain_head": managed.receipt_head,
        "diagnostics": diagnostics
    })
}

fn validate_history_tail(value: usize) -> Result<(), String> {
    if value > MAX_HISTORY_TAIL {
        return Err(format!("history_tail must be at most {MAX_HISTORY_TAIL}"));
    }
    Ok(())
}

fn validate_budget(max_evaluated: usize, memory_mb: Option<usize>) -> Result<(), String> {
    if max_evaluated == 0 || max_evaluated > MAX_SEARCH_BUDGET {
        return Err(format!(
            "max_evaluated must be within 1..={MAX_SEARCH_BUDGET}"
        ));
    }
    if memory_mb.is_some_and(|value| value > MAX_MEMORY_MB) {
        return Err(format!("memory_mb must be at most {MAX_MEMORY_MB}"));
    }
    Ok(())
}

fn require_epoch(managed: &ManagedSession, expected: Option<u64>) -> Result<(), String> {
    if let Some(expected) = expected {
        if managed.epoch != expected {
            return Err(format!(
                "stale session epoch: expected {expected}, observed {}",
                managed.epoch
            ));
        }
    }
    Ok(())
}

fn normalize_prefixes(values: Vec<String>) -> Result<Vec<String>, String> {
    let mut out = Vec::new();
    for value in values {
        let normalized = value
            .trim()
            .trim_matches(|character| character == '(' || character == ')')
            .to_ascii_uppercase();
        if normalized.is_empty()
            || !normalized.bytes().all(|byte| {
                byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b' ' | b':')
            })
        {
            return Err(format!("operator prefix is not canonical: `{value}`"));
        }
        out.push(normalized);
    }
    out.sort();
    out.dedup();
    Ok(out)
}

fn operator_admitted(display: &str, allowed: &[String], denied: &[String]) -> bool {
    let upper = display.to_ascii_uppercase();
    let admitted = allowed.is_empty() || allowed.iter().any(|prefix| upper.starts_with(prefix));
    let refused = denied.iter().any(|prefix| upper.starts_with(prefix));
    admitted && !refused
}

fn digest_bytes(bytes: &[u8]) -> String {
    blake3::hash(bytes).to_hex().to_string()
}

fn validate_telco_name(field: &str, value: &str) -> Result<(), String> {
    if value.is_empty() || value.len() > 256 {
        return Err(format!("{field} must contain 1..=256 characters"));
    }
    if !value.bytes().all(|byte| {
        byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.' | b':' | b'/' | b'@')
    }) {
        return Err(format!("{field} contains a non-canonical character"));
    }
    Ok(())
}

fn telco_basis(envelope: &TelcoEnvelope) -> Result<Value, String> {
    Ok(json!({
        "schema": envelope.schema,
        "version": envelope.version,
        "sender": envelope.sender,
        "recipient": envelope.recipient,
        "channel": envelope.channel,
        "issued_at_ms": envelope.issued_at_ms,
        "expires_at_ms": envelope.expires_at_ms,
        "correlation_id": envelope.correlation_id,
        "causation_id": envelope.causation_id,
        "predecessor_digest": envelope.predecessor_digest,
        "idempotency_key": envelope.idempotency_key,
        "payload": envelope.payload,
        "payload_digest": envelope.payload_digest,
        "integrity": envelope.integrity,
        "authentication": envelope.authentication
    }))
}
