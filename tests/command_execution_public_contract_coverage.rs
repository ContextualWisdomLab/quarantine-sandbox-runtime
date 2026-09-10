//! Public command-execution evidence accessors and error-classification coverage.

use std::{io, path::PathBuf};

use quarantine_sandbox_runtime::{
    CONTRACT_SCHEMA_VERSION, CommandExecutionError, CommandExecutionRequest, CommandExecutionResult,
    IsolationPolicy, PrSourceArtifactError, PrSourceArtifactInput, ResourceRequest,
};
use serde_json::json;

fn result_with_source_receipt() -> CommandExecutionResult {
    serde_json::from_value(json!({
        "schema_version": "1.0.0",
        "request_id": "coverage-public-result",
        "image_reference": format!("localhost/cwl/tool@sha256:{}", "a".repeat(64)),
        "backend_id": "podman",
        "backend_version": "5.6.1",
        "sandbox_id": "sandbox-123",
        "exit_code": 17,
        "timed_out": false,
        "stdout": "stdout evidence",
        "stdout_truncated": true,
        "stderr": "stderr evidence",
        "stderr_truncated": false,
        "started_at_epoch_seconds": 1_700_000_000_u64,
        "finished_at_epoch_seconds": 1_700_000_003_u64,
        "source_artifact_receipt": {
            "revision_sha": "b".repeat(40),
            "tree_sha256": "c".repeat(64),
            "executable_files_stripped": 2,
            "regular_file_count": 7,
            "total_bytes": 4096,
            "mounted_read_only": true,
            "mounted_noexec": true
        }
    }))
    .expect("complete serialized command result should deserialize")
}

#[test]
fn public_result_accessors_preserve_attributable_execution_evidence() {
    let result = result_with_source_receipt();

    assert_eq!(result.schema_version(), "1.0.0");
    assert_eq!(result.request_id(), "coverage-public-result");
    assert!(result.image_reference().ends_with(&"a".repeat(64)));
    assert_eq!(result.backend_id(), "podman");
    assert_eq!(result.backend_version(), "5.6.1");
    assert_eq!(result.sandbox_id(), "sandbox-123");
    assert_eq!(result.exit_code(), 17);
    assert!(!result.timed_out());
    assert_eq!(result.stdout(), "stdout evidence");
    assert!(result.stdout_truncated());
    assert_eq!(result.stderr(), "stderr evidence");
    assert!(!result.stderr_truncated());
    assert_eq!(result.started_at_epoch_seconds(), 1_700_000_000);
    assert_eq!(result.finished_at_epoch_seconds(), 1_700_000_003);

    let receipt = result
        .source_artifact_receipt()
        .expect("serialized source receipt should remain attributable");
    assert_eq!(receipt.revision_sha(), "b".repeat(40));
    assert_eq!(receipt.tree_sha256(), "c".repeat(64));
    assert_eq!(receipt.executable_files_stripped(), 2);
    assert_eq!(receipt.regular_file_count(), 7);
    assert_eq!(receipt.total_bytes(), 4096);
    assert!(receipt.mounted_read_only());
    assert!(receipt.mounted_noexec());
}

fn assert_source_error_reason(error: PrSourceArtifactError, expected_reason: &'static str) {
    assert_eq!(
        CommandExecutionError::from(error),
        CommandExecutionError::InvalidSourceArtifact {
            reason: expected_reason,
        }
    );
}

#[test]
fn source_staging_failures_map_to_stable_non_sensitive_command_reasons() {
    assert_source_error_reason(
        PrSourceArtifactError::InvalidInput {
            field_name: "revision_sha",
        },
        "invalid_input",
    );
    assert_source_error_reason(
        PrSourceArtifactError::UnsupportedEntry {
            relative_path: "unsupported-entry".to_owned(),
        },
        "unsupported_entry",
    );
    assert_source_error_reason(
        PrSourceArtifactError::LimitExceeded {
            limit_name: "total_bytes",
        },
        "limit_exceeded",
    );
    assert_source_error_reason(PrSourceArtifactError::DigestMismatch, "digest_mismatch");
    assert_source_error_reason(
        PrSourceArtifactError::Io(io::Error::other("private staging detail")),
        "staging_failed",
    );
}

fn isolation_policy() -> IsolationPolicy {
    IsolationPolicy {
        policy_id: "ci_default_policy".to_owned(),
        maximum_memory_bytes: 512 * 1024 * 1024,
        maximum_cpu_millicores: 2_000,
        maximum_processes: 64,
        maximum_lease_seconds: 600,
        maximum_tmpfs_bytes: 64 * 1024 * 1024,
        readiness_timeout_millis: 5_000,
        readiness_poll_interval_millis: 100,
        shutdown_grace_seconds: 5,
        run_as_user_id: 65_534,
        run_as_group_id: 65_534,
    }
}

fn bounded_resources() -> ResourceRequest {
    ResourceRequest {
        memory_bytes: 256 * 1024 * 1024,
        cpu_millicores: 1_000,
        maximum_processes: 16,
        lease_seconds: 120,
        tmpfs_bytes: 16 * 1024 * 1024,
    }
}

fn command_request_with_source(source_artifact: PrSourceArtifactInput) -> CommandExecutionRequest {
    CommandExecutionRequest {
        schema_version: CONTRACT_SCHEMA_VERSION.to_owned(),
        request_id: "coverage-source-contract".to_owned(),
        image_reference: format!("localhost/cwl/tool@sha256:{}", "a".repeat(64)),
        command: vec!["cargo".to_owned(), "test".to_owned()],
        source_artifact: Some(source_artifact),
        resources: bounded_resources(),
    }
}

#[test]
fn command_request_validates_present_source_artifact_before_backend_dispatch() {
    let policy = isolation_policy();
    let valid_source = PrSourceArtifactInput {
        host_path: PathBuf::from("/tmp/qsr-source-contract"),
        revision_sha: "b".repeat(40),
        expected_tree_sha256: "c".repeat(64),
    };
    assert_eq!(command_request_with_source(valid_source).validate(&policy), Ok(()));

    let malformed_source = PrSourceArtifactInput {
        host_path: PathBuf::from("/tmp/qsr-source-contract"),
        revision_sha: "g".repeat(40),
        expected_tree_sha256: "c".repeat(64),
    };
    assert_eq!(
        command_request_with_source(malformed_source).validate(&policy),
        Err(CommandExecutionError::InvalidSourceArtifact {
            reason: "invalid_input",
        })
    );
}
