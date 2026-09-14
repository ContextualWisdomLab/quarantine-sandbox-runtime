#!/usr/bin/env python3
"""One-shot review repair for the application-service owner integration.

This script runs after the owner, identity, and cleanup-receipt fixers. It closes review findings
that span the generated production candidate and coordinator-only test fixtures, then is deleted
with the rest of the source-fix machinery before publication.
"""

from pathlib import Path


def replace_once(path: str, old: str, new: str) -> None:
    target = Path(path)
    text = target.read_text()
    count = text.count(old)
    if count != 1:
        raise SystemExit(f"{path}: expected exactly one literal match, found {count}")
    target.write_text(text.replace(old, new, 1))


replace_once(
    "src/infrastructure/podman.rs",
    '''            Ok(None) => stdout_container_id,\n''',
    '''            Ok(None) => {\n                self.cleanup_acquired_container(&plan, &stdout_container_id)?;\n                return Err(ApplicationServiceError::MalformedIsolationInspection {\n                    operation: "container_create_receipt",\n                });\n            }\n''',
)

replace_once(
    "src/application_service/coordinator.rs",
    '''            Ok(ApplicationServiceLease::new(\n                request,\n                crate::sandbox_execution::RuntimeLeaseMetadata {\n                    backend_id: "test_backend",\n                    backend_version: "0.0.0-test".to_owned(),\n                    sandbox_id: "sandbox-registration-gap".to_owned(),\n                    network_id: "network-registration-gap".to_owned(),\n                    policy_id: policy.policy_id.clone(),\n                    policy_sha256: policy.effective_policy_sha256(),\n                    started_at_epoch_seconds,\n                    expires_at_epoch_seconds: started_at_epoch_seconds\n                        + u64::from(request.resources.lease_seconds),\n                    shutdown_grace_seconds: policy.shutdown_grace_seconds,\n                    isolation_state: verified_isolation_state(),\n                },\n                ServiceEndpoint::loopback(45_321, request.protocol),\n            ))\n''',
    '''            Ok(ApplicationServiceLease::new_with_cleanup_sandbox_id(\n                request,\n                crate::sandbox_execution::RuntimeLeaseMetadata {\n                    backend_id: "test_backend",\n                    backend_version: "0.0.0-test".to_owned(),\n                    sandbox_id: "sandbox-registration-gap".to_owned(),\n                    network_id: "network-registration-gap".to_owned(),\n                    policy_id: policy.policy_id.clone(),\n                    policy_sha256: policy.effective_policy_sha256(),\n                    started_at_epoch_seconds,\n                    expires_at_epoch_seconds: started_at_epoch_seconds\n                        + u64::from(request.resources.lease_seconds),\n                    shutdown_grace_seconds: policy.shutdown_grace_seconds,\n                    isolation_state: verified_isolation_state(),\n                },\n                "sandbox-registration-gap".to_owned(),\n                ServiceEndpoint::loopback(45_321, request.protocol),\n            ))\n''',
)

replace_once(
    "src/application_service/coordinator.rs",
    '''            Ok(ApplicationServiceLease::new(\n                request,\n                crate::sandbox_execution::RuntimeLeaseMetadata {\n                    backend_id: "test_backend",\n                    backend_version: "0.0.0-test".to_owned(),\n                    sandbox_id: "sandbox-cleanup-failure".to_owned(),\n                    network_id: "network-cleanup-failure".to_owned(),\n                    policy_id: policy.policy_id.clone(),\n                    policy_sha256: policy.effective_policy_sha256(),\n                    started_at_epoch_seconds,\n                    expires_at_epoch_seconds: started_at_epoch_seconds\n                        + u64::from(request.resources.lease_seconds),\n                    shutdown_grace_seconds: policy.shutdown_grace_seconds,\n                    isolation_state: verified_isolation_state(),\n                },\n                ServiceEndpoint::loopback(45_322, request.protocol),\n            ))\n''',
    '''            Ok(ApplicationServiceLease::new_with_cleanup_sandbox_id(\n                request,\n                crate::sandbox_execution::RuntimeLeaseMetadata {\n                    backend_id: "test_backend",\n                    backend_version: "0.0.0-test".to_owned(),\n                    sandbox_id: "sandbox-cleanup-failure".to_owned(),\n                    network_id: "network-cleanup-failure".to_owned(),\n                    policy_id: policy.policy_id.clone(),\n                    policy_sha256: policy.effective_policy_sha256(),\n                    started_at_epoch_seconds,\n                    expires_at_epoch_seconds: started_at_epoch_seconds\n                        + u64::from(request.resources.lease_seconds),\n                    shutdown_grace_seconds: policy.shutdown_grace_seconds,\n                    isolation_state: verified_isolation_state(),\n                },\n                "sandbox-cleanup-failure".to_owned(),\n                ServiceEndpoint::loopback(45_322, request.protocol),\n            ))\n''',
)
