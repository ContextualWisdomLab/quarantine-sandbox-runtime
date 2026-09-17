//! Concurrency and receipt-integrity coverage for application-service coordination.

use std::{
    sync::{
        Arc,
        atomic::{AtomicBool, AtomicUsize, Ordering},
    },
    thread,
    time::Duration,
};

use quarantine_sandbox_runtime::{
    ApplicationServiceBackend, ApplicationServiceCoordinator, ApplicationServiceCoordinatorError,
    ApplicationServiceError, ApplicationServiceLease, ApplicationServiceRequest, CleanupReceipt,
    IsolationPolicy, LeaseOwnerId, ResourceRequest, ServiceProtocol,
};
use serde_json::json;

#[derive(Clone)]
struct CoordinatorEdgeBackend {
    launch_started: Arc<AtomicBool>,
    terminate_started: Arc<AtomicBool>,
    terminate_calls: Arc<AtomicUsize>,
    slow_launch: bool,
    slow_terminate: bool,
}

impl CoordinatorEdgeBackend {
    fn new(slow_launch: bool, slow_terminate: bool) -> Self {
        Self {
            launch_started: Arc::new(AtomicBool::new(false)),
            terminate_started: Arc::new(AtomicBool::new(false)),
            terminate_calls: Arc::new(AtomicUsize::new(0)),
            slow_launch,
            slow_terminate,
        }
    }
}

