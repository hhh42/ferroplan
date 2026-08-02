use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{anyhow, Context, Result};
use ferroplan::{parse_ppddl, solve_ppddl, validate_ppddl_policy, ProbabilisticOptions};

use super::extract::extract_operators_with_count;
use super::{
    admit, canonical_json, format_probability, seal_receipt, slug, verify_receipt_digest,
    work_identity, write_atomic, write_json, ActuationClass, FinalState, GallCheckpoint,
    HarvestReceipt, MethodCatalog, ObservationPack, OutputArtifact, PlanningOperator, ReplayState,
    SourceRevision, ValidationRecord, ValidationSummary, CATALOG_SCHEMA, RECEIPT_SCHEMA,
};

const MAX_OPERATORS: usize = 128;
const MAX_POLICY_SOLVE_OPERATORS: usize = 8;

pub fn compile_pack(pack: &ObservationPack, output_dir: &Path) -> Result<HarvestReceipt> {
    super::validate_pack(pack)?;
    let source_bytes = canonical_json(pack)?;
    let source_pack_digest = super::digest_bytes(&source_bytes);
    let admission = admit(pack);
    let (operators, raw_operator_count) = extract_operators_with_count(&admission);
    fs::create_dir_all(output_dir)
        .with_context(|| format!("creating {}", output_dir.display()))?;

    let observation_path = output_dir.join("observation-pack.json");
    let admission_path = output_dir.join("admission-report.json");
    write_atomic(&observation_path, &source_bytes)?;
    write_json(&admission_path, &admission)?;

    if operators.is_empty() {
        let mut receipt = HarvestReceipt {
            schema: RECEIPT_SCHEMA.to_owned(),
            run_id: pack.run_id.clone(),
            receipt_digest: String::new(),
            source_pack_digest,
            catalog_digest: super::digest_bytes(b"[]"),
            source_revisions: source_revisions(pack),
            source_work: pack.work_items.iter().map(work_identity).collect(),
            admitted_work: Vec::new(),
            excluded_work: admission.excluded.clone(),
            operators_added: Vec::new(),
            operators_deduplicated: 0,
            probabilistic_operators: 0,
            outputs: Vec::new(),
            validation: empty_validation(),
            replay: ReplayState::NotExecuted,
            generated_outputs_hand_edited: false,
            transport_failures: pack.transport_failures.clone(),
            failures: vec!["NO_ADMITTED_EXECUTED_WORK".to_owned()],
            exclusions: vec![
                "No PPDDL actions were fabricated because no exact-source execution was admitted"
                    .to_owned(),
            ],
            final_state: if pack.transport_failures.is_empty() {
                FinalState::Unknown
            } else {
                FinalState::Blocked
            },
        };
        receipt.outputs = output_artifacts(output_dir, &[&observation_path, &admission_path])?;
        seal_receipt(&mut receipt)?;
        write_json(&output_dir.join("receipt.json"), &receipt)?;
        return Ok(receipt);
    }

    if operators.len() > MAX_OPERATORS {
        return Err(anyhow!(
            "operator bound exceeded: {} > {MAX_OPERATORS}",
            operators.len()
        ));
    }

    let catalog = MethodCatalog {
        schema: CATALOG_SCHEMA.to_owned(),
        run_id: pack.run_id.clone(),
        source_pack_digest: source_pack_digest.clone(),
        raw_operator_count,
        operator_count: operators.len(),
        operators: operators.clone(),
    };
    let catalog_bytes = canonical_json(&catalog)?;
    let catalog_digest = super::digest_bytes(&catalog_bytes);
    let (domain, problem) = render_ppddl(pack, &operators);
    let mut validation = validate_ppddl(&domain, &problem, operators.len());

    let catalog_path = output_dir.join("method-catalog.json");
    let domain_path = output_dir.join("domain.ppddl");
    let problem_path = output_dir.join("problem.ppddl");
    write_atomic(&catalog_path, &catalog_bytes)?;
    write_atomic(&domain_path, domain.as_bytes())?;
    write_atomic(&problem_path, problem.as_bytes())?;

    let replay_render = render_ppddl(pack, &operators);
    let internal_match = replay_render.0 == domain && replay_render.1 == problem;
    validation.records.push(ValidationRecord {
        command: "render_ppddl(pack, operators) deterministic second render".to_owned(),
        result: if internal_match { "PASS" } else { "FAIL" }.to_owned(),
        detail: None,
    });

    let mut exclusions = Vec::new();
    if operators.len() > MAX_POLICY_SOLVE_OPERATORS {
        exclusions.push(format!(
            "Policy synthesis skipped above bounded operator limit {MAX_POLICY_SOLVE_OPERATORS}; parser admission still executed"
        ));
    }
    exclusions.extend(
        pack.transport_failures
            .iter()
            .map(|failure| format!("{}: {}", failure.operation, failure.detail)),
    );
    let validation_failed = !validation.parse_ok
        || validation.solved == Some(false)
        || validation.policy_valid == Some(false);
    let final_state = if validation_failed || !internal_match {
        FinalState::BuildBroken
    } else {
        FinalState::PartialAlive
    };
    let mut failures = Vec::new();
    if !internal_match {
        failures.push("NONDETERMINISTIC_RENDER".to_owned());
    }
    if validation.policy_valid == Some(false) {
        failures.push("POLICY_VALIDATION_FAILED".to_owned());
    }
    if validation.solved == Some(false) {
        failures.push("GENERATED_PROBLEM_UNSOLVED".to_owned());
    }

    let mut receipt = HarvestReceipt {
        schema: RECEIPT_SCHEMA.to_owned(),
        run_id: pack.run_id.clone(),
        receipt_digest: String::new(),
        source_pack_digest,
        catalog_digest,
        source_revisions: source_revisions(pack),
        source_work: pack.work_items.iter().map(work_identity).collect(),
        admitted_work: admission
            .admitted
            .iter()
            .map(|work| work.identity.clone())
            .collect(),
        excluded_work: admission.excluded.clone(),
        operators_added: operators.iter().map(|operator| operator.id.clone()).collect(),
        operators_deduplicated: raw_operator_count.saturating_sub(operators.len()),
        probabilistic_operators: operators
            .iter()
            .filter(|operator| !operator.probabilistic_outcomes.is_empty())
            .count(),
        outputs: Vec::new(),
        validation,
        replay: ReplayState::NotExecuted,
        generated_outputs_hand_edited: false,
        transport_failures: pack.transport_failures.clone(),
        failures,
        exclusions,
        final_state,
    };
    receipt.outputs = output_artifacts(
        output_dir,
        &[
            &observation_path,
            &admission_path,
            &catalog_path,
            &domain_path,
            &problem_path,
        ],
    )?;
    seal_receipt(&mut receipt)?;
    write_json(&output_dir.join("receipt.json"), &receipt)?;
    Ok(receipt)
}

