//! Effective application-service security-option coverage through the public Podman adapter.
//!
//! The configured container evidence and live PID-1 evidence are both part of
//! the P0 admission boundary. These cases exercise real backend-output shapes:
//! alternate accepted no-new-privileges/seccomp spellings and each capability
//! source failing independently. None of these fake-process cases is promoted
//! as real confinement evidence.

#![cfg(target_os = "linux")]

use std::{
    fs,
    net::TcpListener,
    os::unix::fs::PermissionsExt,
    path::PathBuf,
    sync::{
        Mutex,
        atomic::{AtomicU64, Ordering},
    },
    time::{SystemTime, UNIX_EPOCH},
};

use quarantine_sandbox_runtime::{
    ApplicationServiceError, ApplicationServiceRequest, IsolationPolicy, ResourceRequest,
    RootlessPodmanAdapter, ServiceProtocol,
};

static NEXT_PATH_ID: AtomicU64 = AtomicU64::new(0);
static SUBPROCESS_FIXTURE_MUTEX: Mutex<()> = Mutex::new(());

fn temporary_path(name: &str) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock should be after the Unix epoch")
        .as_nanos();
    let unique_id = NEXT_PATH_ID.fetch_add(1, Ordering::Relaxed);
    std::env::temp_dir().join(format!(
        "quarantine-sandbox-runtime-service-security-{name}-{}-{nanos}-{unique_id}",
        std::process::id()
    ))
}

fn policy() -> IsolationPolicy {
    IsolationPolicy {
        policy_id: "service_effective_security_coverage_policy_v1".to_owned(),
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
        request_id: "service-effective-security-coverage".to_owned(),
        image_reference: format!("localhost/cwl/tool@sha256:{}", "d".repeat(64)),
        container_port: 8_080,
        protocol: ServiceProtocol::Tcp,
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

fn write_fake_podman(
    name: &str,
    ready_port: u16,
    effective_caps: &str,
    bounding_caps: &str,
    security_options: &str,
    process_seccomp: &str,
    process_caps: &str,
) -> (PathBuf, PathBuf) {
    let program = temporary_path(name);
    let log = temporary_path(&format!("{name}-log"));
    let script = format!(
        r#"#!/bin/sh
set -eu
printf '%s\n' "$*" >> '{}'
case "${{1:-}}:${{2:-}}" in
  info:--format) printf '%s\n' '{{"host":{{"security":{{"rootless":true,"seccompEnabled":true,"seccompProfilePath":"/usr/share/containers/seccomp.json","apparmorEnabled":true,"selinuxEnabled":false}}}},"version":{{"Version":"5.6.2"}}}}' ;;
  network:create) : ;;
  create:--name) printf 'service-security-container-id\n' ;;
  start:*) : ;;
  container:inspect) printf '%s\n' '[{{"Id":"service-security-container-id","AppArmorProfile":"containers-default","ProcessLabel":"","EffectiveCaps":{effective_caps},"BoundingCaps":{bounding_caps},"Config":{{"User":"65532:65532"}},"HostConfig":{{"ReadonlyRootfs":true,"Privileged":false,"SecurityOpt":{security_options},"UsernsMode":"auto","PidMode":"private","IpcMode":"none","Memory":268435456,"NanoCpus":1000000000,"PidsLimit":32}}}}]' ;;
  top:*) printf 'PID SECCOMP CAPEFF CAPBND CAPINH CAPPRM CAPAMB LABEL\n1 {process_seccomp} {process_caps} {process_caps} {process_caps} {process_caps} {process_caps} containers-default (enforce)\n' ;;
  network:inspect) printf '%s\n' '[{{"internal":true,"dns_enabled":false}}]' ;;
  port:*) printf '127.0.0.1:{ready_port}\n' ;;
  stop:*) : ;;
  rm:*) : ;;
  network:rm) : ;;
  *) exit 91 ;;
esac
"#,
        log.display(),
    );
    fs::write(&program, script).expect("fake Podman should be writable");
    let mut permissions = fs::metadata(&program)
        .expect("fake Podman metadata should exist")
        .permissions();
    permissions.set_mode(0o700);
    fs::set_permissions(&program, permissions).expect("fake Podman should be executable");
    (program, log)
}

