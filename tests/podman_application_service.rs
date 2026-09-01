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

fn write_fake_podman(mode: &str, ready_port: u16) -> (PathBuf, PathBuf) {
    let program = temporary_path("fake-podman");
    let log = temporary_path("fake-podman-log");
    let rootless = if mode == "rootless_false" { "false" } else { "true" };
    let info = format!(
        r#"{{"host":{{"security":{{"rootless":{rootless},"seccompEnabled":true,"seccompProfilePath":"/usr/share/containers/seccomp.json","apparmorEnabled":true,"selinuxEnabled":false}}}},"version":{{"Version":"5.6.2"}}}}"#
    );
    let container = r#"[{"Id":"fake-container-id","AppArmorProfile":"containers-default","ProcessLabel":"","EffectiveCaps":[],"BoundingCaps":[],"Config":{"User":"65532:65532"},"HostConfig":{"ReadonlyRootfs":true,"Privileged":false,"SecurityOpt":["no-new-privileges"],"UsernsMode":"auto","PidMode":"private","IpcMode":"none","Memory":268435456,"NanoCpus":1000000000,"PidsLimit":32}}]"#;
    let network = r#"[{"internal":true,"dns_enabled":false}]"#;
    let script = format!(
        "#!/bin/sh\nset -eu\nMODE='{mode}'\nprintf '%s\\n' \"$*\" >> '{}'\ncase \"$MODE:${{1:-}}:${{2:-}}\" in\n  rootless_command_fail:info:*) exit 20 ;;\n  network_create_fail:network:create) exit 21 ;;\n  container_create_fail:create:*) exit 22 ;;\n  container_create_cleanup_fail:create:*) exit 22 ;;\n  container_create_cleanup_fail:network:rm) exit 23 ;;\n  start_fail:start:*) exit 24 ;;\n  port_fail:port:*) exit 25 ;;\n  invalid_port_host:port:*) printf '0.0.0.0:{ready_port}\\n'; exit 0 ;;\n  invalid_port_text:port:*) printf '127.0.0.1:not-a-port\\n'; exit 0 ;;\n  invalid_port_zero:port:*) printf '127.0.0.1:0\\n'; exit 0 ;;\n  readiness_cleanup_fail:rm:*) exit 26 ;;\n  termination_cleanup_fail:stop:*) exit 27 ;;\nesac\ncase \"${{1:-}}:${{2:-}}\" in\n  info:--format) printf '%s\\n' '{}' ;;\n  network:create) : ;;\n  network:inspect) printf '%s\\n' '{}' ;;\n  network:rm) : ;;\n  container:inspect) printf '%s\\n' '{}' ;;\n  create:--name) printf 'fake-container-id\\n' ;;\n  start:*) : ;;\n  top:*) printf 'PID SECCOMP CAPEFF CAPBND CAPINH CAPPRM CAPAMB LABEL\\n1 filter - - - - - containers-default\\n' ;;\n  port:*) printf '127.0.0.1:{ready_port}\\n' ;;\n  stop:*) : ;;\n  rm:*) : ;;\n  *) exit 91 ;;\nesac\n",
        log.display(),
        info,
        network,
        container,
    );
    fs::write(&program, script).expect("fake Podman should be writable");
    let mut permissions = fs::metadata(&program)
        .expect("fake Podman metadata should exist")
        .permissions();
    permissions.set_mode(0o700);
    fs::set_permissions(&program, permissions).expect("fake Podman should be executable");
    (program, log)
}

fn closed_loopback_port() -> u16 {
    let listener = TcpListener::bind(("127.0.0.1", 0)).expect("loopback port should bind");
    listener
        .local_addr()
        .expect("address should resolve")
        .port()
}

fn remove_fixture(program: PathBuf, log: PathBuf) {
    let _ = fs::remove_file(program);
    let _ = fs::remove_file(log);
}

