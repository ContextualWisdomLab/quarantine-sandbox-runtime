//! Deterministic coverage for reachable application-service and Podman ACL edge paths.
//!
//! The fake Podman executable exercises parsing and fail-closed error mapping only. It
//! does not replace the dedicated real rootless/positive-LSM acceptance evidence.

#![cfg(target_os = "linux")]

use std::{
    fs,
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

fn immutable_fixture_executable() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/fake_podman.sh")
}

fn fixture_sidecar(program: &PathBuf, suffix: &str) -> PathBuf {
    PathBuf::from(format!("{}.{suffix}", program.display()))
}

fn write_fake_podman_fixture(program: &PathBuf, script: String) {
    let script_path = fixture_sidecar(program, "script");
    let config_path = fixture_sidecar(program, "config");
    let log_path = fixture_sidecar(program, "log");
    fs::write(&log_path, b"").expect("fake Podman log sink should be writable");
    std::os::unix::fs::symlink(immutable_fixture_executable(), program)
        .expect("fake Podman immutable symlink should be creatable");
    fs::write(&script_path, script).expect("fake Podman scenario data should be writable");
    fs::write(
        &config_path,
        format!(
            "MODE='source_script'\nLOG='{}'\nSCRIPT='{}'\n",
            log_path.display(),
            script_path.display()
        ),
    )
    .expect("fake Podman dispatcher config should be writable");
}

fn fake_podman(process_top_command: &str, port_output: &str) -> PathBuf {
    let program = temporary_path("root-coverage-edge-podman");
    let info = r#"{"host":{"security":{"rootless":true,"seccompEnabled":true,"seccompProfilePath":"/usr/share/containers/seccomp.json","apparmorEnabled":true,"selinuxEnabled":false}}}"#;
    let container = r#"[{"Id":"0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef","AppArmorProfile":"containers-default","ProcessLabel":"","EffectiveCaps":[],"BoundingCaps":[],"Config":{"User":"65532:65532"},"HostConfig":{"ReadonlyRootfs":true,"Privileged":false,"SecurityOpt":["no-new-privileges"],"UsernsMode":"auto","PidMode":"private","IpcMode":"none","Memory":268435456,"NanoCpus":1000000000,"PidsLimit":32}}]"#;
    let network = r#"[{"internal":true,"dns_enabled":false}]"#;
    let script = format!(
        "if [ \"${{1:-}}\" = info ]; then\n  if [ \"${{3:-}}\" = json ]; then printf '%s\\n' '{}'; else printf 'true\\n'; fi\n  exit 0\nfi\ncase \"${{1:-}}:${{2:-}}\" in\n  network:create) : ;;\n  network:inspect) printf '%s\\n' '{}' ;;\n  network:rm) : ;;\n  container:inspect) printf '%s\\n' '{}' ;;\n  create:--name) printf '0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef\\n' ;;\n  start:*) : ;;\n  top:*) {} ;;\n  port:*) printf '%s\\n' '{}' ;;\n  stop:*) : ;;\n  rm:*) : ;;\n  *) exit 91 ;;\nesac\n",
        info, network, container, process_top_command, port_output,
    );
    write_fake_podman_fixture(&program, script);
    program
}

