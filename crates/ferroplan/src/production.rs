//! Bounded production wrappers for the public capability families that are not
//! ordinary deterministic solves.
//!
//! Compatibility APIs remain available in their original modules. These
//! wrappers are the service-grade entry points: they bound untrusted inputs,
//! return versioned typed envelopes, independently validate candidate outputs,
//! and never grant execution authority.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::api::{Decomposition, Options, ParseReport, Solution};
use crate::ppddl::{ProbabilisticOptions, ProbabilisticSolution};
use crate::readiness::{
    capability_manifest, BuildIdentity, OperationEnvelope, OutcomeClass, ProductionLimits,
    PublicError, ValidationStatus, CANDIDATE_AUTHORITY, OPERATION_ENVELOPE_SCHEMA,
};
use crate::{Plan, Session, StateSnapshot};

const PRODUCTION_SURFACE_HASH_DOMAIN: &[u8] = b"ferroplan.production-surface.v1\0";
const HARD_INPUT_BYTES: usize = 64 * 1024 * 1024;
const HARD_PLAN_BYTES: usize = 64 * 1024 * 1024;
const HARD_TRACE_STEPS: usize = 100_000;
const HARD_PPDDL_STATES: usize = 2_000_000;
const HARD_PPDDL_TRANSITIONS: usize = 50_000_000;
const HARD_PPDDL_POLICY_ENTRIES: usize = 2_000_000;
const HARD_PPDDL_VALUE_CELLS: usize = 100_000_000;
const HARD_PPDDL_HORIZON: usize = 100_000;
const HARD_SESSION_MEMORY_MB: usize = 16 * 1024;
const MAX_DIAGNOSTIC_BYTES: usize = 2_048;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct PlanValidationEvidence {
    pub valid: bool,
    pub reason: Option<String>,
}

/// Bound and parse one PDDL document without grounding or search.
pub fn parse_production(
    source: &str,
    max_input_bytes: usize,
    request_id: Option<&str>,
) -> OperationEnvelope<ParseReport> {
    let clock = crate::clock::Clock::now();
    let fingerprint = surface_fingerprint("fp.core.parse", &[source.as_bytes()]);
    let mut envelope = new_envelope(
        "fp.core.parse",
        "evidence_only",
        request_id,
        &fingerprint,
        elapsed(&clock),
    );
    if let Some(error) = validate_byte_limit("PDDL source", source.len(), max_input_bytes) {
        envelope.outcome = OutcomeClass::Refused;
        envelope.error = Some(error);
        return envelope;
    }
    let report = crate::parse(source);
    if report.ok {
        envelope.outcome = OutcomeClass::Solved;
        envelope.validation = ValidationStatus::Valid;
        envelope.payload = Some(report);
    } else {
        envelope.outcome = OutcomeClass::Refused;
        envelope.validation = ValidationStatus::Failed;
        envelope.error = Some(PublicError::new(
            "FP_PARSE",
            report
                .error
                .as_deref()
                .unwrap_or("PDDL source failed parsing"),
            false,
        ));
    }
    envelope.elapsed_micros = elapsed(&clock);
    envelope
}

