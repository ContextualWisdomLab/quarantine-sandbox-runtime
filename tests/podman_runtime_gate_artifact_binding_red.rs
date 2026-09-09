//! RED for binding a release-authorized runtime gate artifact into command execution.
//!
//! The adapter must consume an already verified [`RuntimeGateArtifact`] rather than deriving trust
//! from hostile image content or from the mutable source path at container-create time. This slice
//! proves only immutable gate delivery and initial-process binding; the bounded release channel and
//! same-head payload-release GREEN remain separate issue #25 gates.

#![cfg(target_os = "linux")]

use std::{
    fs,
    os::unix::fs::PermissionsExt,
    path::PathBuf,
    sync::atomic::{AtomicU64, Ordering},
    time::{SystemTime, UNIX_EPOCH},
};

use quarantine_sandbox_runtime::{
    CommandExecutionRequest, IsolationPolicy, ResourceRequest, RootlessPodmanAdapter,
    RuntimeGateArtifact,
};
use sha2::{Digest, Sha256};

static NEXT_PATH_ID: AtomicU64 = AtomicU64::new(0);

fn temporary_path(name: &str) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock should be after the Unix epoch")
        .as_nanos();
    let unique_id = NEXT_PATH_ID.fetch_add(1, Ordering::Relaxed);
    std::env::temp_dir().join(format!(
        "quarantine-sandbox-runtime-gate-artifact-binding-{name}-{}-{nanos}-{unique_id}",
        std::process::id()
    ))
}

fn write_executable(name: &str, script: &str) -> PathBuf {
    let program = temporary_path(name);
    fs::write(&program, script).expect("fake Podman should be writable");
    let mut permissions = fs::metadata(&program)
        .expect("fake Podman metadata should exist")
        .permissions();
    permissions.set_mode(0o700);
    fs::set_permissions(&program, permissions).expect("fake Podman should be executable");
    program
}

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
fn configured_release_artifact_is_bound_read_only_and_selected_as_initial_process() {
    let gate_source = std::env::current_exe().expect("current test executable should exist");
    let gate_bytes = fs::read(&gate_source).expect("current test executable should be readable");
    let gate_sha256 = format!("{:x}", Sha256::digest(gate_bytes));
    let gate = RuntimeGateArtifact::stage(&gate_source, &gate_sha256, std::env::consts::ARCH)
        .expect("matching release-authorized gate artifact should stage");
    let staged_gate_path = gate.path().display().to_string();

    let calls = temporary_path("calls");
    let security_info = r#"{"host":{"security":{"rootless":true,"seccompEnabled":true,"seccompProfilePath":"/usr/share/containers/seccomp.json","apparmorEnabled":true,"selinuxEnabled":false}},"version":{"Version":"6.1.0"}}"#;
    let script = format!(
        "#!/bin/sh\nset -eu\nprintf '%s\\n' \"$*\" >> '{}'\ncase \"${{1:-}}:${{2:-}}\" in\n  info:--format) printf '%s\\n' '{}' ;;\n  create:--name) printf '{}\\n' ;;\n  init:*) exit 91 ;;\n  rm:--force) : ;;\n  *) exit 92 ;;\nesac\n",
        calls.display(),
        security_info,
        "a".repeat(64),
    );
    let program = write_executable("podman", &script);
    let adapter = RootlessPodmanAdapter::new(program.clone()).with_runtime_gate_artifact(gate);

    let result = adapter.run_command_at(&request(), &policy(), 1_780_000_301);
    assert!(result.is_err(), "the fake backend intentionally stops at init");

    let recorded_calls = fs::read_to_string(&calls).expect("backend calls should be recorded");
    let create_call = recorded_calls
        .lines()
        .find(|line| line.starts_with("create --name "))
        .expect("the command runtime must issue one Podman create request");
    assert!(
        create_call.contains(&format!("{staged_gate_path}:/qsr-runtime-gate:ro")),
        "the verified staged artifact must be delivered read-only: {create_call}"
    );
    assert!(
        create_call.contains("--entrypoint=/qsr-runtime-gate"),
        "the runtime-owned gate must be the initial OCI process: {create_call}"
    );
    assert!(
        !create_call.contains("--entrypoint=[\"payload-sentinel\""),
        "consumer argv must not become the OCI entrypoint when gate authority is configured: {create_call}"
    );
    assert!(
        create_call.contains("payload-sentinel") && create_call.contains("argument with spaces"),
        "exact consumer argv must remain available behind the gate: {create_call}"
    );

    let _ = fs::remove_file(program);
    let _ = fs::remove_file(calls);
}
