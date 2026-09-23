//! Deterministic public-boundary coverage for application-service coordination.
//!
//! These tests exercise coordinator decisions through the public API without
//! changing application-service semantics or reaching into the private registry.

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
        lease_for(request, policy, started_at_epoch_seconds)
    }

    fn terminate_at(
        &self,
        lease: &ApplicationServiceLease,
        terminated_at_epoch_seconds: u64,
    ) -> Result<CleanupReceipt, ApplicationServiceError> {
        self.terminate_calls.fetch_add(1, Ordering::SeqCst);
        cleanup_for(lease, terminated_at_epoch_seconds)
    }
}

#[derive(Clone, Default)]
struct FailingLaunchBackend {
    launch_calls: Arc<AtomicUsize>,
}

impl ApplicationServiceBackend for FailingLaunchBackend {
    fn launch_at(
        &self,
        _request: &ApplicationServiceRequest,
        _policy: &IsolationPolicy,
        _started_at_epoch_seconds: u64,
    ) -> Result<ApplicationServiceLease, ApplicationServiceError> {
        self.launch_calls.fetch_add(1, Ordering::SeqCst);
        Err(ApplicationServiceError::BackendCommandFailed {
            operation: "coordinator_boundary_launch",
        })
    }

    fn terminate_at(
        &self,
        _lease: &ApplicationServiceLease,
        _terminated_at_epoch_seconds: u64,
    ) -> Result<CleanupReceipt, ApplicationServiceError> {
        unreachable!("a failed launch must never make cleanup authority visible")
    }
}

#[derive(Clone, Default)]
struct FailingTerminateBackend {
    terminate_calls: Arc<AtomicUsize>,
}

impl ApplicationServiceBackend for FailingTerminateBackend {
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
        _lease: &ApplicationServiceLease,
        _terminated_at_epoch_seconds: u64,
    ) -> Result<CleanupReceipt, ApplicationServiceError> {
        self.terminate_calls.fetch_add(1, Ordering::SeqCst);
        Err(ApplicationServiceError::CleanupFailed)
    }
}

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
        "backend_id": "coordinator_boundary_backend",
        "backend_version": "test-1",
        "sandbox_id": format!("sandbox-{}-{started_at_epoch_seconds}", request.request_id),
        "network_id": format!("network-{}-{started_at_epoch_seconds}", request.request_id),
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
        operation: "coordinator_boundary_cleanup_receipt_decode",
    })
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
fn active_request_identity_rejects_different_request_content() {
    let backend = CountingBackend::default();
    let launch_calls = Arc::clone(&backend.launch_calls);
    let coordinator = ApplicationServiceCoordinator::new(backend);
    let owner_identity = owner("urn:cwl:agent:coordinator-boundary");
    let request = request();
    let policy = policy();

    coordinator
        .launch_at(&owner_identity, &request, &policy, 1_780_000_000)
        .expect("initial launch should register one active lease");
    let mut conflicting = request.clone();
    conflicting.command = vec!["serve".to_owned(), "--different".to_owned()];

    assert_eq!(
        coordinator.launch_at(&owner_identity, &conflicting, &policy, 1_780_000_001),
        Err(ApplicationServiceCoordinatorError::IdempotencyConflict)
    );
    assert_eq!(launch_calls.load(Ordering::SeqCst), 1);
}

#[test]
fn failed_launch_releases_launching_slot_for_retry() {
    let backend = FailingLaunchBackend::default();
    let launch_calls = Arc::clone(&backend.launch_calls);
    let coordinator = ApplicationServiceCoordinator::new(backend);
    let owner_identity = owner("urn:cwl:agent:coordinator-boundary");
    let request = request();
    let policy = policy();

    for started_at_epoch_seconds in [1_780_000_000, 1_780_000_001] {
        assert!(matches!(
            coordinator.launch_at(&owner_identity, &request, &policy, started_at_epoch_seconds,),
            Err(ApplicationServiceCoordinatorError::Backend(
                ApplicationServiceError::BackendCommandFailed { .. }
            ))
        ));
    }
    assert_eq!(launch_calls.load(Ordering::SeqCst), 2);
}

#[test]
fn synchronized_duplicate_launch_reports_launch_in_progress() {
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
        std::thread::spawn(move || {
            coordinator.launch_at(
                &owner("urn:cwl:agent:coordinator-boundary"),
                &request(),
                &policy(),
                1_780_000_000,
            )
        })
    };

    entered.wait();
    assert_eq!(
        coordinator.launch_at(
            &owner("urn:cwl:agent:coordinator-boundary"),
            &request(),
            &policy(),
            1_780_000_001,
        ),
        Err(ApplicationServiceCoordinatorError::LaunchInProgress)
    );
    release.wait();
    worker
        .join()
        .expect("launch worker must not panic")
        .expect("released backend launch should complete");
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

