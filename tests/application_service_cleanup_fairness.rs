//! Cleanup fairness regression for repeated failed application-service teardown.

use std::sync::{Arc, Mutex};

use quarantine_sandbox_runtime::{
    ApplicationServiceBackend, ApplicationServiceCoordinator, ApplicationServiceError,
    ApplicationServiceLease, ApplicationServiceRequest, CleanupReceipt, IsolationPolicy,
    LeaseOwnerId, ResourceRequest, ServiceProtocol,
};
use serde_json::json;

#[derive(Clone)]
struct FailingCleanupBackend {
    attempted_request_ids: Arc<Mutex<Vec<String>>>,
}

impl ApplicationServiceBackend for FailingCleanupBackend {
    fn launch_at(
        &self,
        request: &ApplicationServiceRequest,
        policy: &IsolationPolicy,
        started_at_epoch_seconds: u64,
    ) -> Result<ApplicationServiceLease, ApplicationServiceError> {
        let lease = json!({
            "schema_version": "1.2.0",
            "request_id": request.request_id.clone(),
            "image_reference": request.image_reference.clone(),
            "backend_id": "failing_cleanup_test_backend",
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
        });
        serde_json::from_value(lease).map_err(|_| ApplicationServiceError::BackendCommandFailed {
            operation: "test_lease_decode",
        })
    }

    fn terminate_at(
        &self,
        lease: &ApplicationServiceLease,
        _terminated_at_epoch_seconds: u64,
    ) -> Result<CleanupReceipt, ApplicationServiceError> {
        self.attempted_request_ids
            .lock()
            .expect("test attempt registry should not be poisoned")
            .push(lease.request_id().to_owned());
        Err(ApplicationServiceError::CleanupFailed)
    }
}

fn policy() -> IsolationPolicy {
    IsolationPolicy {
        policy_id: "cleanup_fairness_policy_v1".to_owned(),
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

fn request(request_id: String) -> ApplicationServiceRequest {
    ApplicationServiceRequest {
        schema_version: "1.0.0".to_owned(),
        request_id,
        image_reference: format!("localhost/cwl/tool@sha256:{}", "f".repeat(64)),
        container_port: 8_080,
        protocol: ServiceProtocol::Http,
        command: vec!["serve".to_owned()],
        resources: ResourceRequest {
            memory_bytes: 256 * 1024 * 1024,
            cpu_millicores: 1_000,
            maximum_processes: 32,
            lease_seconds: 1,
            tmpfs_bytes: 32 * 1024 * 1024,
        },
    }
}

#[test]
fn repeated_cleanup_failures_do_not_starve_other_expired_leases() {
    let attempted_request_ids = Arc::new(Mutex::new(Vec::new()));
    let backend = FailingCleanupBackend {
        attempted_request_ids: Arc::clone(&attempted_request_ids),
    };
    let coordinator = ApplicationServiceCoordinator::new(backend);
    let owner = LeaseOwnerId::new("urn:cwl:agent:contextual-orchestrator")
        .expect("test owner should satisfy the bounded contract");
    let policy = policy();
    let started_at = 1_780_000_000;

    for index in 0..65 {
        coordinator
            .launch_at(
                &owner,
                &request(format!("cleanup_request_{index:03}")),
                &policy,
                started_at,
            )
            .expect("test backend launch should succeed");
    }

    let first_pass = coordinator
        .cleanup_expired_at(started_at + 1)
        .expect("first cleanup pass should access coordinator state");
    assert_eq!(first_pass.len(), 64);
    assert!(first_pass.iter().all(|outcome| outcome.result().is_err()));

    let second_pass = coordinator
        .cleanup_expired_at(started_at + 1)
        .expect("second cleanup pass should access coordinator state");
    assert_eq!(second_pass.len(), 64);
    assert!(
        second_pass
            .iter()
            .any(|outcome| outcome.request_id() == "cleanup_request_064"),
        "a previously unattempted expired lease must be selected before retrying all failed cleanup entries"
    );

    let attempts = attempted_request_ids
        .lock()
        .expect("test attempt registry should remain available");
    assert!(attempts.contains(&"cleanup_request_064".to_owned()));
}
