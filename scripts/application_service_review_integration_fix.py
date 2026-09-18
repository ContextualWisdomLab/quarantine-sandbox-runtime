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

replace_once(
    "src/application_service/mod.rs",
    '''    /// Return runtime-private cleanup authority when this lease originated in this process.\n    pub(crate) const fn cleanup_authority(&self) -> Option<&ApplicationServiceCleanupAuthority> {\n        self.cleanup_authority.as_ref()\n    }\n\n    /// Return effective isolation evidence verified by the runtime backend.\n''',
    '''    /// Return runtime-private cleanup authority when this lease originated in this process.\n    pub(crate) const fn cleanup_authority(&self) -> Option<&ApplicationServiceCleanupAuthority> {\n        self.cleanup_authority.as_ref()\n    }\n\n    /// Compare only caller-visible lease receipt evidence, excluding runtime-private authority.\n    pub(crate) fn matches_public_receipt(&self, other: &Self) -> bool {\n        self.schema_version == other.schema_version\n            && self.request_id == other.request_id\n            && self.image_reference == other.image_reference\n            && self.backend_id == other.backend_id\n            && self.backend_version == other.backend_version\n            && self.sandbox_id == other.sandbox_id\n            && self.network_id == other.network_id\n            && self.policy_id == other.policy_id\n            && self.policy_sha256 == other.policy_sha256\n            && self.endpoint == other.endpoint\n            && self.started_at_epoch_seconds == other.started_at_epoch_seconds\n            && self.expires_at_epoch_seconds == other.expires_at_epoch_seconds\n            && self.shutdown_grace_seconds == other.shutdown_grace_seconds\n            && self.isolation_attestation == other.isolation_attestation\n    }\n\n    /// Return effective isolation evidence verified by the runtime backend.\n''',
)

replace_once(
    "src/application_service/coordinator.rs",
    '''                    if registered_lease != lease {\n                        return Err(ApplicationServiceCoordinatorError::LeaseMismatch);\n                    }\n''',
    '''                    if !registered_lease.matches_public_receipt(lease) {\n                        return Err(ApplicationServiceCoordinatorError::LeaseMismatch);\n                    }\n''',
)
