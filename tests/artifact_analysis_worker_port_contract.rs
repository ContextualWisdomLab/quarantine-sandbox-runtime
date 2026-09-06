//! Contract coverage for the backend-neutral artifact-analyzer worker execution port.

use std::collections::BTreeMap;

use quarantine_sandbox_runtime::{
    AnalyzerWorkerBudget, AnalyzerWorkerContractError, AnalyzerWorkerExecutionError,
    AnalyzerWorkerExecutionPort, AnalyzerWorkerFinding, AnalyzerWorkerIdentity,
    AnalyzerWorkerIsolationEvidence, AnalyzerWorkerOutcome, AnalyzerWorkerReceipt,
    AnalyzerWorkerRequest, EvidenceKind, IngestionPolicy, ingest_bytes,
};

fn analyzer_identity() -> AnalyzerWorkerIdentity {
    AnalyzerWorkerIdentity::new("capa_analyzer", "7.0.0", &"a".repeat(64))
        .expect("valid immutable analyzer identity")
}

fn budget() -> AnalyzerWorkerBudget {
    AnalyzerWorkerBudget {
        maximum_cpu_millis: 5_000,
        maximum_memory_bytes: 256 * 1024 * 1024,
        maximum_pids: 32,
        maximum_wall_time_millis: 10_000,
        maximum_scratch_bytes: 64 * 1024 * 1024,
        maximum_output_bytes: 65_536,
    }
}

fn isolation_evidence() -> AnalyzerWorkerIsolationEvidence {
    AnalyzerWorkerIsolationEvidence {
        worker_id: "worker_0123456789abcdef".to_owned(),
        runtime_backend_id: "rootless_podman".to_owned(),
        runtime_backend_version: "5.4.2".to_owned(),
        isolation_policy_sha256: "b".repeat(64),
        applied_budget: budget(),
        network_access_performed: false,
        credentials_available: false,
        host_filesystem_access_performed: false,
        runtime_socket_access_performed: false,
        uncontrolled_subprocess_performed: false,
        cleanup_completed: true,
    }
}

fn fixture_request<'a>(
    identity: &'a AnalyzerWorkerIdentity,
    artifact: &'a quarantine_sandbox_runtime::IngestedArtifact,
) -> AnalyzerWorkerRequest<'a> {
    AnalyzerWorkerRequest::new(identity, artifact, "artifact_worker_policy_v1", budget())
        .expect("valid worker request must be admitted")
}

fn fixture_receipt(
    identity: &AnalyzerWorkerIdentity,
    artifact: &quarantine_sandbox_runtime::IngestedArtifact,
) -> AnalyzerWorkerReceipt {
    AnalyzerWorkerReceipt {
        analyzer: identity.clone(),
        artifact_sha256: artifact.descriptor().artifact_sha256.clone(),
        policy_id: "artifact_worker_policy_v1".to_owned(),
        isolation: isolation_evidence(),
        outcome: AnalyzerWorkerOutcome::Completed {
            findings: vec![AnalyzerWorkerFinding {
                evidence_kind: EvidenceKind::StaticCapability,
                summary: "Analyzer identified one bounded capability.".to_owned(),
                attributes: BTreeMap::from([(
                    "capability".to_owned(),
                    "network_api".to_owned(),
                )]),
            }],
        },
    }
}

