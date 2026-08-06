//! Bounded production explanation and decomposition surfaces.
//!
//! Explanations are evidence-only. Decompositions are candidate-only. Both
//! independently validate their plan under semantics derived from the input
//! domain rather than inferring temporal semantics from returned timestamps.

use std::collections::BTreeMap;

use serde::Serialize;
use sha2::{Digest, Sha256};

use crate::introspect::Explanation;
use crate::{
    capability_manifest, BuildIdentity, Decomposition, OperationEnvelope, Options, OutcomeClass,
    Plan, ProductionLimits, PublicError, SolveError, ValidationStatus, CANDIDATE_AUTHORITY,
    OPERATION_ENVELOPE_SCHEMA,
};

const EXPLAIN_HASH_DOMAIN: &[u8] = b"ferroplan.production-explain.v1\0";
const DECOMPOSE_HASH_DOMAIN: &[u8] = b"ferroplan.production-decompose.v1\0";
const MAX_REQUEST_ID_BYTES: usize = 128;

pub fn explain_production(
    domain: &str,
    problem: &str,
    plan: &Plan,
    limits: &ProductionLimits,
    request_id: Option<&str>,
) -> OperationEnvelope<Explanation> {
    let clock = crate::clock::Clock::now();
    let plan_bytes = serde_json::to_vec(plan).unwrap_or_else(|_| format!("{plan:?}").into());
    let input_fingerprint = fingerprint(
        EXPLAIN_HASH_DOMAIN,
        &[domain.as_bytes(), problem.as_bytes(), &plan_bytes],
    );
    let mut envelope = new_envelope(
        "fp.core.explain",
        "evidence_only",
        request_id,
        &input_fingerprint,
        elapsed(&clock),
    );
    if envelope.error.is_some() {
        return envelope;
    }
    if let Err(error) = limits.validate() {
        envelope.outcome = OutcomeClass::Refused;
        envelope.error = Some(error);
        return envelope;
    }
    if let Some(error) = validate_model_inputs(domain, problem, limits) {
        envelope.outcome = OutcomeClass::Refused;
        envelope.error = Some(error);
        return envelope;
    }
    if let Some(error) = validate_plan_shape(plan, limits.max_plan_steps) {
        envelope.outcome = if error.code == "FP_LIMIT_PLAN" {
            OutcomeClass::LimitExceeded
        } else {
            OutcomeClass::Failed
        };
        envelope.validation = ValidationStatus::Failed;
        envelope.error = Some(error);
        return envelope;
    }

    let temporal_domain = match domain_is_temporal(domain) {
        Ok(value) => value,
        Err(error) => {
            envelope.outcome = OutcomeClass::Refused;
            envelope.validation = ValidationStatus::Failed;
            envelope.error = Some(error);
            return envelope;
        }
    };
    if let Err(error) = validate_plan(domain, problem, plan, temporal_domain) {
        envelope.outcome = OutcomeClass::Refused;
        envelope.validation = ValidationStatus::Failed;
        envelope.error = Some(PublicError::new("FP_VALIDATION", error, false));
        return envelope;
    }

    match crate::introspect::explain(domain, problem, plan) {
        Ok(explanation) => {
            envelope.outcome = OutcomeClass::Solved;
            envelope.validation = ValidationStatus::Valid;
            envelope.payload = Some(explanation);
        }
        Err(error) => {
            envelope.outcome = OutcomeClass::Refused;
            envelope.validation = ValidationStatus::Failed;
            envelope.error = Some(PublicError::new("FP_VALIDATION", error, false));
        }
    }
    envelope.elapsed_micros = elapsed(&clock);
    enforce_output_limit(&mut envelope, limits.max_output_bytes, "explanation");
    envelope
}

