//! Regression for planning a release-authorized runtime gate binding.
//!
//! The adapter must consume an already verified [`RuntimeGateArtifact`] rather than deriving trust
//! from hostile image content or from the mutable source path at container-create time. This slice
//! proves immutable gate identity and initial-process argv construction only; the canonical Podman
//! execution path and bounded release channel remain separate issue #25 gates.

#![cfg(target_os = "linux")]

#[path = "support/runtime_gate_fixture.rs"]
mod runtime_gate_fixture;

use quarantine_sandbox_runtime::{
    CommandExecutionRequest, IsolationPolicy, ResourceRequest, RootlessPodmanAdapter,
    RuntimeGateArtifact,
};
use runtime_gate_fixture::write_self_contained_gate;
use sha2::{Digest, Sha256};
use tempfile::tempdir;

fn policy() -> IsolationPolicy {
    IsolationPolicy {
        policy_id: "runtime_gate_artifact_binding_policy_v1".to_owned(),
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
        request_id: "runtime-gate-artifact-binding-request".to_owned(),
        image_reference: format!("localhost/cwl/tool@sha256:{}", "a".repeat(64)),
        command: vec![
            "payload-sentinel".to_owned(),
            "argument with spaces".to_owned(),
        ],
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
fn configured_release_artifact_plans_read_only_gate_as_initial_process() {
    let directory = tempdir().expect("temporary directory should be available");
    let (gate_source, gate_bytes) = write_self_contained_gate(directory.path());
    let gate_sha256 = format!("{:x}", Sha256::digest(&gate_bytes));
    let gate = RuntimeGateArtifact::stage(&gate_source, &gate_sha256, std::env::consts::ARCH)
        .expect("matching release-authorized gate artifact should stage");
    let staged_gate_path = gate.path().display().to_string();
    let adapter = RootlessPodmanAdapter::new("podman").with_runtime_gate_artifact(gate);

    let plan = adapter
        .plan_command_binding(&request(), &policy())
        .expect("valid request should produce a fail-closed gate binding plan");
    let args = plan.container_create_binding_args();

    assert_eq!(args[0], "--interactive");
    assert_eq!(args[1], "--volume");
    assert_eq!(args[2], format!("{staged_gate_path}:/qsr-runtime-gate:ro"));
    assert_eq!(args[3], "--entrypoint=/qsr-runtime-gate");
    assert_eq!(args[4], "--");
    assert_eq!(args[5], request().image_reference);
    assert_eq!(args[6].len(), 64, "release token must retain 256 bits");
    assert!(
        args[6]
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte)),
        "release token must be canonical lowercase hex"
    );
    assert_eq!(&args[7..], request().command.as_slice());
    assert_eq!(plan.runtime_gate_sha256(), gate_sha256);
    assert_eq!(plan.runtime_gate_architecture(), std::env::consts::ARCH);
    assert!(
        !args
            .iter()
            .any(|arg| arg.starts_with("--entrypoint=[\"payload-sentinel\"")),
        "consumer argv must never become the OCI entrypoint in a gate binding plan"
    );
}
