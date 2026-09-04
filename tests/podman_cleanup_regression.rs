//! Cleanup regressions preserved while restacking effective-isolation work.

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

fn policy(timeout_millis: u64) -> IsolationPolicy {
    IsolationPolicy {
        policy_id: "cleanup_regression_policy_v1".to_owned(),
        maximum_memory_bytes: 512 * 1024 * 1024,
        maximum_cpu_millicores: 2_000,
        maximum_processes: 128,
        maximum_lease_seconds: 900,
        maximum_tmpfs_bytes: 128 * 1024 * 1024,
        readiness_timeout_millis: timeout_millis,
        readiness_poll_interval_millis: 10,
        shutdown_grace_seconds: 2,
        run_as_user_id: 65_532,
        run_as_group_id: 65_532,
    }
}

fn request() -> ApplicationServiceRequest {
    ApplicationServiceRequest {
        schema_version: "1.0.0".to_owned(),
        request_id: "cleanup_regression_request".to_owned(),
        image_reference: format!("localhost/cwl/tool@sha256:{}", "e".repeat(64)),
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

fn closed_loopback_port() -> u16 {
    let listener = TcpListener::bind(("127.0.0.1", 0)).expect("loopback port should bind");
    listener
        .local_addr()
        .expect("address should resolve")
        .port()
}

fn write_fake_podman(mode: &str, ready_port: u16) -> (PathBuf, PathBuf) {
    let program = temporary_path("cleanup-regression-podman");
    let log = temporary_path("cleanup-regression-log");
    let info = r#"{"host":{"security":{"rootless":true,"seccompEnabled":true,"seccompProfilePath":"/usr/share/containers/seccomp.json","apparmorEnabled":true,"selinuxEnabled":false}},"version":{"Version":"5.6.2"}}"#;
    let container = r#"[{"Id":"fake-container-id","AppArmorProfile":"containers-default","ProcessLabel":"","EffectiveCaps":[],"BoundingCaps":[],"Config":{"User":"65532:65532"},"HostConfig":{"ReadonlyRootfs":true,"Privileged":false,"SecurityOpt":["no-new-privileges"],"UsernsMode":"auto","PidMode":"private","IpcMode":"none","Memory":268435456,"NanoCpus":1000000000,"PidsLimit":32}}]"#;
    let network = r#"[{"internal":true,"dns_enabled":false}]"#;
    let script = format!(
        "#!/bin/sh\nset -eu\nMODE='{mode}'\nprintf '%s\\n' \"$*\" >> '{}'\ncase \"$MODE:${{1:-}}:${{2:-}}\" in\n  start_cleanup_fail:start:*) exit 24 ;;\n  start_cleanup_fail:rm:*) exit 28 ;;\n  start_network_cleanup_fail:start:*) exit 24 ;;\n  start_network_cleanup_fail:network:rm) exit 29 ;;\n  port_stop_cleanup_fail:port:*) exit 25 ;;\n  port_stop_cleanup_fail:stop:*) exit 30 ;;\n  port_network_cleanup_fail:port:*) exit 25 ;;\n  port_network_cleanup_fail:network:rm) exit 31 ;;\n  termination_stop_fail:stop:*) exit 27 ;;\n  termination_remove_fail:rm:*) exit 32 ;;\n  termination_network_fail:network:rm) exit 33 ;;\nesac\ncase \"${{1:-}}:${{2:-}}\" in\n  info:--format) printf '%s\\n' '{}' ;;\n  network:create) : ;;\n  network:inspect) printf '%s\\n' '{}' ;;\n  network:rm) : ;;\n  container:inspect) printf '%s\\n' '{}' ;;\n  create:--name) printf 'fake-container-id\\n' ;;\n  start:*) : ;;\n  top:*) printf 'PID SECCOMP CAPEFF CAPBND CAPINH CAPPRM CAPAMB LABEL\\n1 filter - - - - - containers-default (enforce)\\n' ;;\n  port:*) printf '127.0.0.1:{ready_port}\\n' ;;\n  stop:*) : ;;\n  rm:*) : ;;\n  *) exit 91 ;;\nesac\n",
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

fn remove_fixture(program: PathBuf, log: PathBuf) {
    let _ = fs::remove_file(program);
    let _ = fs::remove_file(log);
}

#[test]
fn partial_launch_cleanup_failures_override_original_backend_error() {
    for mode in ["start_cleanup_fail", "start_network_cleanup_fail"] {
        let (program, log) = write_fake_podman(mode, closed_loopback_port());
        let adapter = RootlessPodmanAdapter::new(program.clone());
        assert_eq!(
            adapter.launch_at(&request(), &policy(50), 1_780_000_000),
            Err(ApplicationServiceError::CleanupFailed)
        );
        let calls = fs::read_to_string(&log).expect("cleanup calls should be recorded");
        assert!(calls.contains("rm --force"));
        assert!(calls.contains("network rm --force"));
        remove_fixture(program, log);
    }
}

#[test]
fn started_cleanup_attempts_every_resource_after_port_failure() {
    for mode in ["port_stop_cleanup_fail", "port_network_cleanup_fail"] {
        let (program, log) = write_fake_podman(mode, closed_loopback_port());
        let adapter = RootlessPodmanAdapter::new(program.clone());
        assert_eq!(
            adapter.launch_at(&request(), &policy(50), 1_780_000_000),
            Err(ApplicationServiceError::CleanupFailed)
        );
        let calls = fs::read_to_string(&log).expect("cleanup calls should be recorded");
        assert!(calls.contains("stop --time 2"));
        assert!(calls.contains("rm --force"));
        assert!(calls.contains("network rm --force"));
        remove_fixture(program, log);
    }
}

#[test]
fn termination_attempts_all_cleanup_resources_when_any_step_fails() {
    for mode in [
        "termination_stop_fail",
        "termination_remove_fail",
        "termination_network_fail",
    ] {
        let listener = TcpListener::bind(("127.0.0.1", 0)).expect("loopback listener should bind");
        let ready_port = listener
            .local_addr()
            .expect("address should resolve")
            .port();
        let (program, log) = write_fake_podman(mode, ready_port);
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
}
