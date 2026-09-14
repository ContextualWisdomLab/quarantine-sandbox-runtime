//! Core sandbox-execution resource, lease, isolation policy, and bounded-command values.

mod bounded_command_execution;

pub use bounded_command_execution::{
    CommandExecutionBackend, CommandExecutionError, CommandExecutionRequest,
    CommandExecutionResult, execute_command,
};
pub(crate) use bounded_command_execution::CommandExecutionOutcome;

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use thiserror::Error;

const MAX_POLICY_IDENTIFIER_BYTES: usize = 128;
pub(crate) const MAX_REQUEST_IDENTIFIER_BYTES: usize = 128;
pub(crate) const MAX_IMAGE_REFERENCE_BYTES: usize = 512;
pub(crate) const MAX_COMMAND_ARGUMENTS: usize = 64;
pub(crate) const MAX_COMMAND_ARGUMENT_BYTES: usize = 1_024;

/// Verification state for one isolation control in an attested sandbox.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum IsolationControlStatus {
    /// The runtime inspected the effective sandbox and verified the control.
    Verified,
    /// The control is explicitly outside the selected isolation profile.
    NotApplicable,
    /// The backend could not prove the control and the workload must not be leased when required.
    Unavailable,
}

impl IsolationControlStatus {
    /// Return whether this control was positively verified.
    #[must_use]
    pub const fn is_verified(self) -> bool {
        matches!(self, Self::Verified)
    }
}

/// Backend-neutral effective isolation facts produced after runtime inspection.
///
/// Infrastructure adapters translate backend-specific inspection payloads into
/// this Core value. Supporting contexts consume it but must not manufacture
/// backend facts from requested configuration alone.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct VerifiedIsolationState {
    pub(crate) rootless: IsolationControlStatus,
    pub(crate) read_only_root_filesystem: IsolationControlStatus,
    pub(crate) all_capabilities_dropped: IsolationControlStatus,
    pub(crate) no_new_privileges: IsolationControlStatus,
    pub(crate) isolated_user_namespace: IsolationControlStatus,
    pub(crate) external_egress_denied: IsolationControlStatus,
    pub(crate) loopback_only_publication: IsolationControlStatus,
    pub(crate) seccomp_enforced: IsolationControlStatus,
    pub(crate) lsm_enforced: IsolationControlStatus,
    pub(crate) resource_limits_verified: IsolationControlStatus,
    pub(crate) credentials_available: bool,
}

impl VerifiedIsolationState {
    /// Return the rootless-execution verification state.
    #[must_use]
    pub const fn rootless_status(self) -> IsolationControlStatus {
        self.rootless
    }

    /// Return whether rootless execution was positively verified.
    #[must_use]
    pub const fn rootless(self) -> bool {
        self.rootless.is_verified()
    }

    /// Return the read-only-root verification state.
    #[must_use]
    pub const fn read_only_root_filesystem_status(self) -> IsolationControlStatus {
        self.read_only_root_filesystem
    }

    /// Return whether the root filesystem was verified read-only.
    #[must_use]
    pub const fn read_only_root_filesystem(self) -> bool {
        self.read_only_root_filesystem.is_verified()
    }

    /// Return the capability-drop verification state.
    #[must_use]
    pub const fn all_capabilities_dropped_status(self) -> IsolationControlStatus {
        self.all_capabilities_dropped
    }

    /// Return whether all application capabilities were verified absent.
    #[must_use]
    pub const fn all_capabilities_dropped(self) -> bool {
        self.all_capabilities_dropped.is_verified()
    }

    /// Return the no-new-privileges verification state.
    #[must_use]
    pub const fn no_new_privileges_status(self) -> IsolationControlStatus {
        self.no_new_privileges
    }

    /// Return whether no-new-privileges was positively verified.
    #[must_use]
    pub const fn no_new_privileges(self) -> bool {
        self.no_new_privileges.is_verified()
    }

    /// Return the user-namespace verification state.
    #[must_use]
    pub const fn isolated_user_namespace_status(self) -> IsolationControlStatus {
        self.isolated_user_namespace
    }

    /// Return whether the workload was verified in an isolated user namespace.
    #[must_use]
    pub const fn isolated_user_namespace(self) -> bool {
        self.isolated_user_namespace.is_verified()
    }

    /// Return the external-egress verification state.
    #[must_use]
    pub const fn external_egress_denied_status(self) -> IsolationControlStatus {
        self.external_egress_denied
    }

    /// Return whether the selected profile was verified to deny external egress.
    #[must_use]
    pub const fn external_egress_denied(self) -> bool {
        self.external_egress_denied.is_verified()
    }

