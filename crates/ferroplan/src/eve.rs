//! Eve: the relational entry point into a Genesis-defined planning world.
//!
//! Eve does not plan, manufacture, actuate, observe, conform, admit, or replay.
//! It deterministically binds human purpose to a pre-existing Genesis world and
//! emits the typed handoff consumed by the formal lifecycle that owns those acts.

use blake3::Hasher;
use serde::{Deserialize, Serialize};

const EVE_PROTOCOL: &str = "ferroplan.eve-genesis.v1";

/// A local Eve reflex may have at most eight primary activators.
/// A ninth activator requires the closure to be split.
pub const MAX_PRIMARY_ACTIVATORS: usize = 8;

/// Human purpose before it is grounded in the Genesis world.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct HumanPurpose {
    pub statement: String,
    pub desired_consequence: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub actor: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub activators: Vec<Activator>,
}

/// One bounded condition presented to Eve.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct Activator {
    pub name: String,
    pub value: String,
}

/// The created world Eve makes relationally accessible.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct GenesisWorld {
    pub ontology_rdf: String,
    pub construct_query: String,
    pub hddl: HddlSurface,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ppddl: Option<PpddlSurface>,
}

/// HDDL source and the compound task representing the grounded purpose.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct HddlSurface {
    pub domain: String,
    pub problem: String,
    pub root_task: String,
}

/// PPDDL source used when the lawful world contains explicit uncertainty.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct PpddlSurface {
    pub domain: String,
    pub problem: String,
}

/// Requested ggen projection.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct ManufactureTarget {
    pub name: String,
    pub template: String,
    pub artifact_kind: String,
    pub output: String,
}

/// MCP+ capability boundary for the manufactured result.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct CapabilityTarget {
    pub capability: String,
    pub route: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub authority_scopes: Vec<String>,
}

/// Input to Eve's deterministic relational compiler.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct EveRequest {
    pub purpose: HumanPurpose,
    pub genesis: GenesisWorld,
    pub manufacture: ManufactureTarget,
    pub capability: CapabilityTarget,
}

/// Whether the grounded purpose uses deterministic planning or bounded PPDDL.
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum PlanningRegime {
    Deterministic,
    Probabilistic,
}

/// One ordered lifecycle boundary emitted by Eve.
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum EveStage {
    GroundHumanPurpose,
    ProjectGenesis,
    DecomposeHddl,
    GovernUncertaintyPpddl,
    ManufactureGgen,
    /// Expose a non-authoritative capability handoff through MCP+.
    ExposeMcpPlus,
    ActuateBrce,
    ObserveOcel2,
    ConformTruexKernel,
    AdmitReceipt,
    ReplayTruex,
}

/// Human purpose after Eve binds it to the selected Genesis world.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct GroundedGoal {
    pub statement: String,
    pub desired_consequence: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub actor: Option<String>,
    pub root_task: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub activators: Vec<Activator>,
}

/// Exact Genesis projection requested by Eve.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct GenesisProjection {
    pub ontology_rdf: String,
    pub construct_query: String,
}

/// Exact hierarchical decomposition request.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct HddlDecompositionRequest {
    pub domain: String,
    pub problem: String,
    pub root_task: String,
}

/// Exact probabilistic policy request, present only for uncertain worlds.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct PpddlPolicyRequest {
    pub domain: String,
    pub problem: String,
}

/// ggen manufacturing contract emitted by Eve.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct GgenManufacturingRequest {
    pub target: ManufactureTarget,
    pub closure_id: String,
    /// ggen constructs a candidate; it cannot self-admit execution.
    pub candidate_only: bool,
}

/// MCP+ handoff. The capability remains non-authoritative until Truex closes
/// the listed evidence, conformance, receipt, and replay obligations.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct McpPlusHandoff {
    pub target: CapabilityTarget,
    pub closure_id: String,
    pub ambient_authority: bool,
    pub brce_required: bool,
    pub receipt_obligations: Vec<String>,
}

/// Downstream consequence obligations owned by Truex rather than Eve.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct TruexContinuation {
    pub expected_process_geometry: String,
    pub observed_path_format: String,
    pub conformance_engine: String,
    pub terminal_authority: String,
    pub replay_required: bool,
}

/// Complete machine-readable handoff from Eve to the formal execution stack.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct EveHandoff {
    pub protocol: String,
    /// BLAKE3 identity of the complete, versioned request. It is not a receipt
    /// and does not prove execution.
    pub closure_id: String,
    pub planning_regime: PlanningRegime,
    pub stages: Vec<EveStage>,
    pub goal: GroundedGoal,
    pub genesis: GenesisProjection,
    pub hddl: HddlDecompositionRequest,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ppddl: Option<PpddlPolicyRequest>,
    pub ggen: GgenManufacturingRequest,
    pub mcp_plus: McpPlusHandoff,
    pub truex: TruexContinuation,
}

