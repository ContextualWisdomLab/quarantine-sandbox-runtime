//! Command prestart verification must reject a backend that reports root identity.
//!
//! This hostile fake-backend witness exercises the production command adapter up to the static
//! prestart boundary. It is not real isolation evidence; it proves only that contradictory applied
//! identity is rejected before consumer start and that cleanup remains scoped to the acquired ID.

#![cfg(target_os = "linux")]

use std::{fs, os::unix::fs::symlink, path::PathBuf};

use quarantine_sandbox_runtime::{
    ApplicationServiceError, CommandExecutionError, CommandExecutionRequest, IsolationPolicy,
    ResourceRequest, RootlessPodmanAdapter,
};
use tempfile::TempDir;

const OWNED_CONTAINER_ID: &str = "2222222222222222222222222222222222222222222222222222222222222222";

fn policy() -> IsolationPolicy {
    IsolationPolicy {
        policy_id: "command_non_root_identity_coverage_v1".to_owned(),
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
        request_id: "command-non-root-identity-coverage".to_owned(),
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

fn fake_podman() -> (TempDir, PathBuf, PathBuf) {
    let directory = tempfile::Builder::new()
        .prefix("qsr-command-non-root-identity-")
        .tempdir()
        .expect("isolated fake-Podman directory");
    let program = directory.path().join("podman");
    let calls = directory.path().join("calls");
    let scenario = directory.path().join("scenario.sh");
    let config = PathBuf::from(format!("{}.config", program.display()));
    let dispatcher =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/fake_podman.sh");

    fs::write(&calls, "").expect("fake Podman call log should be initialized");
    let info = r#"{"host":{"security":{"rootless":true,"seccompEnabled":true,"seccompProfilePath":"/usr/share/containers/seccomp.json","apparmorEnabled":true,"selinuxEnabled":false}},"version":{"Version":"6.1.0"}}"#;
    let inspect = format!(
        r#"[{{"Id":"{OWNED_CONTAINER_ID}","AppArmorProfile":"containers-default","ProcessLabel":"","EffectiveCaps":[],"BoundingCaps":[],"Config":{{"User":"0:0","Timeout":20}},"HostConfig":{{"ReadonlyRootfs":true,"Privileged":false,"SecurityOpt":["no-new-privileges"],"UsernsMode":"auto","PidMode":"private","IpcMode":"none","NetworkMode":"none","UTSMode":"private","CgroupMode":"private","Memory":268435456,"NanoCpus":1000000000,"PidsLimit":16,"Tmpfs":{{"/tmp":"rw,noexec,nosuid,nodev,size=16777216"}}}},"Mounts":[]}}]"#
    );
    let script = format!(
        r#"
COMMAND_INFO='{info}'
COMMAND_INSPECT='{inspect}'
case "${{1:-}}:${{2:-}}" in
  info:--format) printf '%s\n' "$COMMAND_INFO" ;;
  create:--name) printf '%s\n' "$OWNED_CONTAINER_ID" ;;
  init:*) [ "${{2:-}}" = "$OWNED_CONTAINER_ID" ] || exit 97 ;;
  container:inspect) [ "${{5:-}}" = "$OWNED_CONTAINER_ID" ] || exit 97; printf '%s\n' "$COMMAND_INSPECT" ;;
  rm:--force) [ "${{4:-}}" = "$OWNED_CONTAINER_ID" ] || exit 97 ;;
  *) exit 91 ;;
esac
"#
    );
    fs::write(&scenario, script).expect("fake Podman scenario should be writable");
    fs::write(
        &config,
        format!(
            "MODE='source_script'\nLOG='{}'\nSCRIPT='{}'\nOWNED_CONTAINER_ID='{OWNED_CONTAINER_ID}'\n",
            calls.display(),
            scenario.display(),
        ),
    )
    .expect("fake Podman dispatcher config should be writable");
    symlink(&dispatcher, &program).expect("immutable fake Podman dispatcher should be linkable");

    (directory, program, calls)
}

#[test]
fn root_identity_fails_closed_before_start_and_cleans_exact_container() {
    let (_directory, program, calls_path) = fake_podman();
    let adapter = RootlessPodmanAdapter::new(program);

    let result = adapter.run_legacy_command_at_for_test(&request(), &policy(), 1_780_000_303);
    let calls = fs::read_to_string(calls_path).expect("fake Podman calls must be recorded");

    assert_eq!(
        result,
        Err(CommandExecutionError::Backend(
            ApplicationServiceError::IsolationVerificationFailed {
                control_name: "non_root_identity",
            },
        )),
    );
    assert!(
        calls
            .lines()
            .any(|line| line == format!("rm --force --ignore {OWNED_CONTAINER_ID}")),
        "identity contradiction must clean only the acquired container ID: {calls}"
    );
    assert!(
        !calls.lines().any(|line| line.starts_with("start ")),
        "identity contradiction must fail before consumer start: {calls}"
    );
    assert!(
        !calls
            .lines()
            .any(|line| line.starts_with("top ") || line.starts_with("wait ")),
        "prestart failure must not proceed to live process or workload evidence: {calls}"
    );
}