#[test]
fn launch_requires_rootless_backend_and_returns_loopback_lease_then_cleans_up() {
    let listener = TcpListener::bind(("127.0.0.1", 0)).expect("loopback listener should bind");
    let ready_port = listener
        .local_addr()
        .expect("listener should expose its address")
        .port();
    let (program, log) = write_fake_podman("success", ready_port);
    let adapter = RootlessPodmanAdapter::new(program.clone());

    let lease = adapter
        .launch_at(&request(), &policy(500), 1_780_000_000)
        .expect("rootless isolated service should become ready");
    assert_eq!(lease.schema_version(), "1.2.0");
    assert_eq!(lease.request_id(), "process_boundary_request");
    assert_eq!(lease.endpoint().host(), "127.0.0.1");
    assert_eq!(lease.endpoint().port(), ready_port);
    assert_eq!(lease.endpoint().protocol(), ServiceProtocol::Http);
    assert_eq!(lease.image_reference(), digest_image());
    assert_eq!(lease.backend_id(), "rootless_podman");
    assert_eq!(lease.backend_version(), "5.6.2");
    assert_eq!(lease.policy_id(), "process_boundary_policy_v1");
    assert_eq!(lease.policy_sha256(), policy(500).effective_policy_sha256());
    assert_eq!(
        serde_json::to_value(&lease).expect("lease must serialize")["policy_sha256"],
        lease.policy_sha256()
    );
    assert_eq!(lease.started_at_epoch_seconds(), 1_780_000_000);
    assert_eq!(lease.expires_at_epoch_seconds(), 1_780_000_300);
    assert!(lease.isolation_attestation().rootless());
    assert!(lease.isolation_attestation().read_only_root_filesystem());
    assert!(lease.isolation_attestation().all_capabilities_dropped());
    assert!(lease.isolation_attestation().no_new_privileges());
    assert!(lease.isolation_attestation().isolated_user_namespace());
    assert!(lease.isolation_attestation().external_egress_denied());
    assert!(lease.isolation_attestation().loopback_only_publication());
    assert!(lease.isolation_attestation().seccomp_enforced());
    assert!(lease.isolation_attestation().lsm_enforced());
    assert!(lease.isolation_attestation().resource_limits_verified());
    assert!(!lease.isolation_attestation().credentials_available());

    let cleanup = adapter
        .terminate_at(&lease, 1_780_000_010)
        .expect("termination should remove container and network");
    assert_eq!(cleanup.schema_version(), "1.0.0");
    assert_eq!(cleanup.sandbox_id(), lease.sandbox_id());
    assert_eq!(cleanup.network_id(), lease.network_id());
    assert!(cleanup.container_removed());
    assert!(cleanup.network_removed());
    assert_eq!(cleanup.terminated_at_epoch_seconds(), 1_780_000_010);

    let calls = fs::read_to_string(&log).expect("fake Podman calls should be recorded");
    for expected in [
        "info --format json",
        "network create",
        "create --name",
        "--http-proxy=false",
        "start ",
        "container inspect --format json",
        "top ",
        "network inspect --format json",
        "port ",
        "stop --time 2",
        "rm --force",
        "network rm --force",
    ] {
        assert!(
            calls.contains(expected),
            "missing Podman call fragment: {expected}\n{calls}"
        );
    }

    remove_fixture(program, log);
    drop(listener);
}

#[test]
fn missing_or_non_rootless_backend_fails_before_isolation_resources_are_created() {
    let missing = RootlessPodmanAdapter::new(temporary_path("missing-podman"));
    assert_eq!(
        missing.launch_at(&request(), &policy(50), 1_780_000_000),
        Err(ApplicationServiceError::BackendInvocationFailed {
            operation: "backend_security_info",
        })
    );

    for (mode, expected) in [
        (
            "rootless_command_fail",
            ApplicationServiceError::BackendCommandFailed {
                operation: "backend_security_info",
            },
        ),
        (
            "rootless_false",
            ApplicationServiceError::BackendNotRootless,
        ),
    ] {
        let (program, log) = write_fake_podman(mode, closed_loopback_port());
        let adapter = RootlessPodmanAdapter::new(program.clone());
        assert_eq!(
            adapter.launch_at(&request(), &policy(50), 1_780_000_000),
            Err(expected)
        );
        let calls = fs::read_to_string(&log).expect("probe call should be recorded");
        assert!(!calls.contains("network create"));
        remove_fixture(program, log);
    }
}

#[test]
fn creation_failures_cleanup_only_resources_that_were_created() {
    let (program, log) = write_fake_podman("network_create_fail", closed_loopback_port());
    let adapter = RootlessPodmanAdapter::new(program.clone());
    assert_eq!(
        adapter.launch_at(&request(), &policy(50), 1_780_000_000),
        Err(ApplicationServiceError::BackendCommandFailed {
            operation: "network_create",
        })
    );
    let calls = fs::read_to_string(&log).expect("network failure calls should be recorded");
    assert!(!calls.contains("create --name"));
    remove_fixture(program, log);

    let (program, log) = write_fake_podman("container_create_fail", closed_loopback_port());
    let adapter = RootlessPodmanAdapter::new(program.clone());
    assert_eq!(
        adapter.launch_at(&request(), &policy(50), 1_780_000_000),
        Err(ApplicationServiceError::BackendCommandFailed {
            operation: "container_create",
        })
    );
    let calls = fs::read_to_string(&log).expect("container failure calls should be recorded");
    assert!(calls.contains("network rm --force"));
    remove_fixture(program, log);

    let (program, log) = write_fake_podman("container_create_cleanup_fail", closed_loopback_port());
    let adapter = RootlessPodmanAdapter::new(program.clone());
    assert_eq!(
        adapter.launch_at(&request(), &policy(50), 1_780_000_000),
        Err(ApplicationServiceError::CleanupFailed)
    );
    let calls = fs::read_to_string(&log).expect("failed cleanup call should be recorded");
    assert!(calls.contains("network rm --force"));
    remove_fixture(program, log);
}

