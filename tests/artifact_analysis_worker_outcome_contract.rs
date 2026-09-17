//! Contract coverage for bounded normalized analyzer-worker outcomes.

use std::collections::BTreeMap;

use quarantine_sandbox_runtime::{
    AnalyzerWorkerFinding, AnalyzerWorkerIdentity, AnalyzerWorkerOutcome, AnalyzerWorkerReceipt,
    AnalyzerWorkerRequest, EvidenceKind, IngestionPolicy, SandboxWorkerBudget,
    SandboxWorkerIsolationEvidence, SandboxWorkerTerminationEvidence,
    SandboxWorkerTerminationState, VerifiedIsolationState, ingest_bytes,
};
use serde_json::json;

const MAX_IDENTIFIER_BYTES: usize = 128;
const MAX_ATTRIBUTE_COUNT: usize = 32;
const MAX_ATTRIBUTE_KEY_BYTES: usize = 128;
const MAX_ATTRIBUTE_VALUE_BYTES: usize = 1_024;
const MAX_SUMMARY_BYTES: usize = 4_096;
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

fn request_fixture<'a>(
    identity: &'a AnalyzerWorkerIdentity,
    artifact: &'a quarantine_sandbox_runtime::IngestedArtifact,
) -> AnalyzerWorkerRequest<'a> {
    AnalyzerWorkerRequest::new(
        identity,
        artifact,
        "artifact_worker_policy_v1",
        VALID_SHA256,
        budget(),
    )
    .expect("valid worker request must be admitted")
}

fn receipt_fixture(
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
                attributes: BTreeMap::from([("capability".to_owned(), "network_api".to_owned())]),
            }],
        },
    }
}

#[test]
fn worker_failure_code_is_bounded_normalized_text() {
    let artifact = ingest_bytes(
        "sample.bin",
        b"hostile-but-immutable-artifact",
        &IngestionPolicy::default(),
    )
    .expect("fixture ingestion must succeed");
    let identity = analyzer_identity();
    let request = request_fixture(&identity, &artifact);
    let base = receipt_fixture(&identity, &artifact);

    for invalid in [
        String::new(),
        "f".repeat(MAX_IDENTIFIER_BYTES + 1),
        "worker_failure\nforged".to_owned(),
    ] {
        let mut receipt = base.clone();
        receipt.outcome = AnalyzerWorkerOutcome::Failed {
            failure_code: invalid,
        };
        assert!(
            receipt.validate_against(&request).is_err(),
            "worker failure codes must be non-empty, bounded, and control-free"
        );
    }
}

#[test]
fn worker_finding_summary_is_bounded_normalized_text() {
    let artifact = ingest_bytes(
        "sample.bin",
        b"hostile-but-immutable-artifact",
        &IngestionPolicy::default(),
    )
    .expect("fixture ingestion must succeed");
    let identity = analyzer_identity();
    let request = request_fixture(&identity, &artifact);
    let base = receipt_fixture(&identity, &artifact);

    for invalid in [
        String::new(),
        "s".repeat(MAX_SUMMARY_BYTES + 1),
        "capability\nforged".to_owned(),
    ] {
        let mut receipt = base.clone();
        let AnalyzerWorkerOutcome::Completed { findings } = &mut receipt.outcome else {
            unreachable!("fixture must contain a completed outcome")
        };
        findings[0].summary = invalid;
        assert!(
            receipt.validate_against(&request).is_err(),
            "worker finding summaries must match normalized evidence bounds"
        );
    }
}

#[test]
fn worker_finding_attributes_match_normalized_evidence_bounds() {
    let artifact = ingest_bytes(
        "sample.bin",
        b"hostile-but-immutable-artifact",
        &IngestionPolicy::default(),
    )
    .expect("fixture ingestion must succeed");
    let identity = analyzer_identity();
    let request = request_fixture(&identity, &artifact);
    let base = receipt_fixture(&identity, &artifact);

    let invalid_attributes = [
        BTreeMap::from([(String::new(), "value".to_owned())]),
        BTreeMap::from([("key".to_owned(), String::new())]),
        BTreeMap::from([("k".repeat(MAX_ATTRIBUTE_KEY_BYTES + 1), "value".to_owned())]),
        BTreeMap::from([("key".to_owned(), "v".repeat(MAX_ATTRIBUTE_VALUE_BYTES + 1))]),
        BTreeMap::from([("key\nforged".to_owned(), "value".to_owned())]),
        BTreeMap::from([("key".to_owned(), "value\nforged".to_owned())]),
        (0..=MAX_ATTRIBUTE_COUNT)
            .map(|index| (format!("key_{index}"), "value".to_owned()))
            .collect(),
    ];

    for attributes in invalid_attributes {
        let mut receipt = base.clone();
        let AnalyzerWorkerOutcome::Completed { findings } = &mut receipt.outcome else {
            unreachable!("fixture must contain a completed outcome")
        };
        findings[0].attributes = attributes;
        assert!(
            receipt.validate_against(&request).is_err(),
            "worker finding attributes must be bounded before downstream evidence ingestion"
        );
    }
}
