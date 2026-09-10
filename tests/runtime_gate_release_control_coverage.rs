#![cfg(target_os = "linux")]
//! Runtime-gate release-control failure and success witnesses.

use std::{fs, os::unix::fs::PermissionsExt, path::Path, time::Duration};

use quarantine_sandbox_runtime::{
    ApplicationServiceError, CommandExecutionError, CommandExecutionRequest, IsolationPolicy,
    ResourceRequest, RootlessPodmanAdapter, RuntimeGateArtifact,
};
use sha2::{Digest, Sha256};

const EXACT_CONTAINER_ID: &str = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";
const ELF_HEADER_BYTES: usize = 64;
const PROGRAM_HEADER_BYTES: usize = 56;
const FILE_BYTES: usize = 512;

fn self_contained_gate_bytes() -> Vec<u8> {
    let machine = match std::env::consts::ARCH {
        "x86_64" => 62_u16,
        "aarch64" => 183_u16,
        other => panic!("runtime-gate test fixture does not support architecture {other}"),
    };
    let mut bytes = vec![0_u8; FILE_BYTES];
    bytes[..7].copy_from_slice(b"\x7fELF\x02\x01\x01");
    bytes[16..18].copy_from_slice(&2_u16.to_le_bytes());
    bytes[18..20].copy_from_slice(&machine.to_le_bytes());
    bytes[20..24].copy_from_slice(&1_u32.to_le_bytes());
    bytes[24..32].copy_from_slice(&0x400100_u64.to_le_bytes());
    bytes[32..40].copy_from_slice(&(ELF_HEADER_BYTES as u64).to_le_bytes());
    bytes[52..54].copy_from_slice(&(ELF_HEADER_BYTES as u16).to_le_bytes());
    bytes[54..56].copy_from_slice(&(PROGRAM_HEADER_BYTES as u16).to_le_bytes());
    bytes[56..58].copy_from_slice(&1_u16.to_le_bytes());
    let header = ELF_HEADER_BYTES;
    bytes[header..header + 4].copy_from_slice(&1_u32.to_le_bytes());
    bytes[header + 4..header + 8].copy_from_slice(&5_u32.to_le_bytes());
    bytes[header + 16..header + 24].copy_from_slice(&0x400000_u64.to_le_bytes());
    bytes[header + 32..header + 40].copy_from_slice(&(FILE_BYTES as u64).to_le_bytes());
    bytes[header + 40..header + 48].copy_from_slice(&(FILE_BYTES as u64).to_le_bytes());
    bytes[header + 48..header + 56].copy_from_slice(&4096_u64.to_le_bytes());
    bytes
}

fn policy() -> IsolationPolicy {
    IsolationPolicy {
        policy_id: "runtime_gate_release_coverage".to_owned(),
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
        request_id: "runtime-gate-release-coverage".to_owned(),
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
    let source = directory.join("self-contained-runtime-gate");
    let bytes = self_contained_gate_bytes();
    fs::write(&source, &bytes).expect("runtime-gate fixture should be writable");
    let expected_sha256 = format!("{:x}", Sha256::digest(&bytes));
    RuntimeGateArtifact::stage(&source, &expected_sha256, std::env::consts::ARCH)
        .expect("matching runtime gate artifact should stage")
}

fn executable_script(directory: &Path, body: &str) -> std::path::PathBuf {
    let script = directory.join("fake-podman");
    fs::write(&script, format!("#!/bin/sh\nset -eu\n{body}\n"))
        .expect("fake Podman script should be writable");
    let mut permissions = fs::metadata(&script)
        .expect("fake Podman metadata should exist")
        .permissions();
    permissions.set_mode(0o700);
    fs::set_permissions(&script, permissions).expect("fake Podman should be executable");
    script
}

#[test]
fn malformed_release_identity_fails_before_attach_spawn() {
    let directory = tempfile::tempdir().expect("fixture directory should exist");
    let marker = directory.path().join("invoked");
    let program = executable_script(
        directory.path(),
        &format!("touch '{}'; exit 90", marker.display()),
    );
    let adapter = RootlessPodmanAdapter::new(program)
        .with_runtime_gate_artifact(runtime_gate_artifact(directory.path()));
    let plan = adapter
        .plan_command_binding(&request(), &policy())
        .expect("valid binding should plan");

    assert_eq!(
        adapter.release_command_gate("not-an-exact-container-id", plan),
        Err(CommandExecutionError::Backend(
            ApplicationServiceError::MalformedIsolationInspection {
                operation: "runtime_gate_release_identity",
            }
        ))
    );
    assert!(
        !marker.exists(),
        "malformed identity must not authorize attach"
    );
}

#[test]
fn attach_spawn_failure_preserves_bounded_failure_class() {
    let directory = tempfile::tempdir().expect("fixture directory should exist");
    let adapter = RootlessPodmanAdapter::new(directory.path().join("missing-podman"))
        .with_runtime_gate_artifact(runtime_gate_artifact(directory.path()));
    let plan = adapter
        .plan_command_binding(&request(), &policy())
        .expect("valid binding should plan");

    assert_eq!(
        adapter.release_command_gate(EXACT_CONTAINER_ID, plan),
        Err(CommandExecutionError::Backend(
            ApplicationServiceError::BackendSpawnFailed {
                operation: "runtime_gate_release_attach",
                failure_kind: quarantine_sandbox_runtime::BackendInvocationFailureKind::NotFound,
            }
        ))
    );
}

#[test]
fn contradictory_acknowledgement_fails_closed_after_attach_cleanup() {
    let directory = tempfile::tempdir().expect("fixture directory should exist");
    let program = executable_script(
        directory.path(),
        "IFS= read -r release_token\nprintf 'NOT_THE_ACK\\n'",
    );
    let adapter = RootlessPodmanAdapter::new(program)
        .with_command_timeout(Duration::from_millis(250))
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

#[test]
fn missing_acknowledgement_times_out_and_terminates_attach_client() {
    let directory = tempfile::tempdir().expect("fixture directory should exist");
    let program = executable_script(directory.path(), "IFS= read -r release_token\nsleep 5");
    let adapter = RootlessPodmanAdapter::new(program)
        .with_command_timeout(Duration::from_millis(50))
        .with_runtime_gate_artifact(runtime_gate_artifact(directory.path()));
    let plan = adapter
        .plan_command_binding(&request(), &policy())
        .expect("valid binding should plan");

    assert_eq!(
        adapter.release_command_gate(EXACT_CONTAINER_ID, plan),
        Err(CommandExecutionError::Backend(
            ApplicationServiceError::BackendCommandTimedOut {
                operation: "runtime_gate_release_ack",
            }
        ))
    );
}

#[test]
fn exact_acknowledgement_releases_then_detaches_local_attach_client() {
    let directory = tempfile::tempdir().expect("fixture directory should exist");
    let program = executable_script(
        directory.path(),
        "IFS= read -r release_token\nprintf 'QSR_GATE_RELEASED\\n'\nsleep 5",
    );
    let adapter = RootlessPodmanAdapter::new(program)
        .with_command_timeout(Duration::from_millis(250))
        .with_runtime_gate_artifact(runtime_gate_artifact(directory.path()));
    let plan = adapter
        .plan_command_binding(&request(), &policy())
        .expect("valid binding should plan");

    assert_eq!(
        adapter.release_command_gate(EXACT_CONTAINER_ID, plan),
        Ok(())
    );
}
