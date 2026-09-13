//! Public-path coverage for runtime-gate release attach spawn failures.
//!
//! The release controller must classify an operating-system spawn failure before any token can be
//! written to the gate. This regression uses a deliberately absent Podman executable so the real
//! `Command::spawn` path returns `NotFound`; no synthetic backend result or impossible pipe state is
//! injected.

#![cfg(target_os = "linux")]

#[path = "support/runtime_gate_fixture.rs"]
mod runtime_gate_fixture;

use std::path::Path;

use quarantine_sandbox_runtime::{
    ApplicationServiceError, BackendInvocationFailureKind, CommandExecutionError,
    CommandExecutionRequest, IsolationPolicy, ResourceRequest, RootlessPodmanAdapter,
    RuntimeGateArtifact,
};
use runtime_gate_fixture::write_self_contained_gate;
use sha2::{Digest, Sha256};

const OWNED_CONTAINER_ID: &str =
    "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";

fn policy() -> IsolationPolicy {
    IsolationPolicy {
        policy_id: "runtime_gate_release_spawn_failure".to_owned(),
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
        request_id: "runtime-gate-release-spawn-failure".to_owned(),
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

#[test]
fn release_attach_spawn_not_found_is_classified_before_gate_write() {
    let directory = tempfile::tempdir().expect("fixture directory should exist");
    let missing_program = directory.path().join("podman-does-not-exist");
    let adapter = RootlessPodmanAdapter::new(missing_program)
        .with_runtime_gate_artifact(runtime_gate_artifact(directory.path()));
    let plan = adapter
        .plan_command_binding(&request(), &policy())
        .expect("valid request should produce a release plan");

    let error = adapter
        .release_command_gate(OWNED_CONTAINER_ID, plan)
        .expect_err("a missing release backend must fail before writing the release token");

    assert_eq!(
        error,
        CommandExecutionError::Backend(ApplicationServiceError::BackendSpawnFailed {
            operation: "runtime_gate_release_attach",
            failure_kind: BackendInvocationFailureKind::NotFound,
        })
    );
}