/// Independently validate one textual plan under bounded input sizes.
pub fn validate_plan_production(
    domain: &str,
    problem: &str,
    plan: &str,
    max_input_bytes: usize,
    max_plan_bytes: usize,
    request_id: Option<&str>,
) -> OperationEnvelope<PlanValidationEvidence> {
    let clock = crate::clock::Clock::now();
    let fingerprint = surface_fingerprint(
        "fp.core.validate",
        &[domain.as_bytes(), problem.as_bytes(), plan.as_bytes()],
    );
    let mut envelope = new_envelope(
        "fp.core.validate",
        "evidence_only",
        request_id,
        &fingerprint,
        elapsed(&clock),
    );
    if let Some(error) = validate_model_inputs(domain, problem, max_input_bytes)
        .or_else(|| validate_byte_limit("plan", plan.len(), max_plan_bytes))
    {
        envelope.outcome = OutcomeClass::Refused;
        envelope.error = Some(error);
        return envelope;
    }
    match crate::plan::validate_plan(domain, problem, plan) {
        Ok(crate::plan::Validity::Valid) => {
            envelope.outcome = OutcomeClass::Solved;
            envelope.validation = ValidationStatus::Valid;
            envelope.payload = Some(PlanValidationEvidence {
                valid: true,
                reason: None,
            });
        }
        Ok(crate::plan::Validity::Invalid(reason)) => {
            envelope.outcome = OutcomeClass::Refused;
            envelope.validation = ValidationStatus::Failed;
            envelope.error = Some(PublicError::new("FP_VALIDATION", reason, false));
        }
        Err(error) => {
            envelope.outcome = OutcomeClass::Refused;
            envelope.validation = ValidationStatus::Failed;
            envelope.error = Some(PublicError::new("FP_VALIDATION", error, false));
        }
    }
    envelope.elapsed_micros = elapsed(&clock);
    envelope
}

/// Replay a bounded sequential plan and return the observed state snapshots.
pub fn trace_production(
    domain: &str,
    problem: &str,
    plan: &[(String, Vec<String>)],
    limits: &ProductionLimits,
    request_id: Option<&str>,
) -> OperationEnvelope<Vec<StateSnapshot>> {
    let clock = crate::clock::Clock::now();
    let plan_bytes = serde_json::to_vec(plan).unwrap_or_default();
    let fingerprint = surface_fingerprint(
        "fp.core.trace",
        &[domain.as_bytes(), problem.as_bytes(), &plan_bytes],
    );
    let mut envelope = new_envelope(
        "fp.core.trace",
        "evidence_only",
        request_id,
        &fingerprint,
        elapsed(&clock),
    );
    if let Err(error) = limits.validate() {
        envelope.outcome = OutcomeClass::Refused;
        envelope.error = Some(error);
        return envelope;
    }
    if let Some(error) =
        validate_model_inputs(domain, problem, limits.max_domain_bytes).or_else(|| {
            if plan.len() > limits.max_plan_steps.min(HARD_TRACE_STEPS) {
                Some(PublicError::new(
                    "FP_LIMIT_PLAN",
                    format!(
                        "trace contains {} steps; maximum is {}",
                        plan.len(),
                        limits.max_plan_steps.min(HARD_TRACE_STEPS)
                    ),
                    false,
                ))
            } else {
                None
            }
        })
    {
        envelope.outcome = OutcomeClass::Refused;
        envelope.error = Some(error);
        return envelope;
    }
    match crate::trace(domain, problem, plan) {
        Ok(snapshots) => {
            envelope.outcome = OutcomeClass::Solved;
            envelope.validation = ValidationStatus::Valid;
            envelope
                .counters
                .insert("snapshots".to_string(), saturating_u64(snapshots.len()));
            envelope.payload = Some(snapshots);
        }
        Err(error) => {
            envelope.outcome = OutcomeClass::Refused;
            envelope.validation = ValidationStatus::Failed;
            envelope.error = Some(PublicError::new("FP_VALIDATION", error, false));
        }
    }
    envelope.elapsed_micros = elapsed(&clock);
    enforce_output_limit(&mut envelope, limits.max_output_bytes);
    envelope
}

