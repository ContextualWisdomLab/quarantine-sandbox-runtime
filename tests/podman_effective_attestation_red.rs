//! RED regression: configured Podman argv is not effective isolation evidence.
//!
//! A backend that proves only rootless mode and accepts the requested launch
//! flags must not receive an all-positive application-service attestation. The
//! runtime has to obtain positive effective evidence from the running sandbox
//! before returning a lease.

#![cfg(target_os = "linux")]

use std::{
    fs,
    net::TcpListener,
    os::unix::fs::PermissionsExt,
    path::PathBuf,
    time::{SystemTime, UNIX_EPOCH},
};

use quarantine_sandbox_runtime::{
    ApplicationServiceRequest, IsolationPolicy, ResourceRequest, RootlessPodmanAdapter,
    ServiceProtocol,
};

fn fixture_path(name: &str) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("test clock must be after the Unix epoch")
        .as_nanos();
    std::env::temp_dir().join(format!(
        "qsr-effective-attestation-red-{name}-{}-{nanos}",
        std::process::id()
    ))
}

fn policy() -> IsolationPolicy {
    IsolationPolicy {
        policy_id: "effective_attestation_red_v1".to_owned(),
        maximum_memory_bytes: 256 * 1024 * 1024,
        maximum_cpu_millicores: 500,
        maximum_processes: 32,
        maximum_lease_seconds: 60,
        maximum_tmpfs_bytes: 32 * 1024 * 1024,
        readiness_timeout_millis: 500,
        readiness_poll_interval_millis: 10,
        shutdown_grace_seconds: 1,
        run_as_user_id: 65_532,
        run_as_group_id: 65_532,
    }
}

fn request() -> ApplicationServiceRequest {
    ApplicationServiceRequest {
        schema_version: "1.0.0".to_owned(),
        request_id: "effective_attestation_red".to_owned(),
        image_reference: format!("localhost/cwl/tool@sha256:{}", "a".repeat(64)),
        container_port: 8_080,
        protocol: ServiceProtocol::Http,
        command: vec!["serve".to_owned()],
        resources: ResourceRequest {
            memory_bytes: 128 * 1024 * 1024,
            cpu_millicores: 250,
            maximum_processes: 16,
            lease_seconds: 30,
            tmpfs_bytes: 16 * 1024 * 1024,
        },
    }
}

#[test]
fn configured_flags_without_effective_runtime_evidence_fail_closed() {
    let listener = TcpListener::bind(("127.0.0.1", 0)).expect("loopback listener must bind");
    let ready_port = listener
        .local_addr()
        .expect("listener must expose an address")
        .port();

    let program = fixture_path("fake-podman");
    let log = fixture_path("calls");
    let script = format!(
        "#!/bin/sh\nset -eu\nprintf '%s\\n' \"$*\" >> '{}'\ncase \"${{1:-}}\" in\n  info) printf '%s\\n' '{{\"host\":{{\"security\":{{\"rootless\":true,\"seccompEnabled\":true,\"seccompProfilePath\":\"/usr/share/containers/seccomp.json\",\"apparmorEnabled\":true,\"selinuxEnabled\":false}}}},\"version\":{{\"Version\":\"6.1.0\"}}}}' ;;\n  network) : ;;\n  create) printf 'fake-container-id\\n' ;;\n  start) : ;;\n  port) printf '127.0.0.1:{ready_port}\\n' ;;\n  stop) : ;;\n  rm) : ;;\n  *) exit 91 ;;\nesac\n",
        log.display()
    );
    fs::write(&program, script).expect("fake Podman must be writable");
    let mut permissions = fs::metadata(&program)
        .expect("fake Podman metadata must exist")
        .permissions();
    permissions.set_mode(0o700);
    fs::set_permissions(&program, permissions).expect("fake Podman must be executable");

    let adapter = RootlessPodmanAdapter::new(program.clone());
    let result = adapter.launch_at(&request(), &policy(), 1_780_000_000);

    assert!(
        result.is_err(),
        "configured launch flags and rootless host mode are not positive proof of effective seccomp/LSM/capability/resource/network isolation"
    );

    let calls = fs::read_to_string(&log).expect("fake Podman calls must be recorded");
    assert!(calls.contains("info --format"));
    assert!(calls.contains("create --name"));
    assert!(calls.contains("start "));

    let _ = fs::remove_file(program);
    let _ = fs::remove_file(log);
    drop(listener);
}
