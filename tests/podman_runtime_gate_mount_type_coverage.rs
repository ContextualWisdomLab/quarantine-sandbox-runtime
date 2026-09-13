//! Reject a runtime-gate mount that matches the staged source but is not an effective bind mount.
//!
//! The gate path is controller-owned. Matching only the reported source path is
//! insufficient evidence because a different mount type can have different
//! lifecycle and backing-store semantics. This regression exercises that
//! fail-closed branch through the public command adapter.

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

const OWNED_CONTAINER_ID: &str = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";

fn policy() -> IsolationPolicy {
    IsolationPolicy {
        policy_id: "runtime_gate_mount_type_coverage_v1".to_owned(),
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
        request_id: "runtime-gate-mount-type-coverage".to_owned(),
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

#[test]
fn non_bind_runtime_gate_mount_fails_closed_before_workload_completion() {
    let fixture = tempfile::tempdir().expect("isolated gate mount fixture should exist");
    let program = fixture.path().join("podman");
    let calls = fixture.path().join("calls");
    let (gate_source, gate_bytes) = write_self_contained_gate(fixture.path());
    let gate_sha256 = format!("{:x}", Sha256::digest(&gate_bytes));
    let gate = RuntimeGateArtifact::stage(&gate_source, &gate_sha256, std::env::consts::ARCH)
        .expect("verified self-contained gate should stage");
    let gate_path = gate.path().display().to_string();
    let inspect = format!(
        "[{{\"Id\":\"{OWNED_CONTAINER_ID}\",\"AppArmorProfile\":\"containers-default\",\"ProcessLabel\":\"\",\"EffectiveCaps\":[],\"BoundingCaps\":[],\"Config\":{{\"User\":\"65532:65532\",\"Timeout\":20}},\"HostConfig\":{{\"ReadonlyRootfs\":true,\"Privileged\":false,\"SecurityOpt\":[\"no-new-privileges\"],\"UsernsMode\":\"auto\",\"PidMode\":\"private\",\"IpcMode\":\"none\",\"NetworkMode\":\"none\",\"UTSMode\":\"private\",\"CgroupMode\":\"private\",\"Memory\":268435456,\"NanoCpus\":1000000000,\"PidsLimit\":16,\"Tmpfs\":{{\"/tmp\":\"rw,noexec,nosuid,nodev,size=16777216\"}}}},\"Mounts\":[{{\"Source\":\"{gate_path}\",\"Destination\":\"/qsr-runtime-gate\",\"Type\":\"volume\",\"Options\":[\"ro\"],\"RW\":false}}]}}]"
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
    printf 'QSR_GATE_RELEASED\n'
    while :; do :; done
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
    );
    fs::write(&program, script).expect("fake Podman should be writable");
    let mut permissions = fs::metadata(&program)
        .expect("fake Podman metadata should exist")
        .permissions();
    permissions.set_mode(0o700);
    fs::set_permissions(&program, permissions).expect("fake Podman should be executable");

    let adapter = RootlessPodmanAdapter::new(&program).with_runtime_gate_artifact(gate);
    let result = adapter.run_command_at(&request(), &policy(), 1_780_000_202);

    assert_eq!(
        result,
        Err(CommandExecutionError::Backend(
            ApplicationServiceError::IsolationVerificationFailed {
                control_name: "runtime_gate_bind_source",
            },
        )),
        "an exact source path must not compensate for a non-bind effective gate mount"
    );

    let recorded = fs::read_to_string(&calls).expect("backend calls should be recorded");
    assert!(
        recorded
            .lines()
            .any(|line| *line == format!("rm --force --ignore {OWNED_CONTAINER_ID}")),
        "failed mount attestation must clean up the exact acquired container: {recorded}"
    );
    assert!(
        !recorded.lines().any(|line| line.starts_with("attach ")),
        "a rejected gate mount must not receive the one-time release token: {recorded}"
    );
    assert!(
        !recorded.lines().any(|line| line.starts_with("wait ")),
        "a rejected gate mount must not reach workload completion evidence: {recorded}"
    );
}
