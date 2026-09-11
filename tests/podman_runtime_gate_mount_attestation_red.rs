//! RED: effective mount attestation must admit exactly the runtime-owned gate bind.
//!
//! Production gated commands always bind the independently verified gate at
//! `/qsr-runtime-gate`. Podman exposes that bind in `container inspect` Mounts,
//! so treating every mount as unexpected makes the release path reject the very
//! security control it requires. This fixture models the real effective mount
//! and requires the gate bind to be accepted only when it is the exact staged
//! artifact, read-only, and otherwise isolated from consumer mount authority.

#![cfg(target_os = "linux")]

#[path = "support/runtime_gate_fixture.rs"]
mod runtime_gate_fixture;

use std::{fs, os::unix::fs::PermissionsExt};

use quarantine_sandbox_runtime::{
    CommandExecutionRequest, IsolationPolicy, ResourceRequest, RootlessPodmanAdapter,
    RuntimeGateArtifact,
};
use runtime_gate_fixture::write_self_contained_gate;
use sha2::{Digest, Sha256};

const OWNED_CONTAINER_ID: &str =
    "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";

fn policy() -> IsolationPolicy {
    IsolationPolicy {
        policy_id: "runtime_gate_mount_attestation_v1".to_owned(),
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
        request_id: "runtime-gate-mount-attestation".to_owned(),
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
fn exact_runtime_gate_bind_is_part_of_the_expected_effective_mount_set() {
    let fixture = tempfile::tempdir().expect("isolated gate mount fixture should exist");
    let program = fixture.path().join("podman");
    let calls = fixture.path().join("calls");
    let (gate_source, gate_bytes) = write_self_contained_gate(fixture.path());
    let gate_sha256 = format!("{:x}", Sha256::digest(&gate_bytes));
    let gate = RuntimeGateArtifact::stage(&gate_source, &gate_sha256, std::env::consts::ARCH)
        .expect("verified self-contained gate should stage");
    let gate_path = gate.path().display().to_string();
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
    let result = adapter
        .run_command_at(&request(), &policy(), 1_780_000_201)
        .expect("the exact runtime-owned gate bind belongs to the admitted mount set");

    assert_eq!(result.exit_code(), 0);
    assert_eq!(result.stdout(), "consumer-output\n");
    let recorded = fs::read_to_string(&calls).expect("backend calls should be recorded");
    assert!(
        recorded
            .lines()
            .any(|line| *line == format!("rm --force --ignore {OWNED_CONTAINER_ID}")),
        "successful gated execution must still clean up the exact acquired container: {recorded}"
    );
}
