//! Bound Podman wait output before it can become workload exit evidence.

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

const BACKEND_INFO: &str = "{\"host\":{\"security\":{\"rootless\":true,\"seccompEnabled\":true,\"seccompProfilePath\":\"/usr/share/containers/seccomp.json\",\"apparmorEnabled\":true,\"selinuxEnabled\":false}},\"version\":{\"Version\":\"6.1.0\"}}";
const CONTAINER_INSPECTION: &str = "[{\"Id\":\"fake-command-container-id\",\"AppArmorProfile\":\"containers-default\",\"ProcessLabel\":\"\",\"EffectiveCaps\":null,\"BoundingCaps\":null,\"Config\":{\"User\":\"65532:65532\",\"Timeout\":20},\"HostConfig\":{\"ReadonlyRootfs\":true,\"Privileged\":false,\"SecurityOpt\":[\"no-new-privileges\"],\"UsernsMode\":\"auto\",\"Annotations\":{},\"PidMode\":\"private\",\"IpcMode\":\"none\",\"NetworkMode\":\"none\",\"UTSMode\":\"private\",\"CgroupMode\":\"private\",\"Memory\":268435456,\"NanoCpus\":1000000000,\"PidsLimit\":16,\"Tmpfs\":{\"/tmp\":\"rw,noexec,nosuid,nodev,size=16777216\"}},\"Mounts\":[]}]";
const PROCESS_TOP: &str = "PID SECCOMP CAPEFF CAPBND CAPINH CAPPRM CAPAMB LABEL\n1 filter - - - - - containers-default (enforce)";

fn policy() -> IsolationPolicy {
    IsolationPolicy {
        policy_id: "command_wait_output_budget_v1".to_owned(),
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
        request_id: "command-wait-output-budget".to_owned(),
        image_reference: format!("localhost/cwl/tool@sha256:{}", "c".repeat(64)),
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

#[test]
fn oversized_wait_stdout_fails_closed_before_exit_code_acceptance() {
    let directory = tempfile::Builder::new()
        .prefix("qsr-command-wait-output-budget-")
        .tempdir()
        .expect("isolated fake-Podman directory");
    let program = directory.path().join("wait-output-budget-podman");
    let scenario = directory.path().join("scenario.sh");
    let call_log = directory.path().join("calls.log");
    symlink(fixture_executable(), &program).expect("fake Podman symlink must be creatable");

    fs::write(
        &scenario,
        format!(
            r#"#!/bin/sh
set -eu
printf '%s\n' "$*" >> '{call_log}'
case "${{1:-}}:${{2:-}}" in
  info:--format) printf '%s\n' '{BACKEND_INFO}' ;;
  create:*) printf '%s\n' 'fake-command-container-id' ;;
  init:*) : ;;
  container:inspect) printf '%s\n' '{CONTAINER_INSPECTION}' ;;
  start:*) : ;;
  top:*) printf '%s\n' '{PROCESS_TOP}' ;;
  wait:*) head -c 4096 /dev/zero | tr '\000' '7'; printf '\n' ;;
  rm:--force) : ;;
  *) exit 91 ;;
esac
"#,
            call_log = call_log.display(),
        ),
    )
    .expect("fake Podman scenario must be writable");
    fs::write(
        config_path(&program),
        format!(
            "MODE='source_script'\nLOG='/dev/null'\nSCRIPT='{}'\n",
            scenario.display()
        ),
    )
    .expect("fake Podman config must be writable");

    let result = RootlessPodmanAdapter::new(program)
        .with_command_output_limit_bytes(2_048)
        .run_legacy_command_at_for_test(&request(), &policy(), 1_780_000_600);

    assert_eq!(
        result,
        Err(CommandExecutionError::Backend(
            ApplicationServiceError::BackendOutputLimitExceeded {
                operation: "command_wait",
            },
        )),
        "oversized Podman wait output must not become workload exit evidence"
    );

    let calls = fs::read_to_string(call_log).expect("call log must remain readable");
    assert!(
        calls
            .lines()
            .any(|line| line == "wait fake-command-container-id"),
        "the regression must reach exact-ID wait rather than fail in an earlier fixture stage"
    );
    assert!(
        calls
            .lines()
            .any(|line| line == "rm --force --ignore fake-command-container-id"),
        "output-budget failure must still clean up only the acquired container ID"
    );
}
