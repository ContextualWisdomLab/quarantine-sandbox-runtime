//! Deterministic public-boundary coverage for application-service coordination.
//!
//! These tests exercise coordinator decisions through the public API without
//! changing application-service semantics or reaching into the private registry.

use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
};

use quarantine_sandbox_runtime::{
    ApplicationServiceBackend, ApplicationServiceCoordinator, ApplicationServiceCoordinatorError,
    ApplicationServiceError, ApplicationServiceLease, ApplicationServiceRequest, CleanupReceipt,
    IsolationPolicy, LeaseOwnerId, ResourceRequest, ServiceProtocol,
};
use serde_json::json;

#[derive(Clone, Default)]
struct CountingBackend {
    launch_calls: Arc<AtomicUsize>,
    terminate_calls: Arc<AtomicUsize>,
}

impl ApplicationServiceBackend for CountingBackend {
    fn launch_at(
        &self,
        request: &ApplicationServiceRequest,
        policy: &IsolationPolicy,
        started_at_epoch_seconds: u64,
    ) -> Result<ApplicationServiceLease, ApplicationServiceError> {
        self.launch_calls.fetch_add(1, Ordering::SeqCst);
        serde_json::from_value(json!({
            "schema_version": "1.2.0",
            "request_id": request.request_id.clone(),
            "image_reference": request.image_reference.clone(),
            "backend_id": "coordinator_boundary_backend",
            "backend_version": "test-1",
            "sandbox_id": format!("sandbox-{}", request.request_id),
            "network_id": format!("network-{}", request.request_id),
            "policy_id": policy.policy_id.clone(),
            "policy_sha256": policy.effective_policy_sha256(),
            "endpoint": {
                "host": "127.0.0.1",
                "port": 49_153,
                "protocol": "http"
            },
            "started_at_epoch_seconds": started_at_epoch_seconds,
            "expires_at_epoch_seconds": started_at_epoch_seconds
                + u64::from(request.resources.lease_seconds),
            "shutdown_grace_seconds": policy.shutdown_grace_seconds,
            "isolation_attestation": {
                "rootless": "verified",
                "read_only_root_filesystem": "verified",
                "all_capabilities_dropped": "verified",
                "no_new_privileges": "verified",
                "isolated_user_namespace": "verified",
                "external_egress_denied": "verified",
                "loopback_only_publication": "verified",
                "seccomp_enforced": "verified",
                "lsm_enforced": "verified",
                "resource_limits_verified": "verified",
                "credentials_available": false
            }
        }))
        .map_err(|_| ApplicationServiceError::BackendCommandFailed {
            operation: "coordinator_boundary_lease_decode",
        })
    }

    fn terminate_at(
        &self,
        lease: &ApplicationServiceLease,
        terminated_at_epoch_seconds: u64,
    ) -> Result<CleanupReceipt, ApplicationServiceError> {
        self.terminate_calls.fetch_add(1, Ordering::SeqCst);
        serde_json::from_value(json!({
            "schema_version": "1.2.0",
            "sandbox_id": lease.sandbox_id(),
            "network_id": lease.network_id(),
            "container_removed": true,
            "network_removed": true,
            "terminated_at_epoch_seconds": terminated_at_epoch_seconds,
        }))
        .map_err(|_| ApplicationServiceError::BackendCommandFailed {
            operation: "coordinator_boundary_cleanup_receipt_decode",
        })
    }
}

fn policy() -> IsolationPolicy {
    IsolationPolicy {
        policy_id: "coordinator_boundary_policy_v1".to_owned(),
        maximum_memory_bytes: 1_024,
        maximum_cpu_millicores: 2_000,
        maximum_processes: 64,
        maximum_lease_seconds: 120,
        maximum_tmpfs_bytes: 512,
        readiness_timeout_millis: 100,
        readiness_poll_interval_millis: 10,
        shutdown_grace_seconds: 5,
        run_as_user_id: 65_532,
        run_as_group_id: 65_532,
    }
}

fn request() -> ApplicationServiceRequest {
    ApplicationServiceRequest {
        schema_version: "1.0.0".to_owned(),
        request_id: "coordinator_boundary_request".to_owned(),
        image_reference: format!("localhost/cwl/tool@sha256:{}", "a".repeat(64)),
        container_port: 8_080,
        protocol: ServiceProtocol::Http,
        command: vec!["serve".to_owned()],
        resources: ResourceRequest {
            memory_bytes: 512,
            cpu_millicores: 1_000,
            maximum_processes: 32,
            lease_seconds: 60,
            tmpfs_bytes: 256,
        },
    }
}

fn owner(value: &str) -> LeaseOwnerId {
    LeaseOwnerId::new(value).expect("test owner identity should satisfy the bounded contract")
}

#[test]
fn invalid_request_fails_before_backend_launch() {
    let backend = CountingBackend::default();
    let launch_calls = Arc::clone(&backend.launch_calls);
    let coordinator = ApplicationServiceCoordinator::new(backend);
    let policy = policy();
    let mut invalid = request();
    invalid.resources.memory_bytes = policy.maximum_memory_bytes + 1;

    assert!(
        coordinator
            .launch_at(
                &owner("urn:cwl:agent:coordinator-boundary"),
                &invalid,
                &policy,
                1_780_000_000,
            )
            .is_err(),
        "request validation must fail before a backend launch can begin"
    );
    assert_eq!(launch_calls.load(Ordering::SeqCst), 0);
}

#[test]
fn identical_active_replay_returns_registered_lease_without_second_launch() {
    let backend = CountingBackend::default();
    let launch_calls = Arc::clone(&backend.launch_calls);
    let coordinator = ApplicationServiceCoordinator::new(backend);
    let owner_identity = owner("urn:cwl:agent:coordinator-boundary");
    let request = request();
    let policy = policy();

    let first = coordinator
        .launch_at(&owner_identity, &request, &policy, 1_780_000_000)
        .expect("initial launch should register one active lease");
    let replay = coordinator
        .launch_at(&owner_identity, &request, &policy, 1_780_000_001)
        .expect("identical active replay should return the registered lease");

    assert_eq!(replay, first);
    assert_eq!(launch_calls.load(Ordering::SeqCst), 1);
}

#[test]
fn unknown_owner_cannot_authorize_backend_cleanup() {
    let backend = CountingBackend::default();
    let terminate_calls = Arc::clone(&backend.terminate_calls);
    let coordinator = ApplicationServiceCoordinator::new(backend);
    let owner_identity = owner("urn:cwl:agent:coordinator-boundary");
    let request = request();
    let policy = policy();
    let lease = coordinator
        .launch_at(&owner_identity, &request, &policy, 1_780_000_000)
        .expect("test lease should launch");

    assert_eq!(
        coordinator.terminate_at(
            &owner("urn:cwl:agent:coordinator-boundary-other"),
            &lease,
            1_780_000_010,
        ),
        Err(ApplicationServiceCoordinatorError::UnknownLease)
    );
    assert_eq!(terminate_calls.load(Ordering::SeqCst), 0);
}