/// Decompose and independently validate the stitched candidate under a bounded
/// deterministic work profile.
pub fn decompose_production(
    domain: &str,
    problem: &str,
    options: &Options,
    limits: &ProductionLimits,
    request_id: Option<&str>,
) -> OperationEnvelope<Decomposition> {
    let clock = crate::clock::Clock::now();
    let option_bytes =
        serde_json::to_vec(options).unwrap_or_else(|_| format!("{options:?}").into());
    let fingerprint = surface_fingerprint(
        "fp.core.decompose",
        &[domain.as_bytes(), problem.as_bytes(), &option_bytes],
    );
    let mut envelope = new_envelope(
        "fp.core.decompose",
        CANDIDATE_AUTHORITY,
        request_id,
        &fingerprint,
        elapsed(&clock),
    );
    if let Err(error) = limits.validate() {
        envelope.outcome = OutcomeClass::Refused;
        envelope.error = Some(error);
        return envelope;
    }
    if let Some(error) = validate_model_inputs(domain, problem, limits.max_domain_bytes)
        .or_else(|| validate_options(options, limits))
    {
        envelope.outcome = OutcomeClass::Refused;
        envelope.error = Some(error);
        return envelope;
    }
    let mut bounded_options = options.clone();
    bounded_options.threads = bounded_options.threads.max(1);
    bounded_options.max_evaluated = Some(
        bounded_options
            .max_evaluated
            .unwrap_or(limits.max_evaluated)
            .min(limits.max_evaluated),
    );
    match crate::decompose(domain, problem, &bounded_options) {
        Err(error) => {
            envelope.outcome = OutcomeClass::Refused;
            envelope.error = Some(map_solve_error(error));
        }
        Ok(decomposition) if !decomposition.solved => {
            envelope.outcome = OutcomeClass::NoPlan;
            envelope.payload = Some(decomposition);
        }
        Ok(decomposition) => {
            let Some(plan) = decomposition.plan.as_ref() else {
                envelope.outcome = OutcomeClass::Failed;
                envelope.validation = ValidationStatus::Failed;
                envelope.error = Some(PublicError::new(
                    "FP_INVARIANT",
                    "decomposition reported solved without a stitched plan",
                    false,
                ));
                return envelope;
            };
            if plan.length > limits.max_plan_steps || plan.length != plan.steps.len() {
                envelope.outcome = OutcomeClass::LimitExceeded;
                envelope.error = Some(PublicError::new(
                    "FP_LIMIT_PLAN",
                    "stitched decomposition plan exceeds or violates the plan contract",
                    false,
                ));
                return envelope;
            }
            match validate_structured_plan(domain, problem, plan) {
                Ok(()) => {
                    envelope.outcome = OutcomeClass::Solved;
                    envelope.validation = ValidationStatus::Valid;
                    envelope.payload = Some(decomposition);
                }
                Err(error) => {
                    envelope.outcome = OutcomeClass::Failed;
                    envelope.validation = ValidationStatus::Failed;
                    envelope.error = Some(PublicError::new("FP_VALIDATION", error, false));
                }
            }
        }
    }
    envelope.elapsed_micros = elapsed(&clock);
    enforce_output_limit(&mut envelope, limits.max_output_bytes);
    envelope
}

/// Synthesize a bounded PPDDL policy and independently validate its transition
/// probabilities and policy structure before returning success.
pub fn solve_ppddl_production(
    domain: &str,
    problem: &str,
    options: &ProbabilisticOptions,
    max_input_bytes: usize,
    max_output_bytes: usize,
    request_id: Option<&str>,
) -> OperationEnvelope<ProbabilisticSolution> {
    let clock = crate::clock::Clock::now();
    let option_bytes =
        serde_json::to_vec(options).unwrap_or_else(|_| format!("{options:?}").into());
    let fingerprint = surface_fingerprint(
        "fp.core.ppddl",
        &[domain.as_bytes(), problem.as_bytes(), &option_bytes],
    );
    let mut envelope = new_envelope(
        "fp.core.ppddl",
        CANDIDATE_AUTHORITY,
        request_id,
        &fingerprint,
        elapsed(&clock),
    );
    if let Some(error) = validate_model_inputs(domain, problem, max_input_bytes)
        .or_else(|| validate_ppddl_limits(options, max_output_bytes))
    {
        envelope.outcome = OutcomeClass::Refused;
        envelope.error = Some(error);
        return envelope;
    }
    match crate::solve_ppddl(domain, problem, options) {
        Err(error) => {
            envelope.outcome = OutcomeClass::Refused;
            envelope.error = Some(PublicError::new("FP_MODEL", error.to_string(), false));
        }
        Ok(solution) => match crate::validate_ppddl_policy(domain, problem, options, &solution) {
            Ok(validation) if validation.valid => {
                envelope.outcome = OutcomeClass::Solved;
                envelope.validation = ValidationStatus::Valid;
                envelope.counters.insert(
                    "reachable_states".to_string(),
                    saturating_u64(solution.statistics.reachable_states),
                );
                envelope.counters.insert(
                    "transitions".to_string(),
                    saturating_u64(solution.statistics.transitions),
                );
                envelope.payload = Some(solution);
            }
            Ok(validation) => {
                envelope.outcome = OutcomeClass::Failed;
                envelope.validation = ValidationStatus::Failed;
                envelope.error = Some(PublicError::new(
                    "FP_VALIDATION",
                    validation
                        .errors
                        .first()
                        .map(String::as_str)
                        .unwrap_or("PPDDL policy validation failed"),
                    false,
                ));
            }
            Err(error) => {
                envelope.outcome = OutcomeClass::Failed;
                envelope.validation = ValidationStatus::Failed;
                envelope.error = Some(PublicError::new("FP_VALIDATION", error.to_string(), false));
            }
        },
    }
    envelope.elapsed_micros = elapsed(&clock);
    enforce_output_limit(&mut envelope, max_output_bytes);
    envelope
}

