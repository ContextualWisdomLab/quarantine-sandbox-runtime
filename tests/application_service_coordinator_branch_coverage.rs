//! Branch-outcome coverage for caller-scoped coordinator state transitions.

use std::sync::{
    Arc, Barrier,
    atomic::{AtomicUsize, Ordering},
};
use std::thread;

use quarantine_sandbox_runtime::{
    ApplicationServiceBackend, ApplicationServiceCoordinator, ApplicationServiceCoordinatorError,
    ApplicationServiceError, ApplicationServiceLease, ApplicationServiceRequest, CleanupReceipt,
    IsolationPolicy, LeaseOwnerId, ResourceRequest, ServiceProtocol,
};
use serde_json::json;

fn owner() -> LeaseOwnerId {
    LeaseOwnerId::new("urn:cwl:agent:coordinator-branch-test")
        .expect("test owner identity should satisfy the bounded contract")
}

fn policy() -> IsolationPolicy {
    IsolationPolicy {
        policy_id: "coordinator_branch_policy_v1".to_owned(),
        maximum_memory_bytes: 512 * 1024 * 1024,
        maximum_cpu_millicores: 2_000,
        maximum_processes: 128,
        maximum_lease_seconds: 900,
        maximum_tmpfs_bytes: 128 * 1024 * 1024,
        readiness_timeout_millis: 2_000,
        readiness_poll_interval_millis: 10,
        shutdown_grace_seconds: 2,
        run_as_user_id: 65_532,
        run_as_group_id: 65_532,
    }
}

