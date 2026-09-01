//! Regression test that idempotency cannot replay a lease under a changed isolation policy.

#![cfg(target_os = "linux")]

use std::{
    fs,
    net::TcpListener,
    os::unix::fs::PermissionsExt,
    path::PathBuf,
    time::{SystemTime, UNIX_EPOCH},
};

use quarantine_sandbox_runtime::{
    ApplicationServiceCoordinator, ApplicationServiceCoordinatorError, ApplicationServiceRequest,
    IsolationPolicy, LeaseOwnerId, ResourceRequest, RootlessPodmanAdapter, ServiceProtocol,
};

fn temporary_path(name: &str) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock should be after Unix epoch")
        .as_nanos();
    std::env::temp_dir().join(format!(
        "qsr-policy-idempotency-{name}-{}-{nanos}",
        std::process::id()
    ))
}

fn policy() -> IsolationPolicy {
    IsolationPolicy {
        policy_id: "agent_application_policy_v1".to_owned(),
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
        request_id: "same_task_same_request".to_owned(),
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

fn write_fake_podman(program: &PathBuf, ready_port: u16) {
    let info = r#"{"host":{"security":{"rootless":true,"seccompEnabled":true,"seccompProfilePath":"/usr/share/containers/seccomp.json","apparmorEnabled":true,"selinuxEnabled":false}},"version":{"Version":"5.6.2"}}"#;
    let container = r#"[{"Id":"fake-container-id","AppArmorProfile":"containers-default","ProcessLabel":"","EffectiveCaps":[],"Config":{"User":"65532:65532"},"HostConfig":{"ReadonlyRootfs":true,"Privileged":false,"SecurityOpt":["no-new-privileges"],"UsernsMode":"auto","PidMode":"private","IpcMode":"none","Memory":268435456,"NanoCpus":1000000000,"PidsLimit":32}}]"#;
    let network = r#"[{"internal":true,"dns_enabled":false}]"#;
    let script = format!(
        "#!/bin/sh\nset -eu\ncase \"${{1:-}}:${{2:-}}\" in\n  info:--format) printf '%s\\n' '{}' ;;\n  network:create) : ;;\n  network:inspect) printf '%s\\n' '{}' ;;\n  network:rm) : ;;\n  container:inspect) printf '%s\\n' '{}' ;;\n  create:--name) printf 'fake-container-id\\n' ;;\n  start:*) : ;;\n  port:*) printf '127.0.0.1:{ready_port}\\n' ;;\n  stop:*) : ;;\n  rm:*) : ;;\n  *) exit 91 ;;\nesac\n",
        info, network, container,
    );
    fs::write(program, script).expect("fake Podman should be writable");
    let mut permissions = fs::metadata(program)
        .expect("fake Podman metadata should exist")
        .permissions();
    permissions.set_mode(0o700);
    fs::set_permissions(program, permissions).expect("fake Podman should be executable");
}

#[test]
fn identical_request_does_not_reuse_lease_when_effective_policy_changes() {
    let listener = TcpListener::bind(("127.0.0.1", 0)).expect("loopback listener should bind");
    let ready_port = listener.local_addr().expect("address should resolve").port();
    let program = temporary_path("fake-podman");
    write_fake_podman(&program, ready_port);
    let coordinator = ApplicationServiceCoordinator::new(RootlessPodmanAdapter::new(program.clone()));
    let owner = LeaseOwnerId::new("urn:cwl:agent:contextual-orchestrator")
        .expect("opaque owner should validate");
    let initial_policy = policy();
    let lease = coordinator
        .launch_at(&owner, &request(), &initial_policy, 1_780_000_000)
        .expect("first launch should succeed");

    let mut changed_policy = initial_policy;
    changed_policy.maximum_memory_bytes += 1;
    assert_eq!(
        coordinator.launch_at(&owner, &request(), &changed_policy, 1_780_000_001),
        Err(ApplicationServiceCoordinatorError::IdempotencyConflict)
    );

    coordinator
        .terminate_at(&owner, &lease, 1_780_000_010)
        .expect("original lease should still clean up");
    let _ = fs::remove_file(program);
    drop(listener);
}
