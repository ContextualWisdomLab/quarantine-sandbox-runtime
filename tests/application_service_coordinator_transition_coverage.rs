//! Deterministic coverage for fingerprint-sensitive coordinator transitions.
//!
//! These witnesses exercise only the public coordinator API. Barriers hold the
//! backend at a known lifecycle boundary so the registry state is observable
//! through documented errors without sleeps or private-state access.

use std::sync::{
    Arc, Barrier,
    atomic::{AtomicUsize, Ordering},
};

use quarantine_sandbox_runtime::{
    ApplicationServiceBackend, ApplicationServiceCoordinator, ApplicationServiceCoordinatorError,
    ApplicationServiceError, ApplicationServiceLease, ApplicationServiceRequest, CleanupReceipt,
    IsolationPolicy, LeaseOwnerId, ResourceRequest, ServiceProtocol,
};
use serde_json::json;

#[derive(Clone)]
struct BlockingLaunchBackend {
    entered: Arc<Barrier>,
    release: Arc<Barrier>,
    launch_calls: Arc<AtomicUsize>,
}

impl ApplicationServiceBackend for BlockingLaunchBackend {
    fn launch_at(
        &self,
        request: &ApplicationServiceRequest,
        policy: &IsolationPolicy,
        started_at_epoch_seconds: u64,
    ) -> Result<ApplicationServiceLease, ApplicationServiceError> {
        self.launch_calls.fetch_add(1, Ordering::SeqCst);
        self.entered.wait();
        self.release.wait();
        lease_for(request, policy, started_at_epoch_seconds)
    }

    fn terminate_at(
        &self,
        lease: &ApplicationServiceLease,
        terminated_at_epoch_seconds: u64,
    ) -> Result<CleanupReceipt, ApplicationServiceError> {
        cleanup_for(lease, terminated_at_epoch_seconds)
    }
}

#[derive(Clone)]
struct BlockingTerminateBackend {
    entered: Arc<Barrier>,
    release: Arc<Barrier>,
    terminate_calls: Arc<AtomicUsize>,
}

impl ApplicationServiceBackend for BlockingTerminateBackend {
    fn launch_at(
        &self,
        request: &ApplicationServiceRequest,
        policy: &IsolationPolicy,
        started_at_epoch_seconds: u64,
    ) -> Result<ApplicationServiceLease, ApplicationServiceError> {
        lease_for(request, policy, started_at_epoch_seconds)
    }

    fn terminate_at(
        &self,
        lease: &ApplicationServiceLease,
        terminated_at_epoch_seconds: u64,
    ) -> Result<CleanupReceipt, ApplicationServiceError> {
        self.terminate_calls.fetch_add(1, Ordering::SeqCst);
        self.entered.wait();
        self.release.wait();
        cleanup_for(lease, terminated_at_epoch_seconds)
    }
}

fn lease_for(
    request: &ApplicationServiceRequest,
    policy: &IsolationPolicy,
    started_at_epoch_seconds: u64,
) -> Result<ApplicationServiceLease, ApplicationServiceError> {
    serde_json::from_value(json!({
        "schema_version": "1.2.0",
        "request_id": request.request_id.clone(),
        "image_reference": request.image_reference.clone(),
        "backend_id": "coordinator_transition_backend",
        "backend_version": "test-1",
        "sandbox_id": format!("sandbox-{}-{started_at_epoch_seconds}", request.request_id),
        "network_id": format!("network-{}-{started_at_epoch_seconds}", request.request_id),
        "policy_id": policy.policy_id.clone(),
        "policy_sha256": policy.effective_policy_sha256(),
        "endpoint": {
            "host": "127.0.0.1",
            "port": 49_154,
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
        operation: "coordinator_transition_lease_decode",
    })
}

fn cleanup_for(
    lease: &ApplicationServiceLease,
    terminated_at_epoch_seconds: u64,
) -> Result<CleanupReceipt, ApplicationServiceError> {
    serde_json::from_value(json!({
        "schema_version": "1.2.0",
        "sandbox_id": lease.sandbox_id(),
        "network_id": lease.network_id(),
        "container_removed": true,
        "network_removed": true,
        "terminated_at_epoch_seconds": terminated_at_epoch_seconds,
    }))
    .map_err(|_| ApplicationServiceError::BackendCommandFailed {
        operation: "coordinator_transition_cleanup_decode",
    })
}

fn policy() -> IsolationPolicy {
    IsolationPolicy {
        policy_id: "coordinator_transition_policy_v1".to_owned(),
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
        request_id: "coordinator_transition_request".to_owned(),
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

fn owner() -> LeaseOwnerId {
    LeaseOwnerId::new("urn:cwl:agent:coordinator-transition")
        .expect("test owner identity should satisfy the bounded contract")
}

#[test]
fn launching_entry_rejects_conflicting_request_fingerprint() {
    let entered = Arc::new(Barrier::new(2));
    let release = Arc::new(Barrier::new(2));
    let launch_calls = Arc::new(AtomicUsize::new(0));
    let coordinator = Arc::new(ApplicationServiceCoordinator::new(BlockingLaunchBackend {
        entered: Arc::clone(&entered),
        release: Arc::clone(&release),
        launch_calls: Arc::clone(&launch_calls),
    }));

    let worker = {
        let coordinator = Arc::clone(&coordinator);
        std::thread::spawn(move || coordinator.launch_at(&owner(), &request(), &policy(), 100))
    };

    entered.wait();
    let mut conflicting_request = request();
    conflicting_request.command.push("--different".to_owned());
    assert_eq!(
        coordinator.launch_at(&owner(), &conflicting_request, &policy(), 101),
        Err(ApplicationServiceCoordinatorError::IdempotencyConflict)
    );
    assert_eq!(launch_calls.load(Ordering::SeqCst), 1);

    release.wait();
    worker
        .join()
        .expect("launch worker must not panic")
        .expect("released launch should complete");
}

#[test]
fn terminating_entry_distinguishes_replay_from_conflicting_request() {
    let entered = Arc::new(Barrier::new(2));
    let release = Arc::new(Barrier::new(2));
    let terminate_calls = Arc::new(AtomicUsize::new(0));
    let coordinator = Arc::new(ApplicationServiceCoordinator::new(BlockingTerminateBackend {
        entered: Arc::clone(&entered),
        release: Arc::clone(&release),
        terminate_calls: Arc::clone(&terminate_calls),
    }));
    let owner_identity = owner();
    let request = request();
    let policy = policy();
    let lease = coordinator
        .launch_at(&owner_identity, &request, &policy, 100)
        .expect("test lease should launch");

    let worker = {
        let coordinator = Arc::clone(&coordinator);
        let owner_identity = owner_identity.clone();
        let lease = lease.clone();
        std::thread::spawn(move || coordinator.terminate_at(&owner_identity, &lease, 110))
    };

    entered.wait();
    assert_eq!(
        coordinator.launch_at(&owner_identity, &request, &policy, 111),
        Err(ApplicationServiceCoordinatorError::TerminationInProgress)
    );

    let mut conflicting_request = request.clone();
    conflicting_request.command.push("--different".to_owned());
    assert_eq!(
        coordinator.launch_at(&owner_identity, &conflicting_request, &policy, 112),
        Err(ApplicationServiceCoordinatorError::IdempotencyConflict)
    );
    assert_eq!(terminate_calls.load(Ordering::SeqCst), 1);

    release.wait();
    worker
        .join()
        .expect("termination worker must not panic")
        .expect("released termination should complete");
}
