//! Public-path coverage for hostile runtime-gate acknowledgement outcomes.
//!
//! The trusted gate release channel is a bounded control protocol. These regressions exercise
//! malformed and over-budget acknowledgement bytes through the complete Podman command adapter,
//! preserving exact-container cleanup authority instead of reaching private parser helpers
//! directly.

#![cfg(target_os = "linux")]

#[path = "support/runtime_gate_fixture.rs"]
mod runtime_gate_fixture;

use std::{fs, os::unix::fs::PermissionsExt};

use quarantine_sandbox_runtime::{
    ApplicationServiceError, CommandExecutionError, CommandExecutionRequest, IsolationPolicy,
    ResourceRequest, RootlessPodmanAdapter, RuntimeGateArtifact,
};
use runtime_gate_fixture::write_self_contained_gate;
use sha2::{Digest, Sha256};
use tempfile::tempdir;

const OWNED_CONTAINER_ID: &str = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";

fn policy() -> IsolationPolicy {
    IsolationPolicy {
        policy_id: "runtime_gate_ack_coverage_policy_v1".to_owned(),
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
        image_reference: format!("localhost/cwl/tool@sha256:{}", "a".repeat(64)),
        command: vec!["payload-sentinel".to_owned()],
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

fn write_fake_podman(
    directory: &std::path::Path,
    gate_path: &str,
    attach_response: &str,
) -> (std::path::PathBuf, std::path::PathBuf) {
    let program = directory.join("podman");
    let calls = directory.join("calls");
    let inspect = format!(
        "[{{\"Id\":\"{OWNED_CONTAINER_ID}\",\"AppArmorProfile\":\"containers-default\",\"ProcessLabel\":\"\",\"EffectiveCaps\":[],\"BoundingCaps\":[],\"Config\":{{\"User\":\"65532:65532\",\"Timeout\":20}},\"HostConfig\":{{\"ReadonlyRootfs\":true,\"Privileged\":false,\"SecurityOpt\":[\"no-new-privileges\"],\"UsernsMode\":\"auto\",\"PidMode\":\"private\",\"IpcMode\":\"none\",\"NetworkMode\":\"none\",\"UTSMode\":\"private\",\"CgroupMode\":\"private\",\"Memory\":268435456,\"NanoCpus\":1000000000,\"PidsLimit\":16,\"Tmpfs\":{{\"/tmp\":\"rw,noexec,nosuid,nodev,size=16777216\"}}}},\"Mounts\":[{{\"Source\":\"{gate_path}\",\"Destination\":\"/qsr-runtime-gate\",\"Type\":\"bind\",\"Options\":[\"ro\"],\"RW\":false}}]}}]"
    );
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
    for argument in "$@"; do
      if [ "$previous" = '--cidfile' ]; then printf '%s\n' '{owned_id}' > "$argument"; fi
      case "$argument" in --cidfile=*) printf '%s\n' '{owned_id}' > "${{argument#--cidfile=}}" ;; esac
      previous="$argument"
    done
    printf '%s\n' '{owned_id}'
    ;;
  init:*) : ;;
  container:inspect) printf '%s\n' '{inspect}' ;;
  start:*) : ;;
  top:*)
    printf '%s\n' 'PID SECCOMP CAPEFF CAPBND CAPINH CAPPRM CAPAMB LABEL' '1 filter - - - - - containers-default (enforce)'
    ;;
  attach:--sig-proxy=false)
    IFS= read -r token
    [ -n "$token" ] || exit 97
    {attach_response}
    ;;
  wait:*) printf '0\n' ;;
  logs:*) printf 'consumer-output\n' ;;
  rm:--force) : ;;
  *) exit 91 ;;
esac
"#,
        calls = calls.display(),
        owned_id = OWNED_CONTAINER_ID,
        inspect = inspect,
        attach_response = attach_response,
    );
    fs::write(&program, script).expect("fake Podman should be writable");
    let mut permissions = fs::metadata(&program)
        .expect("fake Podman metadata should exist")
        .permissions();
    permissions.set_mode(0o700);
    fs::set_permissions(&program, permissions).expect("fake Podman should be executable");
    (program, calls)
}

fn run_with_hostile_ack(
    request_id: &str,
    attach_response: &str,
) -> (CommandExecutionError, String) {
    let fixture = tempdir().expect("isolated gate acknowledgement fixture should exist");
    let (gate_source, gate_bytes) = write_self_contained_gate(fixture.path());
    let gate_sha256 = format!("{:x}", Sha256::digest(&gate_bytes));
    let gate = RuntimeGateArtifact::stage(&gate_source, &gate_sha256, std::env::consts::ARCH)
        .expect("verified self-contained gate should stage");
    let gate_path = gate.path().display().to_string();
    let (program, calls) = write_fake_podman(fixture.path(), &gate_path, attach_response);
    let adapter = RootlessPodmanAdapter::new(&program).with_runtime_gate_artifact(gate);

    let error = adapter
        .run_command_at(&request(request_id), &policy(), 1_780_000_301)
        .expect_err("hostile release acknowledgement must fail closed");
    let recorded = fs::read_to_string(&calls).expect("backend calls should be recorded");
    (error, recorded)
}

fn assert_exact_id_cleanup(recorded: &str) {
    assert!(
        recorded
            .lines()
            .any(|line| line == format!("rm --force --ignore {OWNED_CONTAINER_ID}")),
        "release-control failure must clean up only the exact acquired container"
    );
}

#[test]
fn malformed_release_acknowledgement_fails_closed_after_exact_id_cleanup() {
    let (error, recorded) = run_with_hostile_ack(
        "runtime-gate-malformed-ack",
        "printf 'NOT_RELEASED\\n'",
    );

    assert_eq!(
        error,
        CommandExecutionError::Backend(ApplicationServiceError::BackendInvocationFailed {
            operation: "runtime_gate_release_ack",
        })
    );
    assert_exact_id_cleanup(&recorded);
}

#[test]
fn over_budget_release_acknowledgement_fails_closed_after_exact_id_cleanup() {
    let (error, recorded) = run_with_hostile_ack(
        "runtime-gate-over-budget-ack",
        "printf '0123456789012345678901234567890123456789012345678901234567890123'",
    );

    assert_eq!(
        error,
        CommandExecutionError::Backend(ApplicationServiceError::BackendInvocationFailed {
            operation: "runtime_gate_release_ack",
        })
    );
    assert_exact_id_cleanup(&recorded);
}