/// Deterministic split directive produced by `Need9 => Split`.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct SplitDirective {
    pub provided: usize,
    pub maximum: usize,
    pub groups: Vec<Vec<Activator>>,
}

/// Eve refuses malformed or unbounded ingress.
#[derive(thiserror::Error, Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[serde(tag = "kind", rename_all = "kebab-case")]
pub enum EveError {
    #[error("missing required field `{field}`")]
    Missing { field: String },
    #[error("Need9 => Split: {directive:?}")]
    SplitRequired { directive: SplitDirective },
}

/// Stateless deterministic Eve compiler.
#[derive(Clone, Copy, Debug, Default)]
pub struct Eve;

impl Eve {
    /// Compile human purpose into a typed handoff across Genesis, HDDL, optional
    /// PPDDL, ggen, MCP+, and the downstream Truex lifecycle.
    ///
    /// This method performs no actuation and emits no receipt.
    pub fn enter(request: EveRequest) -> Result<EveHandoff, EveError> {
        validate_request(&request)?;

        if request.purpose.activators.len() > MAX_PRIMARY_ACTIVATORS {
            return Err(EveError::SplitRequired {
                directive: SplitDirective {
                    provided: request.purpose.activators.len(),
                    maximum: MAX_PRIMARY_ACTIVATORS,
                    groups: request
                        .purpose
                        .activators
                        .chunks(MAX_PRIMARY_ACTIVATORS)
                        .map(<[Activator]>::to_vec)
                        .collect(),
                },
            });
        }

        validate_bounded_members(&request)?;

        let closure_id = closure_id(&request);
        let planning_regime = if request.genesis.ppddl.is_some() {
            PlanningRegime::Probabilistic
        } else {
            PlanningRegime::Deterministic
        };
        let stages = lifecycle_stages(planning_regime);

        Ok(EveHandoff {
            protocol: EVE_PROTOCOL.to_string(),
            closure_id: closure_id.clone(),
            planning_regime,
            stages,
            goal: GroundedGoal {
                statement: request.purpose.statement,
                desired_consequence: request.purpose.desired_consequence,
                actor: request.purpose.actor,
                root_task: request.genesis.hddl.root_task.clone(),
                activators: request.purpose.activators,
            },
            genesis: GenesisProjection {
                ontology_rdf: request.genesis.ontology_rdf,
                construct_query: request.genesis.construct_query,
            },
            hddl: HddlDecompositionRequest {
                domain: request.genesis.hddl.domain,
                problem: request.genesis.hddl.problem,
                root_task: request.genesis.hddl.root_task,
            },
            ppddl: request.genesis.ppddl.map(|surface| PpddlPolicyRequest {
                domain: surface.domain,
                problem: surface.problem,
            }),
            ggen: GgenManufacturingRequest {
                target: request.manufacture,
                closure_id: closure_id.clone(),
                candidate_only: true,
            },
            mcp_plus: McpPlusHandoff {
                target: request.capability,
                closure_id,
                ambient_authority: false,
                brce_required: true,
                receipt_obligations: vec![
                    "artifact-materialized".to_string(),
                    "boundary-evidence".to_string(),
                    "ocel2-observed-path".to_string(),
                    "powl-conformance".to_string(),
                    "receipt-admission-or-refusal".to_string(),
                    "replay".to_string(),
                ],
            },
            truex: TruexContinuation {
                expected_process_geometry: "POWL-v2".to_string(),
                observed_path_format: "OCEL-2.0".to_string(),
                conformance_engine: "Truex-Kernel/wasm4pm".to_string(),
                terminal_authority: "receipt-admission-or-refusal".to_string(),
                replay_required: true,
            },
        })
    }
}

fn validate_request(request: &EveRequest) -> Result<(), EveError> {
    validate_required("purpose.statement", &request.purpose.statement)?;
    validate_required(
        "purpose.desired_consequence",
        &request.purpose.desired_consequence,
    )?;
    if let Some(actor) = &request.purpose.actor {
        validate_required("purpose.actor", actor)?;
    }

    validate_required("genesis.ontology_rdf", &request.genesis.ontology_rdf)?;
    validate_required("genesis.construct_query", &request.genesis.construct_query)?;
    validate_required("genesis.hddl.domain", &request.genesis.hddl.domain)?;
    validate_required("genesis.hddl.problem", &request.genesis.hddl.problem)?;
    validate_required("genesis.hddl.root_task", &request.genesis.hddl.root_task)?;
    if let Some(ppddl) = &request.genesis.ppddl {
        validate_required("genesis.ppddl.domain", &ppddl.domain)?;
        validate_required("genesis.ppddl.problem", &ppddl.problem)?;
    }

    validate_required("manufacture.name", &request.manufacture.name)?;
    validate_required("manufacture.template", &request.manufacture.template)?;
    validate_required(
        "manufacture.artifact_kind",
        &request.manufacture.artifact_kind,
    )?;
    validate_required("manufacture.output", &request.manufacture.output)?;
    validate_required("capability.capability", &request.capability.capability)?;
    validate_required("capability.route", &request.capability.route)?;
    Ok(())
}

