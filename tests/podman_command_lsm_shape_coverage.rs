//! Cover fail-closed AppArmor evidence shapes through the public command adapter.
//!
//! Host LSM availability is admitted before any container is created. These fixtures therefore
//! keep AppArmor enabled and vary only effective per-container evidence while preserving otherwise
//! positive rootless, seccomp, capability, resource, and namespace evidence.

#![cfg(target_os = "linux")]

use std::{fs, os::unix::fs::symlink, path::PathBuf};

use quarantine_sandbox_runtime::{
    ApplicationServiceError, CommandExecutionError, CommandExecutionRequest, IsolationPolicy,
    ResourceRequest, RootlessPodmanAdapter,
};
use tempfile::TempDir;

const OWNED_CONTAINER_ID: &str = "4444444444444444444444444444444444444444444444444444444444444444";

fn policy() -> IsolationPolicy {
    IsolationPolicy {
        policy_id: "command_lsm_shape_coverage_v1".to_owned(),
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
        request_id: "command-lsm-shape-coverage".to_owned(),
        image_reference: format!("localhost/cwl/tool@sha256:{}", "f".repeat(64)),
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

fn fake_podman(apparmor_profile: &str, process_label: &str) -> (TempDir, PathBuf, PathBuf) {
    let directory = tempfile::Builder::new()
        .prefix("qsr-command-lsm-shape-coverage-")
        .tempdir()
        .expect("isolated fake-Podman directory");
    let program = directory.path().join("podman");
    let calls = directory.path().join("calls");
    let scenario = directory.path().join("scenario.sh");
    let config = PathBuf::from(format!("{}.config", program.display()));
    let dispatcher =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/fake_podman.sh");
    let inspect = format!(
        "[{{\"Id\":\"{OWNED_CONTAINER_ID}\",\"AppArmorProfile\":\"{apparmor_profile}\",\"ProcessLabel\":\"\",\"EffectiveCaps\":[],\"BoundingCaps\":[],\"Config\":{{\"User\":\"65532:65532\",\"Timeout\":20}},\"HostConfig\":{{\"ReadonlyRootfs\":true,\"Privileged\":false,\"SecurityOpt\":[\"no-new-privileges=true\"],\"UsernsMode\":\"auto\",\"PidMode\":\"private\",\"IpcMode\":\"none\",\"NetworkMode\":\"none\",\"UTSMode\":\"private\",\"CgroupMode\":\"private\",\"Memory\":268435456,\"NanoCpus\":1000000000,\"PidsLimit\":16,\"Tmpfs\":{{\"/tmp\":\"rw,noexec,nosuid,nodev,size=16777216\"}}}},\"Mounts\":[]}}]"
    );
    let info = "{\"host\":{\"security\":{\"rootless\":true,\"seccompEnabled\":true,\"seccompProfilePath\":\"/usr/share/containers/seccomp.json\",\"apparmorEnabled\":true,\"selinuxEnabled\":false}},\"version\":{\"Version\":\"6.1.0\"}}";
    let top = format!(
        "PID SECCOMP CAPEFF CAPBND CAPINH CAPPRM CAPAMB LABEL\\n1 strict - - - - - {process_label}"
    );

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
  top:*) [ "${{2:-}}" = "$OWNED_CONTAINER_ID" ] || exit 97; printf '%b\n' "$COMMAND_TOP" ;;
  wait:*) [ "${{2:-}}" = "$OWNED_CONTAINER_ID" ] || exit 97; printf '0\n' ;;
  logs:*) [ "${{2:-}}" = "$OWNED_CONTAINER_ID" ] || exit 97 ;;
  rm:--force) [ "${{4:-}}" = "$OWNED_CONTAINER_ID" ] || exit 97 ;;
  *) exit 91 ;;
esac
"#
        ),
    )
    .expect("fake Podman scenario should be writable");
    fs::write(
        &config,
        format!(
            "MODE='source_script'\nLOG='{}'\nSCRIPT='{}'\nOWNED_CONTAINER_ID='{OWNED_CONTAINER_ID}'\n",
            calls.display(),
            scenario.display(),
        ),
    )
    .expect("fake Podman dispatcher config should be writable");
    symlink(dispatcher, &program).expect("immutable fake Podman dispatcher should be linkable");
    (directory, program, calls)
}

fn assert_lsm_rejection(apparmor_profile: &str, process_label: &str) {
    let (_directory, program, calls_path) = fake_podman(apparmor_profile, process_label);
    let adapter = RootlessPodmanAdapter::new(program);
    let result = adapter.run_legacy_command_at_for_test(&request(), &policy(), 1_780_000_500);
    let calls = fs::read_to_string(calls_path).expect("fake Podman calls should be recorded");

    assert_eq!(
        result,
        Err(CommandExecutionError::Backend(
            ApplicationServiceError::IsolationVerificationFailed {
                control_name: "lsm",
            },
        )),
    );
    assert!(
        calls
            .lines()
            .any(|line| line == format!("rm --force --ignore {OWNED_CONTAINER_ID}")),
        "failed LSM attestation must clean up only the acquired container: {calls}"
    );
    assert!(
        !calls.lines().any(|line| line.starts_with("wait ")),
        "failed LSM attestation must not publish workload completion evidence: {calls}"
    );
}

#[test]
fn empty_apparmor_inspect_profile_fails_closed() {
    assert_lsm_rejection("", "containers-default (enforce)");
}