/// Production wrapper around a persistent grounded world. It intentionally
/// exposes only bounded replanning and pure inspection; compatibility Session
/// methods that mutate state remain available on the legacy type.
pub struct ProductionSession {
    inner: Session,
    domain: String,
    problem: String,
    limits: ProductionLimits,
    input_fingerprint: String,
}

impl ProductionSession {
    pub fn new(
        domain: &str,
        problem: &str,
        options: &Options,
        limits: ProductionLimits,
    ) -> Result<Self, PublicError> {
        limits.validate()?;
        if let Some(error) = validate_model_inputs(domain, problem, limits.max_domain_bytes)
            .or_else(|| validate_options(options, &limits))
        {
            return Err(error);
        }
        let mut bounded_options = options.clone();
        bounded_options.threads = bounded_options.threads.max(1);
        bounded_options.max_evaluated = Some(
            bounded_options
                .max_evaluated
                .unwrap_or(limits.max_evaluated)
                .min(limits.max_evaluated),
        );
        let input_fingerprint = surface_fingerprint(
            "fp.core.session",
            &[
                domain.as_bytes(),
                problem.as_bytes(),
                &serde_json::to_vec(&bounded_options)
                    .unwrap_or_else(|_| format!("{bounded_options:?}").into()),
            ],
        );
        let inner = Session::new(domain, problem, &bounded_options)
            .map_err(|error| PublicError::new("FP_MODEL", error.to_string(), false))?;
        Ok(Self {
            inner,
            domain: domain.to_string(),
            problem: problem.to_string(),
            limits,
            input_fingerprint,
        })
    }