fn fake_podman_failure(failure_operation: &str) -> PathBuf {
    let program = temporary_path("root-coverage-failure-podman");
    let info = r#"{"host":{"security":{"rootless":true,"seccompEnabled":true,"seccompProfilePath":"/usr/share/containers/seccomp.json","apparmorEnabled":true,"selinuxEnabled":false}}}"#;
    let container = r#"[{"Id":"0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef","AppArmorProfile":"containers-default","ProcessLabel":"","EffectiveCaps":[],"BoundingCaps":[],"Config":{"User":"65532:65532"},"HostConfig":{"ReadonlyRootfs":true,"Privileged":false,"SecurityOpt":["no-new-privileges"],"UsernsMode":"auto","PidMode":"private","IpcMode":"none","Memory":268435456,"NanoCpus":1000000000,"PidsLimit":32}}]"#;
    let network = r#"[{"internal":true,"dns_enabled":false}]"#;
    let good_top = "PID SECCOMP CAPEFF CAPBND CAPINH CAPPRM CAPAMB LABEL\\n1 filter - - - - - containers-default (enforce)\\n";
    let script = format!(
        "failure='{}'\nif [ \"${{1:-}}\" = info ]; then\n  if [ \"${{3:-}}\" = json ]; then\n    if [ \"$failure\" = backend_security_info ]; then exit 17; fi\n    printf '%s\\n' '{}'\n  else\n    printf 'true\\n'\n  fi\n  exit 0\nfi\ncase \"${{1:-}}:${{2:-}}\" in\n  network:create) : ;;\n  network:inspect) if [ \"$failure\" = network_inspect ]; then exit 17; else printf '%s\\n' '{}'; fi ;;\n  network:rm) : ;;\n  container:inspect) printf '%s\\n' '{}' ;;\n  create:--name)\n    if [ \"$failure\" = non_utf8_identifier ]; then printf '\\377\\n';\n    elif [ \"$failure\" = malformed_identifier_without_receipt ]; then printf 'bad id\\n';\n    else printf '0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef\\n'; fi ;;\n  start:*) : ;;\n  top:*) if [ \"$failure\" = process_security_top ]; then exit 17; else printf '{}'; fi ;;\n  port:*) printf '127.0.0.1:9\\n' ;;\n  stop:*) : ;;\n  rm:*) if [ \"$failure\" = malformed_identifier_without_receipt ]; then exit 17; else :; fi ;;\n  *) exit 91 ;;\nesac\n",
        failure_operation, info, network, container, good_top,
    );
    write_fake_podman_fixture(&program, script);
    program
}

fn launch_with_program(
    program: PathBuf,
) -> Result<quarantine_sandbox_runtime::ApplicationServiceLease, ApplicationServiceError> {
    let result = RootlessPodmanAdapter::new(program.clone()).launch_at(
        &request(format!("localhost/cwl/tool@sha256:{}", digest())),
        &policy(),
        1_780_000_000,
    );
    let _ = fs::remove_file(fixture_sidecar(&program, "script"));
    let _ = fs::remove_file(fixture_sidecar(&program, "config"));
    let _ = fs::remove_file(fixture_sidecar(&program, "log"));
    let _ = fs::remove_file(program);
    result
}

fn launch_with_fake(
    process_top_command: &str,
    port_output: &str,
) -> Result<quarantine_sandbox_runtime::ApplicationServiceLease, ApplicationServiceError> {
    launch_with_program(fake_podman(process_top_command, port_output))
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
    for image_reference in [
        format!("registry.example.com/tool@sha256:{digest}"),
        format!("registry.example.com:5000/repo/tool@sha256:{digest}"),
    ] {
        assert_eq!(request(image_reference).validate(&policy()), Ok(()));
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
fn backend_failures_and_identifier_bytes_remain_typed() {
    for (failure_operation, expected) in [
        (
            "backend_security_info",
            ApplicationServiceError::BackendCommandFailed {
                operation: "backend_security_info",
            },
        ),
        (
            "process_security_top",
            ApplicationServiceError::BackendCommandFailed {
                operation: "process_security_top",
            },
        ),
        (
            "network_inspect",
            ApplicationServiceError::BackendCommandFailed {
                operation: "network_inspect",
            },
        ),
        (
            "non_utf8_identifier",
            ApplicationServiceError::MalformedIsolationInspection {
                operation: "container_create_receipt",
            },
        ),
        (
            "malformed_identifier_without_receipt",
            ApplicationServiceError::MalformedIsolationInspection {
                operation: "container_create_receipt",
            },
        ),
    ] {
        assert_eq!(
            launch_with_program(fake_podman_failure(failure_operation)),
            Err(expected)
        );
    }
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
