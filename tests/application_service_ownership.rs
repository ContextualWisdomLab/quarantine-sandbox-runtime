//! Caller-scoped idempotency and lease-ownership tests for isolated services.

#![cfg(target_os = "linux")]

use std::{
    fs,
    net::TcpListener,
    os::unix::fs::PermissionsExt,
    path::{Path, PathBuf},
    sync::Arc,
    thread,
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use quarantine_sandbox_runtime::{
    ApplicationServiceCoordinator, ApplicationServiceCoordinatorError, ApplicationServiceError,
    ApplicationServiceRequest, IsolationPolicy, LeaseOwnerId, ResourceRequest,
    RootlessPodmanAdapter, ServiceProtocol,
};

fn digest_image() -> String {
    format!("localhost/cwl/tool@sha256:{}", "d".repeat(64))
}

fn owner(value: &str) -> LeaseOwnerId {
    LeaseOwnerId::new(value).expect("test owner identity should satisfy the bounded contract")
}

fn policy() -> IsolationPolicy {
    IsolationPolicy {
        policy_id: "lease_ownership_policy_v1".to_owned(),
        maximum_memory_bytes: 512 * 1024 * 1024,
        maximum_cpu_millicores: 2_000,
        maximum_processes: 128,
        maximum_lease_seconds: 900,
        maximum_tmpfs_bytes: 128 * 1024 * 1024,
        readiness_timeout_millis: 2_000,
        readiness_poll_interval_millis: 10,
        shutdown_grace_seconds: 2,
        run_as_user_id: 65_532,
        run_as_group_id: 65_532,
    }
}

fn request() -> ApplicationServiceRequest {
    ApplicationServiceRequest {
        schema_version: "1.0.0".to_owned(),
        request_id: "agent_task_lease_42".to_owned(),
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
        "qsr-ownership-{name}-{}-{nanos}",
        std::process::id()
    ))
}

fn write_fake_podman(program: &Path, log: &Path, mode: &str, ready_port: u16) {
    let info = r#"{"host":{"security":{"rootless":true,"seccompEnabled":true,"seccompProfilePath":"/usr/share/containers/seccomp.json","apparmorEnabled":true,"selinuxEnabled":false}},"version":{"Version":"5.6.2"}}"#;
    let container = r#"[{"Id":"fake-container-id","AppArmorProfile":"containers-default","ProcessLabel":"","EffectiveCaps":[],"BoundingCaps":[],"Config":{"User":"65532:65532"},"HostConfig":{"ReadonlyRootfs":true,"Privileged":false,"SecurityOpt":["no-new-privileges"],"UsernsMode":"auto","PidMode":"private","IpcMode":"none","Memory":268435456,"NanoCpus":1000000000,"PidsLimit":32}}]"#;
    let network = r#"[{"internal":true,"dns_enabled":false}]"#;
    let script = format!(
        "#!/bin/sh\nset -eu\nMODE='{mode}'\nprintf '%s\\n' \"$*\" >> '{}'\nif [ \"$MODE\" = slow_rootless ] && [ \"${{1:-}}\" = info ]; then sleep 1; fi\nif [ \"$MODE\" = fail_rootless ] && [ \"${{1:-}}\" = info ]; then exit 20; fi\ncase \"${{1:-}}:${{2:-}}\" in\n  info:--format) printf '%s\\n' '{}' ;;\n  network:create) : ;;\n  network:inspect) printf '%s\\n' '{}' ;;\n  network:rm) : ;;\n  container:inspect) printf '%s\\n' '{}' ;;\n  top:*) printf 'PID SECCOMP CAPEFF CAPBND CAPINH CAPPRM CAPAMB LABEL\\n1 filter 0000000000000000 0000000000000000 0000000000000000 0000000000000000 0000000000000000 containers-default\\n' ;;\n  create:--name) printf 'fake-container-id\\n' ;;\n  start:*) : ;;\n  port:*) printf '127.0.0.1:{ready_port}\\n' ;;\n  stop:*) : ;;\n  rm:*) : ;;\n  *) exit 91 ;;\nesac\n",
        log.display(),
        info,
        network,
        container,
    );
    fs::write(program, script).expect("fake Podman should be writable");
    let mut permissions = fs::metadata(program)
        .expect("fake Podman metadata should exist")
        .permissions();
    permissions.set_mode(0o700);
    fs::set_permissions(program, permissions).expect("fake Podman should be executable");
}

fn count_calls(log: &Path, needle: &str) -> usize {
    fs::read_to_string(log)
        .unwrap_or_default()
        .lines()
        .filter(|line| line.contains(needle))
        .count()
}

