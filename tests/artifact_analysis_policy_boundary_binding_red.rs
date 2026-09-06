//! RED contract for binding runtime-owned policy-boundary evidence to the runtime manifest.

use quarantine_sandbox_runtime::{AnalysisEngine, AnalysisProfile, AnalysisRequest, EvidenceKind};

fn static_request() -> AnalysisRequest {
    AnalysisRequest {
        schema_version: "1.0.0".to_owned(),
        request_id: "policy_boundary_binding_red".to_owned(),
        profile: AnalysisProfile::StaticOnly,
        bounded_source_context: None,
    }
}

#[test]
fn policy_boundary_evidence_must_match_the_runtime_manifest() {
    let engine = AnalysisEngine::default();
    let bundle = engine
        .analyze_bytes(&static_request(), b"policy-boundary-binding-fixture")
        .expect("foundation static analysis must produce a valid control bundle");

    assert_eq!(bundle.validate(), Ok(()));

    let policy_boundary_index = bundle
        .evidence
        .iter()
        .position(|record| record.evidence_kind == EvidenceKind::PolicyBoundary)
        .expect("control bundle must contain runtime-owned policy-boundary evidence");

    for attribute_name in [
        "dynamic_execution_performed",
        "network_access_performed",
        "credentials_available",
    ] {
        let mut contradictory = bundle.clone();
        let policy_boundary = &mut contradictory.evidence[policy_boundary_index];
        assert_eq!(
            policy_boundary.attributes.get(attribute_name).map(String::as_str),
            Some("false"),
            "control fixture must begin with the same false boundary fact as RuntimeManifest"
        );
        policy_boundary
            .attributes
            .insert(attribute_name.to_owned(), "true".to_owned());

        assert!(
            contradictory.validate().is_err(),
            "PolicyBoundary attribute {attribute_name} must not contradict the enclosing RuntimeManifest"
        );
    }
}