#[test]
fn registered_owner_rejects_mismatched_lease_receipt() {
    let backend = CountingBackend::default();
    let terminate_calls = Arc::clone(&backend.terminate_calls);
    let coordinator = ApplicationServiceCoordinator::new(backend);
    let owner_identity = owner("urn:cwl:agent:coordinator-boundary");
    let request = request();
    let policy = policy();
    coordinator
        .launch_at(&owner_identity, &request, &policy, 1_780_000_000)
        .expect("test lease should launch");
    let different_lease = lease_for(&request, &policy, 1_780_000_001)
        .expect("independent valid receipt should deserialize");

    assert_eq!(
        coordinator.terminate_at(&owner_identity, &different_lease, 1_780_000_010),
        Err(ApplicationServiceCoordinatorError::LeaseMismatch)
    );
    assert_eq!(terminate_calls.load(Ordering::SeqCst), 0);
}

#[test]
fn successful_termination_removes_registry_entry() {
    let backend = CountingBackend::default();
    let terminate_calls = Arc::clone(&backend.terminate_calls);
    let coordinator = ApplicationServiceCoordinator::new(backend);
    let owner_identity = owner("urn:cwl:agent:coordinator-boundary");
    let request = request();
    let policy = policy();
    let lease = coordinator
        .launch_at(&owner_identity, &request, &policy, 1_780_000_000)
        .expect("test lease should launch");

    coordinator
        .terminate_at(&owner_identity, &lease, 1_780_000_010)
        .expect("successful cleanup should remove the active registry entry");
    assert_eq!(
        coordinator.terminate_at(&owner_identity, &lease, 1_780_000_011),
        Err(ApplicationServiceCoordinatorError::UnknownLease)
    );
    assert_eq!(terminate_calls.load(Ordering::SeqCst), 1);
}

#[test]
fn failed_termination_restores_active_entry_for_retry() {
    let backend = FailingTerminateBackend::default();
    let terminate_calls = Arc::clone(&backend.terminate_calls);
    let coordinator = ApplicationServiceCoordinator::new(backend);
    let owner_identity = owner("urn:cwl:agent:coordinator-boundary");
    let request = request();
    let policy = policy();
    let lease = coordinator
        .launch_at(&owner_identity, &request, &policy, 1_780_000_000)
        .expect("test lease should launch");

    for terminated_at_epoch_seconds in [1_780_000_010, 1_780_000_011] {
        assert_eq!(
            coordinator.terminate_at(&owner_identity, &lease, terminated_at_epoch_seconds,),
            Err(ApplicationServiceCoordinatorError::Backend(
                ApplicationServiceError::CleanupFailed
            ))
        );
    }
    assert_eq!(terminate_calls.load(Ordering::SeqCst), 2);
}

#[test]
fn synchronized_duplicate_termination_reports_termination_in_progress() {
    let entered = Arc::new(Barrier::new(2));
    let release = Arc::new(Barrier::new(2));
    let terminate_calls = Arc::new(AtomicUsize::new(0));
    let coordinator = Arc::new(ApplicationServiceCoordinator::new(
        BlockingTerminateBackend {
            entered: Arc::clone(&entered),
            release: Arc::clone(&release),
            terminate_calls: Arc::clone(&terminate_calls),
        },
    ));
    let owner_identity = owner("urn:cwl:agent:coordinator-boundary");
    let request = request();
    let policy = policy();
    let lease = coordinator
        .launch_at(&owner_identity, &request, &policy, 1_780_000_000)
        .expect("test lease should launch");

    let worker = {
        let coordinator = Arc::clone(&coordinator);
        let owner_identity = owner_identity.clone();
        let lease = lease.clone();
        std::thread::spawn(move || coordinator.terminate_at(&owner_identity, &lease, 1_780_000_010))
    };

    entered.wait();
    assert_eq!(
        coordinator.terminate_at(&owner_identity, &lease, 1_780_000_011),
        Err(ApplicationServiceCoordinatorError::TerminationInProgress)
    );
    release.wait();
    worker
        .join()
        .expect("termination worker must not panic")
        .expect("released backend cleanup should complete");
    assert_eq!(terminate_calls.load(Ordering::SeqCst), 1);
}

#[test]
fn expired_cleanup_skips_live_lease_then_removes_it_at_expiry() {
    let backend = CountingBackend::default();
    let terminate_calls = Arc::clone(&backend.terminate_calls);
    let coordinator = ApplicationServiceCoordinator::new(backend);
    let owner_identity = owner("urn:cwl:agent:coordinator-boundary");
    let request = request();
    let policy = policy();
    let lease = coordinator
        .launch_at(&owner_identity, &request, &policy, 100)
        .expect("test lease should launch");

    assert!(
        coordinator
            .cleanup_expired_at(lease.expires_at_epoch_seconds() - 1)
            .expect("live-lease scan should succeed")
            .is_empty()
    );
    let outcomes = coordinator
        .cleanup_expired_at(lease.expires_at_epoch_seconds())
        .expect("expired cleanup should succeed");
    assert_eq!(outcomes.len(), 1);
    assert_eq!(outcomes[0].lease(), &lease);
    assert!(outcomes[0].result().is_ok());
    assert!(
        coordinator
            .cleanup_expired_at(lease.expires_at_epoch_seconds() + 1)
            .expect("removed lease must not be selected again")
            .is_empty()
    );
    assert_eq!(terminate_calls.load(Ordering::SeqCst), 1);
}
