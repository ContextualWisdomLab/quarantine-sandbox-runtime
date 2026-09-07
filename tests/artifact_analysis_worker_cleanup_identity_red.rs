//! RED coverage binding worker cleanup evidence to the exact runtime-owned worker identity.
//!
//! Core owns worker lifecycle and cleanup evidence. `artifact_analysis` consumes
//! that evidence but must not treat an unscoped cleanup-success boolean as proof
//! that the exact terminated worker was cleaned.

use quarantine_sandbox_runtime::{
    AnalyzerWorkerContractError, AnalyzerWorkerIdentity, AnalyzerWorkerOutcome,
    AnalyzerWorkerReceipt, AnalyzerWorkerRequest, IngestedArtifact, IngestionPolicy,
    SandboxWorkerBudget, SandboxWorkerCleanupEvidence, SandboxWorkerIsolationEvidence,
    SandboxWorkerTerminationEvidence, SandboxWorkerTerminationState, VerifiedIsolationState,
    ingest_bytes,
};
use serde_json::json;

const ISOLATION_POLICY_SHA256: &str =
    "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";
const WORKER_ID: &str = "worker_cleanup_0123456789abcdef";
const OTHER_WORKER_ID: &str = "worker_cleanup_fedcba9876543210";

fn worker_budget() -> SandboxWorkerBudget {
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

fn analyzer_identity() -> AnalyzerWorkerIdentity {
    AnalyzerWorkerIdentity::new("capa_analyzer", "7.0.0", &"a".repeat(64))
        .expect("valid immutable analyzer identity")
}

fn fixture_request<'a>(
    identity: &'a AnalyzerWorkerIdentity,
    artifact: &'a IngestedArtifact,
) -> AnalyzerWorkerRequest<'a> {
    AnalyzerWorkerRequest::new(
        identity,
        artifact,
        "artifact_worker_policy_v1",
        ISOLATION_POLICY_SHA256,
        worker_budget(),
    )
    .expect("valid worker request must be admitted")
}

fn receipt_with_cleanup(
    identity: &AnalyzerWorkerIdentity,
    artifact: &IngestedArtifact,
    cleanup_worker_id: &str,
    cleanup_completed: bool,
) -> AnalyzerWorkerReceipt {
    AnalyzerWorkerReceipt {
        analyzer: identity.clone(),
        artifact_sha256: artifact.descriptor().artifact_sha256.clone(),
        policy_id: "artifact_worker_policy_v1".to_owned(),
        isolation: SandboxWorkerIsolationEvidence {
            worker_id: WORKER_ID.to_owned(),
            runtime_backend_id: "rootless_podman".to_owned(),
            runtime_backend_version: "5.4.2".to_owned(),
            isolation_policy_sha256: ISOLATION_POLICY_SHA256.to_owned(),
            applied_budget: worker_budget(),
            isolation_state: verified_isolation_state(),
            host_loopback_access_performed: false,
            host_filesystem_access_performed: false,
            runtime_socket_access_performed: false,
            uncontrolled_subprocess_performed: false,
            termination: SandboxWorkerTerminationEvidence {
                worker_id: WORKER_ID.to_owned(),
                state: SandboxWorkerTerminationState::Exited { exit_code: 0 },
            },
            cleanup: SandboxWorkerCleanupEvidence {
                worker_id: cleanup_worker_id.to_owned(),
                completed: cleanup_completed,
            },
        },
        outcome: AnalyzerWorkerOutcome::Completed { findings: vec![] },
    }
}

#[test]
fn worker_receipt_rejects_cleanup_for_a_different_worker() {
    let artifact = ingest_bytes(
        "sample.bin",
        b"hostile-but-immutable-artifact",
        &IngestionPolicy::default(),
    )
    .expect("fixture ingestion must succeed");
    let identity = analyzer_identity();
    let request = fixture_request(&identity, &artifact);
    let receipt = receipt_with_cleanup(&identity, &artifact, OTHER_WORKER_ID, true);

    assert!(
        matches!(
            receipt.validate_against(&request),
            Err(AnalyzerWorkerContractError::IsolationBoundaryViolated {
                field_name: "cleanup_worker_id"
            })
        ),
        "cleanup evidence for another worker must not authorize this worker receipt"
    );
}

#[test]
fn worker_receipt_rejects_incomplete_cleanup_for_the_exact_worker() {
    let artifact = ingest_bytes(
        "sample.bin",
        b"hostile-but-immutable-artifact",
        &IngestionPolicy::default(),
    )
    .expect("fixture ingestion must succeed");
    let identity = analyzer_identity();
    let request = fixture_request(&identity, &artifact);
    let receipt = receipt_with_cleanup(&identity, &artifact, WORKER_ID, false);

    assert!(
        matches!(
            receipt.validate_against(&request),
            Err(AnalyzerWorkerContractError::IsolationBoundaryViolated {
                field_name: "cleanup_completed"
            })
        ),
        "exact-worker cleanup must still fail closed while completion is unproven"
    );
}

#[test]
fn worker_receipt_accepts_completed_cleanup_for_the_exact_worker() {
    let artifact = ingest_bytes(
        "sample.bin",
        b"hostile-but-immutable-artifact",
        &IngestionPolicy::default(),
    )
    .expect("fixture ingestion must succeed");
    let identity = analyzer_identity();
    let request = fixture_request(&identity, &artifact);
    let receipt = receipt_with_cleanup(&identity, &artifact, WORKER_ID, true);

    receipt
        .validate_against(&request)
        .expect("exact-worker completed cleanup may admit an otherwise-valid receipt");
}
