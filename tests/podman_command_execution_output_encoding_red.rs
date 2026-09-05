//! RED coverage for byte-faithful command-output evidence.
//!
//! `podman logs` is a byte stream. The command result's current 1.0.0 wire
//! contract exposes UTF-8 text, so invalid UTF-8 must be rejected explicitly;
//! silently replacing bytes with U+FFFD would make forensic evidence lossy.

#![cfg(target_os = "linux")]

use std::{fs, os::unix::fs::PermissionsExt, path::Path};

use quarantine_sandbox_runtime::{
    CommandExecutionRequest, IsolationPolicy, ResourceRequest, RootlessPodmanAdapter,
};

#[derive(Clone, Copy)]
enum InvalidStream {
    Stdout,
    Stderr,
}

fn policy() -> IsolationPolicy {
    IsolationPolicy {
        policy_id: "cmdexec_output_encoding_policy_v1".to_owned(),
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

fn request(request_id: &str) -> CommandExecutionRequest {
    CommandExecutionRequest {
        schema_version: "1.0.0".to_owned(),
        request_id: request_id.to_owned(),
        image_reference: format!("localhost/cwl/tool@sha256:{}", "9".repeat(64)),
        command: vec!["emit-evidence".to_owned()],
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

fn fake_podman(program: &Path, cleanup_marker: &Path, invalid_stream: InvalidStream) {
    let log_body = match invalid_stream {
        InvalidStream::Stdout => "printf '\\377stdout-invalid\\n'; printf 'stderr-valid\\n' >&2",
        InvalidStream::Stderr => "printf 'stdout-valid\\n'; printf '\\376stderr-invalid\\n' >&2",
    };
    let script = format!(
        r#"#!/bin/sh
set -eu
cleanup_marker='{cleanup_marker}'
case "${{1:-}}:${{2:-}}" in
  info:--format)
    printf '%s\n' '{{"host":{{"security":{{"rootless":true,"seccompEnabled":true,"seccompProfilePath":"/usr/share/containers/seccomp.json","apparmorEnabled":true,"selinuxEnabled":false}}}},"version":{{"Version":"6.1.0"}}}}'
    ;;
  create:--name)
    previous=''
    for argument in "$@"; do
      if [ "$previous" = '--cidfile' ]; then
        printf 'fake-command-container-id\n' > "$argument"
      fi
      case "$argument" in
        --cidfile=*) printf 'fake-command-container-id\n' > "${{argument#--cidfile=}}" ;;
      esac
      previous="$argument"
    done
    printf 'fake-command-container-id\n'
    ;;
  start:*) : ;;
  container:inspect)
    printf '%s\n' '[{{"Id":"fake-command-container-id","AppArmorProfile":"containers-default","ProcessLabel":"","EffectiveCaps":null,"BoundingCaps":null,"Config":{{"User":"65532:65532"}},"HostConfig":{{"ReadonlyRootfs":true,"Privileged":false,"SecurityOpt":["no-new-privileges"],"UsernsMode":"","Annotations":{{"io.podman.annotations.userns":"auto"}},"PidMode":"private","IpcMode":"none","NetworkMode":"none","Memory":268435456,"NanoCpus":1000000000,"PidsLimit":16}}}}]'
    ;;
  top:*)
    printf '%s\n' 'PID SECCOMP CAPEFF CAPBND CAPINH CAPPRM CAPAMB LABEL' '1 filter - - - - - containers-default (enforce)'
    ;;
  wait:*) printf '0\n' ;;
  logs:*) {log_body} ;;
  rm:--force) : > "$cleanup_marker" ;;
  *) exit 91 ;;
esac
"#,
        cleanup_marker = cleanup_marker.display(),
    );

    fs::write(program, script).expect("fake Podman should be writable");
    let mut permissions = fs::metadata(program)
        .expect("fake Podman metadata should exist")
        .permissions();
    permissions.set_mode(0o700);
    fs::set_permissions(program, permissions).expect("fake Podman should be executable");
}

fn assert_invalid_output_is_rejected(invalid_stream: InvalidStream, request_id: &str) {
    let fixture = tempfile::tempdir().expect("isolated fake Podman directory");
    let program = fixture.path().join("podman");
    let cleanup_marker = fixture.path().join("cleanup-complete");
    fake_podman(&program, &cleanup_marker, invalid_stream);

    let adapter = RootlessPodmanAdapter::new(&program);
    let result = adapter.run_command_at(&request(request_id), &policy(), 1_780_000_000);
    let cleanup_observed = cleanup_marker.exists();

    assert!(
        result.is_err(),
        "invalid UTF-8 workload output must not become a successful result through replacement-character decoding"
    );
    assert!(
        cleanup_observed,
        "output-encoding rejection must still clean up the exact command container"
    );
}

#[test]
fn invalid_utf8_stdout_is_rejected_without_lossy_evidence() {
    assert_invalid_output_is_rejected(InvalidStream::Stdout, "cmdexec-invalid-stdout-red");
}

#[test]
fn invalid_utf8_stderr_is_rejected_without_lossy_evidence() {
    assert_invalid_output_is_rejected(InvalidStream::Stderr, "cmdexec-invalid-stderr-red");
}
