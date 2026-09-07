//! Focused coverage for reachable application-service and effective-LSM edge paths.
//!
//! These tests exercise public behavior through the application-service ACL. The fake
//! Podman executable is process-backed parser/error evidence only; it does not replace
//! the dedicated real rootless/positive-LSM release lanes.

#![cfg(target_os = "linux")]

use std::{
    fs,
    os::unix::fs::PermissionsExt,
    path::PathBuf,
    sync::atomic::{AtomicU64, Ordering},
    time::{SystemTime, UNIX_EPOCH},
};

use quarantine_sandbox_runtime::{
    ApplicationServiceError, ApplicationServiceRequest, IsolationPolicy, ResourceRequest,
    RootlessPodmanAdapter, ServiceProtocol,
};

static NEXT_TEMP_PATH_ID: AtomicU64 = AtomicU64::new(0);

fn digest() -> String {
    "b".repeat(64)
}

fn policy() -> IsolationPolicy {
    IsolationPolicy {
        policy_id: "root_coverage_reachable_edges_v1".to_owned(),
        maximum_memory_bytes: 512 * 1024 * 1024,
        maximum_cpu_millicores: 2_000,
        maximum_processes: 128,
        maximum_lease_seconds: 900,
        maximum_tmpfs_bytes: 128 * 1024 * 1024,
        readiness_timeout_millis: 25,
        readiness_poll_interval_millis: 5,
        shutdown_grace_seconds: 2,
        run_as_user_id: 65_532,
        run_as_group_id: 65_532,
    }
}

fn request(image_reference: String) -> ApplicationServiceRequest {
    ApplicationServiceRequest {
        schema_version: "1.0.0".to_owned(),
        request_id: "root_coverage_reachable_edges_request".to_owned(),
        image_reference,
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
    let unique_id = NEXT_TEMP_PATH_ID.fetch_add(1, Ordering::Relaxed);
    std::env::temp_dir().join(format!(
        "quarantine-sandbox-runtime-{name}-{}-{nanos}-{unique_id}",
        std::process::id()
    ))
}

fn fake_selinux_unconfined_podman() -> PathBuf {
    let program = temporary_path("root-coverage-selinux-unconfined-podman");
    let info = r#"{"host":{"security":{"rootless":true,"seccompEnabled":true,"seccompProfilePath":"/usr/share/containers/seccomp.json","apparmorEnabled":false,"selinuxEnabled":true}}}"#;
    let container = r#"[{"Id":"fake-container-id","AppArmorProfile":"","ProcessLabel":"unconfined","EffectiveCaps":[],"BoundingCaps":[],"Config":{"User":"65532:65532"},"HostConfig":{"ReadonlyRootfs":true,"Privileged":false,"SecurityOpt":["no-new-privileges"],"UsernsMode":"auto","PidMode":"private","IpcMode":"none","Memory":268435456,"NanoCpus":1000000000,"PidsLimit":32}}]"#;
    let network = r#"[{"internal":true,"dns_enabled":false}]"#;
    let process_top = "PID SECCOMP CAPEFF CAPBND CAPINH CAPPRM CAPAMB LABEL\\n1 filter - - - - - unconfined\\n";
    let script = format!(
        "#!/bin/sh\nset -eu\nif [ \"${{1:-}}\" = info ]; then\n  if [ \"${{3:-}}\" = json ]; then printf '%s\\n' '{}'; else printf 'true\\n'; fi\n  exit 0\nfi\ncase \"${{1:-}}:${{2:-}}\" in\n  network:create) : ;;\n  network:inspect) printf '%s\\n' '{}' ;;\n  network:rm) : ;;\n  container:inspect) printf '%s\\n' '{}' ;;\n  create:--name) printf 'fake-container-id\\n' ;;\n  start:*) : ;;\n  top:*) printf '{}';;\n  stop:*) : ;;\n  rm:*) : ;;\n  port:*) printf '127.0.0.1:9\\n' ;;\n  *) exit 91 ;;\nesac\n",
        info, network, container, process_top,
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
fn empty_registry_authority_is_rejected() {
    let image_reference = format!(":5000/tool@sha256:{}", digest());
    assert_eq!(
        request(image_reference).validate(&policy()),
        Err(ApplicationServiceError::ImageReferenceNotDigestPinned)
    );
}

#[test]
fn selinux_unconfined_runtime_label_fails_closed() {
    let program = fake_selinux_unconfined_podman();
    let result = RootlessPodmanAdapter::new(program.clone()).launch_at(
        &request(format!("localhost/cwl/tool@sha256:{}", digest())),
        &policy(),
        1_780_000_000,
    );
    let _ = fs::remove_file(program);
    assert_eq!(
        result,
        Err(ApplicationServiceError::IsolationVerificationFailed {
            control_name: "lsm",
        })
    );
}
