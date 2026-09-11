//! Runtime-gate control-channel EOF coverage.

#![cfg(target_os = "linux")]

#[path = "support/runtime_gate_fixture.rs"]
mod runtime_gate_fixture;

use std::{fs, os::unix::fs::PermissionsExt, path::Path, time::Duration};

use quarantine_sandbox_runtime::{
    ApplicationServiceError, CommandExecutionError, CommandExecutionRequest, IsolationPolicy,
    ResourceRequest, RootlessPodmanAdapter, RuntimeGateArtifact,
};
use runtime_gate_fixture::write_self_contained_gate;
use sha2::{Digest, Sha256};

const EXACT_CONTAINER_ID: &str = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";

fn policy() -> IsolationPolicy {
    IsolationPolicy {
        policy_id: "runtime_gate_release_eof".to_owned(),
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
        request_id: "runtime-gate-release-eof".to_owned(),
        image_reference: format!("localhost/cwl/tool@sha256:{}", "a".repeat(64)),
        command: vec!["payload-sentinel".to_owned()],
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

fn runtime_gate_artifact(directory: &Path) -> RuntimeGateArtifact {
    let (source, bytes) = write_self_contained_gate(directory);
    let expected_sha256 = format!("{:x}", Sha256::digest(&bytes));
    RuntimeGateArtifact::stage(&source, &expected_sha256, std::env::consts::ARCH)
        .expect("matching runtime gate artifact should stage")
}

fn eof_after_release_token_program(directory: &Path) -> std::path::PathBuf {
    let script = directory.join("fake-podman");
    fs::write(
        &script,
        "#!/bin/sh\nset -eu\nIFS= read -r release_token\nexit 0\n",
    )
    .expect("fake Podman script should be writable");
    let mut permissions = fs::metadata(&script)
        .expect("fake Podman metadata should exist")
        .permissions();
    permissions.set_mode(0o700);
    fs::set_permissions(&script, permissions).expect("fake Podman should be executable");
    script
}

#[test]
fn eof_before_release_acknowledgement_fails_closed_without_waiting_for_timeout() {
    let directory = tempfile::tempdir().expect("fixture directory should exist");
    let adapter = RootlessPodmanAdapter::new(eof_after_release_token_program(directory.path()))
        .with_command_timeout(Duration::from_secs(2))
        .with_runtime_gate_artifact(runtime_gate_artifact(directory.path()));
    let plan = adapter
        .plan_command_binding(&request(), &policy())
        .expect("valid binding should plan");

    assert_eq!(
        adapter.release_command_gate(EXACT_CONTAINER_ID, plan),
        Err(CommandExecutionError::Backend(
            ApplicationServiceError::BackendInvocationFailed {
                operation: "runtime_gate_release_ack",
            }
        ))
    );
}
