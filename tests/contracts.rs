use std::collections::BTreeMap;

use quarantine_sandbox_runtime::{
    AnalysisContext, AnalysisProfile, AnalysisRequest, ArtifactDescriptor, ArtifactKind,
    ContractError, EvidenceBundle, EvidenceKind, EvidenceRecord, RuntimeDisposition,
    RuntimeManifest,
};

fn valid_request() -> AnalysisRequest {
    AnalysisRequest {
        schema_version: "1.0.0".to_owned(),
        request_id: "request_alpha".to_owned(),
        profile: AnalysisProfile::StaticOnly,
        context: AnalysisContext {
            source_system: "unit_test".to_owned(),
            source_reference: "fixture_001".to_owned(),
            attributes: BTreeMap::from([("tenant_reference".to_owned(), "tenant_alpha".to_owned())]),
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
        analysis_job_id: "analysis_job_alpha".to_owned(),
        request_id: "request_alpha".to_owned(),
        artifact: valid_artifact(),
        runtime: RuntimeManifest {
            runtime_name: "quarantine-sandbox-runtime".to_owned(),
            runtime_version: "0.1.0".to_owned(),
            source_revision: "revision_alpha".to_owned(),
            requested_profile: AnalysisProfile::StaticOnly,
            dynamic_execution_performed: false,
            network_access_performed: false,
            credentials_available: false,
        },
        disposition: RuntimeDisposition::Completed,
        consumer_verdict_required: true,
        evidence: vec![EvidenceRecord {
            evidence_id: "evidence_0001".to_owned(),
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
fn enum_codes_are_stable_and_snake_case() {
    assert_eq!(AnalysisProfile::StaticOnly.as_str(), "static_only");
    assert_eq!(AnalysisProfile::LinuxDynamic.as_str(), "linux_dynamic");
    assert_eq!(AnalysisProfile::WindowsDynamic.as_str(), "windows_dynamic");

    assert_eq!(ArtifactKind::PortableExecutable.as_str(), "portable_executable");
    assert_eq!(ArtifactKind::ElfExecutable.as_str(), "elf_executable");
    assert_eq!(ArtifactKind::MachOExecutable.as_str(), "mach_o_executable");
    assert_eq!(ArtifactKind::ZipArchive.as_str(), "zip_archive");
    assert_eq!(ArtifactKind::PdfDocument.as_str(), "pdf_document");
    assert_eq!(ArtifactKind::OleCompoundDocument.as_str(), "ole_compound_document");
    assert_eq!(ArtifactKind::Script.as_str(), "script");
    assert_eq!(ArtifactKind::Text.as_str(), "text");
    assert_eq!(ArtifactKind::Unknown.as_str(), "unknown");

    assert_eq!(EvidenceKind::ArtifactIdentity.as_str(), "artifact_identity");
    assert_eq!(EvidenceKind::FileFormat.as_str(), "file_format");
    assert_eq!(EvidenceKind::PolicyBoundary.as_str(), "policy_boundary");
    assert_eq!(EvidenceKind::StaticCapability.as_str(), "static_capability");
    assert_eq!(EvidenceKind::RuntimeBehavior.as_str(), "runtime_behavior");
    assert_eq!(EvidenceKind::NetworkAttempt.as_str(), "network_attempt");
    assert_eq!(EvidenceKind::ToolFailure.as_str(), "tool_failure");

    assert_eq!(RuntimeDisposition::Completed.as_str(), "completed");
    assert_eq!(RuntimeDisposition::Inconclusive.as_str(), "inconclusive");
    assert_eq!(RuntimeDisposition::Failed.as_str(), "failed");
}

#[test]
fn request_validation_accepts_a_bounded_source_context() {
    let request = valid_request();
    assert_eq!(request.validate(), Ok(()));

    let serialized = serde_json::to_string(&request).expect("test serialization must succeed");
    let round_trip: AnalysisRequest =
        serde_json::from_str(&serialized).expect("test deserialization must succeed");
    assert_eq!(round_trip, request);
}

#[test]
fn request_validation_rejects_each_untrusted_metadata_boundary() {
    let mut empty_id = valid_request();
    empty_id.request_id.clear();
    assert_eq!(
        empty_id.validate(),
        Err(ContractError::EmptyField {
            field_name: "request_id"
        })
    );

    let mut control_character = valid_request();
    control_character.context.source_system = "mail\u{0000}box".to_owned();
    assert_eq!(
        control_character.validate(),
        Err(ContractError::ControlCharacter {
            field_name: "source_system"
        })
    );

    let mut long_reference = valid_request();
    long_reference.context.source_reference = "x".repeat(1_025);
    assert_eq!(
        long_reference.validate(),
        Err(ContractError::FieldTooLong {
            field_name: "source_reference",
            maximum_bytes: 1_024,
        })
    );

    let mut too_many_attributes = valid_request();
    too_many_attributes.context.attributes = (0..33)
        .map(|index| (format!("key_{index}"), "value".to_owned()))
        .collect();
    assert_eq!(
        too_many_attributes.validate(),
        Err(ContractError::TooManyAttributes {
            maximum_attributes: 32
        })
    );

    let mut empty_key = valid_request();
    empty_key
        .context
        .attributes
        .insert(String::new(), "value".to_owned());
    assert_eq!(
        empty_key.validate(),
        Err(ContractError::EmptyAttributeKey)
    );

    let mut empty_value = valid_request();
    empty_value
        .context
        .attributes
        .insert("classification".to_owned(), String::new());
    assert_eq!(
        empty_value.validate(),
        Err(ContractError::EmptyAttributeValue {
            attribute_key: "classification".to_owned()
        })
    );
}

#[test]
fn artifact_validation_rejects_invalid_identity_fields() {
    assert_eq!(valid_artifact().validate(), Ok(()));

    let mut empty_name = valid_artifact();
    empty_name.artifact_name.clear();
    assert_eq!(
        empty_name.validate(),
        Err(ContractError::EmptyField {
            field_name: "artifact_name"
        })
    );

    let mut invalid_hash = valid_artifact();
    invalid_hash.artifact_sha256 = "not-a-sha256".to_owned();
    assert_eq!(
        invalid_hash.validate(),
        Err(ContractError::InvalidSha256)
    );

    let mut zero_size = valid_artifact();
    zero_size.artifact_size_bytes = 0;
    assert_eq!(
        zero_size.validate(),
        Err(ContractError::ZeroArtifactSize)
    );
}

#[test]
fn evidence_bundle_validation_rejects_sequence_and_policy_violations() {
    assert_eq!(valid_bundle().validate(), Ok(()));

    let mut empty_evidence = valid_bundle();
    empty_evidence.evidence.clear();
    assert_eq!(
        empty_evidence.validate(),
        Err(ContractError::EmptyEvidence)
    );

    let mut sequence_gap = valid_bundle();
    sequence_gap.evidence[0].sequence_number = 2;
    assert_eq!(
        sequence_gap.validate(),
        Err(ContractError::InvalidEvidenceSequence {
            expected_sequence: 1,
            actual_sequence: 2,
        })
    );

    let mut no_consumer_verdict = valid_bundle();
    no_consumer_verdict.consumer_verdict_required = false;
    assert_eq!(
        no_consumer_verdict.validate(),
        Err(ContractError::ConsumerVerdictMustBeRequired)
    );

    let mut credential_leak = valid_bundle();
    credential_leak.runtime.credentials_available = true;
    assert_eq!(
        credential_leak.validate(),
        Err(ContractError::RuntimeBoundaryViolated {
            boundary_name: "credentials_available"
        })
    );
}
