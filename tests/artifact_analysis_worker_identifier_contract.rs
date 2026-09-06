//! Contract coverage for bounded worker-port identifiers and immutable SHA-256 identities.

use quarantine_sandbox_runtime::{
    AnalyzerWorkerBudget, AnalyzerWorkerIdentity, AnalyzerWorkerIsolationEvidence,
    AnalyzerWorkerOutcome, AnalyzerWorkerReceipt, AnalyzerWorkerRequest, IngestionPolicy,
    ingest_bytes,
};

const MAX_IDENTIFIER_BYTES: usize = 128;
const VALID_SHA256: &str =
    "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";

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

#[test]
fn analyzer_identity_rejects_oversized_control_text_and_non_sha256_digests() {
    let oversized = "a".repeat(MAX_IDENTIFIER_BYTES + 1);

    for (analyzer_id, version) in [
        (oversized.as_str(), "7.0.0"),
        ("capa\nanalyzer", "7.0.0"),
        ("capa_analyzer", oversized.as_str()),
        ("capa_analyzer", "7.0.0\nforged"),
    ] {
        assert!(
            AnalyzerWorkerIdentity::new(analyzer_id, version, &"a".repeat(64)).is_err(),
            "analyzer identity text must be bounded and control-free"
        );
    }

    for digest in ["a".repeat(63), "g".repeat(64), "A".repeat(64)] {
        assert!(
            AnalyzerWorkerIdentity::new("capa_analyzer", "7.0.0", &digest).is_err(),
            "implementation digest must be exactly 64 lower-case hexadecimal characters"
        );
    }
}

#[test]
fn worker_request_rejects_oversized_or_control_policy_identity_and_malformed_policy_digest() {
    let artifact = ingest_bytes(
        "sample.bin",
        b"hostile-but-immutable-artifact",
        &IngestionPolicy::default(),
    )
    .expect("fixture ingestion must succeed");
    let identity = AnalyzerWorkerIdentity::new("capa_analyzer", "7.0.0", &"a".repeat(64))
        .expect("valid immutable analyzer identity");
    let oversized = "p".repeat(MAX_IDENTIFIER_BYTES + 1);

    for policy_id in [oversized.as_str(), "artifact\nworker\npolicy"] {
        assert!(
            AnalyzerWorkerRequest::new(&identity, &artifact, policy_id, VALID_SHA256, budget())
                .is_err(),
            "policy identity must be bounded and control-free"
        );
    }

    for digest in ["b".repeat(63), "g".repeat(64), "B".repeat(64)] {
        assert!(
            AnalyzerWorkerRequest::new(
                &identity,
                &artifact,
                "artifact_worker_policy_v1",
                &digest,
                budget(),
            )
            .is_err(),
            "isolation-policy digest must be exactly 64 lower-case hexadecimal characters"
        );
    }
}

#[test]
fn worker_receipt_rejects_oversized_or_control_runtime_evidence_identity() {
    let artifact = ingest_bytes(
        "sample.bin",
        b"hostile-but-immutable-artifact",
        &IngestionPolicy::default(),
    )
    .expect("fixture ingestion must succeed");
    let identity = AnalyzerWorkerIdentity::new("capa_analyzer", "7.0.0", &"a".repeat(64))
        .expect("valid immutable analyzer identity");
    let request = AnalyzerWorkerRequest::new(
        &identity,
        &artifact,
        "artifact_worker_policy_v1",
        VALID_SHA256,
        budget(),
    )
    .expect("valid worker request must be admitted");

    let base = AnalyzerWorkerReceipt {
        analyzer: identity,
        artifact_sha256: artifact.descriptor().artifact_sha256.clone(),
        policy_id: "artifact_worker_policy_v1".to_owned(),
        isolation: AnalyzerWorkerIsolationEvidence {
            worker_id: "worker_0123456789abcdef".to_owned(),
            runtime_backend_id: "rootless_podman".to_owned(),
            runtime_backend_version: "5.4.2".to_owned(),
            isolation_policy_sha256: VALID_SHA256.to_owned(),
            applied_budget: budget(),
            network_access_performed: false,
            credentials_available: false,
            host_filesystem_access_performed: false,
            runtime_socket_access_performed: false,
            uncontrolled_subprocess_performed: false,
            cleanup_completed: true,
        },
        outcome: AnalyzerWorkerOutcome::Failed {
            failure_code: "fixture_failure".to_owned(),
        },
    };

    let oversized = "w".repeat(MAX_IDENTIFIER_BYTES + 1);
    for field in ["worker_id", "runtime_backend_id", "runtime_backend_version"] {
        for invalid in [oversized.as_str(), "forged\nidentity"] {
            let mut receipt = base.clone();
            match field {
                "worker_id" => receipt.isolation.worker_id = invalid.to_owned(),
                "runtime_backend_id" => receipt.isolation.runtime_backend_id = invalid.to_owned(),
                "runtime_backend_version" => {
                    receipt.isolation.runtime_backend_version = invalid.to_owned();
                }
                _ => unreachable!(),
            }
            assert!(
                receipt.validate_against(&request).is_err(),
                "{field} must be bounded and control-free"
            );
        }
    }

    for digest in ["b".repeat(63), "g".repeat(64), "B".repeat(64)] {
        let mut receipt = base.clone();
        receipt.isolation.isolation_policy_sha256 = digest;
        assert!(
            receipt.validate_against(&request).is_err(),
            "runtime isolation-policy digest must remain exact lower-case SHA-256"
        );
    }
}
