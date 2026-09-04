//! Regression: the intended internal network object is not proof of container attachment.
//!
//! Application-service readiness may be trusted only after the running container is
//! proven to be attached exclusively to the runtime-owned deny-by-default network.
//! A separately inspectable internal network must not satisfy that control when the
//! container reports an effective default/external network mode.

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

fn fixture_path(name: &str) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("test clock must be after the Unix epoch")
        .as_nanos();
    std::env::temp_dir().join(format!(
        "qsr-network-binding-red-{name}-{}-{nanos}",
        std::process::id()
    ))
}

fn policy() -> IsolationPolicy {
    IsolationPolicy {
        policy_id: "network_binding_red_v1".to_owned(),
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
        request_id: "network_binding_red".to_owned(),
        image_reference: format!("localhost/cwl/tool@sha256:{}", "e".repeat(64)),
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
fn container_on_unexpected_network_fails_closed_before_readiness() {
    let listener = TcpListener::bind(("127.0.0.1", 0)).expect("loopback listener must bind");
    let ready_port = listener
        .local_addr()
        .expect("listener must expose an address")
        .port();

    let program = fixture_path("fake-podman");
    let log = fixture_path("calls");
    let info = r#"{"host":{"security":{"rootless":true,"seccompEnabled":true,"seccompProfilePath":"/usr/share/containers/seccomp.json","apparmorEnabled":true,"selinuxEnabled":false}}}"#;
    let container = r#"[{"Id":"fake-container-id","AppArmorProfile":"containers-default","ProcessLabel":"","EffectiveCaps":[],"BoundingCaps":[],"Config":{"User":"65532:65532"},"HostConfig":{"ReadonlyRootfs":true,"Privileged":false,"SecurityOpt":["no-new-privileges"],"UsernsMode":"auto","PidMode":"private","IpcMode":"none","Memory":134217728,"NanoCpus":250000000,"PidsLimit":16,"NetworkMode":"bridge"}}]"#;
    let network = r#"[{"internal":true,"dns_enabled":false}]"#;
    let script = format!(
        "#!/bin/sh\nset -eu\nprintf '%s\\n' \"$*\" >> '{}'\nif [ \"${{1:-}}\" = info ]; then\n  if [ \"${{3:-}}\" = json ]; then printf '%s\\n' '{}'; else printf 'true\\n'; fi\n  exit 0\nfi\ncase \"${{1:-}}:${{2:-}}\" in\n  network:create) : ;;\n  network:inspect) printf '%s\\n' '{}' ;;\n  network:rm) : ;;\n  create:--name) printf 'fake-container-id\\n' ;;\n  container:inspect) printf '%s\\n' '{}' ;;\n  start:*) : ;;\n  top:*) printf 'PID SECCOMP CAPEFF CAPBND CAPINH CAPPRM CAPAMB LABEL\\n1 filter - - - - - containers-default (enforce)\\n' ;;\n  port:*) printf '127.0.0.1:{ready_port}\\n' ;;\n  stop:*) : ;;\n  rm:*) : ;;\n  *) exit 91 ;;\nesac\n",
        log.display(),
        info,
        network,
        container,
    );
    fs::write(&program, script).expect("fake Podman must be writable");
    let mut permissions = fs::metadata(&program)
        .expect("fake Podman metadata must exist")
        .permissions();
    permissions.set_mode(0o700);
    fs::set_permissions(&program, permissions).expect("fake Podman must be executable");

    let adapter = RootlessPodmanAdapter::new(program.clone());
    assert_eq!(
        adapter.launch_at(&request(), &policy(), 1_780_000_000),
        Err(ApplicationServiceError::IsolationVerificationFailed {
            control_name: "sandbox_network_binding",
        }),
        "an internal network object must not prove that the running container is attached to it"
    );

    let calls = fs::read_to_string(&log).expect("fake Podman calls must be recorded");
    for expected in [
        "network create --internal --disable-dns qsr-net-",
        "container inspect --format json qsr-app-",
        "network inspect --format json qsr-net-",
        "stop --time 1 qsr-app-",
        "rm --force qsr-app-",
        "network rm --force qsr-net-",
    ] {
        assert!(calls.contains(expected), "missing call {expected}: {calls}");
    }
    assert!(
        !calls.contains("port "),
        "port/readiness must not be queried after effective network attachment mismatch: {calls}"
    );

    let _ = fs::remove_file(program);
    let _ = fs::remove_file(log);
    drop(listener);
}
