//! RED contract for binding ArtifactIdentity evidence to the top-level subject.

use quarantine_sandbox_runtime::{AnalysisEngine, AnalysisProfile, AnalysisRequest, EvidenceKind};

fn static_request() -> AnalysisRequest {
    AnalysisRequest {
        schema_version: "1.0.0".to_owned(),
        request_id: "artifact_identity_binding_red".to_owned(),
        profile: AnalysisProfile::StaticOnly,
        bounded_source_context: None,
    }
}

#[test]
fn artifact_identity_evidence_must_match_top_level_artifact_sha256() {
    let engine = AnalysisEngine::default();
    let mut bundle = engine
        .analyze_bytes(&static_request(), b"subject-binding-fixture")
        .expect("foundation static analysis must produce a valid control bundle");

    assert_eq!(bundle.validate(), Ok(()));

    let top_level_sha256 = bundle.artifact.artifact_sha256.clone();
    let forged_sha256 = "0".repeat(64);
    assert_ne!(top_level_sha256, forged_sha256);

    let identity_record = bundle
        .evidence
        .iter_mut()
        .find(|record| record.evidence_kind == EvidenceKind::ArtifactIdentity)
        .expect("engine-produced evidence must include ArtifactIdentity");
    let previous = identity_record
        .attributes
        .insert("artifact_sha256".to_owned(), forged_sha256);
    assert_eq!(previous.as_deref(), Some(top_level_sha256.as_str()));

    assert!(
        bundle.validate().is_err(),
        "a receipt must not validate when ArtifactIdentity evidence names a different SHA-256 subject than the top-level ArtifactDescriptor"
    );
}
