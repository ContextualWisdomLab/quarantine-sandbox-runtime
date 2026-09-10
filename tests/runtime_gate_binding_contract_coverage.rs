//! Public runtime-gate binding contract witnesses.

#![cfg(target_os = "linux")]

#[path = "support/runtime_gate_fixture.rs"]
mod runtime_gate_fixture;

use std::path::Path;

use quarantine_sandbox_runtime::{
    ApplicationServiceError, CommandExecutionError, CommandExecutionRequest, IsolationPolicy,
    ResourceRequest, RootlessPodmanAdapter, RuntimeGateArtifact,
};
use runtime_gate_fixture::write_self_contained_gate;
use sha2::{Digest, Sha256};

fn policy() -> IsolationPolicy {
    IsolationPolicy {
        policy_id: "runtime_gate_binding_contract".to_owned(),
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
        request_id: "runtime-gate-binding-contract".to_owned(),
        image_reference: format!("localhost/cwl/tool@sha256:{}", "a".repeat(64)),
        command: vec!["payload-sentinel".to_owned(), "argument with spaces".to_owned()],
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

fn runtime_gate_artifact(directory: &Path) -> (RuntimeGateArtifact, String) {
    let (source, bytes) = write_self_contained_gate(directory);
    let expected_sha256 = format!("{:x}", Sha256::digest(&bytes));
    let artifact = RuntimeGateArtifact::stage(&source, &expected_sha256, std::env::consts::ARCH)
        .expect("matching runtime gate artifact should stage");
    (artifact, expected_sha256)
}

#[test]
fn binding_plan_exposes_verified_gate_identity_without_reinterpreting_consumer_argv() {
    let directory = tempfile::tempdir().expect("fixture directory should exist");
    let (artifact, expected_sha256) = runtime_gate_artifact(directory.path());
    let adapter = RootlessPodmanAdapter::new("podman").with_runtime_gate_artifact(artifact);
    let request = request();

    let plan = adapter
        .plan_command_binding(&request, &policy())
        .expect("valid binding should plan");

    assert_eq!(plan.runtime_gate_sha256(), expected_sha256);
    assert_eq!(plan.runtime_gate_architecture(), std::env::consts::ARCH);
    assert!(
        plan.container_create_binding_args()
            .windows(2)
            .any(|pair| pair[0] == "--" && pair[1] == request.image_reference),
        "the image boundary must remain explicit before held consumer argv"
    );
    assert_eq!(
        plan.container_create_binding_args().last().map(String::as_str),
        Some("argument with spaces"),
        "consumer argv must remain a direct argument rather than shell text"
    );
}

#[test]
fn gated_run_validates_request_before_any_backend_invocation() {
    let directory = tempfile::tempdir().expect("fixture directory should exist");
    let (artifact, _) = runtime_gate_artifact(directory.path());
    let adapter = RootlessPodmanAdapter::new(directory.path().join("must-not-run-podman"))
        .with_runtime_gate_artifact(artifact);
    let mut invalid_request = request();
    invalid_request.resources.memory_bytes = 0;

    assert_eq!(
        adapter.run_command_at(&invalid_request, &policy(), 1_700_000_000),
        Err(CommandExecutionError::Backend(
            ApplicationServiceError::ResourceLimitExceeded {
                resource_name: "memory_bytes",
            },
        ))
    );
}