fn wait_until_log_contains(log: &Path, needle: &str) {
    for _ in 0..100 {
        if count_calls(log, needle) > 0 {
            return;
        }
        thread::sleep(Duration::from_millis(10));
    }
    panic!("timed out waiting for fake Podman call containing {needle}");
}

#[test]
fn lease_owner_ids_are_bounded_opaque_runtime_context() {
    assert_eq!(
        LeaseOwnerId::new(""),
        Err(ApplicationServiceCoordinatorError::InvalidLeaseOwnerId)
    );
    assert_eq!(
        LeaseOwnerId::new("contains whitespace"),
        Err(ApplicationServiceCoordinatorError::InvalidLeaseOwnerId)
    );
    assert_eq!(
        LeaseOwnerId::new(&"a".repeat(129)),
        Err(ApplicationServiceCoordinatorError::InvalidLeaseOwnerId)
    );
    assert_eq!(
        owner("urn:cwl:agent:contextual-orchestrator").as_str(),
        "urn:cwl:agent:contextual-orchestrator"
    );
}

#[test]
fn identical_retry_returns_existing_lease_without_second_launch() {
    let listener = TcpListener::bind(("127.0.0.1", 0)).expect("loopback listener should bind");
    let ready_port = listener
        .local_addr()
        .expect("address should resolve")
        .port();
    let program = temporary_path("fake-podman");
    let log = temporary_path("fake-podman-log");
    write_fake_podman(&program, &log, "success", ready_port);
    let coordinator =
        ApplicationServiceCoordinator::new(RootlessPodmanAdapter::new(program.clone()));
    let owner = owner("urn:cwl:agent:contextual-orchestrator");

    let first = coordinator
        .launch_at(&owner, &request(), &policy(), 1_780_000_000)
        .expect("first launch should succeed");
    let retry = coordinator
        .launch_at(&owner, &request(), &policy(), 1_780_000_100)
        .expect("identical retry should return the active lease");

    assert_eq!(retry, first);
    assert_eq!(count_calls(&log, "network create"), 1);
    coordinator
        .terminate_at(&owner, &first, 1_780_000_110)
        .expect("owner should terminate its lease");
    let _ = fs::remove_file(program);
    let _ = fs::remove_file(log);
    drop(listener);
}

#[test]
fn same_owner_and_request_id_with_different_content_fails_closed() {
    let listener = TcpListener::bind(("127.0.0.1", 0)).expect("loopback listener should bind");
    let ready_port = listener
        .local_addr()
        .expect("address should resolve")
        .port();
    let program = temporary_path("fake-podman");
    let log = temporary_path("fake-podman-log");
    write_fake_podman(&program, &log, "success", ready_port);
    let coordinator =
        ApplicationServiceCoordinator::new(RootlessPodmanAdapter::new(program.clone()));
    let owner = owner("urn:cwl:agent:contextual-orchestrator");
    let first = coordinator
        .launch_at(&owner, &request(), &policy(), 1_780_000_000)
        .expect("first launch should succeed");
    let mut changed = request();
    changed.command.push("--different".to_owned());

    assert_eq!(
        coordinator.launch_at(&owner, &changed, &policy(), 1_780_000_001),
        Err(ApplicationServiceCoordinatorError::IdempotencyConflict)
    );
    assert_eq!(count_calls(&log, "network create"), 1);
    coordinator
        .terminate_at(&owner, &first, 1_780_000_010)
        .expect("original owner should still terminate the first lease");
    let _ = fs::remove_file(program);
    let _ = fs::remove_file(log);
    drop(listener);
}

#[test]
fn wrong_owner_cannot_terminate_another_callers_lease() {
    let listener = TcpListener::bind(("127.0.0.1", 0)).expect("loopback listener should bind");
    let ready_port = listener
        .local_addr()
        .expect("address should resolve")
        .port();
    let program = temporary_path("fake-podman");
    let log = temporary_path("fake-podman-log");
    write_fake_podman(&program, &log, "success", ready_port);
    let coordinator =
        ApplicationServiceCoordinator::new(RootlessPodmanAdapter::new(program.clone()));
    let correct_owner = owner("urn:cwl:agent:contextual-orchestrator");
    let wrong_owner = owner("urn:cwl:consumer:wardnet");
    let lease = coordinator
        .launch_at(&correct_owner, &request(), &policy(), 1_780_000_000)
        .expect("launch should succeed");

    assert_eq!(
        coordinator.terminate_at(&wrong_owner, &lease, 1_780_000_010),
        Err(ApplicationServiceCoordinatorError::UnknownLease)
    );
    assert_eq!(count_calls(&log, "stop --time"), 0);
    coordinator
        .terminate_at(&correct_owner, &lease, 1_780_000_011)
        .expect("correct owner should terminate the lease");
    assert_eq!(count_calls(&log, "stop --time"), 1);
    let _ = fs::remove_file(program);
    let _ = fs::remove_file(log);
    drop(listener);
}

