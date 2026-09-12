//! Default command-adapter construction preserves pre-backend policy validation.
//!
//! This witness constructs `RootlessPodmanAdapter::default()` and requires an invalid root-user
//! policy to fail through the typed public command boundary before any Podman invocation.

#![cfg(target_os = "linux")]

use quarantine_sandbox_runtime::{
    ApplicationServiceError, CommandExecutionError, CommandExecutionRequest, IsolationPolicy,
    ResourceRequest, RootlessPodmanAdapter,
};

fn policy() -> IsolationPolicy {
    IsolationPolicy {
        policy_id: "default_adapter_contract_v1".to_owned(),
        maximum_memory_bytes: 512 * 1024 * 1024,
        maximum_cpu_millicores: 2_000,
        maximum_processes: 64,
        maximum_lease_seconds: 300,
        maximum_tmpfs_bytes: 64 * 1024 * 1024,
        readiness_timeout_millis: 1_000,
        readiness_poll_interval_millis: 10,
        shutdown_grace_seconds: 2,
        run_as_user_id: 65_532,
        run_as_group_id: 65_532,
    }
}

fn request() -> CommandExecutionRequest {
    CommandExecutionRequest {
        schema_version: "1.0.0".to_owned(),
        request_id: "default-adapter-contract".to_owned(),
        image_reference: format!("localhost/cwl/tool@sha256:{}", "e".repeat(64)),
        command: vec!["true".to_owned()],
        source_artifact: None,
        resources: ResourceRequest {
            memory_bytes: 256 * 1024 * 1024,
            cpu_millicores: 1_000,
            maximum_processes: 16,
            lease_seconds: 20,
            tmpfs_bytes: 16 * 1024 * 1024,
        },
    }
}

#[test]
fn default_adapter_keeps_policy_validation_a_pre_backend_boundary() {
    let adapter = RootlessPodmanAdapter::default();
    let mut invalid_policy = policy();
    invalid_policy.run_as_user_id = 0;

    let result = adapter.run_legacy_command_at_for_test(&request(), &invalid_policy, 1_780_000_000);

    assert_eq!(
        result,
        Err(CommandExecutionError::Backend(
            ApplicationServiceError::InvalidPolicy {
                field_name: "run_as_user_id",
            },
        ))
    );
}
