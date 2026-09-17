//! RED coverage for the runtime-gate Podman option/data boundary.
//!
//! The trusted gate remains runtime-owned, but the workload image is consumer-controlled data.
//! It must therefore be placed after Podman's option terminator before the one-time release token
//! and exact consumer argv are appended as the gate process arguments.

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
        policy_id: "runtime_gate_option_terminator_policy_v1".to_owned(),
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
        request_id: "runtime-gate-option-terminator-request".to_owned(),
        image_reference: format!("-consumer/tool@sha256:{}", "a".repeat(64)),
        command: vec![
            "--consumer-command".to_owned(),
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
fn runtime_gate_binding_terminates_podman_options_before_consumer_image() {
    let directory = tempdir().expect("temporary directory should be available");
    let (gate_source, gate_bytes) = write_self_contained_gate(directory.path());
    let gate_sha256 = format!("{:x}", Sha256::digest(&gate_bytes));
    let gate = RuntimeGateArtifact::stage(&gate_source, &gate_sha256, std::env::consts::ARCH)
        .expect("matching release-authorized gate artifact should stage");
    let adapter = RootlessPodmanAdapter::new("podman").with_runtime_gate_artifact(gate);
    let request = request();

    let plan = adapter
        .plan_command_binding(&request, &policy())
        .expect("valid request should produce a fail-closed gate binding plan");
    let args = plan.container_create_binding_args();
    let image_index = args
        .iter()
        .position(|argument| argument == &request.image_reference)
        .expect("the exact consumer image must remain a positional operand");

    assert!(
        image_index > 0,
        "the image must not be the first binding argument"
    );
    assert_eq!(
        args[image_index - 1],
        "--",
        "Podman option parsing must terminate immediately before consumer image data"
    );
    assert_eq!(
        args[image_index + 1].len(),
        64,
        "the one-time release token must remain the gate's first argv entry"
    );
    assert_eq!(
        &args[image_index + 2..],
        request.command.as_slice(),
        "consumer command entries must remain exact gate argv after the release token"
    );
}