pub fn replay_pack(
    pack: &ObservationPack,
    expected: &HarvestReceipt,
    output_dir: &Path,
) -> Result<HarvestReceipt> {
    verify_receipt_digest(expected)?;
    let mut replayed = compile_pack(pack, output_dir)?;
    let expected_outputs = expected
        .outputs
        .iter()
        .map(|artifact| (artifact.path.clone(), artifact.blake3.clone()))
        .collect::<BTreeMap<_, _>>();
    let actual_outputs = replayed
        .outputs
        .iter()
        .map(|artifact| (artifact.path.clone(), artifact.blake3.clone()))
        .collect::<BTreeMap<_, _>>();
    let matches = expected.source_pack_digest == replayed.source_pack_digest
        && expected.catalog_digest == replayed.catalog_digest
        && expected_outputs == actual_outputs;
    replayed.replay = if matches {
        ReplayState::ReplayMatch
    } else {
        ReplayState::ReplayMismatch
    };
    replayed.final_state = if matches
        && replayed.validation.parse_ok
        && replayed.validation.solved != Some(false)
        && replayed.validation.policy_valid != Some(false)
    {
        if replayed.validation.policy_valid == Some(true) {
            FinalState::Alive
        } else {
            FinalState::PartialAlive
        }
    } else {
        FinalState::BuildBroken
    };
    if !matches {
        replayed.failures.push("REPLAY_MISMATCH".to_owned());
    }
    seal_receipt(&mut replayed)?;
    write_json(&output_dir.join("receipt.json"), &replayed)?;
    Ok(replayed)
}