    pub fn replan(
        &self,
        max_evaluated: usize,
        memory_mb: Option<usize>,
        request_id: Option<&str>,
    ) -> OperationEnvelope<Solution> {
        let clock = crate::clock::Clock::now();
        let mut envelope = new_envelope(
            "fp.core.session",
            CANDIDATE_AUTHORITY,
            request_id,
            &self.input_fingerprint,
            elapsed(&clock),
        );
        if max_evaluated == 0 || max_evaluated > self.limits.max_evaluated {
            envelope.outcome = OutcomeClass::Refused;
            envelope.error = Some(PublicError::new(
                "FP_LIMIT_SEARCH",
                format!(
                    "session max_evaluated must be in 1..={} ",
                    self.limits.max_evaluated
                ),
                false,
            ));
            return envelope;
        }
        if memory_mb.is_some_and(|value| value == 0 || value > HARD_SESSION_MEMORY_MB) {
            envelope.outcome = OutcomeClass::Refused;
            envelope.error = Some(PublicError::new(
                "FP_LIMIT_MEMORY",
                format!("session memory_mb must be in 1..={HARD_SESSION_MEMORY_MB}"),
                false,
            ));
            return envelope;
        }
        let solution = self.inner.replan_budgeted(max_evaluated, memory_mb);
        envelope.counters.insert(
            "evaluated_states".to_string(),
            saturating_u64(solution.statistics.evaluated_states),
        );
        if !solution.solved {
            envelope.outcome = if solution.statistics.evaluated_states >= max_evaluated {
                OutcomeClass::LimitExceeded
            } else {
                OutcomeClass::NoPlan
            };
            envelope.payload = Some(solution);
        } else {
            let Some(plan) = solution.plan.as_ref() else {
                envelope.outcome = OutcomeClass::Failed;
                envelope.error = Some(PublicError::new(
                    "FP_INVARIANT",
                    "session reported solved without a plan",
                    false,
                ));
                return envelope;
            };
            if plan.length > self.limits.max_plan_steps {
                envelope.outcome = OutcomeClass::LimitExceeded;
                envelope.error = Some(PublicError::new(
                    "FP_LIMIT_PLAN",
                    "session plan exceeds the configured plan limit",
                    true,
                ));
                return envelope;
            }
            match validate_structured_plan(&self.domain, &self.problem, plan) {
                Ok(()) => {
                    envelope.outcome = OutcomeClass::Solved;
                    envelope.validation = ValidationStatus::Valid;
                    envelope.payload = Some(solution);
                }
                Err(error) => {
                    envelope.outcome = OutcomeClass::Failed;
                    envelope.validation = ValidationStatus::Failed;
                    envelope.error = Some(PublicError::new("FP_VALIDATION", error, false));
                }
            }
        }
        envelope.elapsed_micros = elapsed(&clock);
        enforce_output_limit(&mut envelope, self.limits.max_output_bytes);
        envelope
    }

    pub fn goal_met(&self) -> bool {
        self.inner.goal_met()
    }

    pub fn world_bytes(&self) -> usize {
        self.inner.world_bytes()
    }

    pub fn mind_bytes(&self) -> usize {
        self.inner.mind_bytes()
    }
}

fn validate_model_inputs(domain: &str, problem: &str, max_bytes: usize) -> Option<PublicError> {
    validate_byte_limit("domain", domain.len(), max_bytes)
        .or_else(|| validate_byte_limit("problem", problem.len(), max_bytes))
        .or_else(|| {
            if domain.is_empty() || problem.is_empty() {
                Some(PublicError::new(
                    "FP_INVALID_REQUEST",
                    "domain and problem must be non-empty",
                    false,
                ))
            } else {
                None
            }
        })
}

fn validate_byte_limit(label: &str, actual: usize, configured: usize) -> Option<PublicError> {
    if configured == 0 || configured > HARD_INPUT_BYTES.max(HARD_PLAN_BYTES) {
        return Some(PublicError::new(
            "FP_INVALID_REQUEST",
            format!("{label} limit must be in 1..={HARD_INPUT_BYTES}"),
            false,
        ));
    }
    if actual > configured {
        Some(PublicError::new(
            "FP_LIMIT_INPUT",
            format!("{label} is {actual} bytes; maximum is {configured}"),
            false,
        ))
    } else {
        None
    }
}

fn validate_options(options: &Options, limits: &ProductionLimits) -> Option<PublicError> {
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
        .is_some_and(|value| value == 0 || value > limits.max_evaluated)
    {
        return Some(PublicError::new(
            "FP_LIMIT_SEARCH",
            format!("max_evaluated must be in 1..={}", limits.max_evaluated),
            false,
        ));
    }
    None
}

fn validate_ppddl_limits(
    options: &ProbabilisticOptions,
    max_output_bytes: usize,
) -> Option<PublicError> {
    if max_output_bytes == 0 || max_output_bytes > 64 * 1024 * 1024 {
        return Some(PublicError::new(
            "FP_INVALID_REQUEST",
            "max_output_bytes must be in 1..=67108864",
            false,
        ));
    }
    if options.threads > 64
        || options.max_states > HARD_PPDDL_STATES
        || options.max_transitions > HARD_PPDDL_TRANSITIONS
        || options.max_policy_entries > HARD_PPDDL_POLICY_ENTRIES
        || options.max_value_cells > HARD_PPDDL_VALUE_CELLS
        || options
            .horizon
            .is_some_and(|value| value > HARD_PPDDL_HORIZON)
    {
        return Some(PublicError::new(
            "FP_LIMIT_SEARCH",
            "PPDDL options exceed the production hard limits",
            false,
        ));
    }
    None
}