fn validate_bounded_members(request: &EveRequest) -> Result<(), EveError> {
    for (index, activator) in request.purpose.activators.iter().enumerate() {
        validate_required(
            &format!("purpose.activators[{index}].name"),
            &activator.name,
        )?;
        validate_required(
            &format!("purpose.activators[{index}].value"),
            &activator.value,
        )?;
    }
    for (index, scope) in request.capability.authority_scopes.iter().enumerate() {
        validate_required(&format!("capability.authority_scopes[{index}]"), scope)?;
    }
    Ok(())
}

fn validate_required(field: &str, value: &str) -> Result<(), EveError> {
    if value.trim().is_empty() {
        Err(EveError::Missing {
            field: field.to_string(),
        })
    } else {
        Ok(())
    }
}

fn lifecycle_stages(regime: PlanningRegime) -> Vec<EveStage> {
    let mut stages = vec![
        EveStage::GroundHumanPurpose,
        EveStage::ProjectGenesis,
        EveStage::DecomposeHddl,
    ];
    if regime == PlanningRegime::Probabilistic {
        stages.push(EveStage::GovernUncertaintyPpddl);
    }
    stages.extend([
        EveStage::ManufactureGgen,
        EveStage::ExposeMcpPlus,
        EveStage::ActuateBrce,
        EveStage::ObserveOcel2,
        EveStage::ConformTruexKernel,
        EveStage::AdmitReceipt,
        EveStage::ReplayTruex,
    ]);
    stages
}

fn closure_id(request: &EveRequest) -> String {
    let mut hasher = Hasher::new();
    hash_field(&mut hasher, EVE_PROTOCOL);
    hash_field(&mut hasher, &request.purpose.statement);
    hash_field(&mut hasher, &request.purpose.desired_consequence);
    hash_optional(&mut hasher, request.purpose.actor.as_deref());

    hash_count(&mut hasher, request.purpose.activators.len());
    for activator in &request.purpose.activators {
        hash_field(&mut hasher, &activator.name);
        hash_field(&mut hasher, &activator.value);
    }

    hash_field(&mut hasher, &request.genesis.ontology_rdf);
    hash_field(&mut hasher, &request.genesis.construct_query);
    hash_field(&mut hasher, &request.genesis.hddl.domain);
    hash_field(&mut hasher, &request.genesis.hddl.problem);
    hash_field(&mut hasher, &request.genesis.hddl.root_task);
    match &request.genesis.ppddl {
        Some(ppddl) => {
            hash_tag(&mut hasher, 1);
            hash_field(&mut hasher, &ppddl.domain);
            hash_field(&mut hasher, &ppddl.problem);
        }
        None => hash_tag(&mut hasher, 0),
    }

    hash_field(&mut hasher, &request.manufacture.name);
    hash_field(&mut hasher, &request.manufacture.template);
    hash_field(&mut hasher, &request.manufacture.artifact_kind);
    hash_field(&mut hasher, &request.manufacture.output);
    hash_field(&mut hasher, &request.capability.capability);
    hash_field(&mut hasher, &request.capability.route);

    hash_count(&mut hasher, request.capability.authority_scopes.len());
    for scope in &request.capability.authority_scopes {
        hash_field(&mut hasher, scope);
    }

    format!("eve:{}", hasher.finalize().to_hex())
}

fn hash_optional(hasher: &mut Hasher, value: Option<&str>) {
    match value {
        Some(value) => {
            hash_tag(hasher, 1);
            hash_field(hasher, value);
        }
        None => hash_tag(hasher, 0),
    }
}

fn hash_count(hasher: &mut Hasher, count: usize) {
    hasher.update(&(count as u64).to_le_bytes());
}

fn hash_tag(hasher: &mut Hasher, tag: u8) {
    hasher.update(&[tag]);
}

fn hash_field(hasher: &mut Hasher, value: &str) {
    hasher.update(&(value.len() as u64).to_le_bytes());
    hasher.update(value.as_bytes());
}
