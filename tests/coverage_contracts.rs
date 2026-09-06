//! Coverage tests for every contract-validation propagation path.

use quarantine_sandbox_runtime::{
    AnalysisEngine, AnalysisProfile, AnalysisRequest, ArtifactDescriptor, ArtifactKind,
    BoundedSourceContext, ContractError, EvidenceBundle, IngestionPolicy,
};

fn valid_request() -> AnalysisRequest {
    AnalysisRequest {
        schema_version: "1.0.0".to_owned(),
        request_id: "request_coverage".to_owned(),
        profile: AnalysisProfile::StaticOnly,
        bounded_source_context: Some(BoundedSourceContext {
            source_channel_code: Some("coverage_test".to_owned()),
            original_file_name: Some("sample.bin".to_owned()),
            declared_media_type: None,
            host_artifact_reference: None,
            submitted_at: None,
        }),
    }
}

fn valid_artifact() -> ArtifactDescriptor {
    ArtifactDescriptor {
        artifact_name: "sample.bin".to_owned(),
        original_file_name: Some("sample.bin".to_owned()),
        artifact_size_bytes: 3,
        artifact_sha256: "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
            .to_owned(),
        artifact_kind: ArtifactKind::Unknown,
    }
}

fn valid_bundle() -> EvidenceBundle {
    AnalysisEngine::with_bundled_static_analyzers(
        IngestionPolicy::default(),
        "foundation_policy_v1",
        "revision_coverage",
    )
    .expect("fixture engine configuration must remain valid")
    .analyze_bytes(&valid_request(), b"abc")
    .expect("production-generated fixture bundle must validate")
}

#[test]
fn bundle_validation_propagates_top_level_identity_errors() {
    let mut unsupported_schema = valid_bundle();
    unsupported_schema.schema_version = "2.0.0".to_owned();
    assert_eq!(
        unsupported_schema.validate(),
        Err(ContractError::UnsupportedSchemaVersion {
            actual_version: "2.0.0".to_owned(),
        })
    );

    let mut empty_job_id = valid_bundle();
    empty_job_id.analysis_job_id.clear();
    assert_eq!(
        empty_job_id.validate(),
        Err(ContractError::EmptyField {
            field_name: "analysis_job_id",
        })
    );

    let mut empty_request_id = valid_bundle();
    empty_request_id.request_id.clear();
    assert_eq!(
        empty_request_id.validate(),
        Err(ContractError::EmptyField {
            field_name: "request_id",
        })
    );

    let mut invalid_artifact = valid_bundle();
    invalid_artifact.artifact.artifact_size_bytes = 0;
    assert_eq!(
        invalid_artifact.validate(),
        Err(ContractError::ZeroArtifactSize)
    );
}

#[test]
fn bundle_validation_propagates_runtime_identity_errors() {
    let mut empty_runtime_name = valid_bundle();
    empty_runtime_name.runtime.runtime_name.clear();
    assert_eq!(
        empty_runtime_name.validate(),
        Err(ContractError::EmptyField {
            field_name: "runtime_name",
        })
    );

    let mut empty_runtime_version = valid_bundle();
    empty_runtime_version.runtime.runtime_version.clear();
    assert_eq!(
        empty_runtime_version.validate(),
        Err(ContractError::EmptyField {
            field_name: "runtime_version",
        })
    );

    let mut empty_source_revision = valid_bundle();
    empty_source_revision.runtime.source_revision.clear();
    assert_eq!(
        empty_source_revision.validate(),
        Err(ContractError::EmptyField {
            field_name: "source_revision",
        })
    );
}

#[test]
fn bundle_validation_propagates_evidence_identity_errors() {
    let mut empty_evidence_id = valid_bundle();
    empty_evidence_id.evidence[0].evidence_id.clear();
    assert_eq!(
        empty_evidence_id.validate(),
        Err(ContractError::EmptyField {
            field_name: "evidence_id",
        })
    );

    let mut empty_producer = valid_bundle();
    empty_producer.evidence[0].producer_id.clear();
    assert_eq!(
        empty_producer.validate(),
        Err(ContractError::EmptyField {
            field_name: "producer_id",
        })
    );

    let mut empty_summary = valid_bundle();
    empty_summary.evidence[0].summary.clear();
    assert_eq!(
        empty_summary.validate(),
        Err(ContractError::EmptyField {
            field_name: "summary",
        })
    );
}

#[test]
fn bundle_validation_propagates_evidence_attribute_text_errors() {
    let mut long_key = valid_bundle();
    long_key.evidence[0]
        .attributes
        .insert("k".repeat(129), "value".to_owned());
    assert_eq!(
        long_key.validate(),
        Err(ContractError::FieldTooLong {
            field_name: "attribute_key",
            maximum_bytes: 128,
        })
    );

    let mut long_value = valid_bundle();
    long_value.evidence[0]
        .attributes
        .insert("capability".to_owned(), "v".repeat(1_025));
    assert_eq!(
        long_value.validate(),
        Err(ContractError::FieldTooLong {
            field_name: "attribute_value",
            maximum_bytes: 1_024,
        })
    );

    let mut controlled_key = valid_bundle();
    controlled_key.evidence[0]
        .attributes
        .insert("bad\u{0007}key".to_owned(), "value".to_owned());
    assert_eq!(
        controlled_key.validate(),
        Err(ContractError::ControlCharacter {
            field_name: "attribute_key",
        })
    );

    let mut controlled_value = valid_bundle();
    controlled_value.evidence[0]
        .attributes
        .insert("capability".to_owned(), "bad\nvalue".to_owned());
    assert_eq!(
        controlled_value.validate(),
        Err(ContractError::ControlCharacter {
            field_name: "attribute_value",
        })
    );
}
