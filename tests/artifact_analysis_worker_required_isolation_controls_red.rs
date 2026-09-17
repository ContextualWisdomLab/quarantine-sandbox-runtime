//! RED coverage for required Core isolation controls on analyzer-worker receipts.
//!
//! `SandboxWorkerIsolationEvidence` composes `VerifiedIsolationState`; required
//! P0 worker controls must fail closed when runtime inspection cannot verify them.

use quarantine_sandbox_runtime::{
    AnalyzerWorkerContractError, AnalyzerWorkerIdentity, AnalyzerWorkerOutcome,
    AnalyzerWorkerReceipt, AnalyzerWorkerRequest, IngestedArtifact, IngestionPolicy,
    SandboxWorkerBudget, SandboxWorkerIsolationEvidence, SandboxWorkerTerminationEvidence,
    SandboxWorkerTerminationState, VerifiedIsolationState, ingest_bytes,
};
use serde_json::json;

const ISOLATION_POLICY_SHA256: &str =
    "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";
const WORKER_ID: &str = "worker_0123456789abcdef";

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

fn isolation_state_with_unavailable(control_name: &str) -> VerifiedIsolationState {
    let mut state = json!({
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
    });
    state[control_name] = json!("unavailable");
    serde_json::from_value(state).expect("fixture isolation state must deserialize")
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

fn fixture_receipt(
    identity: &AnalyzerWorkerIdentity,
    artifact: &IngestedArtifact,
    isolation_state: VerifiedIsolationState,
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
            isolation_state,
            host_loopback_access_performed: false,
            host_filesystem_access_performed: false,
            runtime_socket_access_performed: false,
            uncontrolled_subprocess_performed: false,
            termination: SandboxWorkerTerminationEvidence {
                worker_id: WORKER_ID.to_owned(),
                state: SandboxWorkerTerminationState::Exited { exit_code: 0 },
            },
            cleanup_completed: true,
        },
        outcome: AnalyzerWorkerOutcome::Failed {
            failure_code: "analyzer_failed".to_owned(),
        },
    }
}

#[test]
fn receipt_rejects_unverified_required_worker_isolation_controls() {
    let artifact = ingest_bytes(
        "sample.bin",
        b"hostile-but-immutable-artifact",
        &IngestionPolicy::default(),
    )
    .expect("fixture ingestion must succeed");
    let identity = AnalyzerWorkerIdentity::new("capa_analyzer", "7.0.0", &"a".repeat(64))
        .expect("valid immutable analyzer identity");
    let request = fixture_request(&identity, &artifact);

    for control_name in [
        "rootless",
        "read_only_root_filesystem",
        "all_capabilities_dropped",
        "no_new_privileges",
        "isolated_user_namespace",
        "seccomp_enforced",
        "lsm_enforced",
    ] {
        let receipt = fixture_receipt(
            &identity,
            &artifact,
            isolation_state_with_unavailable(control_name),
        );

        assert!(
            matches!(
                receipt.validate_against(&request),
                Err(AnalyzerWorkerContractError::IsolationBoundaryViolated { field_name })
                    if field_name == control_name
            ),
            "worker receipt must fail closed when required Core control {control_name} is unavailable"
        );
    }
}
