use ferroplan::{
    Activator, CapabilityTarget, Eve, EveError, EveHandoff, EveRequest, EveStage, GenesisWorld,
    HddlSurface, HumanPurpose, ManufactureTarget, PlanningRegime, PpddlSurface,
    MAX_PRIMARY_ACTIVATORS,
};

fn request(ppddl: bool) -> EveRequest {
    EveRequest {
        purpose: HumanPurpose {
            statement: "Deploy the service safely".to_string(),
            desired_consequence: "A replayable admitted deployment".to_string(),
            actor: Some("operator".to_string()),
            activators: vec![Activator {
                name: "environment".to_string(),
                value: "production".to_string(),
            }],
        },
        genesis: GenesisWorld {
            ontology_rdf: "@prefix fp: <urn:ferroplan:> .".to_string(),
            construct_query: "CONSTRUCT { ?s ?p ?o } WHERE { ?s ?p ?o }".to_string(),
            hddl: HddlSurface {
                domain: "(define (domain deploy) (:task deploy-service))".to_string(),
                problem: "(define (problem deploy-prod) (:domain deploy))".to_string(),
                root_task: "(deploy-service production)".to_string(),
            },
            ppddl: ppddl.then(|| PpddlSurface {
                domain: "(define (domain uncertain-deploy) (:requirements :probabilistic-effects))"
                    .to_string(),
                problem: "(define (problem uncertain-prod) (:domain uncertain-deploy))".to_string(),
            }),
        },
        manufacture: ManufactureTarget {
            name: "deploy-service.part".to_string(),
            template: "ggen://truex/part".to_string(),
            artifact_kind: ".part.wasm".to_string(),
            output: "target/parts/deploy-service.part.wasm".to_string(),
        },
        capability: CapabilityTarget {
            capability: "deploy-service".to_string(),
            route: "powl://deploy-service/v1".to_string(),
            authority_scopes: vec!["production:deploy".to_string()],
        },
    }
}

fn assert_missing(input: EveRequest, field: &str) {
    assert_eq!(
        Eve::enter(input),
        Err(EveError::Missing {
            field: field.to_string(),
        })
    );
}

#[test]
fn deterministic_world_compiles_exact_lifecycle() {
    let handoff = Eve::enter(request(false)).expect("valid deterministic handoff");

    assert_eq!(handoff.planning_regime, PlanningRegime::Deterministic);
    assert_eq!(handoff.protocol, "ferroplan.eve-genesis.v1");
    assert_eq!(
        handoff.stages,
        vec![
            EveStage::GroundHumanPurpose,
            EveStage::ProjectGenesis,
            EveStage::DecomposeHddl,
            EveStage::ManufactureGgen,
            EveStage::ExposeMcpPlus,
            EveStage::ActuateBrce,
            EveStage::ObserveOcel2,
            EveStage::ConformTruexKernel,
            EveStage::AdmitReceipt,
            EveStage::ReplayTruex,
        ]
    );
    assert!(handoff.ppddl.is_none());
    assert!(handoff.ggen.candidate_only);
    assert!(!handoff.mcp_plus.ambient_authority);
    assert!(handoff.mcp_plus.brce_required);
    assert!(handoff.truex.replay_required);
    assert_eq!(handoff.truex.expected_process_geometry, "POWL-v2");
    assert_eq!(handoff.truex.observed_path_format, "OCEL-2.0");
}

#[test]
fn probabilistic_world_inserts_ppddl_before_manufacture() {
    let handoff = Eve::enter(request(true)).expect("valid probabilistic handoff");

    assert_eq!(handoff.planning_regime, PlanningRegime::Probabilistic);
    assert!(handoff.ppddl.is_some());
    let ppddl = handoff
        .stages
        .iter()
        .position(|stage| *stage == EveStage::GovernUncertaintyPpddl)
        .unwrap();
    let ggen = handoff
        .stages
        .iter()
        .position(|stage| *stage == EveStage::ManufactureGgen)
        .unwrap();
    assert!(ppddl < ggen);
}

#[test]
fn closure_identity_is_stable_versioned_blake3() {
    let left = Eve::enter(request(false)).unwrap();
    let right = Eve::enter(request(false)).unwrap();

    assert_eq!(left.closure_id, right.closure_id);
    let digest = left.closure_id.strip_prefix("eve:").unwrap();
    assert_eq!(digest.len(), 64);
    assert!(digest.bytes().all(|byte| byte.is_ascii_hexdigit()));
}