/// Decompose a bounded goal and validate the stitched candidate under the
/// semantics declared by the input domain.
///
/// The temporal decomposer may represent a monolithic classical fallback with
/// timestamps. Those timestamps are representation metadata, not permission to
/// validate the plan as a temporal-domain plan.
pub fn decompose_production(
    domain: &str,
    problem: &str,
    options: &Options,
    limits: &ProductionLimits,
    request_id: Option<&str>,
) -> OperationEnvelope<Decomposition> {
    let clock = crate::clock::Clock::now();
    let options_bytes =
        serde_json::to_vec(options).unwrap_or_else(|_| format!("{options:?}").into_bytes());
    let input_fingerprint = fingerprint(
        DECOMPOSE_HASH_DOMAIN,
        &[domain.as_bytes(), problem.as_bytes(), &options_bytes],
    );
    let mut envelope = new_envelope(
        "fp.core.decompose",
        CANDIDATE_AUTHORITY,
        request_id,
        &input_fingerprint,
        elapsed(&clock),
    );
    if envelope.error.is_some() {
        return envelope;
    }
    if let Err(error) = limits.validate() {
        envelope.outcome = OutcomeClass::Refused;
        envelope.error = Some(error);
        return envelope;
    }
    if let Some(error) =
        validate_model_inputs(domain, problem, limits).or_else(|| validate_options(options, limits))
    {
        envelope.outcome = OutcomeClass::Refused;
        envelope.error = Some(error);
        return envelope;
    }

    let temporal_domain = match domain_is_temporal(domain) {
        Ok(value) => value,
        Err(error) => {
            envelope.outcome = OutcomeClass::Refused;
            envelope.validation = ValidationStatus::Failed;
            envelope.error = Some(error);
            return envelope;
        }
    };

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
                envelope.validation = ValidationStatus::Failed;
                envelope.error = Some(PublicError::new(
                    "FP_INVARIANT",
                    "decomposition reported solved without a stitched plan",
                    false,
                ));
                return envelope;
            };
            if let Some(error) = validate_plan_shape(plan, limits.max_plan_steps) {
                envelope.outcome = if error.code == "FP_LIMIT_PLAN" {
                    OutcomeClass::LimitExceeded
                } else {
                    OutcomeClass::Failed
                };
                envelope.validation = ValidationStatus::Failed;
                envelope.error = Some(error);
                return envelope;
            }
            match validate_plan(domain, problem, plan, temporal_domain) {
                Ok(()) => {
                    envelope.outcome = OutcomeClass::Solved;
                    envelope.validation = ValidationStatus::Valid;
                    envelope.counters.insert(
                        "contracts".to_string(),
                        decomposition.contracts.len().try_into().unwrap_or(u64::MAX),
                    );
                    envelope.counters.insert(
                        "plan_steps".to_string(),
                        plan.length.try_into().unwrap_or(u64::MAX),
                    );
                    envelope.payload = Some(decomposition);
                }
                Err(error) => {
                    envelope.validation = ValidationStatus::Failed;
                    envelope.error = Some(PublicError::new("FP_VALIDATION", error, false));
                }
            }
        }
    }

    envelope.elapsed_micros = elapsed(&clock);
    enforce_output_limit(&mut envelope, limits.max_output_bytes, "decomposition");
    envelope
}

fn validate_model_inputs(
    domain: &str,
    problem: &str,
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
                domain.len(),
                limits.max_domain_bytes
            ),
            false,
        ));
    }
    if problem.len() > limits.max_problem_bytes {
        return Some(PublicError::new(
            "FP_LIMIT_INPUT",
            format!(
                "problem is {} bytes; maximum is {}",
                problem.len(),
                limits.max_problem_bytes
            ),
            false,
        ));
    }
    None
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

fn validate_plan_shape(plan: &Plan, max_plan_steps: usize) -> Option<PublicError> {
    if plan.length != plan.steps.len() {
        return Some(PublicError::new(
            "FP_INVARIANT",
            "plan length does not match emitted step count",
            false,
        ));
    }
    if plan.length > max_plan_steps {
        return Some(PublicError::new(
            "FP_LIMIT_PLAN",
            format!(
                "plan contains {} steps; maximum is {max_plan_steps}",
                plan.length
            ),
            false,
        ));
    }
    None
}

fn domain_is_temporal(domain: &str) -> Result<bool, PublicError> {
    crate::parser::parse_domain(domain)
        .map(|parsed| crate::temporal::is_temporal(&parsed))
        .map_err(|error| {
            PublicError::new("FP_PARSE", format!("domain parse error: {error}"), false)
        })
}