    /// Return the loopback-publication verification state.
    #[must_use]
    pub const fn loopback_only_publication_status(self) -> IsolationControlStatus {
        self.loopback_only_publication
    }

    /// Return whether service publication was verified to be loopback-only.
    #[must_use]
    pub const fn loopback_only_publication(self) -> bool {
        self.loopback_only_publication.is_verified()
    }

    /// Return the seccomp verification state.
    #[must_use]
    pub const fn seccomp_status(self) -> IsolationControlStatus {
        self.seccomp_enforced
    }

    /// Return whether effective seccomp confinement was positively verified.
    #[must_use]
    pub const fn seccomp_enforced(self) -> bool {
        self.seccomp_enforced.is_verified()
    }

    /// Return the Linux-security-module verification state.
    #[must_use]
    pub const fn lsm_status(self) -> IsolationControlStatus {
        self.lsm_enforced
    }

    /// Return whether AppArmor, SELinux, or an equivalent selected LSM was verified.
    #[must_use]
    pub const fn lsm_enforced(self) -> bool {
        self.lsm_enforced.is_verified()
    }

    /// Return the resource-enforcement verification state.
    #[must_use]
    pub const fn resource_limits_status(self) -> IsolationControlStatus {
        self.resource_limits_verified
    }

    /// Return whether effective CPU, memory, and process limits were verified.
    #[must_use]
    pub const fn resource_limits_verified(self) -> bool {
        self.resource_limits_verified.is_verified()
    }

    /// Return whether consumer or provider credentials were available to the workload.
    #[must_use]
    pub const fn credentials_available(self) -> bool {
        self.credentials_available
    }
}

/// Sandbox-execution validation errors owned by the Core bounded context.
#[derive(Clone, Debug, Error, PartialEq, Eq)]
pub enum SandboxExecutionError {
    /// The operator isolation policy is internally invalid.
    #[error("invalid isolation policy field: {field_name}")]
    InvalidPolicy {
        /// Invalid policy field.
        field_name: &'static str,
    },
    /// A consumer requested zero or more resource than the policy permits.
    #[error("sandbox resource request exceeds policy: {resource_name}")]
    ResourceLimitExceeded {
        /// Resource that violated the policy.
        resource_name: &'static str,
    },
}

/// Stable consumer-visible class for an OS process-spawn failure.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BackendInvocationFailureKind {
    /// The configured backend executable does not exist at invocation time.
    NotFound,
    /// The operating system denied permission to execute the backend.
    PermissionDenied,
    /// Local process or memory pressure prevented process creation.
    ResourceExhausted,
    /// Another current or future OS spawn error occurred.
    Other,
}

