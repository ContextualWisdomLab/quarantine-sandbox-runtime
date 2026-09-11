//! Command execution must reject applied host UTS/cgroup namespace modes and
//! clean up only by the acquired container identity.
//!
//! These regressions retain the namespace-attestation contract after the
//! command lifecycle moved cleanup authority from the generated correlation name
//! to the container ID returned by a successful create.

#![cfg(target_os = "linux")]

use std::{
    fs,
    os::unix::fs::symlink,
    path::{Path, PathBuf},
};

use quarantine_sandbox_runtime::{
    ApplicationServiceError, CommandExecutionError, CommandExecutionRequest, IsolationPolicy,
    ResourceRequest, RootlessPodmanAdapter,
};
use tempfile::TempDir;

fn policy() -> IsolationPolicy {
    IsolationPolicy {
        policy_id: "command_namespace_red_v1".to_owned(),
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
        request_id: "command-namespace-red".to_owned(),
        image_reference: format!("localhost/cwl/tool@sha256:{}", "d".repeat(64)),
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

fn fixture_executable() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/fake_podman.sh")
}

fn config_path(program: &Path) -> PathBuf {
    PathBuf::from(format!("{}.config", program.display()))
}

fn fake_podman(mode: &str) -> (TempDir, PathBuf, PathBuf) {
    let directory = tempfile::Builder::new()
        .prefix("qsr-command-namespace-red-")
        .tempdir()
        .expect("isolated fake-Podman directory");
    let program = directory.path().join("podman");
    let calls = directory.path().join("calls");
    symlink(fixture_executable(), &program).expect("fake Podman symlink must be creatable");
    fs::write(
        config_path(&program),
        format!("MODE='{mode}'\nREADY_PORT='0'\nLOG='{}'\n", calls.display()),
    )
    .expect("fake Podman data config must be writable");
    (directory, program, calls)
}

fn assert_namespace_config_rejected(mode: &str, control_name: &'static str, evidence_name: &str) {
    let (_directory, program, calls_path) = fake_podman(mode);
    let adapter = RootlessPodmanAdapter::new(program);

    let result = adapter.run_legacy_command_at_for_test(&request(), &policy(), 1_780_000_100);
    let calls = fs::read_to_string(calls_path).expect("fake Podman calls must be recorded");
    let sandbox_name = calls
        .lines()
        .find_map(|line| {
            let mut parts = line.split_whitespace();
            (parts.next() == Some("create") && parts.next() == Some("--name"))
                .then(|| parts.next().map(str::to_owned))
                .flatten()
        })
        .expect("create must record the invocation correlation name");

    assert_eq!(
        result,
        Err(CommandExecutionError::Backend(
            ApplicationServiceError::IsolationVerificationFailed { control_name },
        )),
        "{evidence_name} must fail closed instead of becoming command evidence"
    );
    assert!(
        calls
            .lines()
            .any(|line| line == "rm --force --ignore fake-command-container-id"),
        "{evidence_name} must clean up the acquired container ID: {calls}"
    );
    assert!(
        !calls
            .lines()
            .any(|line| line == format!("rm --force --ignore {sandbox_name}")),
        "{evidence_name} must not regain generated-name cleanup authority: {calls}"
    );
    assert!(
        !calls.lines().any(|line| line.starts_with("logs ")),
        "{evidence_name} must be rejected before command output is trusted: {calls}"
    );
}

#[test]
fn host_uts_namespace_configuration_fails_closed_and_cleans_up() {
    assert_namespace_config_rejected(
        "command_host_uts",
        "isolated_uts_namespace",
        "host UTS namespace configuration",
    );
}

#[test]
fn host_cgroup_namespace_configuration_fails_closed_and_cleans_up() {
    assert_namespace_config_rejected(
        "command_host_cgroup",
        "isolated_cgroup_namespace",
        "host cgroup namespace configuration",
    );
}
