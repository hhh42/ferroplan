use ferroplan::{
    route_planning_request, PlanningCapability, PlanningRail, PlanningRequest, PlanningRoute,
    PlanningRouteError, PlanningType,
};
use std::collections::BTreeSet;

fn admitted_request(planning_type: PlanningType) -> PlanningRequest {
    PlanningRequest {
        subject: format!("test:{}", planning_type.token()),
        planning_type,
        available_capabilities: planning_type.required_capabilities(),
        authority_bound: true,
        verifier_bound: true,
        receipt_bound: true,
    }
}

#[test]
fn every_planning_type_has_a_unique_stable_token_and_route() {
    let mut tokens = BTreeSet::new();
    for planning_type in PlanningType::ALL {
        assert!(tokens.insert(planning_type.token()));
        let route = route_planning_request(&admitted_request(planning_type)).unwrap();
        assert_eq!(route.planning_type, planning_type);
        assert_eq!(route.rail, planning_type.rail());
        assert_eq!(
            route.required_capabilities,
            planning_type.required_capabilities()
        );
    }
    assert_eq!(tokens.len(), PlanningType::ALL.len());
}

#[test]
fn every_solver_family_uses_a_native_or_exact_integration_rail() {
    assert_eq!(
        PlanningType::Classical.rail(),
        PlanningRail::NativeDeterministic
    );
    assert_eq!(
        PlanningType::Probabilistic.rail(),
        PlanningRail::NativeProbabilistic
    );
    assert_eq!(
        PlanningType::Fond.rail(),
        PlanningRail::NativeNondeterministic
    );
    assert_eq!(
        PlanningType::Conformant.rail(),
        PlanningRail::NativeBeliefState
    );
    assert_eq!(
        PlanningType::Contingent.rail(),
        PlanningRail::NativeBeliefState
    );
    assert_eq!(
        PlanningType::Hierarchical.rail(),
        PlanningRail::NativeHierarchical
    );
    assert_eq!(
        PlanningType::PartialOrder.rail(),
        PlanningRail::NativeWorkflow
    );
    assert_eq!(
        PlanningType::FlowConstrained.rail(),
        PlanningRail::NativeFlowConstrained
    );
    assert_eq!(
        PlanningType::MultiAgent.rail(),
        PlanningRail::NativeMultiAgent
    );
    assert_eq!(
        PlanningType::RdfDerived.rail(),
        PlanningRail::GraphProjection
    );
    assert_eq!(PlanningType::A2aDelegated.rail(), PlanningRail::Delegation);
    assert_eq!(
        PlanningType::McpBound.rail(),
        PlanningRail::CapabilityBinding
    );
}

#[test]
fn missing_semantic_capabilities_are_refused_before_authority_checks() {
    let mut request = admitted_request(PlanningType::FlowConstrained);
    request
        .available_capabilities
        .remove(&PlanningCapability::WipBounds);
    let error = route_planning_request(&request).unwrap_err();
    assert_eq!(
        error,
        PlanningRouteError::MissingCapabilities {
            missing: BTreeSet::from([PlanningCapability::WipBounds]),
        }
    );
}

#[test]
fn authority_verifier_and_receipt_are_independent_hard_gates() {
    let mut request = admitted_request(PlanningType::ResolutionAdaptive);
    request.authority_bound = false;
    assert_eq!(
        route_planning_request(&request).unwrap_err(),
        PlanningRouteError::AuthorityUnbound
    );

    request.authority_bound = true;
    request.verifier_bound = false;
    assert_eq!(
        route_planning_request(&request).unwrap_err(),
        PlanningRouteError::VerifierUnbound
    );

    request.verifier_bound = true;
    request.receipt_bound = false;
    assert_eq!(
        route_planning_request(&request).unwrap_err(),
        PlanningRouteError::ReceiptUnbound
    );
}

#[test]
fn empty_subject_is_never_routable() {
    let mut request = admitted_request(PlanningType::Classical);
    request.subject = "   ".to_owned();
    assert_eq!(
        route_planning_request(&request).unwrap_err(),
        PlanningRouteError::EmptySubject
    );
}

#[test]
fn serialization_round_trip_preserves_the_planning_contract() {
    let request = admitted_request(PlanningType::McpBound);
    let encoded = serde_json::to_string(&request).unwrap();
    let decoded: PlanningRequest = serde_json::from_str(&encoded).unwrap();
    assert_eq!(decoded, request);

    let route = route_planning_request(&decoded).unwrap();
    let route_encoded = serde_json::to_string(&route).unwrap();
    let route_decoded: PlanningRoute = serde_json::from_str(&route_encoded).unwrap();
    assert_eq!(route_decoded, route);
}