/// Shared fail-closed runtime failure taxonomy for sandbox execution adapters.
///
/// The public alias `ApplicationServiceError` is retained at the crate boundary
/// for compatibility while bounded-command ownership moves into this Core context.
#[derive(Clone, Debug, Error, PartialEq, Eq)]
pub enum SandboxRuntimeError {
    /// The wire schema is not supported by this runtime.
    #[error("unsupported application-service schema version: {actual_version}")]
    UnsupportedSchemaVersion {
        /// Version supplied by the consumer.
        actual_version: String,
    },
    /// The request identifier is empty, oversized, or contains control text.
    #[error("invalid application-service request identifier")]
    InvalidRequestId,
    /// The image is not pinned to a lower-case SHA-256 digest.
    #[error("application image must be pinned by sha256 digest")]
    ImageReferenceNotDigestPinned,
    /// Container port zero is not a usable TCP service port.
    #[error("invalid container service port")]
    InvalidContainerPort,
    /// The request contains more direct argv entries than the contract permits.
    #[error("too many application command arguments; maximum {maximum_arguments}")]
    TooManyCommandArguments {
        /// Maximum accepted number of direct argv entries.
        maximum_arguments: usize,
    },
    /// A direct argv entry is empty, oversized, or contains control text.
    #[error("invalid application command argument at index {argument_index}")]
    InvalidCommandArgument {
        /// Zero-based argument position.
        argument_index: usize,
    },
    /// The operator isolation policy is internally invalid.
    #[error("invalid isolation policy field: {field_name}")]
    InvalidPolicy {
        /// Invalid policy field.
        field_name: &'static str,
    },
    /// A consumer requested zero or more resource than the policy permits.
    #[error("application resource request exceeds policy: {resource_name}")]
    ResourceLimitExceeded {
        /// Resource that violated the policy.
        resource_name: &'static str,
    },
    /// Adding lease duration to the start timestamp overflowed.
    #[error("application service lease expiry overflow")]
    LeaseExpiryOverflow,
    /// The backend process could not be spawned; the bounded OS failure class is preserved.
    #[error("backend process spawn failed during {operation}: {failure_kind:?}")]
    BackendSpawnFailed {
        /// Stable operation code.
        operation: &'static str,
        /// Bounded failure class; raw errno values and host paths are not exposed.
        failure_kind: BackendInvocationFailureKind,
    },
    /// The configured backend could not complete an invocation after process creation.
    #[error("backend invocation failed during {operation}")]
    BackendInvocationFailed {
        /// Stable operation code.
        operation: &'static str,
    },
    /// The sandbox backend exceeded the bounded wall-clock budget for a required operation.
    #[error("sandbox backend command timed out during {operation}")]
    BackendCommandTimedOut {
        /// Stable operation code.
        operation: &'static str,
    },
    /// The sandbox backend exceeded the bounded retained-output budget for a required operation.
    #[error("sandbox backend command exceeded output limit during {operation}")]
    BackendOutputLimitExceeded {
        /// Stable operation code.
        operation: &'static str,
    },
    /// The sandbox backend returned a nonzero exit status for a required operation.
    #[error("sandbox backend command failed during {operation}")]
    BackendCommandFailed {
        /// Stable operation code.
        operation: &'static str,
    },
    /// The backend did not attest that it is running rootless.
    #[error("sandbox backend is not rootless")]
    BackendNotRootless,
    /// A required effective isolation control was not positively verified.
    #[error("effective isolation verification failed for {control_name}")]
    IsolationVerificationFailed {
        /// Stable isolation-control code.
        control_name: &'static str,
    },
    /// Backend inspection output was malformed, contradictory, or bound to another sandbox.
    #[error("malformed sandbox isolation inspection during {operation}")]
    MalformedIsolationInspection {
        /// Stable inspection operation code.
        operation: &'static str,
    },
    /// The backend returned a service publication other than one IPv4 loopback port.
    #[error("invalid sandbox loopback port mapping")]
    InvalidPortMapping,
    /// The service did not become reachable before the bounded readiness deadline.
    #[error("application service readiness timed out")]
    ReadinessTimeout,
    /// Cleanup could not prove removal of all runtime-owned resources.
    #[error("sandbox cleanup failed")]
    CleanupFailed,
}

