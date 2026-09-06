//! Contract tests for self-verifiable analysis-job identity in artifact-analysis receipts.

use quarantine_sandbox_runtime::{AnalysisEngine, AnalysisProfile, AnalysisRequest, EvidenceKind};

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

#[test]
fn analysis_job_id_must_bind_identity_bearing_receipt_inputs() {
    let bundle = control_bundle();

    assert_eq!(bundle.validate(), Ok(()));
    let original_job_id = bundle.analysis_job_id.clone();

    let mut request_tampered = bundle.clone();
    request_tampered.request_id = "job_identity_binding_forged_request".to_owned();
    assert_eq!(request_tampered.analysis_job_id, original_job_id);
    assert!(
        request_tampered.validate().is_err(),
        "changing request_id without changing analysis_job_id must invalidate the receipt"
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
    assert!(
        subject_tampered.validate().is_err(),
        "changing the consistently represented artifact subject without changing analysis_job_id must invalidate the receipt"
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
    assert!(
        policy_tampered.validate().is_err(),
        "changing the runtime policy identity without changing analysis_job_id must invalidate the receipt"
    );

    let mut revision_tampered = bundle.clone();
    revision_tampered.runtime.source_revision = "forged_source_revision".to_owned();
    assert_eq!(revision_tampered.analysis_job_id, original_job_id);
    assert!(
        revision_tampered.validate().is_err(),
        "changing runtime source_revision without changing analysis_job_id must invalidate the receipt"
    );
}

#[test]
fn analysis_job_id_rejects_malformed_binding_and_ambiguous_policy_identity() {
    let bundle = control_bundle();

    for malformed_job_id in [
        format!("job_{}_{}", "a".repeat(64), "b".repeat(32)),
        format!("analysis_job_{}", "a".repeat(96)),
        format!("analysis_job_{}_{}", "a".repeat(63), "b".repeat(32)),
        format!("analysis_job_{}_{}", "A".repeat(64), "b".repeat(32)),
        format!("analysis_job_{}_{}", "g".repeat(64), "b".repeat(32)),
        format!("analysis_job_{}_{}", "a".repeat(64), "b".repeat(31)),
        format!("analysis_job_{}_{}", "a".repeat(64), "B".repeat(32)),
        format!("analysis_job_{}_{}", "a".repeat(64), "g".repeat(32)),
    ] {
        let mut malformed = bundle.clone();
        malformed.analysis_job_id = malformed_job_id;
        assert!(
            malformed.validate().is_err(),
            "malformed composite job identity must fail closed"
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
        "a job identity cannot be verified without its runtime policy identity"
    );

    let mut duplicate_policy_identity = bundle;
    let mut duplicate_policy_boundary = duplicate_policy_identity
        .evidence
        .iter()
        .find(|record| record.evidence_kind == EvidenceKind::PolicyBoundary)
        .expect("control bundle must contain PolicyBoundary evidence")
        .clone();
    duplicate_policy_boundary
        .attributes
        .insert("policy_id".to_owned(), "forged_policy_v2".to_owned());
    duplicate_policy_boundary.sequence_number = duplicate_policy_identity.evidence.len() + 1;
    duplicate_policy_boundary.evidence_id = format!(
        "{}:evidence:{:04}",
        duplicate_policy_identity.analysis_job_id, duplicate_policy_boundary.sequence_number
    );
    duplicate_policy_identity
        .evidence
        .push(duplicate_policy_boundary);
    assert!(
        duplicate_policy_identity.validate().is_err(),
        "a job identity cannot bind an ambiguous runtime policy identity"
    );
}