fn remove_fixture(program: PathBuf, log: PathBuf) {
    let _ = fs::remove_file(program);
    let _ = fs::remove_file(log);
}

fn serialize_subprocess_fixture() -> std::sync::MutexGuard<'static, ()> {
    // Branch instrumentation materially increases process-heavy test runtime.
    // Keep these fake-backend witnesses from competing for host process-spawn
    // capacity; concurrency semantics are exercised by dedicated coordinator tests.
    SUBPROCESS_FIXTURE_MUTEX
        .lock()
        .expect("subprocess fixture mutex should not be poisoned")
}

#[test]
fn alternate_secure_spellings_are_admitted_only_with_empty_live_capabilities() {
    let _fixture_guard = serialize_subprocess_fixture();
    let listener = TcpListener::bind(("127.0.0.1", 0)).expect("loopback listener should bind");
    let ready_port = listener
        .local_addr()
        .expect("listener address should resolve")
        .port();
    let (program, log) = write_fake_podman(
        "alternate-secure-spellings",
        ready_port,
        "[]",
        "[]",
        r#"["no-new-privileges=true"]"#,
        "strict",
        "0x0",
    );
    let adapter = RootlessPodmanAdapter::new(program.clone());

    let lease = adapter
        .launch_at(&request(), &policy(), 1_780_000_041)
        .expect("supported effective-security spellings should remain admissible");
    assert!(lease.isolation_attestation().no_new_privileges());
    assert!(lease.isolation_attestation().seccomp_enforced());
    assert!(lease.isolation_attestation().all_capabilities_dropped());

    remove_fixture(program, log);
    drop(listener);
}

#[test]
fn each_capability_source_can_independently_fail_closed() {
    let _fixture_guard = serialize_subprocess_fixture();
    let listener = TcpListener::bind(("127.0.0.1", 0)).expect("loopback listener should bind");
    let ready_port = listener
        .local_addr()
        .expect("listener address should resolve")
        .port();

    for (name, effective_caps, bounding_caps, process_caps) in [
        ("effective-cap", r#"["CAP_SYS_ADMIN"]"#, "[]", "-"),
        ("live-process-cap", "[]", "[]", "0x1"),
    ] {
        let (program, log) = write_fake_podman(
            name,
            ready_port,
            effective_caps,
            bounding_caps,
            r#"["no-new-privileges"]"#,
            "filter",
            process_caps,
        );
        let adapter = RootlessPodmanAdapter::new(program.clone());
        assert_eq!(
            adapter.launch_at(&request(), &policy(), 1_780_000_042),
            Err(ApplicationServiceError::IsolationVerificationFailed {
                control_name: "all_capabilities_dropped",
            })
        );
        let calls = fs::read_to_string(&log).expect("fake Podman calls should be recorded");
        assert!(calls.contains("stop --time 2"));
        assert!(calls.contains("rm --force"));
        assert!(calls.contains("network rm --force"));
        remove_fixture(program, log);
    }

    drop(listener);
}

#[test]
fn missing_no_new_privileges_or_explicit_unconfined_seccomp_fails_closed() {
    let _fixture_guard = serialize_subprocess_fixture();
    let listener = TcpListener::bind(("127.0.0.1", 0)).expect("loopback listener should bind");
    let ready_port = listener
        .local_addr()
        .expect("listener address should resolve")
        .port();

    for (name, security_options, control_name) in [
        ("missing-nnp", "[]", "no_new_privileges"),
        (
            "unconfined-seccomp",
            r#"["no-new-privileges","seccomp=unconfined"]"#,
            "seccomp",
        ),
    ] {
        let (program, log) = write_fake_podman(
            name,
            ready_port,
            "[]",
            "[]",
            security_options,
            "filter",
            "-",
        );
        let adapter = RootlessPodmanAdapter::new(program.clone());
        assert_eq!(
            adapter.launch_at(&request(), &policy(), 1_780_000_043),
            Err(ApplicationServiceError::IsolationVerificationFailed { control_name })
        );
        let calls = fs::read_to_string(&log).expect("fake Podman calls should be recorded");
        assert!(calls.contains("stop --time 2"));
        assert!(calls.contains("rm --force"));
        assert!(calls.contains("network rm --force"));
        remove_fixture(program, log);
    }

    drop(listener);
}
