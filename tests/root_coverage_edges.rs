//! Deterministic coverage for reachable application-service and Podman ACL edge paths.
//!
//! The fake Podman executable exercises parsing and fail-closed error mapping only. It
//! does not replace the dedicated real rootless/positive-LSM acceptance evidence.

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
        policy_id: "root_coverage_edges_v1".to_owned(),
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
        request_id: "root_coverage_edges_request".to_owned(),
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

fn fake_podman(process_top_command: &str, port_output: &str) -> PathBuf {
    let program = temporary_path("root-coverage-edge-podman");
    let info = r#"{"host":{"security":{"rootless":true,"seccompEnabled":true,"seccompProfilePath":"/usr/share/containers/seccomp.json","apparmorEnabled":true,"selinuxEnabled":false}}}"#;
    let container = r#"[{"Id":"fake-container-id","AppArmorProfile":"containers-default","ProcessLabel":"","EffectiveCaps":[],"BoundingCaps":[],"Config":{"User":"65532:65532"},"HostConfig":{"ReadonlyRootfs":true,"Privileged":false,"SecurityOpt":["no-new-privileges"],"UsernsMode":"auto","PidMode":"private","IpcMode":"none","Memory":268435456,"NanoCpus":1000000000,"PidsLimit":32}}]"#;
    let network = r#"[{"internal":true,"dns_enabled":false}]"#;
    let script = format!(
        "#!/bin/sh\nset -eu\nif [ \"${{1:-}}\" = info ]; then\n  if [ \"${{3:-}}\" = json ]; then printf '%s\\n' '{}'; else printf 'true\\n'; fi\n  exit 0\nfi\ncase \"${{1:-}}:${{2:-}}\" in\n  network:create) : ;;\n  network:inspect) printf '%s\\n' '{}' ;;\n  network:rm) : ;;\n  container:inspect) printf '%s\\n' '{}' ;;\n  create:--name) printf 'fake-container-id\\n' ;;\n  start:*) : ;;\n  top:*) {} ;;\n  port:*) printf '%s\\n' '{}' ;;\n  stop:*) : ;;\n  rm:*) : ;;\n  *) exit 91 ;;\nesac\n",
        info, network, container, process_top_command, port_output,
    );
    fs::write(&program, script).expect("fake Podman should be writable");
    let mut permissions = fs::metadata(&program)
        .expect("fake Podman metadata should exist")
        .permissions();
    permissions.set_mode(0o700);
    fs::set_permissions(&program, permissions).expect("fake Podman should be executable");
    program
}

fn launch_with_fake(
    process_top_command: &str,
    port_output: &str,
) -> Result<quarantine_sandbox_runtime::ApplicationServiceLease, ApplicationServiceError> {
    let program = fake_podman(process_top_command, port_output);
    let result = RootlessPodmanAdapter::new(program.clone()).launch_at(
        &request(format!("localhost/cwl/tool@sha256:{}", digest())),
        &policy(),
        1_780_000_000,
    );
    let _ = fs::remove_file(program);
    result
}

#[test]
fn repository_validation_rejects_reachable_path_and_registry_edges() {
    let digest = digest();
    for image_reference in [
        format!("registry.example.com//tool@sha256:{digest}"),
        format!("/tool@sha256:{digest}"),
        format!("tool/@sha256:{digest}"),
        format!("registry.example.com/repo/./tool@sha256:{digest}"),
        format!("registry.example.com/repo/../tool@sha256:{digest}"),
        format!("registry.example.com/repo:tag/tool@sha256:{digest}"),
        format!("registry.example.com:abc/tool@sha256:{digest}"),
    ] {
        assert_eq!(
            request(image_reference).validate(&policy()),
            Err(ApplicationServiceError::ImageReferenceNotDigestPinned)
        );
    }
}

#[test]
fn noncanonical_capability_hex_is_not_treated_as_an_empty_set() {
    let process_top = "printf 'PID SECCOMP CAPEFF CAPBND CAPINH CAPPRM CAPAMB LABEL\\n1 filter 0x - - - - containers-default (enforce)\\n'";
    assert_eq!(
        launch_with_fake(process_top, "127.0.0.1:9"),
        Err(ApplicationServiceError::IsolationVerificationFailed {
            control_name: "all_capabilities_dropped",
        })
    );
}

#[test]
fn non_utf8_process_security_output_is_malformed_evidence() {
    let process_top = "printf '\\377\\n'";
    assert_eq!(
        launch_with_fake(process_top, "127.0.0.1:9"),
        Err(ApplicationServiceError::MalformedIsolationInspection {
            operation: "process_security_top",
        })
    );
}

#[test]
fn malformed_and_zero_loopback_ports_fail_closed() {
    let good_top = "printf 'PID SECCOMP CAPEFF CAPBND CAPINH CAPPRM CAPAMB LABEL\\n1 filter - - - - - containers-default (enforce)\\n'";
    for port_output in ["127.0.0.1:not-a-port", "127.0.0.1:0"] {
        assert_eq!(
            launch_with_fake(good_top, port_output),
            Err(ApplicationServiceError::InvalidPortMapping)
        );
    }
}
