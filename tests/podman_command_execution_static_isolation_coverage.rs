//! Command-runtime static isolation controls fail closed before consumer start.
//!
//! These witnesses exercise distinct applied-configuration controls through the real command
//! adapter. They deliberately keep the backend/process evidence valid so each case reaches one
//! specific static isolation decision instead of passing through a synthetic helper seam.

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

const OWNED_CONTAINER_ID: &str = "2222222222222222222222222222222222222222222222222222222222222222";

fn policy() -> IsolationPolicy {
    IsolationPolicy {
        policy_id: "command_static_isolation_coverage_v1".to_owned(),
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
        request_id: "command-static-isolation-coverage".to_owned(),
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

fn fixture_executable() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/fake_podman.sh")
}

fn config_path(program: &Path) -> PathBuf {
    PathBuf::from(format!("{}.config", program.display()))
}

fn fake_podman(mode: &str) -> (TempDir, PathBuf, PathBuf) {
    let directory = tempfile::Builder::new()
        .prefix("qsr-command-static-isolation-")
        .tempdir()
        .expect("isolated fake-Podman directory");
    let program = directory.path().join("podman");
    let calls = directory.path().join("calls");
    let scenario = directory.path().join("scenario.sh");
    symlink(fixture_executable(), &program).expect("fake Podman symlink must be creatable");

    let (readonly_rootfs, privileged, security_options, userns_mode, pid_mode, ipc_mode) = match mode {
        "writable_root" => (false, false, "[\"no-new-privileges\"]", "auto", "private", "none"),
        "privileged" => (true, true, "[\"no-new-privileges\"]", "auto", "private", "none"),
        "missing_nnp" => (true, false, "[]", "auto", "private", "none"),
        "host_userns" => (true, false, "[\"no-new-privileges\"]", "host", "private", "none"),
        "host_pid" => (true, false, "[\"no-new-privileges\"]", "auto", "host", "none"),
        "host_ipc" => (true, false, "[\"no-new-privileges\"]", "auto", "private", "host"),
        other => panic!("unsupported static isolation mode: {other}"),
    };
    let inspect = format!(
        "[{{\"Id\":\"{OWNED_CONTAINER_ID}\",\"AppArmorProfile\":\"containers-default\",\"ProcessLabel\":\"\",\"EffectiveCaps\":[],\"BoundingCaps\":[],\"Config\":{{\"User\":\"65532:65532\",\"Timeout\":20}},\"HostConfig\":{{\"ReadonlyRootfs\":{readonly_rootfs},\"Privileged\":{privileged},\"SecurityOpt\":{security_options},\"UsernsMode\":\"{userns_mode}\",\"PidMode\":\"{pid_mode}\",\"IpcMode\":\"{ipc_mode}\",\"NetworkMode\":\"none\",\"UTSMode\":\"private\",\"CgroupMode\":\"private\",\"Memory\":268435456,\"NanoCpus\":1000000000,\"PidsLimit\":16,\"Tmpfs\":{{\"/tmp\":\"rw,noexec,nosuid,nodev,size=16777216\"}}}},\"Mounts\":[]}}]"
    );
    let script = format!(
        r#"
COMMAND_INFO='{{"host":{{"security":{{"rootless":true,"seccompEnabled":true,"seccompProfilePath":"/usr/share/containers/seccomp.json","apparmorEnabled":true,"selinuxEnabled":false}}}},"version":{{"Version":"6.1.0"}}}}'
COMMAND_INSPECT='{inspect}'
COMMAND_TOP='PID SECCOMP CAPEFF CAPBND CAPINH CAPPRM CAPAMB LABEL
1 filter - - - - - containers-default (enforce)'
case "${{1:-}}:${{2:-}}" in
  info:--format) printf '%s\n' "$COMMAND_INFO" ;;
  create:--name) printf '%s\n' "$OWNED_CONTAINER_ID" ;;
  init:*) [ "${{2:-}}" = "$OWNED_CONTAINER_ID" ] || exit 97 ;;
  start:*) [ "${{2:-}}" = "$OWNED_CONTAINER_ID" ] || exit 97 ;;
  container:inspect) [ "${{5:-}}" = "$OWNED_CONTAINER_ID" ] || exit 97; printf '%s\n' "$COMMAND_INSPECT" ;;
  top:*) [ "${{2:-}}" = "$OWNED_CONTAINER_ID" ] || exit 97; printf '%s\n' "$COMMAND_TOP" ;;
  wait:*) [ "${{2:-}}" = "$OWNED_CONTAINER_ID" ] || exit 97; printf '0\n' ;;
  logs:*) [ "${{2:-}}" = "$OWNED_CONTAINER_ID" ] || exit 97; printf 'consumer-output\n' ;;
  kill:*) [ "${{2:-}}" = "$OWNED_CONTAINER_ID" ] || exit 97 ;;
  rm:--force) [ "${{4:-}}" = "$OWNED_CONTAINER_ID" ] || exit 97 ;;
  *) exit 91 ;;
esac
"#
    );
    fs::write(&scenario, script).expect("fake Podman scenario data must be writable");
    fs::write(
        config_path(&program),
        format!(
            "MODE='source_script'\nLOG='{}'\nSCRIPT='{}'\nOWNED_CONTAINER_ID='{OWNED_CONTAINER_ID}'\n",
            calls.display(),
            scenario.display(),
        ),
    )
    .expect("fake Podman data config must be writable");

    (directory, program, calls)
}

fn assert_static_control_fails_closed(mode: &str, control_name: &'static str) {
    let (_directory, program, calls_path) = fake_podman(mode);
    let adapter = RootlessPodmanAdapter::new(program);

    let result = adapter.run_legacy_command_at_for_test(&request(), &policy(), 1_780_000_202);
    let calls = fs::read_to_string(calls_path).expect("fake Podman calls must be recorded");

    assert_eq!(
        result,
        Err(CommandExecutionError::Backend(
            ApplicationServiceError::IsolationVerificationFailed { control_name },
        )),
        "{mode} must fail at its own static isolation control"
    );
    assert!(
        calls
            .lines()
            .any(|line| line == format!("rm --force --ignore {OWNED_CONTAINER_ID}")),
        "{mode} must clean up only the acquired container ID: {calls}"
    );
    assert!(
        !calls.lines().any(|line| line.starts_with("start ")),
        "{mode} must fail before consumer start: {calls}"
    );
}

#[test]
fn writable_root_filesystem_fails_closed_before_start() {
    assert_static_control_fails_closed("writable_root", "read_only_root_filesystem");
}

#[test]
fn privileged_container_fails_closed_before_start() {
    assert_static_control_fails_closed("privileged", "unprivileged_container");
}

#[test]
fn missing_no_new_privileges_fails_closed_before_start() {
    assert_static_control_fails_closed("missing_nnp", "no_new_privileges");
}

#[test]
fn host_user_namespace_fails_closed_before_start() {
    assert_static_control_fails_closed("host_userns", "isolated_user_namespace");
}

#[test]
fn host_pid_namespace_fails_closed_before_start() {
    assert_static_control_fails_closed("host_pid", "isolated_pid_namespace");
}

#[test]
fn host_ipc_namespace_fails_closed_before_start() {
    assert_static_control_fails_closed("host_ipc", "isolated_ipc_namespace");
}
