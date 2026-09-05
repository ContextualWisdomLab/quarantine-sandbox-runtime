//! RED for truthful dynamic artifact-analysis execution attestation.
//!
//! Issue #52 is independent from worker containment (#49) and bounded result
//! ingestion (#50). The public contract already models dynamic profiles and
//! runtime-behavior evidence, so an isolated worker must be able to attest that
//! dynamic execution actually occurred without weakening the static-only
//! boundary.

use std::collections::BTreeMap;

use quarantine_sandbox_runtime::{
    AnalysisProfile, ArtifactDescriptor, ArtifactKind, ContractError, EvidenceBundle,
    EvidenceKind, EvidenceRecord, RuntimeDisposition, RuntimeManifest,
};

fn dynamic_bundle(profile: AnalysisProfile) -> EvidenceBundle {
    EvidenceBundle {
        schema_version: "1.0.0".to_owned(),
        analysis_job_id: "analysis_job_dynamic_attestation_red".to_owned(),
        request_id: "dynamic-attestation-red-001".to_owned(),
        artifact: ArtifactDescriptor {
            artifact_name: "artifact.bin".to_owned(),
            original_file_name: None,
            artifact_size_bytes: 3,
            artifact_sha256:
                "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
                    .to_owned(),
            artifact_kind: ArtifactKind::Unknown,
        },
        runtime: RuntimeManifest {
            runtime_name: "quarantine-sandbox-runtime".to_owned(),
            runtime_version: "0.1.0".to_owned(),
            source_revision: "dynamic-attestation-red".to_owned(),
            requested_profile: profile,
            dynamic_execution_performed: true,
            network_access_performed: false,
            credentials_available: false,
        },
        disposition: RuntimeDisposition::Completed,
        consumer_verdict_required: true,
        evidence: vec![
            EvidenceRecord {
                evidence_id: "analysis_job_dynamic_attestation_red:evidence:0001".to_owned(),
                sequence_number: 1,
                evidence_kind: EvidenceKind::ArtifactIdentity,
                producer_id: "runtime_core".to_owned(),
                summary: "Artifact identity established.".to_owned(),
                attributes: BTreeMap::from([(
                    "artifact_sha256".to_owned(),
                    "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
                        .to_owned(),
                )]),
            },
            EvidenceRecord {
                evidence_id: "analysis_job_dynamic_attestation_red:evidence:0002".to_owned(),
                sequence_number: 2,
                evidence_kind: EvidenceKind::RuntimeBehavior,
                producer_id: "isolated_dynamic_worker".to_owned(),
                summary: "Approved dynamic probe executed in the isolated worker.".to_owned(),
                attributes: BTreeMap::from([(
                    "worker_invocation_id".to_owned(),
                    "worker_invocation_dynamic_attestation_red".to_owned(),
                )]),
            },
        ],
        limitations: vec!["runtime_does_not_determine_maliciousness".to_owned()],
    }
}

#[test]
fn approved_dynamic_profiles_can_truthfully_attest_isolated_execution() {
    for profile in [AnalysisProfile::LinuxDynamic, AnalysisProfile::WindowsDynamic] {
        let bundle = dynamic_bundle(profile);
        bundle.validate().unwrap_or_else(|error| {
            panic!(
                "a completed dynamic profile must be able to attest actual isolated execution: profile={profile:?}, error={error:?}"
            )
        });
    }
}

#[test]
fn static_only_profile_still_rejects_dynamic_execution_attestation() {
    let bundle = dynamic_bundle(AnalysisProfile::StaticOnly);

    assert_eq!(
        bundle.validate(),
        Err(ContractError::RuntimeBoundaryViolated {
            boundary_name: "dynamic_execution_performed",
        }),
        "StaticOnly must remain non-executing even after dynamic-profile attestation becomes representable"
    );
}