fn validate_structured_plan(domain: &str, problem: &str, plan: &Plan) -> Result<(), String> {
    if plan.length != plan.steps.len() {
        return Err("plan length does not match emitted step count".to_string());
    }
    let text = render_plan(plan);
    match crate::plan::validate_plan(domain, problem, &text)? {
        crate::plan::Validity::Valid => Ok(()),
        crate::plan::Validity::Invalid(reason) => Err(reason),
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

fn map_solve_error(error: crate::SolveError) -> PublicError {
    match error {
        crate::SolveError::DomainParse(error) => {
            PublicError::new("FP_PARSE", format!("domain parse error: {error}"), false)
        }
        crate::SolveError::ProblemParse(error) => {
            PublicError::new("FP_PARSE", format!("problem parse error: {error}"), false)
        }
        crate::SolveError::Unsupported(message) => {
            PublicError::new("FP_UNSUPPORTED", message, false)
        }
        crate::SolveError::EmptyType { kind, pred, ty } => PublicError::new(
            "FP_MODEL",
            format!("{kind} {pred} uses an unknown or empty type {ty}"),
            false,
        ),
        crate::SolveError::Derived(message) => PublicError::new("FP_MODEL", message, false),
    }
}

fn new_envelope<T>(
    capability_id: &str,
    authority: &str,
    request_id: Option<&str>,
    fingerprint: &str,
    elapsed_micros: u64,
) -> OperationEnvelope<T> {
    let manifest_fingerprint = capability_manifest().fingerprint().ok();
    OperationEnvelope {
        schema_version: OPERATION_ENVELOPE_SCHEMA.to_string(),
        request_id: request_id
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(|value| truncate_utf8(value, 128))
            .unwrap_or_else(|| format!("req-{}", &fingerprint[..16.min(fingerprint.len())])),
        capability_id: capability_id.to_string(),
        capability_version: env!("CARGO_PKG_VERSION").to_string(),
        build_identity: BuildIdentity {
            product_version: env!("CARGO_PKG_VERSION").to_string(),
            source_revision: option_env!("FERROPLAN_BUILD_SHA")
                .filter(|value| !value.trim().is_empty())
                .map(ToOwned::to_owned),
            manifest_fingerprint,
        },
        input_fingerprint: fingerprint.to_string(),
        authority: authority.to_string(),
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
    match serde_json::to_vec(&*envelope) {
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

fn surface_fingerprint(capability_id: &str, parts: &[&[u8]]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(PRODUCTION_SURFACE_HASH_DOMAIN);
    hasher.update((capability_id.len() as u64).to_be_bytes());
    hasher.update(capability_id.as_bytes());
    for part in parts {
        hasher.update((part.len() as u64).to_be_bytes());
        hasher.update(part);
    }
    format!("{:x}", hasher.finalize())
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
    fn parse_boundary_refuses_before_parser_allocation() {
        let envelope = parse_production(DOMAIN, 8, None);
        assert_eq!(envelope.outcome, OutcomeClass::Refused);
        assert_eq!(
            envelope.error.as_ref().map(|error| error.code.as_str()),
            Some("FP_LIMIT_INPUT")
        );
    }

    #[test]
    fn production_session_requires_explicit_budgets_and_validates_replay() {
        let session = ProductionSession::new(
            DOMAIN,
            PROBLEM,
            &Options::default(),
            ProductionLimits::default(),
        )
        .unwrap();
        let envelope = session.replan(1_000, Some(64), Some("session-test"));
        assert_eq!(envelope.outcome, OutcomeClass::Solved);
        assert_eq!(envelope.validation, ValidationStatus::Valid);
        assert_eq!(envelope.authority, CANDIDATE_AUTHORITY);
    }
}
