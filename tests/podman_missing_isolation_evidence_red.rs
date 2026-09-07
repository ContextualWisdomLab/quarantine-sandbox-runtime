//! RED coverage for required Podman isolation fields that are currently defaulted on absence.
//!
//! These fake-Podman fixtures prove only the infrastructure ACL semantics. They do not
//! substitute for the real rootless/positive-LSM release evidence required by AGENTS.md.

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
use serde_json::{json, Value};

static NEXT_TEMP_PATH_ID: AtomicU64 = AtomicU64::new(0);
const GOOD_TOP: &str = "PID SECCOMP CAPEFF CAPBND CAPINH CAPPRM CAPAMB LABEL\n1 filter - - - - - containers-default (enforce)\n";

fn digest_image() -> String {
    format!("localhost/cwl/tool@sha256:{}", "b".repeat(64))
}

fn policy() -> IsolationPolicy {
    IsolationPolicy {
        policy_id: "missing_isolation_evidence_v1".to_owned(),
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

fn request() -> ApplicationServiceRequest {
    ApplicationServiceRequest {
        schema_version: "1.0.0".to_owned(),
        request_id: "missing_isolation_evidence_request".to_owned(),
        image_reference: digest_image(),
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

#[derive(Clone)]
struct Fixture {
    container: Value,
    network: Value,
}

impl Default for Fixture {
    fn default() -> Self {
        Self {
            container: json!([{
                "Id": "fake-container-id",
                "AppArmorProfile": "containers-default",
                "ProcessLabel": "",
                "EffectiveCaps": [],
                "BoundingCaps": [],
                "Config": {"User": "65532:65532"},
                "HostConfig": {
                    "ReadonlyRootfs": true,
                    "Privileged": false,
                    "SecurityOpt": ["no-new-privileges"],
                    "UsernsMode": "auto",
                    "PidMode": "private",
                    "IpcMode": "none",
                    "Memory": 268435456,
                    "NanoCpus": 1000000000_u64,
                    "PidsLimit": 32
                }
            }]),
            network: json!([{"internal": true, "dns_enabled": false}]),
        }
    }
}

fn write_fake_podman(fixture: &Fixture) -> (PathBuf, PathBuf) {
    let program = temporary_path("missing-isolation-evidence-podman");
    let log = temporary_path("missing-isolation-evidence-log");
    let info = json!({
        "host": {
            "security": {
                "rootless": true,
                "seccompEnabled": true,
                "seccompProfilePath": "/usr/share/containers/seccomp.json",
                "apparmorEnabled": true,
                "selinuxEnabled": false
            }
        }
    })
    .to_string();
    let container = fixture.container.to_string();
    let network = fixture.network.to_string();
    let script = format!(
        "#!/bin/sh\nset -eu\nprintf '%s\\n' \"$*\" >> '{}'\nif [ \"${{1:-}}\" = info ]; then\n  if [ \"${{3:-}}\" = json ]; then printf '%s\\n' '{}'; else printf 'true\\n'; fi\n  exit 0\nfi\ncase \"${{1:-}}:${{2:-}}\" in\n  network:create) : ;;\n  network:inspect) printf '%s\\n' '{}' ;;\n  network:rm) : ;;\n  container:inspect) printf '%s\\n' '{}' ;;\n  create:--name) printf 'fake-container-id\\n' ;;\n  start:*) : ;;\n  top:*) printf '%s' '{}' ;;\n  port:*) printf '127.0.0.1:9\\n' ;;\n  stop:*) : ;;\n  rm:*) : ;;\n  *) exit 91 ;;\nesac\n",
        log.display(),
        info,
        network,
        container,
        GOOD_TOP,
    );
    fs::write(&program, script).expect("fake Podman should be writable");
    let mut permissions = fs::metadata(&program)
        .expect("fake Podman metadata should exist")
        .permissions();
    permissions.set_mode(0o700);
    fs::set_permissions(&program, permissions).expect("fake Podman should be executable");
    (program, log)
}

fn launch(
    fixture: Fixture,
) -> Result<quarantine_sandbox_runtime::ApplicationServiceLease, ApplicationServiceError> {
    let (program, log) = write_fake_podman(&fixture);
    let result = RootlessPodmanAdapter::new(program.clone()).launch_at(
        &request(),
        &policy(),
        1_780_000_000,
    );
    let _ = fs::remove_file(program);
    let _ = fs::remove_file(log);
    result
}

#[test]
fn missing_effective_capabilities_is_malformed_container_evidence() {
    let mut fixture = Fixture::default();
    fixture.container[0]
        .as_object_mut()
        .expect("container fixture should be an object")
        .remove("EffectiveCaps");

    assert_eq!(
        launch(fixture),
        Err(ApplicationServiceError::MalformedIsolationInspection {
            operation: "container_inspect",
        })
    );
}

#[test]
fn missing_bounding_capabilities_is_malformed_container_evidence() {
    let mut fixture = Fixture::default();
    fixture.container[0]
        .as_object_mut()
        .expect("container fixture should be an object")
        .remove("BoundingCaps");

    assert_eq!(
        launch(fixture),
        Err(ApplicationServiceError::MalformedIsolationInspection {
            operation: "container_inspect",
        })
    );
}

#[test]
fn missing_dns_enabled_is_malformed_network_evidence() {
    let mut fixture = Fixture::default();
    fixture.network[0]
        .as_object_mut()
        .expect("network fixture should be an object")
        .remove("dns_enabled");

    assert_eq!(
        launch(fixture),
        Err(ApplicationServiceError::MalformedIsolationInspection {
            operation: "network_inspect",
        })
    );
}

#[test]
fn explicit_secure_values_remain_distinct_from_missing_evidence() {
    assert_eq!(
        launch(Fixture::default()),
        Err(ApplicationServiceError::ReadinessTimeout)
    );
}
