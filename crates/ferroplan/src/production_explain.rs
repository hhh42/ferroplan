//! Bounded production explanation surface.
//!
//! Explanations are evidence-only. The supplied plan is independently validated
//! before introspection, and the resulting envelope carries no actuation authority.

use std::collections::BTreeMap;

use sha2::{Digest, Sha256};

use crate::introspect::Explanation;
use crate::{
    capability_manifest, BuildIdentity, OperationEnvelope, OutcomeClass, Plan, ProductionLimits,
    PublicError, ValidationStatus, OPERATION_ENVELOPE_SCHEMA,
};

const EXPLAIN_HASH_DOMAIN: &[u8] = b"ferroplan.production-explain.v1\0";
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
    let input_fingerprint = fingerprint(domain, problem, &plan_bytes);
    let manifest_fingerprint = capability_manifest().fingerprint();
    let mut envelope = OperationEnvelope {
        schema_version: OPERATION_ENVELOPE_SCHEMA.to_string(),
        request_id: normalize_request_id(request_id, &input_fingerprint),
        capability_id: "fp.core.explain".to_string(),
        capability_version: env!("CARGO_PKG_VERSION").to_string(),
        build_identity: BuildIdentity {
            product_version: env!("CARGO_PKG_VERSION").to_string(),
            source_revision: option_env!("FERROPLAN_BUILD_SHA")
                .filter(|value| !value.trim().is_empty())
                .map(ToOwned::to_owned),
            manifest_fingerprint: manifest_fingerprint.clone().ok(),
        },
        input_fingerprint,
        authority: "evidence_only".to_string(),
        outcome: OutcomeClass::Failed,
        validation: ValidationStatus::NotApplicable,
        elapsed_micros: elapsed(&clock),
        counters: BTreeMap::new(),
        warnings: Vec::new(),
        payload: None,
        error: None,
    };

    if let Err(error) = manifest_fingerprint {
        envelope.error = Some(PublicError::new("FP_INVARIANT", error.to_string(), false));
        return envelope;
    }
    if let Err(error) = limits.validate() {
        envelope.outcome = OutcomeClass::Refused;
        envelope.error = Some(error);
        return envelope;
    }
    if domain.is_empty() || problem.is_empty() {
        envelope.outcome = OutcomeClass::Refused;
        envelope.error = Some(PublicError::new(
            "FP_INVALID_REQUEST",
            "domain and problem must both be non-empty",
            false,
        ));
        return envelope;
    }
    if domain.len() > limits.max_domain_bytes || problem.len() > limits.max_problem_bytes {
        envelope.outcome = OutcomeClass::Refused;
        envelope.error = Some(PublicError::new(
            "FP_LIMIT_INPUT",
            "domain or problem exceeds the configured explanation input limit",
            false,
        ));
        return envelope;
    }
    if plan.length != plan.steps.len() {
        envelope.validation = ValidationStatus::Failed;
        envelope.error = Some(PublicError::new(
            "FP_INVARIANT",
            "plan length does not match emitted step count",
            false,
        ));
        return envelope;
    }
    if plan.length > limits.max_plan_steps {
        envelope.outcome = OutcomeClass::LimitExceeded;
        envelope.error = Some(PublicError::new(
            "FP_LIMIT_PLAN",
            format!(
                "plan contains {} steps; maximum is {}",
                plan.length, limits.max_plan_steps
            ),
            false,
        ));
        return envelope;
    }

    let rendered = render_plan(plan);
    match crate::plan::validate_plan(domain, problem, &rendered) {
        Ok(crate::plan::Validity::Valid) => {}
        Ok(crate::plan::Validity::Invalid(reason)) => {
            envelope.outcome = OutcomeClass::Refused;
            envelope.validation = ValidationStatus::Failed;
            envelope.error = Some(PublicError::new("FP_VALIDATION", reason, false));
            return envelope;
        }
        Err(error) => {
            envelope.outcome = OutcomeClass::Refused;
            envelope.validation = ValidationStatus::Failed;
            envelope.error = Some(PublicError::new("FP_VALIDATION", error, false));
            return envelope;
        }
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

    match serde_json::to_vec(&envelope) {
        Ok(bytes) if bytes.len() <= limits.max_output_bytes => {}
        Ok(bytes) => {
            envelope.payload = None;
            envelope.outcome = OutcomeClass::LimitExceeded;
            envelope.validation = ValidationStatus::NotApplicable;
            envelope.error = Some(PublicError::new(
                "FP_LIMIT_OUTPUT",
                format!(
                    "explanation envelope is {} bytes; maximum is {}",
                    bytes.len(), limits.max_output_bytes
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
                "explanation envelope could not be serialized",
                false,
            ));
        }
    }
    envelope
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

fn fingerprint(domain: &str, problem: &str, plan: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(EXPLAIN_HASH_DOMAIN);
    for part in [domain.as_bytes(), problem.as_bytes(), plan] {
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
    use crate::{solve_production, Options};

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
        let explained = explain_production(
            DOMAIN,
            PROBLEM,
            &plan,
            &ProductionLimits::default(),
            None,
        );
        assert_eq!(explained.outcome, OutcomeClass::Solved);
        assert_eq!(explained.validation, ValidationStatus::Valid);
        assert_eq!(explained.authority, "evidence_only");
    }
}
