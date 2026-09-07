//! RED contract for preserving causal subprocess-spawn failure classes.

#![cfg(target_os = "linux")]

use std::path::PathBuf;

use quarantine_sandbox_runtime::{
    ApplicationServiceError, ApplicationServiceRequest, BackendInvocationFailureKind,
    IsolationPolicy, ResourceRequest, RootlessPodmanAdapter, ServiceProtocol,
};

fn policy() -> IsolationPolicy {
    IsolationPolicy {
        policy_id: "backend_spawn_failure_classification_v1".to_owned(),
        maximum_memory_bytes: 512 * 1024 * 1024,
        maximum_cpu_millicores: 2_000,
        maximum_processes: 128,
        maximum_lease_seconds: 900,
        maximum_tmpfs_bytes: 128 * 1024 * 1024,
        readiness_timeout_millis: 100,
        readiness_poll_interval_millis: 10,
        shutdown_grace_seconds: 2,
        run_as_user_id: 65_532,
        run_as_group_id: 65_532,
    }
}

fn request() -> ApplicationServiceRequest {
    ApplicationServiceRequest {
        schema_version: "1.0.0".to_owned(),
        request_id: "missing_backend_spawn_classification".to_owned(),
        image_reference: format!("localhost/cwl/tool@sha256:{}", "a".repeat(64)),
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

#[test]
fn missing_backend_executable_preserves_not_found_spawn_class() {
    let definitely_missing = PathBuf::from(format!(
        "/tmp/quarantine-sandbox-runtime-missing-backend-{}",
        std::process::id()
    ));
    let adapter = RootlessPodmanAdapter::new(definitely_missing);

    assert_eq!(
        adapter.launch_at(&request(), &policy(), 1_780_000_000),
        Err(ApplicationServiceError::BackendSpawnFailed {
            operation: "rootless_probe",
            failure_kind: BackendInvocationFailureKind::NotFound,
        })
    );
}
