//! Reject non-UTF-8 live process-security evidence through the public command adapter.

#![cfg(target_os = "linux")]

use std::{fs, os::unix::fs::symlink, path::PathBuf};

use quarantine_sandbox_runtime::{
    ApplicationServiceError, CommandExecutionError, CommandExecutionRequest, IsolationPolicy,
    ResourceRequest, RootlessPodmanAdapter,
};

const OWNED_CONTAINER_ID: &str = "3333333333333333333333333333333333333333333333333333333333333333";

fn policy() -> IsolationPolicy {
    IsolationPolicy {
        policy_id: "process_security_encoding_v1".to_owned(),
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
        request_id: "process-security-encoding".to_owned(),
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

#[test]
fn non_utf8_live_process_security_evidence_fails_closed_before_workload_wait() {
    let directory = tempfile::Builder::new()
        .prefix("qsr-process-security-encoding-")
        .tempdir()
        .expect("isolated fake-Podman directory");
    let program = directory.path().join("podman");
    let calls = directory.path().join("calls");
    let scenario = directory.path().join("scenario.sh");
    let fixture = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/fake_podman.sh");
    symlink(fixture, &program).expect("fake Podman symlink must be creatable");

    let info = "{\"host\":{\"security\":{\"rootless\":true,\"seccompEnabled\":true,\"seccompProfilePath\":\"/usr/share/containers/seccomp.json\",\"apparmorEnabled\":true,\"selinuxEnabled\":false}},\"version\":{\"Version\":\"6.1.0\"}}";
    let inspect = format!(
        "[{{\"Id\":\"{OWNED_CONTAINER_ID}\",\"AppArmorProfile\":\"containers-default\",\"ProcessLabel\":\"\",\"EffectiveCaps\":[],\"BoundingCaps\":[],\"Config\":{{\"User\":\"65532:65532\",\"Timeout\":20}},\"HostConfig\":{{\"ReadonlyRootfs\":true,\"Privileged\":false,\"SecurityOpt\":[\"no-new-privileges=true\"],\"UsernsMode\":\"auto\",\"PidMode\":\"private\",\"IpcMode\":\"none\",\"NetworkMode\":\"none\",\"UTSMode\":\"private\",\"CgroupMode\":\"private\",\"Memory\":268435456,\"NanoCpus\":1000000000,\"PidsLimit\":16,\"Tmpfs\":{{\"/tmp\":\"rw,noexec,nosuid,nodev,size=16777216\"}}}},\"Mounts\":[]}}]"
    );
    fs::write(
        &scenario,
        format!(
            r#"
case "${{1:-}}:${{2:-}}" in
  info:--format) printf '%s\n' '{info}' ;;
  create:--name) printf '%s\n' '{OWNED_CONTAINER_ID}' ;;
  init:*) [ "${{2:-}}" = '{OWNED_CONTAINER_ID}' ] || exit 97 ;;
  start:*) [ "${{2:-}}" = '{OWNED_CONTAINER_ID}' ] || exit 97 ;;
  container:inspect) [ "${{5:-}}" = '{OWNED_CONTAINER_ID}' ] || exit 97; printf '%s\n' '{inspect}' ;;
  top:*) [ "${{2:-}}" = '{OWNED_CONTAINER_ID}' ] || exit 97; printf '\377' ;;
  rm:--force) [ "${{4:-}}" = '{OWNED_CONTAINER_ID}' ] || exit 97 ;;
  *) exit 91 ;;
esac
"#
        ),
    )
    .expect("fake Podman scenario data must be writable");
    fs::write(
        PathBuf::from(format!("{}.config", program.display())),
        format!(
            "MODE='source_script'\nLOG='{}'\nSCRIPT='{}'\n",
            calls.display(),
            scenario.display(),
        ),
    )
    .expect("fake Podman config must be writable");

    let result = RootlessPodmanAdapter::new(program).run_legacy_command_at_for_test(
        &request(),
        &policy(),
        1_780_000_400,
    );
    let calls = fs::read_to_string(calls).expect("fake Podman calls must be recorded");

    assert_eq!(
        result,
        Err(CommandExecutionError::Backend(
            ApplicationServiceError::MalformedIsolationInspection {
                operation: "process_security_top",
            },
        )),
        "non-UTF-8 live process evidence must fail closed at the parser boundary"
    );
    assert!(
        calls
            .lines()
            .any(|line| line == format!("rm --force --ignore {OWNED_CONTAINER_ID}")),
        "parse failure must clean up only the acquired exact container ID: {calls}"
    );
    assert!(
        !calls.lines().any(|line| line.starts_with("wait ")),
        "malformed live security evidence must not reach workload completion evidence: {calls}"
    );
}
