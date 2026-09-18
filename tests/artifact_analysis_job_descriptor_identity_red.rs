use quarantine_sandbox_runtime::{
    AnalysisEngine, AnalysisProfile, AnalysisRequest, BoundedSourceContext, CONTRACT_SCHEMA_VERSION,
    IngestionPolicy,
};

fn request_with_file_name(original_file_name: &str) -> AnalysisRequest {
    AnalysisRequest {
        schema_version: CONTRACT_SCHEMA_VERSION.to_owned(),
        request_id: "descriptor_identity_fixture".to_owned(),
        profile: AnalysisProfile::StaticOnly,
        bounded_source_context: Some(BoundedSourceContext {
            source_channel_code: Some("direct_api".to_owned()),
            original_file_name: Some(original_file_name.to_owned()),
            declared_media_type: Some("text/plain".to_owned()),
            host_artifact_reference: None,
            submitted_at: None,
        }),
    }
}

#[test]
fn canonical_artifact_descriptor_identity_must_change_analysis_job_id() {
    let engine = AnalysisEngine::with_bundled_static_analyzers(
        IngestionPolicy::default(),
        "foundation_policy_v1",
        "descriptor_identity_red",
    )
    .expect("fixture engine configuration must remain valid");

    let first = engine
        .analyze_bytes(&request_with_file_name("first.txt"), b"same artifact bytes")
        .expect("first analysis must complete");
    let second = engine
        .analyze_bytes(&request_with_file_name("second.txt"), b"same artifact bytes")
        .expect("second analysis must complete");

    assert_eq!(
        first.artifact.artifact_sha256, second.artifact.artifact_sha256,
        "the witness must keep immutable artifact bytes identical"
    );
    assert_eq!(
        first.artifact.artifact_kind, second.artifact.artifact_kind,
        "the witness must not depend on format-classification drift"
    );
    assert_ne!(
        first.artifact.original_file_name, second.artifact.original_file_name,
        "the canonical descriptors must differ only through admitted source identity metadata"
    );
    assert_ne!(
        first.artifact.artifact_name, second.artifact.artifact_name,
        "the canonical descriptor must retain the distinct admitted leaf identity"
    );
    assert_ne!(
        first.analysis_job_id, second.analysis_job_id,
        "distinct canonical artifact descriptors must not alias one analysis job identity"
    );
    assert_ne!(
        first.evidence[0].evidence_id, second.evidence[0].evidence_id,
        "job-bound evidence identity must inherit the descriptor distinction"
    );
}
