//! AppArmor process-label normalization regression for effective isolation.

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

fn policy() -> IsolationPolicy {
    IsolationPolicy {
        policy_id: "apparmor_mode_suffix_policy".to_owned(),
        maximum_memory_bytes: 128 * 1024 * 1024,
        maximum_cpu_millicores: 1_000,
        maximum_processes: 32,
        maximum_lease_seconds: 60,
        maximum_tmpfs_bytes: 16 * 1024 * 1024,
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
        request_id: "apparmor-mode-suffix".to_owned(),
        image_reference: format!("localhost/cwl/tool@sha256:{}", "d".repeat(64)),
        container_port: 8_080,
        protocol: ServiceProtocol::Http,
        command: vec!["serve".to_owned()],
        resources: ResourceRequest {
            memory_bytes: 64 * 1024 * 1024,
            cpu_millicores: 500,
            maximum_processes: 16,
            lease_seconds: 30,
            tmpfs_bytes: 8 * 1024 * 1024,
        },
    }
}

fn write_fake_podman(ready_port: u16) -> PathBuf {
    let program = temporary_path("apparmor-mode-suffix-podman");
    let script = format!(
        "#!/bin/sh\nset -eu\ncase \"${{1:-}}:${{2:-}}\" in\n  info:--format) printf '%s\\n' '{{\"host\":{{\"security\":{{\"rootless\":true,\"seccompEnabled\":true,\"seccompProfilePath\":\"/usr/share/containers/seccomp.json\",\"apparmorEnabled\":true,\"selinuxEnabled\":false}}}},\"version\":{{\"Version\":\"5.8.4\"}}}}' ;;\n  network:create) : ;;\n  create:--name) printf 'fake-container-id\\n' ;;\n  start:*) : ;;\n  container:inspect) printf '%s\\n' '[{{\"Id\":\"fake-container-id\",\"AppArmorProfile\":\"containers-default\",\"ProcessLabel\":\"\",\"EffectiveCaps\":[],\"BoundingCaps\":[],\"Config\":{{\"User\":\"65532:65532\"}},\"HostConfig\":{{\"ReadonlyRootfs\":true,\"Privileged\":false,\"SecurityOpt\":[\"no-new-privileges\"],\"UsernsMode\":\"auto\",\"PidMode\":\"private\",\"IpcMode\":\"none\",\"Memory\":67108864,\"NanoCpus\":500000000,\"PidsLimit\":16}}}}]' ;;\n  top:*) printf 'PID SECCOMP CAPEFF CAPBND CAPINH CAPPRM CAPAMB LABEL\\n1 filter - - - - - containers-default (enforce)\\n' ;;\n  network:inspect) printf '%s\\n' '[{{\"internal\":true,\"dns_enabled\":false}}]' ;;\n  port:*) printf '127.0.0.1:{ready_port}\\n' ;;\n  stop:*) : ;;\n  rm:*) : ;;\n  network:rm) : ;;\n  *) exit 91 ;;\nesac\n"
    );
    fs::write(&program, script).expect("fake Podman should be writable");
    let mut permissions = fs::metadata(&program)
        .expect("fake Podman metadata should exist")
        .permissions();
    permissions.set_mode(0o700);
    fs::set_permissions(&program, permissions).expect("fake Podman should be executable");
    program
}

#[test]
fn confined_apparmor_mode_suffix_is_the_same_effective_profile() {
    let listener = TcpListener::bind(("127.0.0.1", 0)).expect("loopback listener should bind");
    let ready_port = listener
        .local_addr()
        .expect("listener address should resolve")
        .port();
    let program = write_fake_podman(ready_port);
    let adapter = RootlessPodmanAdapter::new(program.clone());

    let result = adapter.launch_at(&request(), &policy(), 1_780_000_000);

    let _ = fs::remove_file(program);
    drop(listener);
    assert!(
        result.is_ok(),
        "a confined AppArmor process label may append its enforcement mode while referring to the inspect-reported profile"
    );
}
