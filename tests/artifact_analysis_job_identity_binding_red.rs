//! Contract tests for versioned, self-verifiable analysis-job identity evidence.

use quarantine_sandbox_runtime::{
    AnalysisEngine, AnalysisProfile, AnalysisRequest, EVIDENCE_BUNDLE_SCHEMA_VERSION, EvidenceKind,
};
use serde_json::Value;

fn static_request() -> AnalysisRequest {
    AnalysisRequest {
        schema_version: "1.0.0".to_owned(),
        request_id: "job_identity_binding_control".to_owned(),
        profile: AnalysisProfile::StaticOnly,
        bounded_source_context: None,
    }
}

fn control_bundle() -> quarantine_sandbox_runtime::EvidenceBundle {
    AnalysisEngine::default()
        .analyze_bytes(&static_request(), b"analysis-job-identity-binding-fixture")
        .expect("static analysis must produce a valid control bundle")
}

fn append_policy_boundary(
    bundle: &mut quarantine_sandbox_runtime::EvidenceBundle,
    policy_id: Option<&str>,
) {
    let mut additional = bundle
        .evidence
        .iter()
        .find(|record| record.evidence_kind == EvidenceKind::PolicyBoundary)
        .expect("control bundle must contain PolicyBoundary evidence")
        .clone();
    match policy_id {
        Some(value) => {
            additional
                .attributes
                .insert("policy_id".to_owned(), value.to_owned());
        }
        None => {
            additional.attributes.remove("policy_id");
        }
    }
    additional.sequence_number = bundle.evidence.len() + 1;
    additional.evidence_id = format!(
        "{}:evidence:{:04}",
        bundle.analysis_job_id, additional.sequence_number
    );
    bundle.evidence.push(additional);
}

#[test]
fn analysis_job_identity_must_bind_identity_bearing_receipt_inputs() {
    let bundle = control_bundle();

    assert_eq!(bundle.validate(), Ok(()));
    assert_eq!(bundle.schema_version, EVIDENCE_BUNDLE_SCHEMA_VERSION);
    let original_job_id = bundle.analysis_job_id.clone();
    let original_identity = bundle.analysis_job_identity_sha256.clone();

    let mut job_id_tampered = bundle.clone();
    job_id_tampered.analysis_job_id = "opaque_alternate_analysis_job".to_owned();
    assert_eq!(
        job_id_tampered.analysis_job_identity_sha256,
        original_identity
    );
    assert!(
        job_id_tampered.validate().is_err(),
        "changing analysis_job_id without changing its identity digest must invalidate the receipt"
    );

    let mut request_tampered = bundle.clone();
    request_tampered.request_id = "job_identity_binding_forged_request".to_owned();
    assert_eq!(request_tampered.analysis_job_id, original_job_id);
    assert_eq!(
        request_tampered.analysis_job_identity_sha256,
        original_identity
    );
    assert!(
        request_tampered.validate().is_err(),
        "changing request_id without changing job identity evidence must invalidate the receipt"
    );

    let mut subject_tampered = bundle.clone();
    let alternate_sha256 = "0".repeat(64);
    assert_ne!(subject_tampered.artifact.artifact_sha256, alternate_sha256);
    subject_tampered.artifact.artifact_sha256 = alternate_sha256.clone();
    let artifact_identity = subject_tampered
        .evidence
        .iter_mut()
        .find(|record| record.evidence_kind == EvidenceKind::ArtifactIdentity)
        .expect("control bundle must contain ArtifactIdentity evidence");
    artifact_identity
        .attributes
        .insert("artifact_sha256".to_owned(), alternate_sha256);
    assert_eq!(subject_tampered.analysis_job_id, original_job_id);
    assert_eq!(
        subject_tampered.analysis_job_identity_sha256,
        original_identity
    );
    assert!(
        subject_tampered.validate().is_err(),
        "changing the consistently represented artifact subject without changing job identity evidence must invalidate the receipt"
    );

    let mut policy_tampered = bundle.clone();
    let policy_boundary = policy_tampered
        .evidence
        .iter_mut()
        .find(|record| record.evidence_kind == EvidenceKind::PolicyBoundary)
        .expect("control bundle must contain PolicyBoundary evidence");
    policy_boundary
        .attributes
        .insert("policy_id".to_owned(), "forged_policy_v2".to_owned());
    assert_eq!(policy_tampered.analysis_job_id, original_job_id);
    assert_eq!(
        policy_tampered.analysis_job_identity_sha256,
        original_identity
    );
    assert!(
        policy_tampered.validate().is_err(),
        "changing runtime policy identity without changing job identity evidence must invalidate the receipt"
    );

    let mut revision_tampered = bundle.clone();
    revision_tampered.runtime.source_revision = "forged_source_revision".to_owned();
    assert_eq!(revision_tampered.analysis_job_id, original_job_id);
    assert_eq!(
        revision_tampered.analysis_job_identity_sha256,
        original_identity
    );
    assert!(
        revision_tampered.validate().is_err(),
        "changing runtime source_revision without changing job identity evidence must invalidate the receipt"
    );
}