#[test]
fn failed_launch_releases_idempotency_reservation_for_retry() {
    let listener = TcpListener::bind(("127.0.0.1", 0)).expect("loopback listener should bind");
    let ready_port = listener
        .local_addr()
        .expect("address should resolve")
        .port();
    let program = temporary_path("fake-podman");
    let log = temporary_path("fake-podman-log");
    write_fake_podman(&program, &log, "fail_rootless", ready_port);
    let coordinator =
        ApplicationServiceCoordinator::new(RootlessPodmanAdapter::new(program.clone()));
    let owner = owner("urn:cwl:agent:contextual-orchestrator");

    assert_eq!(
        coordinator.launch_at(&owner, &request(), &policy(), 1_780_000_000),
        Err(ApplicationServiceCoordinatorError::Backend(
            ApplicationServiceError::BackendCommandFailed {
                operation: "backend_security_info",
            }
        ))
    );
    write_fake_podman(&program, &log, "success", ready_port);
    let lease = coordinator
        .launch_at(&owner, &request(), &policy(), 1_780_000_001)
        .expect("retry after a failed launch should be allowed");
    coordinator
        .terminate_at(&owner, &lease, 1_780_000_010)
        .expect("retried lease should terminate cleanly");
    let _ = fs::remove_file(program);
    let _ = fs::remove_file(log);
    drop(listener);
}

#[test]
fn concurrent_duplicate_launch_is_rejected_while_first_launch_is_in_flight() {
    let listener = TcpListener::bind(("127.0.0.1", 0)).expect("loopback listener should bind");
    let ready_port = listener
        .local_addr()
        .expect("address should resolve")
        .port();
    let program = temporary_path("fake-podman");
    let log = temporary_path("fake-podman-log");
    write_fake_podman(&program, &log, "slow_rootless", ready_port);
    let coordinator = Arc::new(ApplicationServiceCoordinator::new(
        RootlessPodmanAdapter::new(program.clone()),
    ));
    let owner = owner("urn:cwl:agent:contextual-orchestrator");
    let worker_coordinator = Arc::clone(&coordinator);
    let worker_owner = owner.clone();
    let worker = thread::spawn(move || {
        worker_coordinator.launch_at(&worker_owner, &request(), &policy(), 1_780_000_000)
    });
    wait_until_log_contains(&log, "info --format json");

    assert_eq!(
        coordinator.launch_at(&owner, &request(), &policy(), 1_780_000_001),
        Err(ApplicationServiceCoordinatorError::LaunchInProgress)
    );
    let lease = worker
        .join()
        .expect("launch thread should not panic")
        .expect("first launch should finish successfully");
    assert_eq!(count_calls(&log, "network create"), 1);
    coordinator
        .terminate_at(&owner, &lease, 1_780_000_010)
        .expect("owner should terminate the finished lease");
    let _ = fs::remove_file(program);
    let _ = fs::remove_file(log);
    drop(listener);
}

#[test]
fn expired_lease_cleanup_is_bounded_and_attributed_to_owner_and_request() {
    let listener = TcpListener::bind(("127.0.0.1", 0)).expect("loopback listener should bind");
    let ready_port = listener
        .local_addr()
        .expect("address should resolve")
        .port();
    let program = temporary_path("fake-podman");
    let log = temporary_path("fake-podman-log");
    write_fake_podman(&program, &log, "success", ready_port);
    let coordinator =
        ApplicationServiceCoordinator::new(RootlessPodmanAdapter::new(program.clone()));
    let owner = owner("urn:cwl:agent:contextual-orchestrator");
    let mut short_request = request();
    short_request.resources.lease_seconds = 1;
    coordinator
        .launch_at(&owner, &short_request, &policy(), 1_780_000_000)
        .expect("short lease should launch");

    assert!(
        coordinator
            .cleanup_expired_at(1_780_000_000)
            .expect("registry should be available")
            .is_empty()
    );
    let outcomes = coordinator
        .cleanup_expired_at(1_780_000_001)
        .expect("expired cleanup should access registry");
    assert_eq!(outcomes.len(), 1);
    assert_eq!(outcomes[0].lease_owner_id().as_str(), owner.as_str());
    assert_eq!(outcomes[0].request_id(), "agent_task_lease_42");
    assert!(outcomes[0].result().is_ok());
    assert_eq!(count_calls(&log, "stop --time"), 1);
    assert_eq!(
        coordinator.terminate_at(&owner, outcomes[0].lease(), 1_780_000_002),
        Err(ApplicationServiceCoordinatorError::UnknownLease)
    );
    let _ = fs::remove_file(program);
    let _ = fs::remove_file(log);
    drop(listener);
}
