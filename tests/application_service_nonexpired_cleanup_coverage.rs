//! Coverage for the cleanup scheduler's non-expired lease boundary.
//!
//! Cleanup selection must leave a still-valid lease active and must not invoke the backend merely
//! because the registry is being swept. This is a lifecycle invariant, not a timing shortcut.

use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
};

use quarantine_sandbox_runtime::{
    ApplicationServiceBackend, ApplicationServiceCoordinator, ApplicationServiceError,
    ApplicationServiceLease, ApplicationServiceRequest, CleanupReceipt, IsolationPolicy,
    LeaseOwnerId, ResourceRequest, ServiceProtocol,
};
use serde_json::json;

#[derive(Clone)]
struct NonexpiredCleanupBackend {
    terminate_calls: Arc<AtomicUsize>,
}

impl ApplicationServiceBackend for NonexpiredCleanupBackend {
    fn launch_at(
        &self,
        request: &ApplicationServiceRequest,
        policy: &IsolationPolicy,
        started_at_epoch_seconds: u64,
    ) -> Result<ApplicationServiceLease, ApplicationServiceError> {
        serde_json::from_value(json!({
            "schema_version": "1.2.0",
            "request_id": request.request_id.clone(),
            "image_reference": request.image_reference.clone(),
            "backend_id": "nonexpired_cleanup_backend",
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
            operation: "nonexpired_cleanup_lease_decode",
        })
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

fn policy() -> IsolationPolicy {
    IsolationPolicy {
        policy_id: "nonexpired_cleanup_policy_v1".to_owned(),
        maximum_memory_bytes: 512 * 1024 * 1024,
        maximum_cpu_millicores: 2_000,
        maximum_processes: 64,
        maximum_lease_seconds: 300,
        maximum_tmpfs_bytes: 64 * 1024 * 1024,
        readiness_timeout_millis: 1_000,
        readiness_poll_interval_millis: 10,
        shutdown_grace_seconds: 2,
        run_as_user_id: 65_532,
        run_as_group_id: 65_532,
    }
}

fn request() -> ApplicationServiceRequest {
    ApplicationServiceRequest {
        schema_version: "1.0.0".to_owned(),
        request_id: "nonexpired-cleanup-request".to_owned(),
        image_reference: format!("localhost/cwl/tool@sha256:{}", "d".repeat(64)),
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

#[test]
fn cleanup_sweep_does_not_touch_a_lease_before_its_expiry() {
    let terminate_calls = Arc::new(AtomicUsize::new(0));
    let coordinator = ApplicationServiceCoordinator::new(NonexpiredCleanupBackend {
        terminate_calls: Arc::clone(&terminate_calls),
    });
    let owner = LeaseOwnerId::new("urn:cwl:agent:nonexpired-cleanup")
        .expect("test owner should satisfy the bounded contract");
    let started_at = 1_780_000_400;

    let lease = coordinator
        .launch_at(&owner, &request(), &policy(), started_at)
        .expect("test lease should launch");
    assert_eq!(lease.expires_at_epoch_seconds(), started_at + 60);

    let outcomes = coordinator
        .cleanup_expired_at(started_at + 59)
        .expect("non-expired cleanup sweep should retain available registry state");

    assert!(outcomes.is_empty());
    assert_eq!(terminate_calls.load(Ordering::SeqCst), 0);
    assert_eq!(
        coordinator
            .launch_at(&owner, &request(), &policy(), started_at)
            .expect("identical retry should still replay the active lease"),
        lease
    );
}
