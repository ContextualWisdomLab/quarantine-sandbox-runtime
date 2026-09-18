//! Cover independently malformed command tmpfs evidence through the public Podman adapter.
//!
//! Each case preserves otherwise-valid resource evidence so the failing control is the
//! exact `/tmp` shape rather than an earlier resource-limit predicate.

#![cfg(target_os = "linux")]

use std::{fs, os::unix::fs::symlink, path::PathBuf};

use quarantine_sandbox_runtime::{
    ApplicationServiceError, CommandExecutionError, CommandExecutionRequest, IsolationPolicy,
    ResourceRequest, RootlessPodmanAdapter,
};
use tempfile::TempDir;

const OWNED_CONTAINER_ID: &str = "3333333333333333333333333333333333333333333333333333333333333333";

fn policy() -> IsolationPolicy {
    IsolationPolicy {
        policy_id: "command_resource_option_coverage_v1".to_owned(),
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
        request_id: "command-resource-option-coverage".to_owned(),
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

fn fake_podman(tmpfs_json: &str) -> (TempDir, PathBuf, PathBuf) {
    let directory = tempfile::Builder::new()
        .prefix("qsr-command-resource-option-coverage-")
        .tempdir()
        .expect("isolated fake-Podman directory");
    let program = directory.path().join("podman");
    let calls = directory.path().join("calls");
    let scenario = directory.path().join("scenario.sh");
    let config = PathBuf::from(format!("{}.config", program.display()));
    let dispatcher =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/fake_podman.sh");
    let inspect = format!(
        "[{{\"Id\":\"{OWNED_CONTAINER_ID}\",\"AppArmorProfile\":\"containers-default\",\"ProcessLabel\":\"\",\"EffectiveCaps\":[],\"BoundingCaps\":[],\"Config\":{{\"User\":\"65532:65532\",\"Timeout\":20}},\"HostConfig\":{{\"ReadonlyRootfs\":true,\"Privileged\":false,\"SecurityOpt\":[\"no-new-privileges\"],\"UsernsMode\":\"auto\",\"PidMode\":\"private\",\"IpcMode\":\"none\",\"NetworkMode\":\"none\",\"UTSMode\":\"private\",\"CgroupMode\":\"private\",\"Memory\":268435456,\"NanoCpus\":1000000000,\"PidsLimit\":16,\"Tmpfs\":{tmpfs_json}}},\"Mounts\":[]}}]"
    );
    let info = "{\"host\":{\"security\":{\"rootless\":true,\"seccompEnabled\":true,\"seccompProfilePath\":\"/usr/share/containers/seccomp.json\",\"apparmorEnabled\":true,\"selinuxEnabled\":false}},\"version\":{\"Version\":\"6.1.0\"}}";
    let top = "PID SECCOMP CAPEFF CAPBND CAPINH CAPPRM CAPAMB LABEL\\n1 filter - - - - - containers-default (enforce)";
    fs::write(
        &scenario,
        format!(
            r#"
case "${{1:-}}:${{2:-}}" in
  info:--format) printf '%s\n' '{info}' ;;
  create:--name) printf '%s\n' '{OWNED_CONTAINER_ID}' ;;
  init:*) : ;;
  container:inspect) printf '%s\n' '{inspect}' ;;
  start:*) : ;;
  top:*) printf '%b\n' '{top}' ;;
  wait:*) printf '0\n' ;;
  logs:*) : ;;
  rm:--force) : ;;
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

#[test]
fn independently_malformed_tmpfs_evidence_fails_closed() {
    for (name, tmpfs_json) in [
        (
            "wrong-only-destination",
            r#"{"/cache":"rw,noexec,nosuid,nodev,size=16777216"}"#,
        ),
        (
            "empty-option",
            r#"{"/tmp":"rw,,noexec,nosuid,nodev,size=16777216"}"#,
        ),
        (
            "missing-read-write",
            r#"{"/tmp":"ro,noexec,nosuid,nodev,size=16777216"}"#,
        ),
        (
            "missing-nosuid",
            r#"{"/tmp":"rw,noexec,suid,nodev,size=16777216"}"#,
        ),
        (
            "missing-nodev",
            r#"{"/tmp":"rw,noexec,nosuid,dev,size=16777216"}"#,
        ),
        (
            "wrong-size-only",
            r#"{"/tmp":"rw,noexec,nosuid,nodev,size=16777217"}"#,
        ),
    ] {
        let (_directory, program, calls_path) = fake_podman(tmpfs_json);
        let adapter = RootlessPodmanAdapter::new(program);
        let result = adapter.run_legacy_command_at_for_test(&request(), &policy(), 1_780_000_400);
        let calls = fs::read_to_string(calls_path).expect("backend calls should be recorded");

        assert_eq!(
            result,
            Err(CommandExecutionError::Backend(
                ApplicationServiceError::IsolationVerificationFailed {
                    control_name: "resource_limits",
                },
            )),
            "{name} must not be admitted as exact hardened /tmp evidence"
        );
        assert!(
            calls
                .lines()
                .any(|line| line == format!("rm --force --ignore {OWNED_CONTAINER_ID}")),
            "{name} must clean up only the exact acquired container: {calls}"
        );
        assert!(
            !calls.lines().any(|line| line.starts_with("start ")),
            "{name} must fail before the consumer starts: {calls}"
        );
    }
}
