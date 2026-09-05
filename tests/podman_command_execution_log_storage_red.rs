//! RED coverage for host-disk bounds on command-container logs.

#![cfg(target_os = "linux")]

use std::{fs, os::unix::fs::PermissionsExt};

use quarantine_sandbox_runtime::{
    CommandExecutionRequest, IsolationPolicy, ResourceRequest, RootlessPodmanAdapter,
};

fn policy() -> IsolationPolicy {
    IsolationPolicy {
        policy_id: "cmdexec_log_storage_policy_v1".to_owned(),
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
        request_id: "cmdexec-log-storage-red".to_owned(),
        image_reference: format!("localhost/cwl/tool@sha256:{}", "e".repeat(64)),
        command: vec!["printf".to_owned(), "bounded".to_owned()],
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
fn command_container_requires_one_positive_finite_runtime_log_file_limit() {
    let fixture = tempfile::tempdir().expect("isolated fake Podman directory");
    let program = fixture.path().join("podman");
    let script = r#"#!/bin/sh
set -eu
case "${1:-}:${2:-}" in
  info:--format)
    printf '%s\n' '{"host":{"security":{"rootless":true,"seccompEnabled":true,"seccompProfilePath":"/usr/share/containers/seccomp.json","apparmorEnabled":true,"selinuxEnabled":false}},"version":{"Version":"6.1.0"}}'
    ;;
  create:--name)
    previous=''
    bounded_log=''
    bounded_log_count=0
    for argument in "$@"; do
      if [ "$previous" = '--log-opt' ]; then
        case "$argument" in
          max-size=*)
            bounded_log=${argument#max-size=}
            bounded_log_count=$((bounded_log_count + 1))
            ;;
        esac
      fi
      previous="$argument"
    done

    # Keep the acceptance token canonical so malformed strings cannot
    # false-GREEN this RED. Podman documents max-size with integer values and
    # optional b/k/m/g-style units (for example, 10mb).
    [ "$bounded_log_count" -eq 1 ] || exit 86
    printf '%s\n' "$bounded_log" | grep -Eq '^[1-9][0-9]*([kKmMgG]([bB])?)?$' || exit 86

    printf 'fake-command-container-id\n'
    ;;
  start:*) : ;;
  container:inspect)
    printf '%s\n' '[{"Id":"fake-command-container-id","AppArmorProfile":"containers-default","ProcessLabel":"","EffectiveCaps":null,"BoundingCaps":null,"Config":{"User":"65532:65532"},"HostConfig":{"ReadonlyRootfs":true,"Privileged":false,"SecurityOpt":["no-new-privileges"],"UsernsMode":"","Annotations":{"io.podman.annotations.userns":"auto"},"PidMode":"private","IpcMode":"none","NetworkMode":"none","Memory":268435456,"NanoCpus":1000000000,"PidsLimit":16}}]'
    ;;
  top:*)
    printf '%s\n' 'PID SECCOMP CAPEFF CAPBND CAPINH CAPPRM CAPAMB LABEL' '1 filter - - - - - containers-default (enforce)'
    ;;
  wait:*) printf '0\n' ;;
  logs:*) printf 'bounded\n' ;;
  rm:--force) : ;;
  *) exit 91 ;;
esac
"#;
    fs::write(&program, script).expect("fake Podman should be writable");
    let mut permissions = fs::metadata(&program)
        .expect("fake Podman metadata should exist")
        .permissions();
    permissions.set_mode(0o700);
    fs::set_permissions(&program, permissions).expect("fake Podman should be executable");

    let adapter = RootlessPodmanAdapter::new(&program);
    let result = adapter.run_command_at(&request(), &policy(), 1_780_000_000);

    result.expect(
        "k8s-file command execution must set exactly one positive finite runtime-owned host log-storage bound before launch",
    );
}
