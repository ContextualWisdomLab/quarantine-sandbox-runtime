//! Reject a source-artifact workspace mount that reports the staged source but is not a bind mount.
//!
//! Source-path equality alone is not effective filesystem authority evidence. The runtime must
//! verify that `/workspace` is a read-only bind of the exact staged source before starting the
//! consumer payload.

#![cfg(target_os = "linux")]

use std::{fs, os::unix::fs::PermissionsExt};

use quarantine_sandbox_runtime::{
    ApplicationServiceError, CommandExecutionError, CommandExecutionRequest, IsolationPolicy,
    PrSourceArtifactInput, ResourceRequest, RootlessPodmanAdapter,
};
use sha2::{Digest, Sha256};

const OWNED_CONTAINER_ID: &str = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";

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
        policy_id: "source_mount_type_coverage_v1".to_owned(),
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

#[test]
fn non_bind_workspace_mount_fails_closed_before_payload_start() {
    let fixture = tempfile::tempdir().expect("isolated source mount fixture should exist");
    let source = fixture.path().join("source");
    fs::create_dir(&source).expect("source directory should be creatable");
    let source_bytes = b"bounded source\n";
    fs::write(source.join("data.txt"), source_bytes).expect("source file should be writable");

    let request = CommandExecutionRequest {
        schema_version: "1.0.0".to_owned(),
        request_id: "source-mount-type-coverage".to_owned(),
        image_reference: format!("localhost/cwl/tool@sha256:{}", "b".repeat(64)),
        command: vec!["payload-sentinel".to_owned()],
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
    };

    let program = fixture.path().join("podman");
    let calls = fixture.path().join("calls");
    let volume_record = fixture.path().join("volume");
    let script = format!(
        r#"#!/bin/sh
set -eu
printf '%s\n' "$*" >> '{calls}'
case "${{1:-}}:${{2:-}}" in
  info:--format)
    printf '%s\n' '{{"host":{{"security":{{"rootless":true,"seccompEnabled":true,"seccompProfilePath":"/usr/share/containers/seccomp.json","apparmorEnabled":true,"selinuxEnabled":false}}}},"version":{{"Version":"6.1.0"}}}}'
    ;;
  create:--name)
    previous=''
    volume=''
    for argument in "$@"; do
      if [ "$previous" = '--cidfile' ]; then printf '%s\n' '{owned_id}' > "$argument"; fi
      case "$argument" in --cidfile=*) printf '%s\n' '{owned_id}' > "${{argument#--cidfile=}}" ;; esac
      if [ "$previous" = '--volume' ]; then volume="$argument"; fi
      previous="$argument"
    done
    [ -n "$volume" ] || exit 92
    printf '%s\n' "$volume" > '{volume_record}'
    printf '%s\n' '{owned_id}'
    ;;
  init:*) : ;;
  container:inspect)
    volume=$(cat '{volume_record}')
    source_path=${{volume%%:/workspace:*}}
    printf '[{{"Id":"{owned_id}","AppArmorProfile":"containers-default","ProcessLabel":"","EffectiveCaps":null,"BoundingCaps":null,"Config":{{"User":"65532:65532","Timeout":20}},"HostConfig":{{"ReadonlyRootfs":true,"Privileged":false,"SecurityOpt":["no-new-privileges"],"UsernsMode":"","Annotations":{{"io.podman.annotations.userns":"auto"}},"PidMode":"private","IpcMode":"none","NetworkMode":"none","UTSMode":"private","CgroupMode":"private","Memory":268435456,"NanoCpus":1000000000,"PidsLimit":16,"Tmpfs":{{"/tmp":"rw,noexec,nosuid,nodev,size=16777216"}}}},"Mounts":[{{"Source":"%s","Destination":"/workspace","Type":"volume","Options":["ro","noexec","nosuid","nodev"],"RW":false}}]}}]\n' "$source_path"
    ;;
  start:*) : ;;
  top:*) printf '%s\n' 'PID SECCOMP CAPEFF CAPBND CAPINH CAPPRM CAPAMB LABEL' '1 filter - - - - - containers-default (enforce)' ;;
  wait:*) printf '0\n' ;;
  logs:*) : ;;
  rm:--force) : ;;
  *) exit 91 ;;
esac
"#,
        calls = calls.display(),
        volume_record = volume_record.display(),
        owned_id = OWNED_CONTAINER_ID,
    );
    fs::write(&program, script).expect("fake Podman should be writable");
    let mut permissions = fs::metadata(&program)
        .expect("fake Podman metadata should exist")
        .permissions();
    permissions.set_mode(0o700);
    fs::set_permissions(&program, permissions).expect("fake Podman should be executable");

    let adapter = RootlessPodmanAdapter::new(&program);
    let result = adapter.run_legacy_command_at_for_test(&request, &policy(), 1_780_000_203);

    assert_eq!(
        result,
        Err(CommandExecutionError::Backend(
            ApplicationServiceError::IsolationVerificationFailed {
                control_name: "source_artifact_bind_source",
            },
        )),
        "an exact staged source path must not compensate for a non-bind workspace mount"
    );

    let recorded = fs::read_to_string(&calls).expect("backend calls should be recorded");
    assert!(
        recorded
            .lines()
            .any(|line| *line == format!("rm --force --ignore {OWNED_CONTAINER_ID}")),
        "failed workspace mount attestation must clean up the exact acquired container: {recorded}"
    );
    assert!(
        !recorded.lines().any(|line| line.starts_with("start ")),
        "a rejected workspace mount must not start the consumer payload: {recorded}"
    );
    assert!(
        !recorded.lines().any(|line| line.starts_with("wait ")),
        "a rejected workspace mount must not reach workload completion evidence: {recorded}"
    );
}
