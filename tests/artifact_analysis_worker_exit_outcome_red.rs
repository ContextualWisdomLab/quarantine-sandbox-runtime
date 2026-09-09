//! RED coverage binding analyzer completion to runtime-observed worker termination.
//!
//! Core owns the worker termination observation; `artifact_analysis` owns whether
//! that observation is sufficient to admit a completed analyzer outcome.

use quarantine_sandbox_runtime::{
    AnalyzerWorkerContractError, AnalyzerWorkerIdentity, AnalyzerWorkerOutcome,
    AnalyzerWorkerReceipt, AnalyzerWorkerRequest, IngestedArtifact, IngestionPolicy,
    SandboxWorkerBudget, SandboxWorkerIsolationEvidence, SandboxWorkerTerminationEvidence,
    SandboxWorkerTerminationState, VerifiedIsolationState, ingest_bytes,
};
use serde_json::json;

const ISOLATION_POLICY_SHA256: &str =
    "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";
const WORKER_ID: &str = "worker_exit_status_0123456789abcdef";

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

fn completed_receipt(
    identity: &AnalyzerWorkerIdentity,
    artifact: &IngestedArtifact,
    exit_code: i32,
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
                state: SandboxWorkerTerminationState::Exited { exit_code },
            },
            cleanup_completed: true,
        },
        outcome: AnalyzerWorkerOutcome::Completed { findings: vec![] },
    }
}

#[test]
fn completed_worker_outcome_rejects_nonzero_runtime_exit() {
    let artifact = ingest_bytes(
        "sample.bin",
        b"hostile-but-immutable-artifact",
        &IngestionPolicy::default(),
    )
    .expect("fixture ingestion must succeed");
    let identity = analyzer_identity();
    let request = fixture_request(&identity, &artifact);
    let receipt = completed_receipt(&identity, &artifact, 137);

    assert!(
        matches!(
            receipt.validate_against(&request),
            Err(AnalyzerWorkerContractError::InvalidOutcome {
                field_name: "worker_exit_code"
            })
        ),
        "a nonzero runtime exit must not be admitted as a completed analyzer outcome"
    );
}

#[test]
fn completed_worker_outcome_accepts_zero_runtime_exit() {
    let artifact = ingest_bytes(
        "sample.bin",
        b"hostile-but-immutable-artifact",
        &IngestionPolicy::default(),
    )
    .expect("fixture ingestion must succeed");
    let identity = analyzer_identity();
    let request = fixture_request(&identity, &artifact);
    let receipt = completed_receipt(&identity, &artifact, 0);

    receipt
        .validate_against(&request)
        .expect("zero exit status with otherwise-valid evidence may admit Completed");
}

#[test]
fn semantic_failed_outcome_remains_valid_after_zero_runtime_exit() {
    let artifact = ingest_bytes(
        "sample.bin",
        b"hostile-but-immutable-artifact",
        &IngestionPolicy::default(),
    )
    .expect("fixture ingestion must succeed");
    let identity = analyzer_identity();
    let request = fixture_request(&identity, &artifact);
    let mut receipt = completed_receipt(&identity, &artifact, 0);
    receipt.outcome = AnalyzerWorkerOutcome::Failed {
        failure_code: "analyzer_failed".to_owned(),
    };

    receipt.validate_against(&request).expect(
        "semantic analyzer failure may be reported even when the worker process exits successfully",
    );
}
