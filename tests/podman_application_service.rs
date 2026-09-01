//! Process-boundary tests for the rootless Podman application-service adapter.

#![cfg(target_os = "linux")]

use std::{
    fs,
    net::TcpListener,
    os::unix::fs::PermissionsExt,
    path::PathBuf,
    time::{SystemTime, UNIX_EPOCH},
};

use quarantine_sandbox_runtime::{
    ApplicationServiceError, ApplicationServiceRequest, IsolationPolicy, ResourceRequest,
    RootlessPodmanAdapter, ServiceProtocol,
};

fn digest_image() -> String {
    format!("localhost/cwl/tool@sha256:{}", "b".repeat(64))
}

fn policy(readiness_timeout_millis: u64) -> IsolationPolicy {
    IsolationPolicy {
        policy_id: "process_boundary_policy_v1".to_owned(),
        maximum_memory_bytes: 512 * 1024 * 1024,
        maximum_cpu_millicores: 2_000,
        maximum_processes: 128,
        maximum_lease_seconds: 900,
        maximum_tmpfs_bytes: 128 * 1024 * 1024,
        readiness_timeout_millis,
        readiness_poll_interval_millis: 10,
        shutdown_grace_seconds: 2,
        run_as_user_id: 65_532,
        run_as_group_id: 65_532,
    }
}

fn request() -> ApplicationServiceRequest {
    ApplicationServiceRequest {
        schema_version: "1.0.0".to_owned(),
        request_id: "process_boundary_request".to_owned(),
        image_reference: digest_image(),
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

fn temporary_path(name: &str) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock should be after the Unix epoch")
        .as_nanos();
    std::env::temp_dir().join(format!(
        "quarantine-sandbox-runtime-{name}-{}-{nanos}",
        std::process::id()
    ))
}

fn write_fake_podman(ready_port: u16) -> (PathBuf, PathBuf) {
    let program = temporary_path("fake-podman");
    let log = temporary_path("fake-podman-log");
    let script = format!(
        "#!/bin/sh\nset -eu\nprintf '%s\\n' \"$*\" >> '{}'\ncase \"${{1:-}}\" in\n  info) printf 'true\\n' ;;\n  network) : ;;\n  create) printf 'fake-container-id\\n' ;;\n  start) : ;;\n  port) printf '127.0.0.1:{ready_port}\\n' ;;\n  stop) : ;;\n  rm) : ;;\n  *) exit 91 ;;\nesac\n",
        log.display()
    );
    fs::write(&program, script).expect("fake Podman should be writable");
    let mut permissions = fs::metadata(&program)
        .expect("fake Podman metadata should exist")
        .permissions();
    permissions.set_mode(0o700);
    fs::set_permissions(&program, permissions).expect("fake Podman should be executable");
    (program, log)
}

#[test]
fn launch_requires_rootless_backend_and_returns_loopback_lease_then_cleans_up() {
    let listener = TcpListener::bind(("127.0.0.1", 0)).expect("loopback listener should bind");
    let ready_port = listener
        .local_addr()
        .expect("listener should expose its address")
        .port();
    let (program, log) = write_fake_podman(ready_port);
    let adapter = RootlessPodmanAdapter::new(program.clone());

    let lease = adapter
        .launch_at(&request(), &policy(500), 1_780_000_000)
        .expect("rootless isolated service should become ready");
    assert_eq!(lease.endpoint().host(), "127.0.0.1");
    assert_eq!(lease.endpoint().port(), ready_port);
    assert_eq!(lease.endpoint().protocol(), ServiceProtocol::Http);
    assert_eq!(lease.image_reference(), digest_image());
    assert_eq!(lease.backend_id(), "rootless_podman");
    assert!(lease.isolation_attestation().rootless());
    assert!(lease.isolation_attestation().read_only_root_filesystem());
    assert!(lease.isolation_attestation().all_capabilities_dropped());
    assert!(lease.isolation_attestation().no_new_privileges());
    assert!(lease.isolation_attestation().isolated_user_namespace());
    assert!(lease.isolation_attestation().external_egress_denied());
    assert!(lease.isolation_attestation().loopback_only_publication());
    assert!(!lease.isolation_attestation().credentials_available());

    let cleanup = adapter
        .terminate_at(&lease, 1_780_000_010)
        .expect("termination should remove container and network");
    assert!(cleanup.container_removed());
    assert!(cleanup.network_removed());
    assert_eq!(cleanup.terminated_at_epoch_seconds(), 1_780_000_010);

    let calls = fs::read_to_string(&log).expect("fake Podman calls should be recorded");
    for expected in ["info --format", "network create", "create --name", "start ", "port ", "stop --time", "rm --force", "network rm --force"] {
        assert!(calls.contains(expected), "missing Podman call fragment: {expected}\n{calls}");
    }

    let _ = fs::remove_file(program);
    let _ = fs::remove_file(log);
    drop(listener);
}

#[test]
fn readiness_timeout_fails_closed_and_removes_created_isolation_resources() {
    let unavailable_port = {
        let listener = TcpListener::bind(("127.0.0.1", 0)).expect("loopback port should bind");
        listener.local_addr().expect("address should resolve").port()
    };
    let (program, log) = write_fake_podman(unavailable_port);
    let adapter = RootlessPodmanAdapter::new(program.clone());

    assert!(matches!(
        adapter.launch_at(&request(), &policy(30), 1_780_000_000),
        Err(ApplicationServiceError::ReadinessTimeout)
    ));

    let calls = fs::read_to_string(&log).expect("cleanup calls should be recorded");
    assert!(calls.contains("rm --force"));
    assert!(calls.contains("network rm --force"));

    let _ = fs::remove_file(program);
    let _ = fs::remove_file(log);
}