fn empty_validation() -> ValidationSummary {
    ValidationSummary {
        parse_ok: false,
        parse_error: None,
        solve_attempted: false,
        solved: None,
        initial_value: None,
        policy_valid: None,
        policy_errors: Vec::new(),
        records: Vec::new(),
    }
}

fn validate_ppddl(domain: &str, problem: &str, operator_count: usize) -> ValidationSummary {
    let parsed = parse_ppddl(domain, problem);
    let mut records = vec![ValidationRecord {
        command: "ferroplan::parse_ppddl(domain, problem)".to_owned(),
        result: if parsed.ok { "PASS" } else { "FAIL" }.to_owned(),
        detail: parsed.error.clone(),
    }];
    if !parsed.ok {
        return ValidationSummary {
            parse_ok: false,
            parse_error: parsed.error,
            solve_attempted: false,
            solved: None,
            initial_value: None,
            policy_valid: None,
            policy_errors: Vec::new(),
            records,
        };
    }
    if operator_count > MAX_POLICY_SOLVE_OPERATORS {
        records.push(ValidationRecord {
            command: "ferroplan::solve_ppddl bounded semantic probe".to_owned(),
            result: "NOT_EXECUTED".to_owned(),
            detail: Some(format!(
                "operator_count={operator_count} exceeds bound {MAX_POLICY_SOLVE_OPERATORS}"
            )),
        });
        return ValidationSummary {
            parse_ok: true,
            parse_error: None,
            solve_attempted: false,
            solved: None,
            initial_value: None,
            policy_valid: None,
            policy_errors: Vec::new(),
            records,
        };
    }

    let options = ProbabilisticOptions {
        horizon: Some(operator_count.saturating_mul(3).saturating_add(1)),
        max_states: 100_000,
        max_transitions: 1_000_000,
        ..Default::default()
    };
    match solve_ppddl(domain, problem, &options) {
        Ok(solution) => {
            records.push(ValidationRecord {
                command: "ferroplan::solve_ppddl(domain, problem, bounded_options)".to_owned(),
                result: if solution.solved { "PASS" } else { "UNSOLVED" }.to_owned(),
                detail: None,
            });
            match validate_ppddl_policy(domain, problem, &options, &solution) {
                Ok(validation) => {
                    records.push(ValidationRecord {
                        command:
                            "ferroplan::validate_ppddl_policy(domain, problem, options, policy)"
                                .to_owned(),
                        result: if validation.valid { "PASS" } else { "FAIL" }.to_owned(),
                        detail: if validation.errors.is_empty() {
                            None
                        } else {
                            Some(validation.errors.join("; "))
                        },
                    });
                    ValidationSummary {
                        parse_ok: true,
                        parse_error: None,
                        solve_attempted: true,
                        solved: Some(solution.solved),
                        initial_value: Some(solution.initial_value),
                        policy_valid: Some(validation.valid),
                        policy_errors: validation.errors,
                        records,
                    }
                }
                Err(error) => {
                    records.push(ValidationRecord {
                        command:
                            "ferroplan::validate_ppddl_policy(domain, problem, options, policy)"
                                .to_owned(),
                        result: "FAIL".to_owned(),
                        detail: Some(error.to_string()),
                    });
                    ValidationSummary {
                        parse_ok: true,
                        parse_error: None,
                        solve_attempted: true,
                        solved: Some(solution.solved),
                        initial_value: Some(solution.initial_value),
                        policy_valid: Some(false),
                        policy_errors: vec![error.to_string()],
                        records,
                    }
                }
            }
        }
        Err(error) => {
            records.push(ValidationRecord {
                command: "ferroplan::solve_ppddl(domain, problem, bounded_options)".to_owned(),
                result: "FAIL".to_owned(),
                detail: Some(error.to_string()),
            });
            ValidationSummary {
                parse_ok: true,
                parse_error: None,
                solve_attempted: true,
                solved: Some(false),
                initial_value: None,
                policy_valid: Some(false),
                policy_errors: vec![error.to_string()],
                records,
            }
        }
    }
}