#[test]
fn worker_request_requires_immutable_analyzer_identity_and_nonzero_budgets() {
    let artifact = ingest_bytes(
        "sample.bin",
        b"hostile-but-immutable-artifact",
        &IngestionPolicy::default(),
    )
    .expect("fixture ingestion must succeed");
    let identity = analyzer_identity();

    fixture_request(&identity, &artifact);

    for (field, malformed) in [
        (
            "analyzer_id",
            AnalyzerWorkerIdentity::new("", "7.0.0", &"a".repeat(64)),
        ),
        (
            "implementation_version",
            AnalyzerWorkerIdentity::new("capa_analyzer", "", &"a".repeat(64)),
        ),
        (
            "implementation_sha256",
            AnalyzerWorkerIdentity::new("capa_analyzer", "7.0.0", &"A".repeat(64)),
        ),
    ] {
        assert!(
            matches!(
                malformed,
                Err(AnalyzerWorkerContractError::InvalidIdentity { field_name })
                    if field_name == field
            ),
            "{field} must fail closed"
        );
    }

    assert!(
        AnalyzerWorkerRequest::new(&identity, &artifact, "", budget()).is_err(),
        "empty policy identity must fail closed"
    );

    for field in [
        "maximum_cpu_millis",
        "maximum_memory_bytes",
        "maximum_pids",
        "maximum_wall_time_millis",
        "maximum_scratch_bytes",
        "maximum_output_bytes",
    ] {
        let mut invalid = budget();
        match field {
            "maximum_cpu_millis" => invalid.maximum_cpu_millis = 0,
            "maximum_memory_bytes" => invalid.maximum_memory_bytes = 0,
            "maximum_pids" => invalid.maximum_pids = 0,
            "maximum_wall_time_millis" => invalid.maximum_wall_time_millis = 0,
            "maximum_scratch_bytes" => invalid.maximum_scratch_bytes = 0,
            "maximum_output_bytes" => invalid.maximum_output_bytes = 0,
            _ => unreachable!(),
        }
        assert!(
            matches!(
                AnalyzerWorkerRequest::new(
                    &identity,
                    &artifact,
                    "artifact_worker_policy_v1",
                    invalid,
                ),
                Err(AnalyzerWorkerContractError::InvalidBudget { field_name })
                    if field_name == field
            ),
            "{field} must be bounded and non-zero"
        );
    }
}

#[test]
fn worker_receipt_must_bind_exact_request_and_deny_ambient_capabilities() {
    let artifact = ingest_bytes(
        "sample.bin",
        b"hostile-but-immutable-artifact",
        &IngestionPolicy::default(),
    )
    .expect("fixture ingestion must succeed");
    let identity = analyzer_identity();
    let request = fixture_request(&identity, &artifact);
    let receipt = fixture_receipt(&identity, &artifact);

    receipt
        .validate_against(&request)
        .expect("exact denied-capability worker receipt must validate");

    let mut artifact_mismatch = receipt.clone();
    artifact_mismatch.artifact_sha256 = "c".repeat(64);
    assert!(matches!(
        artifact_mismatch.validate_against(&request),
        Err(AnalyzerWorkerContractError::ReceiptMismatch {
            field_name: "artifact_sha256"
        })
    ));

    let mut policy_mismatch = receipt.clone();
    policy_mismatch.policy_id = "other_policy".to_owned();
    assert!(matches!(
        policy_mismatch.validate_against(&request),
        Err(AnalyzerWorkerContractError::ReceiptMismatch {
            field_name: "policy_id"
        })
    ));

    let mut analyzer_mismatch = receipt.clone();
    analyzer_mismatch.analyzer =
        AnalyzerWorkerIdentity::new("capa_analyzer", "7.0.1", &"d".repeat(64))
            .expect("alternate immutable analyzer identity");
    assert!(matches!(
        analyzer_mismatch.validate_against(&request),
        Err(AnalyzerWorkerContractError::ReceiptMismatch {
            field_name: "analyzer"
        })
    ));

    for field in [
        "maximum_cpu_millis",
        "maximum_memory_bytes",
        "maximum_pids",
        "maximum_wall_time_millis",
        "maximum_scratch_bytes",
        "maximum_output_bytes",
    ] {
        let mut budget_mismatch = receipt.clone();
        match field {
            "maximum_cpu_millis" => {
                budget_mismatch.isolation.applied_budget.maximum_cpu_millis += 1;
            }
            "maximum_memory_bytes" => {
                budget_mismatch.isolation.applied_budget.maximum_memory_bytes += 1;
            }
            "maximum_pids" => budget_mismatch.isolation.applied_budget.maximum_pids += 1,
            "maximum_wall_time_millis" => {
                budget_mismatch.isolation.applied_budget.maximum_wall_time_millis += 1;
            }
            "maximum_scratch_bytes" => {
                budget_mismatch.isolation.applied_budget.maximum_scratch_bytes += 1;
            }
            "maximum_output_bytes" => {
                budget_mismatch.isolation.applied_budget.maximum_output_bytes += 1;
            }
            _ => unreachable!(),
        }
        assert!(
            matches!(
                budget_mismatch.validate_against(&request),
                Err(AnalyzerWorkerContractError::ReceiptMismatch {
                    field_name: "applied_budget"
                })
            ),
            "{field} must be bound to runtime-owned applied-budget evidence"
        );
    }

    for field in [
        "network_access_performed",
        "credentials_available",
        "host_filesystem_access_performed",
        "runtime_socket_access_performed",
        "uncontrolled_subprocess_performed",
        "cleanup_completed",
    ] {
        let mut violated = receipt.clone();
        match field {
            "network_access_performed" => violated.isolation.network_access_performed = true,
            "credentials_available" => violated.isolation.credentials_available = true,
            "host_filesystem_access_performed" => {
                violated.isolation.host_filesystem_access_performed = true;
            }
            "runtime_socket_access_performed" => {
                violated.isolation.runtime_socket_access_performed = true;
            }
            "uncontrolled_subprocess_performed" => {
                violated.isolation.uncontrolled_subprocess_performed = true;
            }
            "cleanup_completed" => violated.isolation.cleanup_completed = false,
            _ => unreachable!(),
        }
        assert!(
            matches!(
                violated.validate_against(&request),
                Err(AnalyzerWorkerContractError::IsolationBoundaryViolated {
                    field_name
                }) if field_name == field
            ),
            "{field} must fail closed"
        );
    }
}

