//! Protocol-level HTTP readiness tests for the application-service boundary.

#![cfg(target_os = "linux")]

use std::{
    fs,
    io::{Read, Write},
    net::TcpListener,
    os::unix::fs::PermissionsExt,
    path::PathBuf,
    sync::atomic::{AtomicU64, Ordering},
    thread,
    time::{SystemTime, UNIX_EPOCH},
};

use quarantine_sandbox_runtime::{
    ApplicationServiceError, ApplicationServiceRequest, IsolationPolicy, ResourceRequest,
    RootlessPodmanAdapter, ServiceProtocol,
};

static NEXT_TEMP_PATH_ID: AtomicU64 = AtomicU64::new(0);

fn temporary_path(name: &str) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock should be after the Unix epoch")
        .as_nanos();
    let unique_id = NEXT_TEMP_PATH_ID.fetch_add(1, Ordering::Relaxed);
    std::env::temp_dir().join(format!(
        "quarantine-sandbox-runtime-http-readiness-{name}-{}-{nanos}-{unique_id}",
        std::process::id()
    ))
}

fn digest_image() -> String {
    format!("localhost/cwl/caido@sha256:{}", "b".repeat(64))
}

fn policy() -> IsolationPolicy {
    IsolationPolicy {
        policy_id: "http_readiness_policy_v1".to_owned(),
        maximum_memory_bytes: 512 * 1024 * 1024,
        maximum_cpu_millicores: 2_000,
        maximum_processes: 128,
        maximum_lease_seconds: 900,
        maximum_tmpfs_bytes: 128 * 1024 * 1024,
        readiness_timeout_millis: 80,
        readiness_poll_interval_millis: 10,
        shutdown_grace_seconds: 2,
        run_as_user_id: 65_532,
        run_as_group_id: 65_532,
    }
}