#[test]
fn start_and_port_failures_stop_or_remove_started_resources() {
    for (mode, expected_operation) in [
        ("start_fail", "container_start"),
        ("port_fail", "port_query"),
    ] {
        let (program, log) = write_fake_podman(mode, closed_loopback_port());
        let adapter = RootlessPodmanAdapter::new(program.clone());
        assert_eq!(
            adapter.launch_at(&request(), &policy(50), 1_780_000_000),
            Err(ApplicationServiceError::BackendCommandFailed {
                operation: expected_operation,
            })
        );
        let calls = fs::read_to_string(&log).expect("cleanup calls should be recorded");
        assert!(calls.contains("rm --force"));
        assert!(calls.contains("network rm --force"));
        if mode == "port_fail" {
            assert!(calls.contains("stop --time 2"));
        }
        remove_fixture(program, log);
    }
}

#[test]
fn malformed_port_mappings_fail_closed_after_cleanup() {
    for mode in [
        "invalid_port_host",
        "invalid_port_text",
        "invalid_port_zero",
    ] {
        let (program, log) = write_fake_podman(mode, closed_loopback_port());
        let adapter = RootlessPodmanAdapter::new(program.clone());
        assert_eq!(
            adapter.launch_at(&request(), &policy(50), 1_780_000_000),
            Err(ApplicationServiceError::InvalidPortMapping)
        );
        let calls = fs::read_to_string(&log).expect("port cleanup should be recorded");
        assert!(calls.contains("stop --time 2"));
        assert!(calls.contains("rm --force"));
        assert!(calls.contains("network rm --force"));
        remove_fixture(program, log);
    }
}

#[test]
fn readiness_timeout_fails_closed_and_removes_created_isolation_resources() {
    let unavailable_port = closed_loopback_port();
    let (program, log) = write_fake_podman("success", unavailable_port);
    let adapter = RootlessPodmanAdapter::new(program.clone());

    assert_eq!(
        adapter.launch_at(&request(), &policy(30), 1_780_000_000),
        Err(ApplicationServiceError::ReadinessTimeout)
    );

    let calls = fs::read_to_string(&log).expect("cleanup calls should be recorded");
    assert!(calls.contains("stop --time 2"));
    assert!(calls.contains("rm --force"));
    assert!(calls.contains("network rm --force"));
    remove_fixture(program, log);
}

#[test]
fn cleanup_failure_is_never_hidden_by_readiness_or_termination_results() {
    let unavailable_port = closed_loopback_port();
    let (program, log) = write_fake_podman("readiness_cleanup_fail", unavailable_port);
    let adapter = RootlessPodmanAdapter::new(program.clone());
    assert_eq!(
        adapter.launch_at(&request(), &policy(30), 1_780_000_000),
        Err(ApplicationServiceError::CleanupFailed)
    );
    remove_fixture(program, log);

    let listener = TcpListener::bind(("127.0.0.1", 0)).expect("loopback listener should bind");
    let ready_port = listener
        .local_addr()
        .expect("address should resolve")
        .port();
    let (program, log) = write_fake_podman("termination_cleanup_fail", ready_port);
    let adapter = RootlessPodmanAdapter::new(program.clone());
    let lease = adapter
        .launch_at(&request(), &policy(100), 1_780_000_000)
        .expect("launch should succeed before termination failure");
    assert_eq!(
        adapter.terminate_at(&lease, 1_780_000_001),
        Err(ApplicationServiceError::CleanupFailed)
    );
    let calls = fs::read_to_string(&log).expect("all cleanup attempts should be recorded");
    assert!(calls.contains("stop --time 2"));
    assert!(calls.contains("rm --force"));
    assert!(calls.contains("network rm --force"));
    remove_fixture(program, log);
    drop(listener);
}
