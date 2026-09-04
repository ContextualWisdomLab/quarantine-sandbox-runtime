//! Regression RED for the one-shot command path's Podman 4.9 create contract.
//!
//! The dependency-root application-service path already removed `--no-hostname`
//! after real Ubuntu 24.04 / Podman 4.9.3 execution proved that flag unsupported.
//! The command-execution descendant must not reintroduce the same incompatible
//! argv while claiming the same supported rootless-Podman profile.

#![cfg(target_os = "linux")]

use std::{
    fs,
    os::unix::fs::PermissionsExt,
    path::PathBuf,
    sync::atomic::{AtomicU64, Ordering},
    time::{SystemTime, UNIX_EPOCH},
};

use quarantine_sandbox_runtime::{
    CommandExecutionRequest, IsolationPolicy, ResourceRequest, RootlessPodmanAdapter,
};

static NEXT_PATH_ID: AtomicU64 = AtomicU64::new(0);

fn temporary_path(name: &str) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock should be after the Unix epoch")
        .as_nanos();
    let unique_id = NEXT_PATH_ID.fetch_add(1, Ordering::Relaxed);
    std::env::temp_dir().join(format!(
        "quarantine-sandbox-runtime-podman49-{name}-{}-{nanos}-{unique_id}",
        std::process::id()
    ))
}

fn write_executable(name: &str, script: &str) -> PathBuf {
    let program = temporary_path(name);
    fs::write(&program, script).expect("fake Podman should be writable");
    let mut permissions = fs::metadata(&program)
        .expect("fake Podman metadata should exist")
        .permissions();
    permissions.set_mode(0o700);
    fs::set_permissions(&program, permissions).expect("fake Podman should be executable");
    program
}

fn policy() -> IsolationPolicy {
    IsolationPolicy {
        policy_id: "podman_4_9_compat_policy_v1".to_owned(),
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
        request_id: "podman-4-9-command".to_owned(),
        image_reference: format!("localhost/cwl/tool@sha256:{}", "e".repeat(64)),
        command: vec!["true".to_owned()],
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
fn command_create_avoids_the_podman_4_9_unsupported_no_hostname_flag() {
    let script = r#"#!/bin/sh
set -eu
case "${1:-}:${2:-}" in
  info:--format)
    printf '%s\n' '{"host":{"security":{"rootless":true,"seccompEnabled":true,"seccompProfilePath":"/usr/share/containers/seccomp.json","apparmorEnabled":true,"selinuxEnabled":false}},"version":{"Version":"4.9.3"}}'
    ;;
  create:--name)
    for argument in "$@"; do
      if [ "$argument" = "--no-hostname" ]; then
        printf '%s\n' 'Error: unknown flag: --no-hostname' >&2
        exit 125
      fi
    done
    printf '%s\n' 'fake-command-container-id'
    ;;
  start:*)
    :
    ;;
  container:inspect)
    printf '%s\n' '[{"Id":"fake-command-container-id","AppArmorProfile":"containers-default","ProcessLabel":"","EffectiveCaps":[],"BoundingCaps":[],"Config":{"User":"65532:65532"},"HostConfig":{"ReadonlyRootfs":true,"Privileged":false,"SecurityOpt":["no-new-privileges"],"UsernsMode":"auto","Annotations":{},"PidMode":"private","IpcMode":"none","Memory":268435456,"NanoCpus":1000000000,"PidsLimit":16}}]'
    ;;
  top:*)
    printf '%s\n' 'PID SECCOMP CAPEFF CAPBND CAPINH CAPPRM CAPAMB LABEL'
    printf '%s\n' '1 filter - - - - - containers-default (enforce)'
    ;;
  wait:*)
    printf '%s\n' '0'
    ;;
  logs:*)
    :
    ;;
  rm:--force)
    :
    ;;
  *)
    exit 91
    ;;
esac
"#;
    let program = write_executable("create-argv", script);
    let adapter = RootlessPodmanAdapter::new(program.clone());

    let result = adapter.run_command_at(&request(), &policy(), 1_780_000_000);

    assert!(
        result.is_ok(),
        "the supported Podman 4.9 command path must not emit --no-hostname: {result:?}"
    );

    let _ = fs::remove_file(program);
}