fn request() -> ApplicationServiceRequest {
    ApplicationServiceRequest {
        schema_version: "1.0.0".to_owned(),
        request_id: "http_readiness_request".to_owned(),
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

fn write_fake_podman(ready_port: u16) -> (PathBuf, PathBuf) {
    let program = temporary_path("fake-podman");
    let log = temporary_path("fake-podman-log");
    let info = r#"{"host":{"security":{"rootless":true,"seccompEnabled":true,"seccompProfilePath":"/usr/share/containers/seccomp.json","apparmorEnabled":true,"selinuxEnabled":false}}}"#;
    let container = r#"[{"Id":"fake-container-id","AppArmorProfile":"containers-default","ProcessLabel":"","EffectiveCaps":[],"BoundingCaps":[],"Config":{"User":"65532:65532"},"HostConfig":{"ReadonlyRootfs":true,"Privileged":false,"SecurityOpt":["no-new-privileges"],"UsernsMode":"auto","PidMode":"private","IpcMode":"none","Memory":268435456,"NanoCpus":1000000000,"PidsLimit":32}}]"#;
    let network = r#"[{"internal":true,"dns_enabled":false}]"#;
    let script = format!(
        "#!/bin/sh\nset -eu\nprintf '%s\\n' \"$*\" >> '{}'\nif [ \"${{1:-}}\" = info ]; then\n  if [ \"${{3:-}}\" = json ]; then printf '%s\\n' '{}'; else printf 'true\\n'; fi\n  exit 0\nfi\ncase \"${{1:-}}:${{2:-}}\" in\n  network:create) : ;;\n  network:inspect) printf '%s\\n' '{}' ;;\n  network:rm) : ;;\n  container:inspect) printf '%s\\n' '{}' ;;\n  create:--name) printf 'fake-container-id\\n' ;;\n  start:*) : ;;\n  top:*) printf 'PID SECCOMP CAPEFF CAPBND CAPINH CAPPRM CAPAMB LABEL\\n1 filter - - - - - containers-default (enforce)\\n' ;;\n  port:*) printf '127.0.0.1:{ready_port}\\n' ;;\n  stop:*) : ;;\n  rm:*) : ;;\n  *) exit 91 ;;\nesac\n",
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

fn spawn_http_response(listener: TcpListener, response: &'static [u8]) -> thread::JoinHandle<()> {
    thread::spawn(move || {
        let (mut stream, _) = listener.accept().expect("readiness probe should connect");
        stream
            .write_all(response)
            .expect("HTTP response should be writable");
    })
}

fn spawn_http_response_capturing_request(
    listener: TcpListener,
    response: &'static [u8],
) -> thread::JoinHandle<Vec<u8>> {
    thread::spawn(move || {
        let (mut stream, _) = listener.accept().expect("readiness probe should connect");
        let mut request = Vec::new();
        let mut chunk = [0_u8; 128];
        while !request.ends_with(b"\r\n\r\n") {
            let bytes_read = stream
                .read(&mut chunk)
                .expect("HTTP readiness request should be readable");
            if bytes_read == 0 {
                break;
            }
            request.extend_from_slice(&chunk[..bytes_read]);
            assert!(
                request.len() <= 1_024,
                "HTTP readiness request must stay bounded"
            );
        }
        stream
            .write_all(response)
            .expect("HTTP response should be writable");
        request
    })
}

fn assert_malformed_http_status_is_not_ready(response: &'static [u8]) {
    let listener = TcpListener::bind(("127.0.0.1", 0)).expect("loopback listener should bind");
    let ready_port = listener
        .local_addr()
        .expect("listener should expose its address")
        .port();
    let responder = spawn_http_response(listener, response);
    let (program, log) = write_fake_podman(ready_port);
    let adapter = RootlessPodmanAdapter::new(program.clone());

    assert_eq!(
        adapter.launch_at(&request(), &policy(), 1_780_000_000),
        Err(ApplicationServiceError::ReadinessTimeout)
    );
    responder.join().expect("HTTP responder should finish");
    remove_fixture(program, log);
}

#[test]
fn http_service_does_not_become_ready_from_tcp_acceptance_alone() {
    let listener = TcpListener::bind(("127.0.0.1", 0)).expect("loopback listener should bind");
    let ready_port = listener
        .local_addr()
        .expect("listener should expose its address")
        .port();
    let (program, log) = write_fake_podman(ready_port);
    let adapter = RootlessPodmanAdapter::new(program.clone());

    let result = adapter.launch_at(&request(), &policy(), 1_780_000_000);
    let calls = fs::read_to_string(&log).expect("fake Podman calls should be recorded");
    remove_fixture(program, log);
    drop(listener);

    assert_eq!(result, Err(ApplicationServiceError::ReadinessTimeout));
    assert!(calls.contains("stop --time 2"));
    assert!(calls.contains("rm --force"));
    assert!(calls.contains("network rm --force"));
}

#[test]
fn http_service_becomes_ready_after_a_bounded_success_response() {
    let listener = TcpListener::bind(("127.0.0.1", 0)).expect("loopback listener should bind");
    let ready_port = listener
        .local_addr()
        .expect("listener should expose its address")
        .port();
    let responder = spawn_http_response(
        listener,
        b"HTTP/1.1 204 No Content\r\nContent-Length: 0\r\nConnection: close\r\n\r\n",
    );
    let (program, log) = write_fake_podman(ready_port);
    let adapter = RootlessPodmanAdapter::new(program.clone());

    let lease = adapter
        .launch_at(&request(), &policy(), 1_780_000_000)
        .expect("valid HTTP response should establish protocol readiness");
    responder.join().expect("HTTP responder should finish");
    assert_eq!(lease.endpoint().protocol(), ServiceProtocol::Http);
    assert_eq!(lease.endpoint().host(), "127.0.0.1");
    assert_eq!(lease.endpoint().port(), ready_port);

    adapter
        .terminate_at(&lease, 1_780_000_001)
        .expect("successful HTTP lease should remain cleanable");
    remove_fixture(program, log);
}

#[test]
fn http_readiness_request_uses_the_selected_loopback_authority() {
    let listener = TcpListener::bind(("127.0.0.1", 0)).expect("loopback listener should bind");
    let ready_port = listener
        .local_addr()
        .expect("listener should expose its address")
        .port();
    let responder = spawn_http_response_capturing_request(
        listener,
        b"HTTP/1.1 204 No Content\r\nContent-Length: 0\r\nConnection: close\r\n\r\n",
    );
    let (program, log) = write_fake_podman(ready_port);
    let adapter = RootlessPodmanAdapter::new(program.clone());

    let lease = adapter
        .launch_at(&request(), &policy(), 1_780_000_000)
        .expect("valid HTTP response should establish protocol readiness");
    let captured_request = responder.join().expect("HTTP responder should finish");
    let captured_request = std::str::from_utf8(&captured_request)
        .expect("HTTP readiness request should use ASCII header syntax");
    assert!(captured_request.contains(&format!("\r\nHost: 127.0.0.1:{ready_port}\r\n")));

    adapter
        .terminate_at(&lease, 1_780_000_001)
        .expect("successful HTTP lease should remain cleanable");
    remove_fixture(program, log);
}

#[test]
fn malformed_http_protocol_prefix_is_not_readiness() {
    assert_malformed_http_status_is_not_ready(
        b"NOTP/1.1 204 Invalid\r\nContent-Length: 0\r\nConnection: close\r\n\r\n",
    );
}

#[test]
fn malformed_http_status_with_invalid_second_digit_is_not_readiness() {
    assert_malformed_http_status_is_not_ready(
        b"HTTP/1.1 2x0 Invalid\r\nContent-Length: 0\r\nConnection: close\r\n\r\n",
    );
}

#[test]
fn malformed_http_status_with_invalid_third_digit_is_not_readiness() {
    assert_malformed_http_status_is_not_ready(
        b"HTTP/1.1 20x Invalid\r\nContent-Length: 0\r\nConnection: close\r\n\r\n",
    );
}

#[test]
fn http_server_error_is_not_readiness() {
    let listener = TcpListener::bind(("127.0.0.1", 0)).expect("loopback listener should bind");
    let ready_port = listener
        .local_addr()
        .expect("listener should expose its address")
        .port();
    let responder = spawn_http_response(
        listener,
        b"HTTP/1.1 503 Service Unavailable\r\nContent-Length: 0\r\nConnection: close\r\n\r\n",
    );
    let (program, log) = write_fake_podman(ready_port);
    let adapter = RootlessPodmanAdapter::new(program.clone());

    assert_eq!(
        adapter.launch_at(&request(), &policy(), 1_780_000_000),
        Err(ApplicationServiceError::ReadinessTimeout)
    );
    responder.join().expect("HTTP responder should finish");
    let calls = fs::read_to_string(&log).expect("cleanup calls should be recorded");
    assert!(calls.contains("stop --time 2"));
    assert!(calls.contains("rm --force"));
    assert!(calls.contains("network rm --force"));
    remove_fixture(program, log);
}