#[test]
fn every_request_component_changes_closure_identity() {
    let baseline = Eve::enter(request(false)).unwrap().closure_id;

    let mut changed = request(false);
    changed.purpose.statement.push('!');
    assert_ne!(baseline, Eve::enter(changed).unwrap().closure_id);

    let mut changed = request(false);
    changed.purpose.actor = None;
    assert_ne!(baseline, Eve::enter(changed).unwrap().closure_id);

    let mut changed = request(false);
    changed.purpose.activators[0].value = "staging".to_string();
    assert_ne!(baseline, Eve::enter(changed).unwrap().closure_id);

    let mut changed = request(false);
    changed.genesis.construct_query.push(' ');
    assert_ne!(baseline, Eve::enter(changed).unwrap().closure_id);

    let mut changed = request(false);
    changed.manufacture.output.push_str(".next");
    assert_ne!(baseline, Eve::enter(changed).unwrap().closure_id);

    let mut changed = request(false);
    changed
        .capability
        .authority_scopes
        .push("audit".to_string());
    assert_ne!(baseline, Eve::enter(changed).unwrap().closure_id);

    assert_ne!(baseline, Eve::enter(request(true)).unwrap().closure_id);
}

#[test]
fn closure_identity_is_bound_into_both_downstream_handoffs() {
    let handoff = Eve::enter(request(false)).unwrap();
    assert_eq!(handoff.closure_id, handoff.ggen.closure_id);
    assert_eq!(handoff.closure_id, handoff.mcp_plus.closure_id);
}

#[test]
fn need9_refuses_and_returns_deterministic_split() {
    let mut input = request(false);
    input.purpose.activators = (0..=MAX_PRIMARY_ACTIVATORS)
        .map(|index| Activator {
            name: format!("condition-{index}"),
            value: "active".to_string(),
        })
        .collect();

    let error = Eve::enter(input).expect_err("ninth activator must split");
    match error {
        EveError::SplitRequired { directive } => {
            assert_eq!(directive.provided, 9);
            assert_eq!(directive.maximum, 8);
            assert_eq!(directive.groups.len(), 2);
            assert_eq!(directive.groups[0].len(), 8);
            assert_eq!(directive.groups[1].len(), 1);
            assert_eq!(directive.groups[0][0].name, "condition-0");
            assert_eq!(directive.groups[1][0].name, "condition-8");
        }
        other => panic!("unexpected refusal: {other:?}"),
    }
}

#[test]
fn whitespace_only_required_fields_are_refused() {
    let mut input = request(false);
    input.purpose.statement = " \n\t ".to_string();
    assert_missing(input, "purpose.statement");

    let mut input = request(false);
    input.genesis.ontology_rdf.clear();
    assert_missing(input, "genesis.ontology_rdf");

    let mut input = request(false);
    input.manufacture.artifact_kind = "  ".to_string();
    assert_missing(input, "manufacture.artifact_kind");

    let mut input = request(false);
    input.capability.route.clear();
    assert_missing(input, "capability.route");
}

#[test]
fn empty_optional_actor_is_refused_instead_of_colliding_with_none() {
    let mut input = request(false);
    input.purpose.actor = Some(String::new());
    assert_missing(input, "purpose.actor");
}

#[test]
fn empty_activator_members_are_refused() {
    let mut input = request(false);
    input.purpose.activators[0].name.clear();
    assert_missing(input, "purpose.activators[0].name");

    let mut input = request(false);
    input.purpose.activators[0].value = "  ".to_string();
    assert_missing(input, "purpose.activators[0].value");
}

#[test]
fn empty_authority_scope_is_refused() {
    let mut input = request(false);
    input.capability.authority_scopes[0].clear();
    assert_missing(input, "capability.authority_scopes[0]");
}

#[test]
fn incomplete_ppddl_surface_is_refused() {
    let mut input = request(true);
    input.genesis.ppddl.as_mut().unwrap().problem.clear();
    assert_missing(input, "genesis.ppddl.problem");
}

#[test]
fn receipt_obligations_cover_materialization_evidence_conformance_and_replay() {
    let handoff = Eve::enter(request(false)).unwrap();
    assert_eq!(
        handoff.mcp_plus.receipt_obligations,
        vec![
            "artifact-materialized",
            "boundary-evidence",
            "ocel2-observed-path",
            "powl-conformance",
            "receipt-admission-or-refusal",
            "replay",
        ]
    );
}

#[test]
fn handoff_round_trips_as_structured_json() {
    let handoff = Eve::enter(request(true)).unwrap();
    let encoded = serde_json::to_string(&handoff).unwrap();
    let decoded: EveHandoff = serde_json::from_str(&encoded).unwrap();
    assert_eq!(handoff, decoded);
}

#[test]
fn refusal_round_trips_as_structured_json() {
    let mut input = request(false);
    input.purpose.actor = Some(String::new());
    let refusal = Eve::enter(input).unwrap_err();
    let encoded = serde_json::to_string(&refusal).unwrap();
    let decoded: EveError = serde_json::from_str(&encoded).unwrap();
    assert_eq!(refusal, decoded);
}
