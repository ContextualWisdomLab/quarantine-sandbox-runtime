//! Coverage for termination requests while an application-service launch owns the lease key.

use std::{
    sync::{
        Arc, Barrier,
        atomic::{AtomicUsize, Ordering},
    },
    thread,
};

use quarantine_sandbox_runtime::{
    ApplicationServiceBackend, ApplicationServiceCoordinator, ApplicationServiceCoordinatorError,
    ApplicationServiceError, ApplicationServiceLease, ApplicationServiceRequest, CleanupReceipt,
    IsolationPolicy, LeaseOwnerId, ResourceRequest, ServiceProtocol,
};
use serde_json::json;

struct BlockingLaunchBackend {
    launch_entered: Arc<Barrier>,
    launch_resume: Arc<Barrier>,
    terminate_calls: Arc<AtomicUsize>,
}

impl ApplicationServiceBackend for BlockingLaunchBackend {
    fn launch_at(
        &self,
        request: &ApplicationServiceRequest,
        policy: &IsolationPolicy,
        started_at_epoch_seconds: u64,
    ) -> Result<ApplicationServiceLease, ApplicationServiceError> {
        self.launch_entered.wait();
        self.launch_resume.wait();
        Ok(lease_fixture(request, policy, started_at_epoch_seconds))
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

fn request() -> ApplicationServiceRequest {
    ApplicationServiceRequest {
        schema_version: "1.0.0".to_owned(),
        request_id: "terminate_during_launch".to_owned(),
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

fn policy() -> IsolationPolicy {
    IsolationPolicy {
        policy_id: "terminate_during_launch_policy_v1".to_owned(),
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

fn owner() -> LeaseOwnerId {
    LeaseOwnerId::new("urn:cwl:agent:terminate-during-launch")
        .expect("test owner identity should satisfy the bounded contract")
}

fn lease_fixture(
    request: &ApplicationServiceRequest,
    policy: &IsolationPolicy,
    started_at_epoch_seconds: u64,
) -> ApplicationServiceLease {
    serde_json::from_value(json!({
        "schema_version": "1.2.0",
        "request_id": request.request_id.clone(),
        "image_reference": request.image_reference.clone(),
        "backend_id": "terminate_during_launch_backend",
        "backend_version": "test-1",
        "sandbox_id": "sandbox-terminate-during-launch",
        "network_id": "network-terminate-during-launch",
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
    .expect("test lease should satisfy the public lease contract")
}

#[test]
fn termination_during_launch_fails_before_backend_cleanup() {
    let launch_entered = Arc::new(Barrier::new(2));
    let launch_resume = Arc::new(Barrier::new(2));
    let terminate_calls = Arc::new(AtomicUsize::new(0));
    let coordinator = Arc::new(ApplicationServiceCoordinator::new(BlockingLaunchBackend {
        launch_entered: Arc::clone(&launch_entered),
        launch_resume: Arc::clone(&launch_resume),
        terminate_calls: Arc::clone(&terminate_calls),
    }));
    let owner = owner();
    let launch_request = request();
    let launch_policy = policy();
    let worker_coordinator = Arc::clone(&coordinator);
    let worker_owner = owner.clone();
    let worker_request = launch_request.clone();
    let worker_policy = launch_policy.clone();
    let worker = thread::spawn(move || {
        worker_coordinator.launch_at(
            &worker_owner,
            &worker_request,
            &worker_policy,
            1_780_000_000,
        )
    });

    launch_entered.wait();
    let presented_lease = lease_fixture(&launch_request, &launch_policy, 1_780_000_000);
    assert_eq!(
        coordinator.terminate_at(&owner, &presented_lease, 1_780_000_001),
        Err(ApplicationServiceCoordinatorError::LaunchInProgress)
    );
    assert_eq!(
        terminate_calls.load(Ordering::SeqCst),
        0,
        "a reserved launch key must reject termination before backend cleanup"
    );

    launch_resume.wait();
    worker
        .join()
        .expect("launch worker should not panic")
        .expect("original launch should remain authoritative and complete");
}