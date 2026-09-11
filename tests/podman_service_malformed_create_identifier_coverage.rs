//! Service-profile regression for malformed Podman create identity.
//!
//! A successful `podman create` exit status is not ownership evidence by itself. If stdout cannot
//! be admitted as a bounded backend identifier, service launch must fail closed and remove both
//! runtime-owned resources created before the malformed response is observed.

#![cfg(target_os = "linux")]

use std::{
    fs,
    os::unix::fs::symlink,
    path::{Path, PathBuf},
};

use quarantine_sandbox_runtime::{
    ApplicationServiceError, ApplicationServiceRequest, IsolationPolicy, ResourceRequest,
    RootlessPodmanAdapter, ServiceProtocol,
};
use tempfile::tempdir;

fn policy() -> IsolationPolicy {
    IsolationPolicy {
        policy_id: "malformed_service_create_identity_v1".to_owned(),
        maximum_memory_bytes: 512 * 1024 * 1024,
        maximum_cpu_millicores: 2_000,
        maximum_processes: 64,
        maximum_lease_seconds: 300,
        maximum_tmpfs_bytes: 64 * 1024 * 1024,
        readiness_timeout_millis: 50,
        readiness_poll_interval_millis: 10,
        shutdown_grace_seconds: 2,
        run_as_user_id: 65_532,
        run_as_group_id: 65_532,
    }
}

fn request() -> ApplicationServiceRequest {
    ApplicationServiceRequest {
        schema_version: "1.0.0".to_owned(),
        request_id: "malformed-service-create-identity".to_owned(),
        image_reference: format!("localhost/cwl/tool@sha256:{}", "b".repeat(64)),
        container_port: 8_080,
        protocol: ServiceProtocol::Http,
        command: vec!["serve".to_owned()],
        resources: ResourceRequest {
            memory_bytes: 256 * 1024 * 1024,
            cpu_millicores: 1_000,
            maximum_processes: 16,
            lease_seconds: 60,
            tmpfs_bytes: 16 * 1024 * 1024,
        },
    }
}

fn immutable_fixture_executable() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/fake_podman.sh")
}

fn sidecar(program: &Path, suffix: &str) -> PathBuf {
    PathBuf::from(format!("{}.{suffix}", program.display()))
}

#[test]
fn malformed_successful_create_identity_fails_closed_after_container_and_network_cleanup() {
    let directory = tempdir().expect("isolated malformed-create fixture should exist");
    let program = directory.path().join("podman");
    let log = directory.path().join("calls");
    let scenario = directory.path().join("scenario.sh");
    symlink(immutable_fixture_executable(), &program)
        .expect("immutable fake Podman executable should be linked");

    let script = r#"
case "${1:-}:${2:-}" in
  info:--format)
    printf '%s\n' '{"host":{"security":{"rootless":true,"seccompEnabled":true,"seccompProfilePath":"/usr/share/containers/seccomp.json","apparmorEnabled":true,"selinuxEnabled":false}},"version":{"Version":"6.1.0"}}'
    ;;
  network:create) : ;;
  create:--name) printf 'bad identifier with spaces\n' ;;
  rm:--force) : ;;
  network:rm) : ;;
  *) exit 91 ;;
esac
"#;
    fs::write(&scenario, script).expect("scenario data should be writable");
    fs::write(
        sidecar(&program, "config"),
        format!(
            "MODE='source_script'\nLOG='{}'\nSCRIPT='{}'\n",
            log.display(),
            scenario.display()
        ),
    )
    .expect("dispatcher configuration should be writable");

    let adapter = RootlessPodmanAdapter::new(&program);
    assert_eq!(
        adapter.launch_at(&request(), &policy(), 1_780_000_500),
        Err(ApplicationServiceError::MalformedIsolationInspection {
            operation: "container_create",
        })
    );

    let calls = fs::read_to_string(&log).expect("backend calls should be recorded");
    assert!(calls.contains("network create"));
    assert!(calls.contains("create --name"));
    assert!(calls.contains("rm --force qsr-service-"));
    assert!(calls.contains("network rm --force qsr-net-"));
    assert!(!calls.contains("start "));
}
