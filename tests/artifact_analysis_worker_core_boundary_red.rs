//! RED coverage for Core-owned analyzer-worker isolation and lifecycle evidence.
//!
//! `artifact_analysis` owns analyzer/artifact/result semantics. Reusable worker
//! resource, isolation, termination, and cleanup semantics belong to the Core
//! `sandbox_execution` bounded context and are composed by the analyzer port.

use quarantine_sandbox_runtime::{
    AnalyzerWorkerContractError, AnalyzerWorkerIdentity, AnalyzerWorkerOutcome,
    AnalyzerWorkerReceipt, AnalyzerWorkerRequest, IngestedArtifact, IngestionPolicy,
    SandboxWorkerBudget, SandboxWorkerIsolationEvidence, SandboxWorkerTerminationEvidence,
    SandboxWorkerTerminationState, ingest_bytes,
};

const ISOLATION_POLICY_SHA256: &str =
    "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";
const WORKER_ID: &str = "worker_0123456789abcdef";

fn analyzer_identity() -> AnalyzerWorkerIdentity {
    AnalyzerWorkerIdentity::new("capa_analyzer", "7.0.0", &"a".repeat(64))
        .expect("valid immutable analyzer identity")
}

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

fn termination(worker_id: &str, state: SandboxWorkerTerminationState) -> SandboxWorkerTerminationEvidence {
    SandboxWorkerTerminationEvidence {
        worker_id: worker_id.to_owned(),
        state,
    }
}

fn isolation_evidence(
    termination: SandboxWorkerTerminationEvidence,
) -> SandboxWorkerIsolationEvidence {
    SandboxWorkerIsolationEvidence {
        worker_id: WORKER_ID.to_owned(),
        runtime_backend_id: "rootless_podman".to_owned(),
        runtime_backend_version: "5.4.2".to_owned(),
        isolation_policy_sha256: ISOLATION_POLICY_SHA256.to_owned(),
        applied_budget: worker_budget(),
        network_access_performed: false,
        credentials_available: false,
        host_filesystem_access_performed: false,
        runtime_socket_access_performed: false,
        uncontrolled_subprocess_performed: false,
        termination,
        cleanup_completed: true,
    }
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
    isolation: SandboxWorkerIsolationEvidence,
) -> AnalyzerWorkerReceipt {
    AnalyzerWorkerReceipt {
        analyzer: identity.clone(),
        artifact_sha256: artifact.descriptor().artifact_sha256.clone(),
        policy_id: "artifact_worker_policy_v1".to_owned(),
        isolation,
        outcome: AnalyzerWorkerOutcome::Failed {
            failure_code: "analyzer_failed".to_owned(),
        },
    }
}

#[test]
fn artifact_analysis_composes_core_owned_worker_isolation_values() {
    let core_source = include_str!("../src/sandbox_execution/mod.rs");
    let supporting_source = include_str!("../src/artifact_analysis/analyzer_worker.rs");

    for core_type in [
        "pub struct SandboxWorkerBudget",
        "pub struct SandboxWorkerIsolationEvidence",
        "pub struct SandboxWorkerTerminationEvidence",
        "pub enum SandboxWorkerTerminationState",
    ] {
        assert!(
            core_source.contains(core_type),
            "Core sandbox_execution must own {core_type}"
        );
    }
    for duplicate_type in [
        "pub struct AnalyzerWorkerBudget",
        "pub struct AnalyzerWorkerIsolationEvidence",
    ] {
        assert!(
            !supporting_source.contains(duplicate_type),
            "artifact_analysis must not duplicate Core isolation vocabulary: {duplicate_type}"
        );
    }

    let artifact = ingest_bytes(
        "sample.bin",
        b"hostile-but-immutable-artifact",
        &IngestionPolicy::default(),
    )
    .expect("fixture ingestion must succeed");
    let identity = analyzer_identity();
    let request = fixture_request(&identity, &artifact);
    let receipt = fixture_receipt(
        &identity,
        &artifact,
        isolation_evidence(termination(
            WORKER_ID,
            SandboxWorkerTerminationState::Exited { exit_code: 0 },
        )),
    );

    receipt
        .validate_against(&request)
        .expect("Core-owned terminal worker evidence must compose with the artifact port");
}

#[test]
fn receipt_rejects_nonterminal_or_other_worker_termination_evidence() {
    let artifact = ingest_bytes(
        "sample.bin",
        b"hostile-but-immutable-artifact",
        &IngestionPolicy::default(),
    )
    .expect("fixture ingestion must succeed");
    let identity = analyzer_identity();
    let request = fixture_request(&identity, &artifact);

    let running = fixture_receipt(
        &identity,
        &artifact,
        isolation_evidence(termination(WORKER_ID, SandboxWorkerTerminationState::Running)),
    );
    assert!(
        matches!(
            running.validate_against(&request),
            Err(AnalyzerWorkerContractError::IsolationBoundaryViolated {
                field_name: "termination_state"
            })
        ),
        "a worker that has not reached a terminal state must fail closed"
    );

    let wrong_worker = fixture_receipt(
        &identity,
        &artifact,
        isolation_evidence(termination(
            "worker_other_0123456789abcdef",
            SandboxWorkerTerminationState::Exited { exit_code: 0 },
        )),
    );
    assert!(
        matches!(
            wrong_worker.validate_against(&request),
            Err(AnalyzerWorkerContractError::IsolationBoundaryViolated {
                field_name: "termination_worker_id"
            })
        ),
        "termination evidence for another worker must not authorize cleanup/outcome acceptance"
    );
}
