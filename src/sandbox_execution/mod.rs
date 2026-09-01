//! Core sandbox-execution resource and isolation policy values.

mod podman;

pub use podman::{PodmanLaunchPlan, RootlessPodmanAdapter};

use serde::{Deserialize, Serialize};

use crate::ApplicationServiceError;

const MAX_POLICY_IDENTIFIER_BYTES: usize = 128;

/// Resource budget requested by one isolated workload.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ResourceRequest {
    /// Maximum resident memory available to the workload.
    pub memory_bytes: u64,
    /// CPU allowance in thousandths of one CPU core.
    pub cpu_millicores: u32,
    /// Maximum process count visible inside the sandbox.
    pub maximum_processes: u32,
    /// Maximum lifetime of the sandbox lease.
    pub lease_seconds: u32,
    /// Maximum ephemeral `/tmp` size.
    pub tmpfs_bytes: u64,
}

impl ResourceRequest {
    pub(crate) fn validate_against(
        &self,
        policy: &IsolationPolicy,
    ) -> Result<(), ApplicationServiceError> {
        for (resource_name, invalid) in [
            ("memory_bytes", self.memory_bytes == 0 || self.memory_bytes > policy.maximum_memory_bytes),
            (
                "cpu_millicores",
                self.cpu_millicores == 0 || self.cpu_millicores > policy.maximum_cpu_millicores,
            ),
            (
                "maximum_processes",
                self.maximum_processes == 0
                    || self.maximum_processes > policy.maximum_processes,
            ),
            (
                "lease_seconds",
                self.lease_seconds == 0 || self.lease_seconds > policy.maximum_lease_seconds,
            ),
            ("tmpfs_bytes", self.tmpfs_bytes == 0 || self.tmpfs_bytes > policy.maximum_tmpfs_bytes),
        ] {
            if invalid {
                return Err(ApplicationServiceError::ResourceLimitExceeded { resource_name });
            }
        }
        Ok(())
    }
}

/// Operator-owned maximum isolation and readiness policy.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct IsolationPolicy {
    /// Stable identifier included in sandbox receipts and deterministic plans.
    pub policy_id: String,
    /// Maximum memory request accepted from a consumer.
    pub maximum_memory_bytes: u64,
    /// Maximum CPU request accepted from a consumer.
    pub maximum_cpu_millicores: u32,
    /// Maximum process count accepted from a consumer.
    pub maximum_processes: u32,
    /// Maximum sandbox lease accepted from a consumer.
    pub maximum_lease_seconds: u32,
    /// Maximum ephemeral tmpfs request accepted from a consumer.
    pub maximum_tmpfs_bytes: u64,
    /// Maximum time to wait for a launched service to become reachable.
    pub readiness_timeout_millis: u64,
    /// Delay between bounded readiness probes.
    pub readiness_poll_interval_millis: u64,
    /// Grace period offered to a service before forced cleanup.
    pub shutdown_grace_seconds: u32,
    /// Numeric non-root user identifier used inside the workload.
    pub run_as_user_id: u32,
    /// Numeric non-root group identifier used inside the workload.
    pub run_as_group_id: u32,
}

impl IsolationPolicy {
    /// Validate the operator policy before it can authorize a workload.
    ///
    /// # Errors
    ///
    /// Returns [`ApplicationServiceError`] when a bound is zero, internally
    /// inconsistent, or the policy identity is malformed.
    pub fn validate(&self) -> Result<(), ApplicationServiceError> {
        if self.policy_id.is_empty()
            || self.policy_id.len() > MAX_POLICY_IDENTIFIER_BYTES
            || self.policy_id.chars().any(char::is_control)
        {
            return Err(ApplicationServiceError::InvalidPolicy {
                field_name: "policy_id",
            });
        }

        for (field_name, invalid) in [
            ("maximum_memory_bytes", self.maximum_memory_bytes == 0),
            ("maximum_cpu_millicores", self.maximum_cpu_millicores == 0),
            ("maximum_processes", self.maximum_processes == 0),
            ("maximum_lease_seconds", self.maximum_lease_seconds == 0),
            ("maximum_tmpfs_bytes", self.maximum_tmpfs_bytes == 0),
            ("readiness_timeout_millis", self.readiness_timeout_millis == 0),
            (
                "readiness_poll_interval_millis",
                self.readiness_poll_interval_millis == 0
                    || self.readiness_poll_interval_millis > self.readiness_timeout_millis,
            ),
            ("shutdown_grace_seconds", self.shutdown_grace_seconds == 0),
            ("run_as_user_id", self.run_as_user_id == 0),
            ("run_as_group_id", self.run_as_group_id == 0),
        ] {
            if invalid {
                return Err(ApplicationServiceError::InvalidPolicy { field_name });
            }
        }
        Ok(())
    }
}