#[test]
fn worker_receipt_requires_bounded_runtime_owned_evidence_identity() {
    let artifact = ingest_bytes(
        "sample.bin",
        b"hostile-but-immutable-artifact",
        &IngestionPolicy::default(),
    )
    .expect("fixture ingestion must succeed");
    let identity = analyzer_identity();
    let request = fixture_request(&identity, &artifact);
    let receipt = fixture_receipt(&identity, &artifact);

    for field in [
        "worker_id",
        "runtime_backend_id",
        "runtime_backend_version",
        "isolation_policy_sha256",
    ] {
        let mut malformed = receipt.clone();
        match field {
            "worker_id" => malformed.isolation.worker_id.clear(),
            "runtime_backend_id" => malformed.isolation.runtime_backend_id.clear(),
            "runtime_backend_version" => malformed.isolation.runtime_backend_version.clear(),
            "isolation_policy_sha256" => {
                malformed.isolation.isolation_policy_sha256 = "B".repeat(64);
            }
            _ => unreachable!(),
        }
        assert!(
            malformed.validate_against(&request).is_err(),
            "malformed {field} must not become isolation authority"
        );
    }

    let mut control_text = receipt;
    control_text.isolation.worker_id = "worker\nforged".to_owned();
    assert!(
        control_text.validate_against(&request).is_err(),
        "runtime-owned evidence identifiers must reject control text"
    );
}

struct FakeWorker;

impl AnalyzerWorkerExecutionPort for FakeWorker {
    fn execute(
        &self,
        request: &AnalyzerWorkerRequest<'_>,
    ) -> Result<AnalyzerWorkerReceipt, AnalyzerWorkerExecutionError> {
        Ok(AnalyzerWorkerReceipt {
            analyzer: request.analyzer.clone(),
            artifact_sha256: request.artifact.descriptor().artifact_sha256.clone(),
            policy_id: request.policy_id.to_owned(),
            isolation: isolation_evidence(),
            outcome: AnalyzerWorkerOutcome::Failed {
                failure_code: "fixture_failure".to_owned(),
            },
        })
    }
}

#[test]
fn worker_port_is_backend_neutral_and_receipt_validation_is_controller_owned() {
    let artifact = ingest_bytes(
        "sample.bin",
        b"hostile-but-immutable-artifact",
        &IngestionPolicy::default(),
    )
    .expect("fixture ingestion must succeed");
    let identity = analyzer_identity();
    let request = fixture_request(&identity, &artifact);

    let receipt = FakeWorker
        .execute(&request)
        .expect("fake worker transport must return fixture receipt");
    receipt
        .validate_against(&request)
        .expect("controller validates worker receipt independently of adapter implementation");
}
