//! RED for the missing inverse dynamic-attestation relationship.
//!
//! `RuntimeBehavior` already requires `dynamic_execution_performed=true`. The
//! receipt must also reject the inverse contradiction: a dynamic execution flag
//! without any observed runtime-behavior evidence. Otherwise the boolean alone
//! can masquerade as execution evidence.

use std::collections::BTreeMap;

use quarantine_sandbox_runtime::{
    AnalysisProfile, ArtifactDescriptor, ArtifactKind, ContractError, EvidenceBundle, EvidenceKind,
    EvidenceRecord, RuntimeDisposition, RuntimeManifest,
};

fn completed_dynamic_bundle_without_runtime_behavior(profile: AnalysisProfile) -> EvidenceBundle {
    EvidenceBundle {
        schema_version: "1.0.0".to_owned(),
        analysis_job_id: "analysis_job_dynamic_attestation_bidirectional_red".to_owned(),
        request_id: "dynamic-attestation-bidirectional-red-001".to_owned(),
        artifact: ArtifactDescriptor {
            artifact_name: "artifact.bin".to_owned(),
            original_file_name: None,
            artifact_size_bytes: 3,
            artifact_sha256: "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
                .to_owned(),
            artifact_kind: ArtifactKind::Unknown,
        },
        runtime: RuntimeManifest {
            runtime_name: "quarantine-sandbox-runtime".to_owned(),
            runtime_version: "0.1.0".to_owned(),
            source_revision: "dynamic-attestation-bidirectional-red".to_owned(),
            requested_profile: profile,
            dynamic_execution_performed: true,
            network_access_performed: false,
            credentials_available: false,
        },
        disposition: RuntimeDisposition::Completed,
        consumer_verdict_required: true,
        evidence: vec![EvidenceRecord {
            evidence_id: "analysis_job_dynamic_attestation_bidirectional_red:evidence:0001"
                .to_owned(),
            sequence_number: 1,
            evidence_kind: EvidenceKind::ArtifactIdentity,
            producer_id: "runtime_core".to_owned(),
            summary: "Artifact identity established.".to_owned(),
            attributes: BTreeMap::from([(
                "artifact_sha256".to_owned(),
                "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
                    .to_owned(),
            )]),
        }],
        limitations: vec!["runtime_does_not_determine_maliciousness".to_owned()],
    }
}

#[test]
fn dynamic_execution_flag_requires_runtime_behavior_evidence() {
    for profile in [
        AnalysisProfile::LinuxDynamic,
        AnalysisProfile::WindowsDynamic,
    ] {
        let bundle = completed_dynamic_bundle_without_runtime_behavior(profile);

        assert_eq!(
            bundle.validate(),
            Err(ContractError::RuntimeBoundaryViolated {
                boundary_name: "dynamic_execution_without_runtime_behavior",
            }),
            "a completed dynamic receipt must reject the exact execution-without-RuntimeBehavior contradiction: profile={profile:?}"
        );
    }
}