impl ApplicationServiceBackend for CoordinatorEdgeBackend {
    fn launch_at(
        &self,
        request: &ApplicationServiceRequest,
        policy: &IsolationPolicy,
        started_at_epoch_seconds: u64,
    ) -> Result<ApplicationServiceLease, ApplicationServiceError> {
        self.launch_started.store(true, Ordering::SeqCst);
        if self.slow_launch {
            thread::sleep(Duration::from_millis(100));
        }
        serde_json::from_value(json!({
            "schema_version": "1.2.0",
            "request_id": request.request_id.clone(),
            "image_reference": request.image_reference.clone(),
            "backend_id": "coordinator_edge_backend",
            "backend_version": "test-1",
            "sandbox_id": format!("sandbox-{}", request.request_id),
            "network_id": format!("network-{}", request.request_id),
            "policy_id": policy.policy_id.clone(),
            "policy_sha256": policy.effective_policy_sha256(),
            "endpoint": {
                "host": "127.0.0.1",
                "port": 49_152,
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
            operation: "coordinator_edge_lease_decode",
        })
    }

    fn terminate_at(
        &self,
        _lease: &ApplicationServiceLease,
        _terminated_at_epoch_seconds: u64,
    ) -> Result<CleanupReceipt, ApplicationServiceError> {
        self.terminate_calls.fetch_add(1, Ordering::SeqCst);
        self.terminate_started.store(true, Ordering::SeqCst);
        if self.slow_terminate {
            thread::sleep(Duration::from_millis(100));
        }
        Err(ApplicationServiceError::CleanupFailed)
    }
}

fn policy() -> IsolationPolicy {
    IsolationPolicy {
        policy_id: "coordinator_edge_policy_v1".to_owned(),
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
        request_id: "coordinator_edge_request".to_owned(),
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
    LeaseOwnerId::new("urn:cwl:agent:coordinator-edge")
        .expect("test owner identity should satisfy the bounded contract")
}

fn wait_until(flag: &AtomicBool) {
    for _ in 0..100 {
        if flag.load(Ordering::SeqCst) {
            return;
        }
        thread::sleep(Duration::from_millis(5));
    }
    panic!("timed out waiting for coordinator backend boundary");
}

#[test]
fn changed_payload_cannot_reuse_an_inflight_idempotency_key() {
    let backend = CoordinatorEdgeBackend::new(true, false);
    let launch_started = Arc::clone(&backend.launch_started);
    let coordinator = Arc::new(ApplicationServiceCoordinator::new(backend));
    let worker_coordinator = Arc::clone(&coordinator);
    let worker_owner = owner();
    let worker = thread::spawn(move || {
        worker_coordinator.launch_at(&worker_owner, &request(), &policy(), 1_780_000_000)
    });

    wait_until(&launch_started);
    let mut changed = request();
    changed.command.push("--different".to_owned());
    assert_eq!(
        coordinator.launch_at(&owner(), &changed, &policy(), 1_780_000_001),
        Err(ApplicationServiceCoordinatorError::IdempotencyConflict)
    );

    worker
        .join()
        .expect("launch worker should not panic")
        .expect("original launch should retain the reservation and finish");
}

#[test]
fn altered_lease_receipt_cannot_authorize_backend_cleanup() {
    let backend = CoordinatorEdgeBackend::new(false, false);
    let terminate_calls = Arc::clone(&backend.terminate_calls);
    let coordinator = ApplicationServiceCoordinator::new(backend);
    let owner = owner();
    let lease = coordinator
        .launch_at(&owner, &request(), &policy(), 1_780_000_000)
        .expect("test lease should launch");

    let mut altered_value = serde_json::to_value(&lease).expect("lease should serialize");
    altered_value["sandbox_id"] = json!("sandbox-attacker-substitution");
    let altered: ApplicationServiceLease =
        serde_json::from_value(altered_value).expect("altered receipt should remain schema-valid");

    assert_eq!(
        coordinator.terminate_at(&owner, &altered, 1_780_000_010),
        Err(ApplicationServiceCoordinatorError::LeaseMismatch)
    );
    assert_eq!(terminate_calls.load(Ordering::SeqCst), 0);

    assert_eq!(
        coordinator.terminate_at(&owner, &lease, 1_780_000_011),
        Err(ApplicationServiceCoordinatorError::Backend(
            ApplicationServiceError::CleanupFailed
        ))
    );
    assert_eq!(terminate_calls.load(Ordering::SeqCst), 1);
}

#[test]
fn terminating_lease_rejects_parallel_termination_and_relaunch() {
    let backend = CoordinatorEdgeBackend::new(false, true);
    let terminate_started = Arc::clone(&backend.terminate_started);
    let coordinator = Arc::new(ApplicationServiceCoordinator::new(backend));
    let owner = owner();
    let lease = coordinator
        .launch_at(&owner, &request(), &policy(), 1_780_000_000)
        .expect("test lease should launch");

    let worker_coordinator = Arc::clone(&coordinator);
    let worker_owner = owner.clone();
    let worker_lease = lease.clone();
    let worker = thread::spawn(move || {
        worker_coordinator.terminate_at(&worker_owner, &worker_lease, 1_780_000_010)
    });

    wait_until(&terminate_started);
    assert_eq!(
        coordinator.terminate_at(&owner, &lease, 1_780_000_011),
        Err(ApplicationServiceCoordinatorError::TerminationInProgress)
    );
    assert_eq!(
        coordinator.launch_at(&owner, &request(), &policy(), 1_780_000_012),
        Err(ApplicationServiceCoordinatorError::TerminationInProgress)
    );

    assert_eq!(
        worker.join().expect("termination worker should not panic"),
        Err(ApplicationServiceCoordinatorError::Backend(
            ApplicationServiceError::CleanupFailed
        ))
    );
}

#[test]
fn changed_payload_cannot_reuse_an_active_idempotency_key() {
    let coordinator = ApplicationServiceCoordinator::new(CoordinatorEdgeBackend::new(false, false));
    let owner = owner();
    coordinator
        .launch_at(&owner, &request(), &policy(), 1_780_000_000)
        .expect("original launch should become active");

    let mut changed = request();
    changed.command.push("--different".to_owned());
    assert_eq!(
        coordinator.launch_at(&owner, &changed, &policy(), 1_780_000_001),
        Err(ApplicationServiceCoordinatorError::IdempotencyConflict)
    );
}

#[test]
fn changed_payload_cannot_reuse_a_terminating_idempotency_key() {
    let backend = CoordinatorEdgeBackend::new(false, true);
    let terminate_started = Arc::clone(&backend.terminate_started);
    let coordinator = Arc::new(ApplicationServiceCoordinator::new(backend));
    let owner = owner();
    let lease = coordinator
        .launch_at(&owner, &request(), &policy(), 1_780_000_000)
        .expect("original launch should become active");

    let worker_coordinator = Arc::clone(&coordinator);
    let worker_owner = owner.clone();
    let worker_lease = lease.clone();
    let worker = thread::spawn(move || {
        worker_coordinator.terminate_at(&worker_owner, &worker_lease, 1_780_000_010)
    });

    wait_until(&terminate_started);
    let mut changed = request();
    changed.command.push("--different".to_owned());
    assert_eq!(
        coordinator.launch_at(&owner, &changed, &policy(), 1_780_000_011),
        Err(ApplicationServiceCoordinatorError::IdempotencyConflict)
    );

    assert_eq!(
        worker.join().expect("termination worker should not panic"),
        Err(ApplicationServiceCoordinatorError::Backend(
            ApplicationServiceError::CleanupFailed
        ))
    );
}
