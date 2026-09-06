//! RED contract for self-verifiable analysis-job identity in artifact-analysis receipts.

use quarantine_sandbox_runtime::{AnalysisEngine, AnalysisProfile, AnalysisRequest, EvidenceKind};

fn static_request() -> AnalysisRequest {
    AnalysisRequest {
        schema_version: "1.0.0".to_owned(),
        request_id: "job_identity_binding_control".to_owned(),
        profile: AnalysisProfile::StaticOnly,
        bounded_source_context: None,
    }
}

#[test]
fn analysis_job_id_must_bind_identity_bearing_receipt_inputs() {
    let engine = AnalysisEngine::default();
    let bundle = engine
        .analyze_bytes(&static_request(), b"analysis-job-identity-binding-fixture")
        .expect("static analysis must produce a valid control bundle");

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

    let mut revision_tampered = bundle.clone();
    revision_tampered.runtime.source_revision = "forged_source_revision".to_owned();
    assert_eq!(revision_tampered.analysis_job_id, original_job_id);
    assert!(
        revision_tampered.validate().is_err(),
        "changing runtime source_revision without changing analysis_job_id must invalidate the receipt"
    );
}