impl From<SandboxExecutionError> for SandboxRuntimeError {
    fn from(error: SandboxExecutionError) -> Self {
        match error {
            SandboxExecutionError::InvalidPolicy { field_name } => {
                Self::InvalidPolicy { field_name }
            }
            SandboxExecutionError::ResourceLimitExceeded { resource_name } => {
                Self::ResourceLimitExceeded { resource_name }
            }
        }
    }
}

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
    ) -> Result<(), SandboxExecutionError> {
        for (resource_name, invalid) in [
            (
                "memory_bytes",
                self.memory_bytes == 0 || self.memory_bytes > policy.maximum_memory_bytes,
            ),
            (
                "cpu_millicores",
                self.cpu_millicores == 0 || self.cpu_millicores > policy.maximum_cpu_millicores,
            ),
            (
                "maximum_processes",
                self.maximum_processes == 0 || self.maximum_processes > policy.maximum_processes,
            ),
            (
                "lease_seconds",
                self.lease_seconds == 0 || self.lease_seconds > policy.maximum_lease_seconds,
            ),
            (
                "tmpfs_bytes",
                self.tmpfs_bytes == 0 || self.tmpfs_bytes > policy.maximum_tmpfs_bytes,
            ),
        ] {
            if invalid {
                return Err(SandboxExecutionError::ResourceLimitExceeded { resource_name });
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
    /// Return the canonical SHA-256 identity of every effective policy field.
    ///
    /// The `qsr.isolation-policy.v1` profile frames each ordered field name and value
    /// with an unsigned 64-bit big-endian byte length; numeric values are big-endian.
    #[must_use]
    pub fn effective_policy_sha256(&self) -> String {
        let mut hasher = Sha256::new();
        hash_component(&mut hasher, "profile", b"qsr.isolation-policy.v1");
        hash_component(&mut hasher, "policy_id", self.policy_id.as_bytes());
        hash_component(
            &mut hasher,
            "maximum_memory_bytes",
            &self.maximum_memory_bytes.to_be_bytes(),
        );
        hash_component(
            &mut hasher,
            "maximum_cpu_millicores",
            &self.maximum_cpu_millicores.to_be_bytes(),
        );
        hash_component(
            &mut hasher,
            "maximum_processes",
            &self.maximum_processes.to_be_bytes(),
        );
        hash_component(
            &mut hasher,
            "maximum_lease_seconds",
            &self.maximum_lease_seconds.to_be_bytes(),
        );
        hash_component(
            &mut hasher,
            "maximum_tmpfs_bytes",
            &self.maximum_tmpfs_bytes.to_be_bytes(),
        );
        hash_component(
            &mut hasher,
            "readiness_timeout_millis",
            &self.readiness_timeout_millis.to_be_bytes(),
        );
        hash_component(
            &mut hasher,
            "readiness_poll_interval_millis",
            &self.readiness_poll_interval_millis.to_be_bytes(),
        );
        hash_component(
            &mut hasher,
            "shutdown_grace_seconds",
            &self.shutdown_grace_seconds.to_be_bytes(),
        );
        hash_component(
            &mut hasher,
            "run_as_user_id",
            &self.run_as_user_id.to_be_bytes(),
        );
        hash_component(
            &mut hasher,
            "run_as_group_id",
            &self.run_as_group_id.to_be_bytes(),
        );
        format!("{:x}", hasher.finalize())
    }

    /// Validate the operator policy before it can authorize a workload.
    ///
    /// # Errors
    ///
    /// Returns [`SandboxExecutionError`] when a bound is zero, internally
    /// inconsistent, or the policy identity is malformed.
    pub fn validate(&self) -> Result<(), SandboxExecutionError> {
        if self.policy_id.is_empty()
            || self.policy_id.len() > MAX_POLICY_IDENTIFIER_BYTES
            || self.policy_id.chars().any(char::is_control)
        {
            return Err(SandboxExecutionError::InvalidPolicy {
                field_name: "policy_id",
            });
        }

        for (field_name, invalid) in [
            ("maximum_memory_bytes", self.maximum_memory_bytes == 0),
            ("maximum_cpu_millicores", self.maximum_cpu_millicores == 0),
            ("maximum_processes", self.maximum_processes == 0),
            ("maximum_lease_seconds", self.maximum_lease_seconds == 0),
            ("maximum_tmpfs_bytes", self.maximum_tmpfs_bytes == 0),
            (
                "readiness_timeout_millis",
                self.readiness_timeout_millis == 0,
            ),
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
                return Err(SandboxExecutionError::InvalidPolicy { field_name });
            }
        }
        Ok(())
    }
}

pub(crate) fn is_digest_pinned_image_reference(value: &str) -> bool {
    if value.is_empty()
        || value.len() > MAX_IMAGE_REFERENCE_BYTES
        || value
            .chars()
            .any(|character| character.is_control() || character.is_whitespace())
    {
        return false;
    }
    let Some((repository, digest)) = value.rsplit_once("@sha256:") else {
        return false;
    };
    registry_repository_is_safe(repository)
        && digest.len() == 64
        && digest.bytes().all(|byte| {
            byte.is_ascii_digit() || (byte.is_ascii_lowercase() && byte.is_ascii_hexdigit())
        })
}

fn registry_repository_is_safe(repository: &str) -> bool {
    if repository.is_empty()
        || repository.contains('@')
        || repository.contains("//")
        || repository.starts_with('/')
        || repository.ends_with('/')
    {
        return false;
    }

    let mut components = repository.split('/');
    let first_component = components.next().unwrap_or_default();
    if !registry_authority_or_name_is_safe(first_component) {
        return false;
    }
    components.all(|component| component != "." && component != ".." && !component.contains(':'))
}

fn registry_authority_or_name_is_safe(component: &str) -> bool {
    let Some((host, port)) = component.rsplit_once(':') else {
        return !component.is_empty();
    };
    !host.is_empty() && !port.is_empty() && port.bytes().all(|byte| byte.is_ascii_digit())
}

fn hash_component(hasher: &mut Sha256, name: &str, value: &[u8]) {
    // Rust supports pointer widths up to 64 bits, so usize lengths fit the
    // canonical unsigned 64-bit wire framing without a fallible conversion.
    hasher.update((name.len() as u64).to_be_bytes());
    hasher.update(name.as_bytes());
    hasher.update((value.len() as u64).to_be_bytes());
    hasher.update(value);
}

pub(crate) struct RuntimeLeaseMetadata {
    pub(crate) backend_id: &'static str,
    pub(crate) backend_version: String,
    pub(crate) sandbox_id: String,
    pub(crate) network_id: String,
    pub(crate) policy_id: String,
    pub(crate) policy_sha256: String,
    pub(crate) started_at_epoch_seconds: u64,
    pub(crate) expires_at_epoch_seconds: u64,
    pub(crate) shutdown_grace_seconds: u32,
    pub(crate) isolation_state: VerifiedIsolationState,
}
