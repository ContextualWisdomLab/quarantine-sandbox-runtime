//! Boundary tests for hostile contract metadata and attestations.

use quarantine_sandbox_runtime::{
    AnalysisEngine, AnalysisProfile, AnalysisRequest, ArtifactDescriptor, ArtifactKind,
    BoundedSourceContext, ContractError, EvidenceBundle, IngestionPolicy,
};

fn valid_request() -> AnalysisRequest {
    AnalysisRequest {
        schema_version: "1.0.0".to_owned(),
        request_id: "request_boundary".to_owned(),
        profile: AnalysisProfile::StaticOnly,
        bounded_source_context: Some(BoundedSourceContext {
            source_channel_code: Some("boundary_test".to_owned()),
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
        "revision_boundary",
    )
    .expect("fixture engine configuration must remain valid")
    .analyze_bytes(&valid_request(), b"abc")
    .expect("production-generated fixture bundle must validate")
}

#[test]
fn request_validation_rejects_schema_and_request_id_shape_errors() {
    let mut unsupported_schema = valid_request();
    unsupported_schema.schema_version = "2.0.0".to_owned();
    assert_eq!(
        unsupported_schema.validate(),
        Err(ContractError::UnsupportedSchemaVersion {
            actual_version: "2.0.0".to_owned(),
        })
    );

    let mut long_request_id = valid_request();
    long_request_id.request_id = "x".repeat(129);
    assert_eq!(
        long_request_id.validate(),
        Err(ContractError::FieldTooLong {
            field_name: "request_id",
            maximum_bytes: 128,
        })
    );

    let mut controlled_request_id = valid_request();
    controlled_request_id.request_id = "bad\nid".to_owned();
    assert_eq!(
        controlled_request_id.validate(),
        Err(ContractError::ControlCharacter {
            field_name: "request_id",
        })
    );
}

#[test]
fn artifact_validation_rejects_name_and_hash_edge_cases() {
    let mut long_name = valid_artifact();
    long_name.artifact_name = "x".repeat(256);
    assert_eq!(
        long_name.validate(),
        Err(ContractError::FieldTooLong {
            field_name: "artifact_name",
            maximum_bytes: 255,
        })
    );

    let mut controlled_name = valid_artifact();
    controlled_name.artifact_name = "bad\nname.bin".to_owned();
    assert_eq!(
        controlled_name.validate(),
        Err(ContractError::ControlCharacter {
            field_name: "artifact_name",
        })
    );

    for invalid_hash in ["A".repeat(64), "g".repeat(64)] {
        let mut artifact = valid_artifact();
        artifact.artifact_sha256 = invalid_hash;
        assert_eq!(artifact.validate(), Err(ContractError::InvalidSha256));
    }
}

#[test]
fn evidence_bundle_validation_rejects_runtime_and_metadata_errors() {
    for boundary_name in [
        "dynamic_execution_performed",
        "network_access_performed",
        "credentials_available",
    ] {
        let mut bundle = valid_bundle();
        match boundary_name {
            "dynamic_execution_performed" => bundle.runtime.dynamic_execution_performed = true,
            "network_access_performed" => bundle.runtime.network_access_performed = true,
            "credentials_available" => bundle.runtime.credentials_available = true,
            _ => unreachable!("the fixture contains only known boundary names"),
        }
        assert_eq!(
            bundle.validate(),
            Err(ContractError::RuntimeBoundaryViolated { boundary_name })
        );
    }

    let mut too_many_attributes = valid_bundle();
    too_many_attributes.evidence[0].attributes = (0..33)
        .map(|index| (format!("key_{index}"), "value".to_owned()))
        .collect();
    assert_eq!(
        too_many_attributes.validate(),
        Err(ContractError::TooManyAttributes {
            maximum_attributes: 32,
        })
    );

    let mut empty_key = valid_bundle();
    empty_key.evidence[0]
        .attributes
        .insert(String::new(), "value".to_owned());
    assert_eq!(empty_key.validate(), Err(ContractError::EmptyAttributeKey));

    let mut empty_value = valid_bundle();
    empty_value.evidence[0]
        .attributes
        .insert("capability".to_owned(), String::new());
    assert_eq!(
        empty_value.validate(),
        Err(ContractError::EmptyAttributeValue {
            attribute_key: "capability".to_owned(),
        })
    );
}

#[test]
fn limitation_validation_rejects_empty_duplicate_and_oversized_codes() {
    let mut empty = valid_bundle();
    empty.limitations = vec![String::new()];
    assert_eq!(
        empty.validate(),
        Err(ContractError::EmptyField {
            field_name: "limitation",
        })
    );

    let mut duplicate = valid_bundle();
    duplicate
        .limitations
        .push("runtime_does_not_determine_maliciousness".to_owned());
    assert_eq!(
        duplicate.validate(),
        Err(ContractError::DuplicateLimitation {
            limitation: "runtime_does_not_determine_maliciousness".to_owned(),
        })
    );

    let mut oversized = valid_bundle();
    oversized.limitations = vec!["x".repeat(257)];
    assert_eq!(
        oversized.validate(),
        Err(ContractError::FieldTooLong {
            field_name: "limitation",
            maximum_bytes: 256,
        })
    );
}
