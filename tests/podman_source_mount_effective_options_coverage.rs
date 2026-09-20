//! Source-artifact attestation must reject writable or weak effective workspace binds.
//!
//! These hostile fake-backend cases preserve an exact bind of the runtime-staged source while
//! varying only applied write authority or required mount options. They exercise command-owner
//! static attestation and are not real filesystem-isolation evidence.

#![cfg(target_os = "linux")]

use std::{fs, os::unix::fs::symlink, path::PathBuf};

use quarantine_sandbox_runtime::{
    ApplicationServiceError, CommandExecutionError, CommandExecutionRequest, IsolationPolicy,
    PrSourceArtifactInput, ResourceRequest, RootlessPodmanAdapter,
};
use sha2::{Digest, Sha256};
use tempfile::TempDir;

const OWNED_CONTAINER_ID: &str = "4444444444444444444444444444444444444444444444444444444444444444";

fn tree_digest(path: &str, bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update((path.len() as u64).to_be_bytes());
    hasher.update(path.as_bytes());
    hasher.update((bytes.len() as u64).to_be_bytes());
    hasher.update(bytes);
    format!("{:x}", hasher.finalize())
}

fn policy() -> IsolationPolicy {
    IsolationPolicy {
        policy_id: "source_mount_effective_options_coverage_v1".to_owned(),
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

fn request(source: PathBuf, source_bytes: &[u8]) -> CommandExecutionRequest {
    CommandExecutionRequest {
        schema_version: "1.0.0".to_owned(),
        request_id: "source-mount-effective-options-coverage".to_owned(),
        image_reference: format!("localhost/cwl/tool@sha256:{}", "c".repeat(64)),
        command: vec!["true".to_owned()],
        source_artifact: Some(PrSourceArtifactInput {
            host_path: source,
            revision_sha: "a".repeat(40),
            expected_tree_sha256: tree_digest("data.txt", source_bytes),
        }),
        resources: ResourceRequest {
            memory_bytes: 256 * 1024 * 1024,
            cpu_millicores: 1_000,
            maximum_processes: 16,
            lease_seconds: 20,
            tmpfs_bytes: 16 * 1024 * 1024,
        },
    }
}

fn fake_podman(read_write: bool, options_json: &str) -> (TempDir, PathBuf, PathBuf) {
    let directory = tempfile::Builder::new()
        .prefix("qsr-source-mount-options-")
        .tempdir()
        .expect("isolated fake-Podman directory");
    let program = directory.path().join("podman");
    let calls = directory.path().join("calls");
    let volume_record = directory.path().join("volume");
    let scenario = directory.path().join("scenario.sh");
    let config = PathBuf::from(format!("{}.config", program.display()));
    let dispatcher =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/fake_podman.sh");
    let read_write_json = if read_write { "true" } else { "false" };

    fs::write(&calls, "").expect("fake Podman call log should be initialized");
    let script = format!(
        r#"
case "${{1:-}}:${{2:-}}" in
  info:--format)
    printf '%s\n' '{{"host":{{"security":{{"rootless":true,"seccompEnabled":true,"seccompProfilePath":"/usr/share/containers/seccomp.json","apparmorEnabled":true,"selinuxEnabled":false}}}},"version":{{"Version":"6.1.0"}}}}'
    ;;
  create:--name)
    previous=''
    volume=''
    for argument in "$@"; do
      if [ "$previous" = '--cidfile' ]; then printf '%s\n' "$OWNED_CONTAINER_ID" > "$argument"; fi
      case "$argument" in --cidfile=*) printf '%s\n' "$OWNED_CONTAINER_ID" > "${{argument#--cidfile=}}" ;; esac
      if [ "$previous" = '--volume' ]; then volume="$argument"; fi
      previous="$argument"
    done
    [ -n "$volume" ] || exit 92
    printf '%s\n' "$volume" > '{volume_record}'
    printf '%s\n' "$OWNED_CONTAINER_ID"
    ;;
  init:*) [ "${{2:-}}" = "$OWNED_CONTAINER_ID" ] || exit 97 ;;
  container:inspect)
    [ "${{5:-}}" = "$OWNED_CONTAINER_ID" ] || exit 97
    volume=$(cat '{volume_record}')
    source_path=${{volume%%:/workspace:*}}
    printf '[{{"Id":"%s","AppArmorProfile":"containers-default","ProcessLabel":"","EffectiveCaps":[],"BoundingCaps":[],"Config":{{"User":"65532:65532","Timeout":20}},"HostConfig":{{"ReadonlyRootfs":true,"Privileged":false,"SecurityOpt":["no-new-privileges"],"UsernsMode":"auto","PidMode":"private","IpcMode":"none","NetworkMode":"none","UTSMode":"private","CgroupMode":"private","Memory":268435456,"NanoCpus":1000000000,"PidsLimit":16,"Tmpfs":{{"/tmp":"rw,noexec,nosuid,nodev,size=16777216"}}}},"Mounts":[{{"Source":"%s","Destination":"/workspace","Type":"bind","Options":{options_json},"RW":{read_write_json}}}]}}]\n' "$OWNED_CONTAINER_ID" "$source_path"
    ;;
  rm:--force) [ "${{4:-}}" = "$OWNED_CONTAINER_ID" ] || exit 97 ;;
  *) exit 91 ;;
esac
"#,
        volume_record = volume_record.display(),
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

fn execute_case(read_write: bool, options_json: &str, expected_control: &'static str) {
    let source_directory = tempfile::tempdir().expect("source fixture directory should exist");
    let source = source_directory.path().join("source");
    fs::create_dir(&source).expect("source directory should be creatable");
    let source_bytes = b"bounded source\n";
    fs::write(source.join("data.txt"), source_bytes).expect("source file should be writable");

    let (_directory, program, calls_path) = fake_podman(read_write, options_json);
    let adapter = RootlessPodmanAdapter::new(program);
    let result = adapter.run_legacy_command_at_for_test(
        &request(source, source_bytes),
        &policy(),
        1_780_000_305,
    );
    let calls = fs::read_to_string(calls_path).expect("fake Podman calls must be recorded");

    assert_eq!(
        result,
        Err(CommandExecutionError::Backend(
            ApplicationServiceError::IsolationVerificationFailed {
                control_name: expected_control,
            },
        )),
    );
    assert!(
        calls
            .lines()
            .any(|line| line == format!("rm --force --ignore {OWNED_CONTAINER_ID}")),
        "source mount rejection must clean only the acquired container ID: {calls}"
    );
    assert!(
        !calls.lines().any(|line| line.starts_with("start ")),
        "source mount rejection must fail before consumer start: {calls}"
    );
    assert!(
        !calls
            .lines()
            .any(|line| line.starts_with("top ") || line.starts_with("wait ")),
        "rejected source mount must not reach live-process or workload evidence: {calls}"
    );
}

#[test]
fn writable_source_bind_fails_closed() {
    execute_case(
        true,
        r#"["rw","noexec","nosuid","nodev"]"#,
        "source_artifact_read_only",
    );
}

#[test]
fn source_bind_missing_noexec_fails_closed() {
    execute_case(
        false,
        r#"["ro","nosuid","nodev"]"#,
        "source_artifact_mount_options",
    );
}
