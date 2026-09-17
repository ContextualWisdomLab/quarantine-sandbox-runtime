//! RED coverage for analyzer-worker evidence authority.

use std::collections::BTreeMap;

use quarantine_sandbox_runtime::{
    AnalyzerWorkerContractError, AnalyzerWorkerFinding, AnalyzerWorkerIdentity,
    AnalyzerWorkerOutcome, AnalyzerWorkerReceipt, AnalyzerWorkerRequest, EvidenceKind,
    IngestionPolicy, SandboxWorkerBudget, SandboxWorkerIsolationEvidence,
    SandboxWorkerTerminationEvidence, SandboxWorkerTerminationState, VerifiedIsolationState,
    ingest_bytes,
};
use serde_json::json;

const VALID_SHA256: &str = "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";
const WORKER_ID: &str = "worker_0123456789abcdef";

fn budget() -> SandboxWorkerBudget {
    SandboxWorkerBudget {
        maximum_cpu_millis: 5_000,
        maximum_memory_bytes: 256 * 1024 * 1024,
        maximum_pids: 32,
        maximum_wall_time_millis: 10_000,
        maximum_scratch_bytes: 64 * 1024 * 1024,
        maximum_output_bytes: 65_536,
    }
}

fn verified_isolation_state() -> VerifiedIsolationState {
    serde_json::from_value(json!({
        "rootless": "verified",
        "read_only_root_filesystem": "verified",
        "all_capabilities_dropped": "verified",
        "no_new_privileges": "verified",
        "isolated_user_namespace": "verified",
        "external_egress_denied": "verified",
        "loopback_only_publication": "not_applicable",
        "seccomp_enforced": "verified",
        "lsm_enforced": "verified",
        "resource_limits_verified": "verified",
        "credentials_available": false
    }))
    .expect("fixture isolation state must deserialize")
}

fn isolation_evidence() -> SandboxWorkerIsolationEvidence {
    SandboxWorkerIsolationEvidence {
        worker_id: WORKER_ID.to_owned(),
        runtime_backend_id: "rootless_podman".to_owned(),
        runtime_backend_version: "5.4.2".to_owned(),
        isolation_policy_sha256: VALID_SHA256.to_owned(),
        applied_budget: budget(),
        isolation_state: verified_isolation_state(),
        host_loopback_access_performed: false,
        host_filesystem_access_performed: false,
        runtime_socket_access_performed: false,
        uncontrolled_subprocess_performed: false,
        termination: SandboxWorkerTerminationEvidence {
            worker_id: WORKER_ID.to_owned(),
            state: SandboxWorkerTerminationState::Exited { exit_code: 0 },
        },
        cleanup_completed: true,
    }
}

fn analyzer_identity() -> AnalyzerWorkerIdentity {
    AnalyzerWorkerIdentity::new("capa_analyzer", "7.0.0", &"a".repeat(64))
        .expect("valid immutable analyzer identity")
}

fn receipt_fixture(
    identity: &AnalyzerWorkerIdentity,
    artifact: &quarantine_sandbox_runtime::IngestedArtifact,
    evidence_kind: EvidenceKind,
) -> AnalyzerWorkerReceipt {
    AnalyzerWorkerReceipt {
        analyzer: identity.clone(),
        artifact_sha256: artifact.descriptor().artifact_sha256.clone(),
        policy_id: "artifact_worker_policy_v1".to_owned(),
        isolation: isolation_evidence(),
        outcome: AnalyzerWorkerOutcome::Completed {
            findings: vec![AnalyzerWorkerFinding {
                evidence_kind,
                summary: "Analyzer returned one normalized finding.".to_owned(),
                attributes: BTreeMap::from([("source".to_owned(), "worker".to_owned())]),
            }],
        },
    }
}

#[test]
fn worker_cannot_claim_controller_or_runtime_owned_foundation_evidence() {
    let artifact = ingest_bytes(
        "sample.bin",
        b"hostile-but-immutable-artifact",
        &IngestionPolicy::default(),
    )
    .expect("fixture ingestion must succeed");
    let identity = analyzer_identity();
    let request = AnalyzerWorkerRequest::new(
        &identity,
        &artifact,
        "artifact_worker_policy_v1",
        VALID_SHA256,
        budget(),
    )
    .expect("valid worker request must be admitted");

    for forbidden_kind in [EvidenceKind::ArtifactIdentity, EvidenceKind::PolicyBoundary] {
        let receipt = receipt_fixture(&identity, &artifact, forbidden_kind);
        assert_eq!(
            receipt.validate_against(&request),
            Err(AnalyzerWorkerContractError::InvalidOutcome {
                field_name: "evidence_kind",
            }),
            "untrusted worker output must not mint controller/runtime-owned foundation evidence"
        );
    }
}

#[test]
fn worker_owned_static_evidence_remains_admissible() {
    let artifact = ingest_bytes(
        "sample.bin",
        b"hostile-but-immutable-artifact",
        &IngestionPolicy::default(),
    )
    .expect("fixture ingestion must succeed");
    let identity = analyzer_identity();
    let request = AnalyzerWorkerRequest::new(
        &identity,
        &artifact,
        "artifact_worker_policy_v1",
        VALID_SHA256,
        budget(),
    )
    .expect("valid worker request must be admitted");

    for allowed_kind in [EvidenceKind::FileFormat, EvidenceKind::StaticCapability] {
        let receipt = receipt_fixture(&identity, &artifact, allowed_kind);
        assert!(
            receipt.validate_against(&request).is_ok(),
            "worker-owned static evidence kinds must remain valid"
        );
    }
}
