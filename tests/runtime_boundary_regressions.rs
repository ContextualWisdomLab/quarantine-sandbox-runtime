//! Regression tests for subtle runtime-boundary behavior exposed by exact coverage gates.

#![cfg(target_os = "linux")]

use std::{
    fs,
    net::TcpListener,
    os::unix::fs::PermissionsExt,
    path::PathBuf,
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use quarantine_sandbox_runtime::{
    ApplicationServiceError, ApplicationServiceRequest, IsolationPolicy, ResourceRequest,
    RootlessPodmanAdapter, ServiceProtocol,
};

fn policy() -> IsolationPolicy {
    IsolationPolicy {
        policy_id: "runtime_boundary_regression_v1".to_owned(),
        maximum_memory_bytes: 512 * 1024 * 1024,
        maximum_cpu_millicores: 2_000,
        maximum_processes: 128,
        maximum_lease_seconds: 900,
        maximum_tmpfs_bytes: 128 * 1024 * 1024,
        readiness_timeout_millis: 50,
        readiness_poll_interval_millis: 10,
        shutdown_grace_seconds: 2,
        run_as_user_id: 65_532,
        run_as_group_id: 65_532,
    }
}

fn request(digest: &str) -> ApplicationServiceRequest {
    ApplicationServiceRequest {
        schema_version: "1.0.0".to_owned(),
        request_id: "runtime_boundary_regression".to_owned(),
        image_reference: format!("localhost/cwl/tool@sha256:{digest}"),
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

fn write_executable(name: &str, script: &str) -> PathBuf {
    let program = temporary_path(name);
    fs::write(&program, script).expect("fake runtime executable should be writable");
    let mut permissions = fs::metadata(&program)
        .expect("fake runtime executable metadata should exist")
        .permissions();
    permissions.set_mode(0o700);
    fs::set_permissions(&program, permissions).expect("fake runtime executable should run");
    program
}

fn security_info_json() -> &'static str {
    r#"{"host":{"security":{"rootless":true,"seccompEnabled":true,"seccompProfilePath":"/usr/share/containers/seccomp.json","apparmorEnabled":true,"selinuxEnabled":false}},"version":{"Version":"5.8.4"}}"#
}

#[test]
fn digest_pinned_image_accepts_numeric_sha256() {
    let candidate = request(&"0".repeat(64));
    assert_eq!(candidate.validate(&policy()), Ok(()));
}

#[test]
fn slow_successful_backend_command_is_polled_until_exit() {
    let script = format!(
        "#!/bin/sh\nset -eu\ncase \"${{1:-}}:${{2:-}}\" in\n  info:*) sleep 0.03; printf '%s\\n' '{}' ;;\n  network:create) exit 21 ;;\n  *) exit 91 ;;\nesac\n",
        security_info_json()
    );
    let program = write_executable("slow-podman", &script);
    let adapter = RootlessPodmanAdapter::new(program.clone())
        .with_command_timeout(Duration::from_millis(200));

    assert_eq!(
        adapter.launch_at(&request(&"a".repeat(64)), &policy(), 1_780_000_000),
        Err(ApplicationServiceError::BackendCommandFailed {
            operation: "network_create",
        })
    );

    let _ = fs::remove_file(program);
}

#[test]
fn non_utf8_port_output_fails_closed_and_cleans_every_created_resource() {
    let log = temporary_path("non-utf8-podman-log");
    let script = format!(
        "#!/bin/sh\nset -eu\nprintf '%s\\n' \"$*\" >> '{}'\ncase \"${{1:-}}:${{2:-}}\" in\n  info:*) printf '%s\\n' '{}' ;;\n  network:create) : ;;\n  network:inspect) printf '%s\\n' '[{{\"internal\":true,\"dns_enabled\":false}}]' ;;\n  network:rm) : ;;\n  create:*) printf 'fake-container-id\\n' ;;\n  start:*) : ;;\n  container:inspect) printf '%s\\n' '[{{\"Id\":\"fake-container-id\",\"AppArmorProfile\":\"containers-default\",\"ProcessLabel\":\"\",\"EffectiveCaps\":[],\"BoundingCaps\":[],\"Config\":{{\"User\":\"65532:65532\"}},\"HostConfig\":{{\"ReadonlyRootfs\":true,\"Privileged\":false,\"SecurityOpt\":[\"no-new-privileges\"],\"UsernsMode\":\"auto\",\"PidMode\":\"private\",\"IpcMode\":\"none\",\"Memory\":268435456,\"NanoCpus\":1000000000,\"PidsLimit\":32}}}}]' ;;\n  top:*) printf 'PID SECCOMP CAPEFF CAPBND CAPINH CAPPRM CAPAMB LABEL\\n1 filter - - - - - containers-default (enforce)\\n' ;;\n  port:*) printf '\\377' ;;\n  stop:*) : ;;\n  rm:*) : ;;\n  *) exit 91 ;;\nesac\n",
        log.display(),
        security_info_json()
    );
    let program = write_executable("non-utf8-podman", &script);
    let adapter = RootlessPodmanAdapter::new(program.clone());

    assert_eq!(
        adapter.launch_at(&request(&"a".repeat(64)), &policy(), 1_780_000_000),
        Err(ApplicationServiceError::InvalidPortMapping)
    );

    let calls = fs::read_to_string(&log).expect("cleanup calls should be recorded");
    assert!(calls.contains("container inspect --format json"));
    assert!(calls.contains("network inspect --format json"));
    assert!(calls.contains("stop --time 2"));
    assert!(calls.contains("rm --force"));
    assert!(calls.contains("network rm --force"));

    let _ = fs::remove_file(program);
    let _ = fs::remove_file(log);
}

#[test]
fn cleanup_command_output_overflow_fails_closed_without_skipping_other_cleanup() {
    let listener = TcpListener::bind(("127.0.0.1", 0)).expect("loopback listener should bind");
    let ready_port = listener
        .local_addr()
        .expect("listener should expose its address")
        .port();
    let log = temporary_path("cleanup-output-limit-log");
    let script = format!(
        "#!/bin/sh\nset -eu\nprintf '%s\\n' \"$*\" >> '{}'\ncase \"${{1:-}}:${{2:-}}\" in\n  info:*) printf '%s\\n' '{}' ;;\n  network:create) : ;;\n  network:inspect) printf '%s\\n' '[{{\"internal\":true,\"dns_enabled\":false}}]' ;;\n  network:rm) : ;;\n  create:*) printf 'fake-container-id\\n' ;;\n  start:*) : ;;\n  container:inspect) printf '%s\\n' '[{{\"Id\":\"fake-container-id\",\"AppArmorProfile\":\"containers-default\",\"ProcessLabel\":\"\",\"EffectiveCaps\":[],\"BoundingCaps\":[],\"Config\":{{\"User\":\"65532:65532\"}},\"HostConfig\":{{\"ReadonlyRootfs\":true,\"Privileged\":false,\"SecurityOpt\":[\"no-new-privileges\"],\"UsernsMode\":\"auto\",\"PidMode\":\"private\",\"IpcMode\":\"none\",\"Memory\":268435456,\"NanoCpus\":1000000000,\"PidsLimit\":32}}}}]' ;;\n  top:*) printf 'PID SECCOMP CAPEFF CAPBND CAPINH CAPPRM CAPAMB LABEL\\n1 filter - - - - - containers-default (enforce)\\n' ;;\n  port:*) printf '127.0.0.1:{ready_port}\\n' ;;\n  stop:*) i=0; while [ \"$i\" -lt 1024 ]; do printf x; i=$((i + 1)); done ;;\n  rm:*) : ;;\n  *) exit 91 ;;\nesac\n",
        log.display(),
        security_info_json()
    );
    let program = write_executable("cleanup-output-limit-podman", &script);
    let adapter = RootlessPodmanAdapter::new(program.clone())
        .with_command_output_limit_bytes(512)
        .with_command_timeout(Duration::from_secs(1));
    let lease = adapter
        .launch_at(&request(&"a".repeat(64)), &policy(), 1_780_000_000)
        .expect("launch should succeed before bounded cleanup failure");

    assert_eq!(
        adapter.terminate_at(&lease, 1_780_000_001),
        Err(ApplicationServiceError::CleanupFailed)
    );
    let calls = fs::read_to_string(&log).expect("all cleanup calls should be recorded");
    assert!(calls.contains("stop --time 2"));
    assert!(calls.contains("rm --force"));
    assert!(calls.contains("network rm --force"));

    let _ = fs::remove_file(program);
    let _ = fs::remove_file(log);
    drop(listener);
}
