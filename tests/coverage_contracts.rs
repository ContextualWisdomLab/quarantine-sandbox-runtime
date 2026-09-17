//! Coverage tests for every contract-validation propagation path.

use std::collections::BTreeMap;

use quarantine_sandbox_runtime::{
    AnalysisProfile, ArtifactDescriptor, ArtifactKind, ContractError, EvidenceBundle, EvidenceKind,
    EvidenceRecord, RuntimeDisposition, RuntimeManifest,
};

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
    EvidenceBundle {
        schema_version: "1.0.0".to_owned(),
        analysis_job_id: "analysis_job_coverage".to_owned(),
        request_id: "request_coverage".to_owned(),
        artifact: valid_artifact(),
        runtime: RuntimeManifest {
            runtime_name: "quarantine-sandbox-runtime".to_owned(),
            runtime_version: "0.1.0".to_owned(),
            source_revision: "revision_coverage".to_owned(),
            requested_profile: AnalysisProfile::StaticOnly,
            dynamic_execution_performed: false,
            network_access_performed: false,
            credentials_available: false,
        },
        disposition: RuntimeDisposition::Completed,
        consumer_verdict_required: true,
        evidence: vec![EvidenceRecord {
            evidence_id: "evidence_coverage_0001".to_owned(),
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