fn validate_plan(
    domain: &str,
    problem: &str,
    plan: &Plan,
    temporal_domain: bool,
) -> Result<(), String> {
    let rendered = render_plan(plan, temporal_domain);
    match crate::plan::validate_plan(domain, problem, &rendered)? {
        crate::plan::Validity::Valid => Ok(()),
        crate::plan::Validity::Invalid(reason) => Err(reason),
    }
}

fn render_plan(plan: &Plan, temporal_domain: bool) -> String {
    let mut output = String::new();
    for step in &plan.steps {
        let args = if step.args.is_empty() {
            String::new()
        } else {
            format!(" {}", step.args.join(" "))
        };
        if temporal_domain {
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

fn new_envelope<T>(
    capability_id: &str,
    authority: &str,
    request_id: Option<&str>,
    input_fingerprint: &str,
    elapsed_micros: u64,
) -> OperationEnvelope<T> {
    let manifest_fingerprint = capability_manifest().fingerprint();
    let mut envelope = OperationEnvelope {
        schema_version: OPERATION_ENVELOPE_SCHEMA.to_string(),
        request_id: normalize_request_id(request_id, input_fingerprint),
        capability_id: capability_id.to_string(),
        capability_version: env!("CARGO_PKG_VERSION").to_string(),
        build_identity: BuildIdentity {
            product_version: env!("CARGO_PKG_VERSION").to_string(),
            source_revision: option_env!("FERROPLAN_BUILD_SHA")
                .filter(|value| !value.trim().is_empty())
                .map(ToOwned::to_owned),
            manifest_fingerprint: manifest_fingerprint.clone().ok(),
        },
        input_fingerprint: input_fingerprint.to_string(),
        authority: authority.to_string(),
        outcome: OutcomeClass::Failed,
        validation: ValidationStatus::NotApplicable,
        elapsed_micros,
        counters: BTreeMap::new(),
        warnings: Vec::new(),
        payload: None,
        error: None,
    };
    if let Err(error) = manifest_fingerprint {
        envelope.error = Some(PublicError::new("FP_INVARIANT", error.to_string(), false));
    }
    envelope
}

fn enforce_output_limit<T: Serialize>(
    envelope: &mut OperationEnvelope<T>,
    max_bytes: usize,
    operation: &str,
) {
    match serde_json::to_vec(&*envelope) {
        Ok(bytes) if bytes.len() <= max_bytes => {}
        Ok(bytes) => {
            envelope.payload = None;
            envelope.outcome = OutcomeClass::LimitExceeded;
            envelope.validation = ValidationStatus::NotApplicable;
            envelope.error = Some(PublicError::new(
                "FP_LIMIT_OUTPUT",
                format!(
                    "{operation} envelope is {} bytes; maximum is {max_bytes}",
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
                format!("{operation} envelope could not be serialized"),
                false,
            ));
        }
    }
}

fn fingerprint(domain: &[u8], parts: &[&[u8]]) -> String {
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::solve_production;

    const DOMAIN: &str = "(define (domain smoke) (:requirements :strips) \
        (:predicates (done)) (:action finish :parameters () \
        :precondition (and) :effect (done)))";
    const PROBLEM: &str = "(define (problem smoke-p) (:domain smoke) \
        (:init) (:goal (done)))";

    #[test]
    fn explanation_requires_independent_validation() {
        let solved = solve_production(
            DOMAIN,
            PROBLEM,
            &Options::default(),
            &ProductionLimits::default(),
            None,
        );
        let plan = solved.payload.unwrap().plan.unwrap();
        let explained =
            explain_production(DOMAIN, PROBLEM, &plan, &ProductionLimits::default(), None);
        assert_eq!(explained.outcome, OutcomeClass::Solved);
        assert_eq!(explained.validation, ValidationStatus::Valid);
        assert_eq!(explained.authority, "evidence_only");
    }

    #[test]
    fn classical_decomposition_fallback_validates_as_classical() {
        let decomposed = decompose_production(
            DOMAIN,
            PROBLEM,
            &Options::default(),
            &ProductionLimits::default(),
            None,
        );
        assert_eq!(decomposed.outcome, OutcomeClass::Solved);
        assert_eq!(decomposed.validation, ValidationStatus::Valid);
        assert_eq!(decomposed.authority, CANDIDATE_AUTHORITY);
    }
}