#[test]
fn analysis_job_identity_rejects_malformed_digest_and_ambiguous_policy_identity() {
    let bundle = control_bundle();

    for malformed_digest in [
        String::new(),
        "a".repeat(63),
        "a".repeat(65),
        "A".repeat(64),
        "g".repeat(64),
    ] {
        let mut malformed = bundle.clone();
        malformed.analysis_job_identity_sha256 = malformed_digest;
        assert!(
            malformed.validate().is_err(),
            "malformed job-identity SHA-256 must fail closed"
        );
    }

    let mut missing_policy_identity = bundle.clone();
    let policy_boundary = missing_policy_identity
        .evidence
        .iter_mut()
        .find(|record| record.evidence_kind == EvidenceKind::PolicyBoundary)
        .expect("control bundle must contain PolicyBoundary evidence");
    policy_boundary.attributes.remove("policy_id");
    assert!(
        missing_policy_identity.validate().is_err(),
        "job identity cannot be verified without a policy-identifying source"
    );

    let mut non_identifying_policy_boundary = bundle.clone();
    append_policy_boundary(&mut non_identifying_policy_boundary, None);
    assert_eq!(
        non_identifying_policy_boundary.validate(),
        Ok(()),
        "a non-identifying PolicyBoundary must not become a second policy identity source"
    );

    let control_policy_id = bundle
        .evidence
        .iter()
        .find(|record| record.evidence_kind == EvidenceKind::PolicyBoundary)
        .and_then(|record| record.attributes.get("policy_id"))
        .expect("control PolicyBoundary must identify its policy")
        .clone();

    let mut duplicate_same_policy_identity = bundle.clone();
    append_policy_boundary(
        &mut duplicate_same_policy_identity,
        Some(control_policy_id.as_str()),
    );
    assert!(
        duplicate_same_policy_identity.validate().is_err(),
        "two policy-identifying sources are ambiguous even when their policy IDs match"
    );

    let mut duplicate_different_policy_identity = bundle;
    append_policy_boundary(
        &mut duplicate_different_policy_identity,
        Some("forged_policy_v2"),
    );
    assert!(
        duplicate_different_policy_identity.validate().is_err(),
        "two contradictory policy-identifying sources must fail closed"
    );
}

#[test]
fn evidence_bundle_schema_versioning_is_explicit_and_preserves_v1_0() {
    let bundle = control_bundle();
    assert_eq!(bundle.schema_version, "1.1.0");

    let mut legacy_version = bundle;
    legacy_version.schema_version = "1.0.0".to_owned();
    assert!(matches!(
        legacy_version.validate(),
        Err(quarantine_sandbox_runtime::ContractError::UnsupportedSchemaVersion {
            actual_version
        }) if actual_version == "1.0.0"
    ));

    let current_schema: Value =
        serde_json::from_str(include_str!("../schemas/evidence-bundle.schema.json"))
            .expect("current evidence schema must be valid JSON");
    let versioned_current_schema: Value =
        serde_json::from_str(include_str!("../schemas/evidence-bundle-1.1.0.schema.json"))
            .expect("versioned current evidence schema must be valid JSON");
    let legacy_schema: Value =
        serde_json::from_str(include_str!("../schemas/evidence-bundle-1.0.0.schema.json"))
            .expect("legacy evidence schema must be valid JSON");

    assert_eq!(current_schema, versioned_current_schema);
    assert_eq!(
        current_schema["properties"]["schema_version"]["const"],
        "1.1.0"
    );
    assert_eq!(
        legacy_schema["properties"]["schema_version"]["const"],
        "1.0.0"
    );

    let current_required = current_schema["required"]
        .as_array()
        .expect("current evidence schema must declare required fields");
    assert!(
        current_required
            .iter()
            .any(|field| field.as_str() == Some("analysis_job_identity_sha256"))
    );
    let legacy_required = legacy_schema["required"]
        .as_array()
        .expect("legacy evidence schema must declare required fields");
    assert!(
        !legacy_required
            .iter()
            .any(|field| field.as_str() == Some("analysis_job_identity_sha256"))
    );
}