fn render_ppddl(pack: &ObservationPack, operators: &[PlanningOperator]) -> (String, String) {
    let probabilistic = operators
        .iter()
        .any(|operator| !operator.probabilistic_outcomes.is_empty());
    let requirements = if probabilistic {
        ":strips :typing :negative-preconditions :probabilistic-effects"
    } else {
        ":strips :typing :negative-preconditions"
    };
    let constants = operators
        .iter()
        .map(|operator| operator.id.as_str())
        .collect::<Vec<_>>()
        .join(" ");
    let term_predicates = planning_term_predicates(operators);
    let mut predicate_declarations = vec![
        "    (evidence-admitted ?m - method)".to_owned(),
        "    (execution-observed ?m - method)".to_owned(),
        "    (method-integrated ?m - method)".to_owned(),
        "    (method-failed ?m - method)".to_owned(),
        "    (method-refused ?m - method)".to_owned(),
        "    (receipt-hook-required ?m - method)".to_owned(),
        "    (replay-hook-required ?m - method)".to_owned(),
        "    (receipt-emitted ?m - method)".to_owned(),
        "    (replay-matched ?m - method)".to_owned(),
    ];
    predicate_declarations.extend(
        term_predicates
            .iter()
            .map(|predicate| format!("    ({predicate} ?m - method)")),
    );

    let mut actions = String::new();
    for operator in operators {
        actions.push_str(&render_operator_action(operator));
    }
    actions.push_str(
        "  (:action emit-receipt\n    :parameters (?m - method)\n    :precondition (and (method-integrated ?m) (receipt-hook-required ?m) (not (receipt-emitted ?m)))\n    :effect (receipt-emitted ?m))\n",
    );
    actions.push_str(
        "  (:action verify-replay\n    :parameters (?m - method)\n    :precondition (and (receipt-emitted ?m) (replay-hook-required ?m) (not (replay-matched ?m)))\n    :effect (replay-matched ?m))\n",
    );

    let domain = format!(
        "(define (domain ferroplan-daily-method-harvest)\n  (:requirements {requirements})\n  (:types method)\n  (:constants {constants} - method)\n  (:predicates\n{}\n  )\n{actions})\n",
        predicate_declarations.join("\n")
    );
    let init = operators
        .iter()
        .flat_map(operator_initial_facts)
        .map(|fact| format!("    {fact}"))
        .collect::<Vec<_>>()
        .join("\n");
    let goals = operators
        .iter()
        .flat_map(|operator| {
            [
                format!("      (method-integrated {})", operator.id),
                format!("      (receipt-emitted {})", operator.id),
                format!("      (replay-matched {})", operator.id),
            ]
        })
        .collect::<Vec<_>>()
        .join("\n");
    let date = pack.window.start_utc.chars().take(10).collect::<String>();
    let problem_name = format!("harvest-{}", slug(&date));
    let problem = format!(
        "(define (problem {problem_name})\n  (:domain ferroplan-daily-method-harvest)\n  (:init\n{init})\n  (:goal\n    (and\n{goals})))\n"
    );
    (domain, problem)
}

fn planning_term_predicates(operators: &[PlanningOperator]) -> BTreeSet<String> {
    let mut predicates = BTreeSet::new();
    for operator in operators {
        for term in operator
            .preconditions
            .iter()
            .chain(&operator.effects)
            .chain(&operator.invariants)
            .chain(&operator.failures)
            .chain(&operator.refusals)
        {
            predicates.insert(term_predicate(term));
        }
        predicates.insert(checkpoint_predicate(operator.checkpoint));
        predicates.insert(actuation_predicate(operator.actuation_class));
        for outcome in &operator.probabilistic_outcomes {
            predicates.insert(format!("outcome-{}", slug(&outcome.label)));
        }
    }
    predicates
}

