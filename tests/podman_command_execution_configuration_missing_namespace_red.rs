//! Missing applied UTS/cgroup namespace evidence must fail closed before command start.

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

const OWNED_CONTAINER_ID: &str = "1111111111111111111111111111111111111111111111111111111111111111";

fn policy() -> IsolationPolicy {
    IsolationPolicy {
        policy_id: "command_missing_namespace_evidence_v1".to_owned(),
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
        request_id: "command-missing-namespace-evidence".to_owned(),
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

fn fake_podman(namespace_mode: &str) -> (TempDir, PathBuf, PathBuf) {
    let directory = tempfile::Builder::new()
        .prefix("qsr-command-missing-namespace-")
        .tempdir()
        .expect("isolated fake-Podman directory");
    let program = directory.path().join("podman");
    let calls = directory.path().join("calls");
    let scenario = directory.path().join("scenario.sh");
    symlink(fixture_executable(), &program).expect("fake Podman symlink must be creatable");

    fs::write(
        &scenario,
        r#"
COMMAND_INFO='{"host":{"security":{"rootless":true,"seccompEnabled":true,"seccompProfilePath":"/usr/share/containers/seccomp.json","apparmorEnabled":true,"selinuxEnabled":false}},"version":{"Version":"6.1.0"}}'
COMMAND_MISSING_UTS='[{"Id":"1111111111111111111111111111111111111111111111111111111111111111","AppArmorProfile":"containers-default","ProcessLabel":"","EffectiveCaps":[],"BoundingCaps":[],"Config":{"User":"65532:65532","Timeout":20},"HostConfig":{"ReadonlyRootfs":true,"Privileged":false,"SecurityOpt":["no-new-privileges"],"UsernsMode":"auto","PidMode":"private","IpcMode":"none","NetworkMode":"none","CgroupMode":"private","Memory":268435456,"NanoCpus":1000000000,"PidsLimit":16,"Tmpfs":{"/tmp":"rw,noexec,nosuid,nodev,size=16777216"}},"Mounts":[]}]'
COMMAND_MISSING_CGROUP='[{"Id":"1111111111111111111111111111111111111111111111111111111111111111","AppArmorProfile":"containers-default","ProcessLabel":"","EffectiveCaps":[],"BoundingCaps":[],"Config":{"User":"65532:65532","Timeout":20},"HostConfig":{"ReadonlyRootfs":true,"Privileged":false,"SecurityOpt":["no-new-privileges"],"UsernsMode":"auto","PidMode":"private","IpcMode":"none","NetworkMode":"none","UTSMode":"private","Memory":268435456,"NanoCpus":1000000000,"PidsLimit":16,"Tmpfs":{"/tmp":"rw,noexec,nosuid,nodev,size=16777216"}},"Mounts":[]}]'
COMMAND_TOP='PID SECCOMP CAPEFF CAPBND CAPINH CAPPRM CAPAMB LABEL
1 filter - - - - - containers-default (enforce)'
case "${1:-}:${2:-}" in
  info:--format) printf '%s\n' "$COMMAND_INFO" ;;
  create:--name) printf '%s\n' "$OWNED_CONTAINER_ID" ;;
  init:*) [ "${2:-}" = "$OWNED_CONTAINER_ID" ] || exit 97 ;;
  start:*) [ "${2:-}" = "$OWNED_CONTAINER_ID" ] || exit 97 ;;
  container:inspect)
    [ "${5:-}" = "$OWNED_CONTAINER_ID" ] || exit 97
    case "$NAMESPACE_MODE" in
      missing_uts) printf '%s\n' "$COMMAND_MISSING_UTS" ;;
      missing_cgroup) printf '%s\n' "$COMMAND_MISSING_CGROUP" ;;
      *) exit 98 ;;
    esac
    ;;
  top:*) [ "${2:-}" = "$OWNED_CONTAINER_ID" ] || exit 97; printf '%s\n' "$COMMAND_TOP" ;;
  wait:*) [ "${2:-}" = "$OWNED_CONTAINER_ID" ] || exit 97; printf '0\n' ;;
  logs:*) [ "${2:-}" = "$OWNED_CONTAINER_ID" ] || exit 97; printf 'consumer-output\n' ;;
  kill:*) [ "${2:-}" = "$OWNED_CONTAINER_ID" ] || exit 97 ;;
  rm:--force) [ "${4:-}" = "$OWNED_CONTAINER_ID" ] || exit 97 ;;
  *) exit 91 ;;
esac
"#,
    )
    .expect("fake Podman scenario data must be writable");

    fs::write(
        config_path(&program),
        format!(
            "MODE='source_script'\nLOG='{}'\nSCRIPT='{}'\nNAMESPACE_MODE='{namespace_mode}'\nOWNED_CONTAINER_ID='{OWNED_CONTAINER_ID}'\n",
            calls.display(),
            scenario.display(),
        ),
    )
    .expect("fake Podman data config must be writable");

    (directory, program, calls)
}

fn assert_missing_namespace_fails_closed(
    namespace_mode: &str,
    control_name: &'static str,
    evidence_name: &str,
) {
    let (_directory, program, calls_path) = fake_podman(namespace_mode);
    let adapter = RootlessPodmanAdapter::new(program);

    let result = adapter.run_command_at(&request(), &policy(), 1_780_000_200);
    let calls = fs::read_to_string(calls_path).expect("fake Podman calls must be recorded");

    assert_eq!(
        result,
        Err(CommandExecutionError::Backend(
            ApplicationServiceError::IsolationVerificationFailed { control_name },
        )),
        "{evidence_name} must fail closed instead of being treated as private"
    );
    assert!(
        calls
            .lines()
            .any(|line| line == format!("rm --force --ignore {OWNED_CONTAINER_ID}")),
        "{evidence_name} must clean up only the acquired container ID: {calls}"
    );
    assert!(
        !calls.lines().any(|line| line.starts_with("start ")),
        "{evidence_name} must fail before the consumer can be started: {calls}"
    );
}

#[test]
fn missing_uts_namespace_configuration_fails_closed_before_start() {
    assert_missing_namespace_fails_closed(
        "missing_uts",
        "isolated_uts_namespace",
        "missing applied UTS namespace evidence",
    );
}

#[test]
fn missing_cgroup_namespace_configuration_fails_closed_before_start() {
    assert_missing_namespace_fails_closed(
        "missing_cgroup",
        "isolated_cgroup_namespace",
        "missing applied cgroup namespace evidence",
    );
}
