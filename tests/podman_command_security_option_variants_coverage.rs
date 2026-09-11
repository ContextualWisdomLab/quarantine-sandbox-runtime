//! Exercise supported Podman security-option spellings through the public command adapter.

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
        policy_id: "command_security_option_variants_v1".to_owned(),
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
        request_id: "command-security-option-variants".to_owned(),
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

fn fake_podman(seccomp_unconfined: bool) -> (TempDir, PathBuf, PathBuf) {
    let directory = tempfile::Builder::new()
        .prefix("qsr-command-security-option-variants-")
        .tempdir()
        .expect("isolated fake-Podman directory");
    let program = directory.path().join("podman");
    let calls = directory.path().join("calls");
    let scenario = directory.path().join("scenario.sh");
    symlink(fixture_executable(), &program).expect("fake Podman symlink must be creatable");

    let security_options = if seccomp_unconfined {
        "[\"no-new-privileges=true\",\"seccomp=unconfined\"]"
    } else {
        "[\"no-new-privileges=true\"]"
    };
    let inspect = format!(
        "[{{\"Id\":\"{OWNED_CONTAINER_ID}\",\"AppArmorProfile\":\"containers-default\",\"ProcessLabel\":\"\",\"EffectiveCaps\":[],\"BoundingCaps\":[],\"Config\":{{\"User\":\"65532:65532\",\"Timeout\":20}},\"HostConfig\":{{\"ReadonlyRootfs\":true,\"Privileged\":false,\"SecurityOpt\":{security_options},\"UsernsMode\":\"auto\",\"PidMode\":\"private\",\"IpcMode\":\"none\",\"NetworkMode\":\"none\",\"UTSMode\":\"private\",\"CgroupMode\":\"private\",\"Memory\":268435456,\"NanoCpus\":1000000000,\"PidsLimit\":16,\"Tmpfs\":{{\"/tmp\":\"rw,noexec,nosuid,nodev,size=16777216\"}}}},\"Mounts\":[]}}]"
    );
    let info = "{\"host\":{\"security\":{\"rootless\":true,\"seccompEnabled\":true,\"seccompProfilePath\":\"/usr/share/containers/seccomp.json\",\"apparmorEnabled\":true,\"selinuxEnabled\":false}},\"version\":{\"Version\":\"6.1.0\"}}";
    let top = "PID SECCOMP CAPEFF CAPBND CAPINH CAPPRM CAPAMB LABEL\n1 strict - - - - - containers-default (enforce)";

    fs::write(
        &scenario,
        format!(
            r#"
COMMAND_INFO='{info}'
COMMAND_INSPECT='{inspect}'
COMMAND_TOP='{top}'
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
        ),
    )
    .expect("fake Podman scenario data must be writable");

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

#[test]
fn explicit_no_new_privileges_true_and_strict_seccomp_are_accepted() {
    let (_directory, program, calls_path) = fake_podman(false);
    let adapter = RootlessPodmanAdapter::new(program);

    let result = adapter.run_legacy_command_at_for_test(&request(), &policy(), 1_780_000_300);
    let calls = fs::read_to_string(calls_path).expect("fake Podman calls must be recorded");

    assert!(result.is_ok(), "supported security variants must be accepted: {result:?}");
    assert!(
        calls
            .lines()
            .any(|line| line == format!("rm --force --ignore {OWNED_CONTAINER_ID}")),
        "successful execution must clean up only the acquired container ID: {calls}"
    );
}

#[test]
fn seccomp_unconfined_option_overrides_strict_process_evidence() {
    let (_directory, program, calls_path) = fake_podman(true);
    let adapter = RootlessPodmanAdapter::new(program);

    let result = adapter.run_legacy_command_at_for_test(&request(), &policy(), 1_780_000_300);
    let calls = fs::read_to_string(calls_path).expect("fake Podman calls must be recorded");

    assert_eq!(
        result,
        Err(CommandExecutionError::Backend(
            ApplicationServiceError::IsolationVerificationFailed {
                control_name: "seccomp",
            },
        )),
        "explicit unconfined seccomp must fail closed even when live process evidence says strict"
    );
    assert!(
        calls
            .lines()
            .any(|line| line == format!("rm --force --ignore {OWNED_CONTAINER_ID}")),
        "failed live attestation must clean up only the acquired container ID: {calls}"
    );
    assert!(
        !calls.lines().any(|line| line.starts_with("wait ")),
        "failed live attestation must not proceed to workload completion evidence: {calls}"
    );
}