fn render_operator_action(operator: &PlanningOperator) -> String {
    let mut preconditions = vec![
        format!("(evidence-admitted {})", operator.id),
        format!("(execution-observed {})", operator.id),
        format!("({} {})", checkpoint_predicate(operator.checkpoint), operator.id),
        format!("({} {})", actuation_predicate(operator.actuation_class), operator.id),
        format!("(not (method-integrated {}))", operator.id),
        format!("(not (method-failed {}))", operator.id),
        format!("(not (method-refused {}))", operator.id),
    ];
    preconditions.extend(
        operator
            .preconditions
            .iter()
            .chain(&operator.invariants)
            .map(|term| format!("({} {})", term_predicate(term), operator.id)),
    );
    let effect = if operator.probabilistic_outcomes.is_empty() {
        success_effect(operator, None)
    } else {
        let branches = operator
            .probabilistic_outcomes
            .iter()
            .map(|outcome| {
                let branch = if outcome.success {
                    success_effect(operator, Some(&outcome.label))
                } else {
                    failure_effect(operator, &outcome.label)
                };
                format!("{} {branch}", format_probability(outcome.probability))
            })
            .collect::<Vec<_>>()
            .join(" ");
        format!("(probabilistic {branches})")
    };
    format!(
        "  (:action apply-{}\n    :parameters ()\n    :precondition (and {})\n    :effect {})\n",
        operator.id,
        preconditions.join(" "),
        effect
    )
}

fn success_effect(operator: &PlanningOperator, outcome_label: Option<&str>) -> String {
    let mut effects = vec![format!("(method-integrated {})", operator.id)];
    effects.extend(
        operator
            .effects
            .iter()
            .map(|term| format!("({} {})", term_predicate(term), operator.id)),
    );
    if let Some(label) = outcome_label {
        effects.push(format!("(outcome-{} {})", slug(label), operator.id));
    }
    format!("(and {})", effects.join(" "))
}

fn failure_effect(operator: &PlanningOperator, outcome_label: &str) -> String {
    let mut effects = vec![
        format!("(method-failed {})", operator.id),
        format!("(method-refused {})", operator.id),
        format!("(outcome-{} {})", slug(outcome_label), operator.id),
    ];
    effects.extend(
        operator
            .failures
            .iter()
            .chain(&operator.refusals)
            .map(|term| format!("({} {})", term_predicate(term), operator.id)),
    );
    format!("(and {})", effects.join(" "))
}

fn operator_initial_facts(operator: &PlanningOperator) -> Vec<String> {
    let mut facts = vec![
        format!("(evidence-admitted {})", operator.id),
        format!("(execution-observed {})", operator.id),
        format!("({} {})", checkpoint_predicate(operator.checkpoint), operator.id),
        format!("({} {})", actuation_predicate(operator.actuation_class), operator.id),
    ];
    if operator.receipt_hook {
        facts.push(format!("(receipt-hook-required {})", operator.id));
    }
    if operator.replay_hook {
        facts.push(format!("(replay-hook-required {})", operator.id));
    }
    facts.extend(
        operator
            .preconditions
            .iter()
            .chain(&operator.invariants)
            .map(|term| format!("({} {})", term_predicate(term), operator.id)),
    );
    facts
}

fn checkpoint_predicate(checkpoint: GallCheckpoint) -> String {
    format!("checkpoint-{}", slug(&format!("{checkpoint:?}")))
}

fn actuation_predicate(class: ActuationClass) -> String {
    format!("actuation-{}", slug(&format!("{class:?}")))
}

fn term_predicate(term: &str) -> String {
    format!("term-{}", slug(term))
}

fn source_revisions(pack: &ObservationPack) -> Vec<SourceRevision> {
    let mut revisions = pack
        .work_items
        .iter()
        .map(|work| SourceRevision {
            repository: work.repository.clone(),
            base_sha: work.parent_sha.clone(),
            head_sha: work.sha.clone(),
        })
        .collect::<Vec<_>>();
    revisions.sort_by(|left, right| {
        left.repository
            .cmp(&right.repository)
            .then(left.head_sha.cmp(&right.head_sha))
    });
    revisions.dedup_by(|left, right| {
        left.repository == right.repository && left.head_sha == right.head_sha
    });
    revisions
}

fn output_artifacts(output_dir: &Path, paths: &[&PathBuf]) -> Result<Vec<OutputArtifact>> {
    let mut artifacts = Vec::new();
    for path in paths {
        let bytes = fs::read(path).with_context(|| format!("reading {}", path.display()))?;
        let relative = path
            .strip_prefix(output_dir)
            .unwrap_or(path)
            .to_string_lossy()
            .replace('\\', "/");
        artifacts.push(OutputArtifact {
            path: relative,
            bytes: bytes.len(),
            blake3: super::digest_bytes(&bytes),
        });
    }
    artifacts.sort_by(|left, right| left.path.cmp(&right.path));
    Ok(artifacts)
}
