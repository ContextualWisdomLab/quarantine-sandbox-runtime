//! Exercise fail-closed Podman log retrieval outcomes through the public command adapter.

#![cfg(target_os = "linux")]

use std::{
    fs,
    os::unix::fs::symlink,
    path::{Path, PathBuf},
    time::Duration,
};

use quarantine_sandbox_runtime::{
    ApplicationServiceError, CommandExecutionError, CommandExecutionRequest, IsolationPolicy,
    ResourceRequest, RootlessPodmanAdapter,
};

const BACKEND_INFO: &str = "{\"host\":{\"security\":{\"rootless\":true,\"seccompEnabled\":true,\"seccompProfilePath\":\"/usr/share/containers/seccomp.json\",\"apparmorEnabled\":true,\"selinuxEnabled\":false}},\"version\":{\"Version\":\"6.1.0\"}}";
const CONTAINER_INSPECTION: &str = "[{\"Id\":\"fake-command-container-id\",\"AppArmorProfile\":\"containers-default\",\"ProcessLabel\":\"\",\"EffectiveCaps\":null,\"BoundingCaps\":null,\"Config\":{\"User\":\"65532:65532\",\"Timeout\":20},\"HostConfig\":{\"ReadonlyRootfs\":true,\"Privileged\":false,\"SecurityOpt\":[\"no-new-privileges\"],\"UsernsMode\":\"auto\",\"Annotations\":{},\"PidMode\":\"private\",\"IpcMode\":\"none\",\"NetworkMode\":\"none\",\"UTSMode\":\"private\",\"CgroupMode\":\"private\",\"Memory\":268435456,\"NanoCpus\":1000000000,\"PidsLimit\":16,\"Tmpfs\":{\"/tmp\":\"rw,noexec,nosuid,nodev,size=16777216\"}},\"Mounts\":[]}]";
const PROCESS_TOP: &str = "PID SECCOMP CAPEFF CAPBND CAPINH CAPPRM CAPAMB LABEL\n1 filter - - - - - containers-default (enforce)";

fn policy() -> IsolationPolicy {
    IsolationPolicy {
        policy_id: "command_logs_failure_coverage_v1".to_owned(),
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
        request_id: "command-logs-failure-coverage".to_owned(),
        image_reference: format!("localhost/cwl/tool@sha256:{}", "b".repeat(64)),
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

fn fake_podman(logs_script: &str) -> (tempfile::TempDir, PathBuf, PathBuf) {
    let directory = tempfile::Builder::new()
        .prefix("qsr-command-logs-failure-")
        .tempdir()
        .expect("isolated fake-Podman directory");
    let program = directory.path().join("podman");
    let scenario = directory.path().join("scenario.sh");
    let calls = directory.path().join("calls.log");
    symlink(fixture_executable(), &program).expect("fake Podman symlink must be creatable");

    fs::write(
        &scenario,
        format!(
            r#"#!/bin/sh
set -eu
case "${{1:-}}:${{2:-}}" in
  info:--format) printf '%s\n' '{BACKEND_INFO}' ;;
  create:*) printf '%s\n' 'fake-command-container-id' ;;
  init:*) : ;;
  container:inspect) printf '%s\n' '{CONTAINER_INSPECTION}' ;;
  start:*) : ;;
  top:*) printf '%s\n' '{PROCESS_TOP}' ;;
  wait:*) printf '0\n' ;;
  logs:*) {logs_script} ;;
  rm:--force) : ;;
  *) exit 91 ;;
esac
"#,
        ),
    )
    .expect("fake Podman scenario must be writable");
    fs::write(
        config_path(&program),
        format!(
            "MODE='source_script'\nLOG='{}'\nSCRIPT='{}'\n",
            calls.display(),
            scenario.display()
        ),
    )
    .expect("fake Podman config must be writable");

    (directory, program, calls)
}

fn assert_exact_log_and_cleanup_calls(calls_path: &Path) {
    let calls = fs::read_to_string(calls_path).expect("fake Podman calls must be readable");
    assert!(
        calls
            .lines()
            .any(|line| line == "logs fake-command-container-id"),
        "regression must reach exact-ID log retrieval: {calls}"
    );
    assert!(
        calls
            .lines()
            .any(|line| line == "rm --force --ignore fake-command-container-id"),
        "log retrieval failure must clean only the acquired container ID: {calls}"
    );
}

#[test]
fn nonzero_logs_backend_status_fails_closed_and_cleans_up() {
    let (_directory, program, calls) = fake_podman("printf 'log backend failed\\n' >&2; exit 42");

    let result = RootlessPodmanAdapter::new(program).run_legacy_command_at_for_test(
        &request(),
        &policy(),
        1_780_000_700,
    );

    assert_eq!(
        result,
        Err(CommandExecutionError::Backend(
            ApplicationServiceError::BackendCommandFailed {
                operation: "container_logs",
            },
        )),
        "a failed Podman logs command is infrastructure failure, not empty workload output"
    );
    assert_exact_log_and_cleanup_calls(&calls);
}

#[test]
fn timed_out_logs_backend_fails_closed_and_cleans_up() {
    let (_directory, program, calls) = fake_podman("exec sleep 2");

    let result = RootlessPodmanAdapter::new(program)
        .with_command_timeout(Duration::from_millis(250))
        .run_legacy_command_at_for_test(&request(), &policy(), 1_780_000_700);

    assert_eq!(
        result,
        Err(CommandExecutionError::Backend(
            ApplicationServiceError::BackendCommandTimedOut {
                operation: "container_logs",
            },
        )),
        "a hung log driver must fail closed without changing workload timeout evidence"
    );
    assert_exact_log_and_cleanup_calls(&calls);
}