fn request() -> ApplicationServiceRequest {
    ApplicationServiceRequest {
        schema_version: "1.0.0".to_owned(),
        request_id: "coordinator_branch_request".to_owned(),
        image_reference: format!("localhost/cwl/tool@sha256:{}", "d".repeat(64)),
        container_port: 8_080,
        protocol: ServiceProtocol::Http,
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

fn lease_for(
    request: &ApplicationServiceRequest,
    policy: &IsolationPolicy,
    started_at_epoch_seconds: u64,
) -> ApplicationServiceLease {
    serde_json::from_value(json!({
        "schema_version": "1.1.0",
        "request_id": request.request_id,
        "image_reference": request.image_reference,
        "backend_id": "branch_test_backend",
        "sandbox_id": "sandbox_branch_test",
        "network_id": "network_branch_test",
        "policy_id": policy.policy_id,
        "policy_sha256": policy.effective_policy_sha256(),
        "endpoint": {
            "host": "127.0.0.1",
            "port": 43123,
            "protocol": "http"
        },
        "started_at_epoch_seconds": started_at_epoch_seconds,
        "expires_at_epoch_seconds": started_at_epoch_seconds
            + u64::from(request.resources.lease_seconds),
        "shutdown_grace_seconds": policy.shutdown_grace_seconds,
        "isolation_attestation": {
            "rootless": true,
            "read_only_root_filesystem": true,
            "all_capabilities_dropped": true,
            "no_new_privileges": true,
            "isolated_user_namespace": true,
            "external_egress_denied": true,
            "loopback_only_publication": true,
            "credentials_available": false
        }
    }))
    .expect("test lease should match the public serialized contract")
}

fn cleanup_receipt(
    lease: &ApplicationServiceLease,
    terminated_at_epoch_seconds: u64,
) -> CleanupReceipt {
    serde_json::from_value(json!({
        "schema_version": "1.0.0",
        "sandbox_id": lease.sandbox_id(),
        "network_id": lease.network_id(),
        "container_removed": true,
        "network_removed": true,
        "terminated_at_epoch_seconds": terminated_at_epoch_seconds
    }))
    .expect("test cleanup receipt should match the public serialized contract")
}

#[derive(Clone)]
struct LaunchBlockingBackend {
    launch_entered: Arc<Barrier>,
    launch_release: Arc<Barrier>,
}

impl ApplicationServiceBackend for LaunchBlockingBackend {
    fn launch_at(
        &self,
        request: &ApplicationServiceRequest,
        policy: &IsolationPolicy,
        started_at_epoch_seconds: u64,
    ) -> Result<ApplicationServiceLease, ApplicationServiceError> {
        self.launch_entered.wait();
        self.launch_release.wait();
        Ok(lease_for(request, policy, started_at_epoch_seconds))
    }

    fn terminate_at(
        &self,
        lease: &ApplicationServiceLease,
        terminated_at_epoch_seconds: u64,
    ) -> Result<CleanupReceipt, ApplicationServiceError> {
        Ok(cleanup_receipt(lease, terminated_at_epoch_seconds))
    }
}

#[derive(Clone)]
struct TerminationBlockingBackend {
    termination_entered: Arc<Barrier>,
    termination_release: Arc<Barrier>,
}

impl ApplicationServiceBackend for TerminationBlockingBackend {
    fn launch_at(
        &self,
        request: &ApplicationServiceRequest,
        policy: &IsolationPolicy,
        started_at_epoch_seconds: u64,
    ) -> Result<ApplicationServiceLease, ApplicationServiceError> {
        Ok(lease_for(request, policy, started_at_epoch_seconds))
    }

    fn terminate_at(
        &self,
        lease: &ApplicationServiceLease,
        terminated_at_epoch_seconds: u64,
    ) -> Result<CleanupReceipt, ApplicationServiceError> {
        self.termination_entered.wait();
        self.termination_release.wait();
        Ok(cleanup_receipt(lease, terminated_at_epoch_seconds))
    }
}

#[derive(Clone)]
struct CountingBackend {
    termination_calls: Arc<AtomicUsize>,
}

impl ApplicationServiceBackend for CountingBackend {
    fn launch_at(
        &self,
        request: &ApplicationServiceRequest,
        policy: &IsolationPolicy,
        started_at_epoch_seconds: u64,
    ) -> Result<ApplicationServiceLease, ApplicationServiceError> {
        Ok(lease_for(request, policy, started_at_epoch_seconds))
    }

    fn terminate_at(
        &self,
        lease: &ApplicationServiceLease,
        terminated_at_epoch_seconds: u64,
    ) -> Result<CleanupReceipt, ApplicationServiceError> {
        self.termination_calls.fetch_add(1, Ordering::SeqCst);
        Ok(cleanup_receipt(lease, terminated_at_epoch_seconds))
    }
}

#[test]
fn conflicting_retry_while_launching_reaches_the_idempotency_conflict_guard() {
    let launch_entered = Arc::new(Barrier::new(2));
    let launch_release = Arc::new(Barrier::new(2));
    let coordinator = Arc::new(ApplicationServiceCoordinator::new(LaunchBlockingBackend {
        launch_entered: Arc::clone(&launch_entered),
        launch_release: Arc::clone(&launch_release),
    }));
    let lease_owner = owner();
    let worker_coordinator = Arc::clone(&coordinator);
    let worker_owner = lease_owner.clone();
    let worker = thread::spawn(move || {
        worker_coordinator.launch_at(&worker_owner, &request(), &policy(), 1_780_000_000)
    });

    launch_entered.wait();
    let mut conflicting_request = request();
    conflicting_request.command.push("--different".to_owned());
    assert_eq!(
        coordinator.launch_at(
            &lease_owner,
            &conflicting_request,
            &policy(),
            1_780_000_001,
        ),
        Err(ApplicationServiceCoordinatorError::IdempotencyConflict)
    );

    launch_release.wait();
    let lease = worker
        .join()
        .expect("launch thread should not panic")
        .expect("original launch should complete");
    coordinator
        .terminate_at(&lease_owner, &lease, 1_780_000_010)
        .expect("original lease should remain terminable");
}

#[test]
fn terminating_entry_distinguishes_identical_retry_from_conflicting_content() {
    let termination_entered = Arc::new(Barrier::new(2));
    let termination_release = Arc::new(Barrier::new(2));
    let coordinator = Arc::new(ApplicationServiceCoordinator::new(TerminationBlockingBackend {
        termination_entered: Arc::clone(&termination_entered),
        termination_release: Arc::clone(&termination_release),
    }));
    let lease_owner = owner();
    let lease = coordinator
        .launch_at(&lease_owner, &request(), &policy(), 1_780_000_000)
        .expect("launch should complete before termination starts");
    let worker_coordinator = Arc::clone(&coordinator);
    let worker_owner = lease_owner.clone();
    let worker_lease = lease.clone();
    let worker = thread::spawn(move || {
        worker_coordinator.terminate_at(&worker_owner, &worker_lease, 1_780_000_010)
    });

    termination_entered.wait();
    assert_eq!(
        coordinator.launch_at(&lease_owner, &request(), &policy(), 1_780_000_011),
        Err(ApplicationServiceCoordinatorError::TerminationInProgress)
    );
    let mut conflicting_request = request();
    conflicting_request.command.push("--different".to_owned());
    assert_eq!(
        coordinator.launch_at(
            &lease_owner,
            &conflicting_request,
            &policy(),
            1_780_000_012,
        ),
        Err(ApplicationServiceCoordinatorError::IdempotencyConflict)
    );

    termination_release.wait();
    worker
        .join()
        .expect("termination thread should not panic")
        .expect("original termination should complete");
}

#[test]
fn forged_receipt_for_known_owner_and_request_fails_before_backend_cleanup() {
    let termination_calls = Arc::new(AtomicUsize::new(0));
    let coordinator = ApplicationServiceCoordinator::new(CountingBackend {
        termination_calls: Arc::clone(&termination_calls),
    });
    let lease_owner = owner();
    let lease = coordinator
        .launch_at(&lease_owner, &request(), &policy(), 1_780_000_000)
        .expect("launch should succeed");
    let mut forged_value = serde_json::to_value(&lease).expect("lease should serialize");
    forged_value["backend_id"] = json!("forged_backend");
    let forged_lease: ApplicationServiceLease = serde_json::from_value(forged_value)
        .expect("forged test value should remain structurally deserializable");

    assert_eq!(
        coordinator.terminate_at(&lease_owner, &forged_lease, 1_780_000_010),
        Err(ApplicationServiceCoordinatorError::LeaseMismatch)
    );
    assert_eq!(termination_calls.load(Ordering::SeqCst), 0);

    coordinator
        .terminate_at(&lease_owner, &lease, 1_780_000_011)
        .expect("registered lease should remain terminable");
    assert_eq!(termination_calls.load(Ordering::SeqCst), 1);
}
