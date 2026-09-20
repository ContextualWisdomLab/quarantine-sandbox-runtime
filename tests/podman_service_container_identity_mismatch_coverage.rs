//! Fail-closed coverage for application-service container identity contradictions.
//!
//! `podman create` is the authority for the newly acquired container ID. Later
//! `container inspect` evidence must describe that same object; a contradictory
//! `Id` is not eligible to contribute isolation evidence and must trigger full
//! service-resource cleanup before any process/network attestation is trusted.

#![cfg(target_os = "linux")]

use std::{
    fs,
    os::unix::fs::PermissionsExt,
    path::PathBuf,
    sync::atomic::{AtomicU64, Ordering},
    time::{SystemTime, UNIX_EPOCH},
};

use quarantine_sandbox_runtime::{
    ApplicationServiceError, ApplicationServiceRequest, IsolationPolicy, ResourceRequest,
    RootlessPodmanAdapter, ServiceProtocol,
};

static NEXT_PATH_ID: AtomicU64 = AtomicU64::new(0);

fn temporary_path(name: &str) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock should be after the Unix epoch")
        .as_nanos();
    let unique_id = NEXT_PATH_ID.fetch_add(1, Ordering::Relaxed);
    std::env::temp_dir().join(format!(
        "quarantine-sandbox-runtime-service-identity-{name}-{}-{nanos}-{unique_id}",
        std::process::id()
    ))
}

fn policy() -> IsolationPolicy {
    IsolationPolicy {
        policy_id: "service_container_identity_mismatch_policy_v1".to_owned(),
        maximum_memory_bytes: 512 * 1024 * 1024,
        maximum_cpu_millicores: 2_000,
        maximum_processes: 128,
        maximum_lease_seconds: 900,
        maximum_tmpfs_bytes: 128 * 1024 * 1024,
        readiness_timeout_millis: 500,
        readiness_poll_interval_millis: 10,
        shutdown_grace_seconds: 2,
        run_as_user_id: 65_532,
        run_as_group_id: 65_532,
    }
}

fn request() -> ApplicationServiceRequest {
    ApplicationServiceRequest {
        schema_version: "1.0.0".to_owned(),
        request_id: "service-container-identity-mismatch".to_owned(),
        image_reference: format!("localhost/cwl/tool@sha256:{}", "d".repeat(64)),
        container_port: 8_080,
        protocol: ServiceProtocol::Http,
        command: vec!["serve".to_owned()],
        resources: ResourceRequest {
            memory_bytes: 256 * 1024 * 1024,
            cpu_millicores: 1_000,
            maximum_processes: 32,
            lease_seconds: 300,
            tmpfs_bytes: 32 * 1024 * 1024,
        },
    }
}

#[test]
fn contradictory_inspect_identity_is_rejected_before_effective_evidence_is_trusted() {
    let program = temporary_path("fake-podman");
    let call_log = temporary_path("call-log");
    let script = format!(
        r#"#!/bin/sh
set -eu
printf '%s\n' "$*" >> '{}'
case "${{1:-}}:${{2:-}}" in
  info:--format) printf '%s\n' '{{"host":{{"security":{{"rootless":true,"seccompEnabled":true,"seccompProfilePath":"/usr/share/containers/seccomp.json","apparmorEnabled":true,"selinuxEnabled":false}}}},"version":{{"Version":"5.6.2"}}}}' ;;
  network:create) : ;;
  create:--name) printf 'acquired-service-container-id\n' ;;
  start:*) : ;;
  container:inspect) printf '%s\n' '[{{"Id":"different-service-container-id","AppArmorProfile":"containers-default","ProcessLabel":"","EffectiveCaps":[],"BoundingCaps":[],"Config":{{"User":"65532:65532"}},"HostConfig":{{"ReadonlyRootfs":true,"Privileged":false,"SecurityOpt":["no-new-privileges"],"UsernsMode":"auto","PidMode":"private","IpcMode":"none","Memory":268435456,"NanoCpus":1000000000,"PidsLimit":32}}}}]' ;;
  stop:*) : ;;
  rm:*) : ;;
  network:rm) : ;;
  top:*|network:inspect|port:*) exit 98 ;;
  *) exit 91 ;;
esac
"#,
        call_log.display(),
    );
    fs::write(&program, script).expect("fake Podman should be writable");
    let mut permissions = fs::metadata(&program)
        .expect("fake Podman metadata should exist")
        .permissions();
    permissions.set_mode(0o700);
    fs::set_permissions(&program, permissions).expect("fake Podman should be executable");

    let adapter = RootlessPodmanAdapter::new(program.clone());
    assert_eq!(
        adapter.launch_at(&request(), &policy(), 1_780_000_040),
        Err(ApplicationServiceError::MalformedIsolationInspection {
            operation: "container_inspect",
        }),
        "inspect evidence for a different container must never be promoted into isolation attestation"
    );

    let calls = fs::read_to_string(&call_log).expect("fake Podman calls should be recorded");
    assert!(calls.contains("container inspect --format json"));
    assert!(calls.contains("stop --time 2"));
    assert!(calls.contains("rm --force"));
    assert!(calls.contains("network rm --force"));
    assert!(
        !calls.lines().any(|line| {
            line.starts_with("top ")
                || line.starts_with("network inspect ")
                || line.starts_with("port ")
        }),
        "no later effective/process/network evidence may be consumed after identity contradiction: {calls}"
    );

    let _ = fs::remove_file(program);
    let _ = fs::remove_file(call_log);
}
