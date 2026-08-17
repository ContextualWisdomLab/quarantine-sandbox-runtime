use std::collections::BTreeMap;

use quarantine_sandbox_runtime::{
    AnalysisContext, AnalysisProfile, AnalysisRequest, ArtifactDescriptor, ArtifactKind,
    ContractError, EvidenceBundle, EvidenceKind, EvidenceRecord, RuntimeDisposition,
    RuntimeManifest,
};

fn valid_request() -> AnalysisRequest {
    AnalysisRequest {
        schema_version: "1.0.0".to_owned(),
        request_id: "request_boundary".to_owned(),
        profile: AnalysisProfile::StaticOnly,
        context: AnalysisContext {
            source_system: "boundary_test".to_owned(),
            source_reference: "fixture_boundary".to_owned(),
            attributes: BTreeMap::new(),
        },
    }
}

fn valid_artifact() -> ArtifactDescriptor {
    ArtifactDescriptor {
        artifact_name: "sample.bin".to_owned(),
        artifact_size_bytes: 3,
        artifact_sha256:
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad".to_owned(),
        artifact_kind: ArtifactKind::Unknown,
    }
}

fn valid_bundle() -> EvidenceBundle {
    EvidenceBundle {
        schema_version: "1.0.0".to_owned(),
        analysis_job_id: "analysis_job_boundary".to_owned(),
        request_id: "request_boundary".to_owned(),
        artifact: valid_artifact(),
        runtime: RuntimeManifest {
            runtime_name: "quarantine-sandbox-runtime".to_owned(),
            runtime_version: "0.1.0".to_owned(),
            source_revision: "revision_boundary".to_owned(),
            requested_profile: AnalysisProfile::StaticOnly,
            dynamic_execution_performed: false,
            network_access_performed: false,
            credentials_available: false,
        },
        disposition: RuntimeDisposition::Completed,
        consumer_verdict_required: true,
        evidence: vec![EvidenceRecord {
            evidence_id: "evidence_boundary_0001".to_owned(),
            sequence_number: 1,
            evidence_kind: EvidenceKind::ArtifactIdentity,
            producer_id: "runtime_core".to_owned(),
            summary: "Artifact identity established.".to_owned(),
            attributes: BTreeMap::new(),
        }],
        limitations: vec!["runtime_does_not_determine_maliciousness".to_owned()],
    }
}

#[test]
fn request_validation_rejects_schema_and_attribute_shape_errors() {
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

    let mut long_key = valid_request();
    long_key
        .context
        .attributes
        .insert("k".repeat(129), "value".to_owned());
    assert_eq!(
        long_key.validate(),
        Err(ContractError::FieldTooLong {
            field_name: "attribute_key",
            maximum_bytes: 128,
        })
    );

    let mut long_value = valid_request();
    long_value
        .context
        .attributes
        .insert("classification".to_owned(), "v".repeat(1_025));
    assert_eq!(
        long_value.validate(),
        Err(ContractError::FieldTooLong {
            field_name: "attribute_value",
            maximum_bytes: 1_024,
        })
    );

    let mut controlled_key = valid_request();
    controlled_key
        .context
        .attributes
        .insert("bad\u{0007}key".to_owned(), "value".to_owned());
    assert_eq!(
        controlled_key.validate(),
        Err(ContractError::ControlCharacter {
            field_name: "attribute_key",
        })
    );

    let mut controlled_value = valid_request();
    controlled_value
        .context
        .attributes
        .insert("classification".to_owned(), "bad\nvalue".to_owned());
    assert_eq!(
        controlled_value.validate(),
        Err(ContractError::ControlCharacter {
            field_name: "attribute_value",
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
    let boundary_mutations: [(&str, fn(&mut RuntimeManifest)); 3] = [
        (
            "dynamic_execution_performed",
            |runtime| runtime.dynamic_execution_performed = true,
        ),
        (
            "network_access_performed",
            |runtime| runtime.network_access_performed = true,
        ),
        (
            "credentials_available",
            |runtime| runtime.credentials_available = true,
        ),
    ];
    for (boundary_name, mutate) in boundary_mutations {
        let mut bundle = valid_bundle();
        mutate(&mut bundle.runtime);
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
    assert_eq!(
        empty_key.validate(),
        Err(ContractError::EmptyAttributeKey)
    );

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
