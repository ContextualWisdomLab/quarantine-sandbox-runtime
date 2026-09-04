//! Supporting bounded context for launching an approved application as an isolated service.

mod coordinator;

pub use coordinator::{
    ApplicationServiceBackend, ApplicationServiceCoordinator, ApplicationServiceCoordinatorError,
    ExpiredLeaseCleanupResult, LeaseOwnerId,
};

use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::{
    CONTRACT_SCHEMA_VERSION, IsolationPolicy, ResourceRequest,
    sandbox_execution::RuntimeLeaseMetadata,
};

const MAX_REQUEST_IDENTIFIER_BYTES: usize = 128;
const MAX_IMAGE_REFERENCE_BYTES: usize = 512;
const MAX_COMMAND_ARGUMENTS: usize = 64;
const MAX_COMMAND_ARGUMENT_BYTES: usize = 1_024;
const APPLICATION_SERVICE_LEASE_SCHEMA_VERSION: &str = "1.1.0";

/// Service protocol exposed on the consumer-visible loopback endpoint.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ServiceProtocol {
    /// Plain TCP service whose application protocol is consumer-defined.
    Tcp,
    /// HTTP service; readiness uses a bounded TCP connect in the P0 adapter.
    Http,
}

impl ServiceProtocol {
    /// Return the stable wire representation.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Tcp => "tcp",
            Self::Http => "http",
        }
    }
}

/// Consumer request to start one immutable application image as an isolated service.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ApplicationServiceRequest {
    /// Wire-contract version, currently `1.0.0`.
    pub schema_version: String,
    /// Opaque consumer idempotency and correlation identifier.
    pub request_id: String,
    /// Registry image reference pinned to an immutable lower-case SHA-256 digest.
    pub image_reference: String,
    /// TCP port exposed by the application inside the sandbox.
    pub container_port: u16,
    /// Consumer-facing protocol metadata.
    pub protocol: ServiceProtocol,
    /// Optional direct argv appended after the image reference; no shell is used.
    pub command: Vec<String>,
    /// Requested resources, which must remain within the operator policy.
    pub resources: ResourceRequest,
}

impl ApplicationServiceRequest {
    /// Validate all untrusted consumer fields against an operator policy.
    ///
    /// # Errors
    ///
    /// Returns [`ApplicationServiceError`] when identity, image, command, port,
    /// or resource bounds do not satisfy the application-service contract.
    pub fn validate(&self, policy: &IsolationPolicy) -> Result<(), ApplicationServiceError> {
        policy.validate()?;
        if self.schema_version != CONTRACT_SCHEMA_VERSION {
            return Err(ApplicationServiceError::UnsupportedSchemaVersion {
                actual_version: self.schema_version.clone(),
            });
        }
        if self.request_id.is_empty()
            || self.request_id.len() > MAX_REQUEST_IDENTIFIER_BYTES
            || self.request_id.chars().any(char::is_control)
        {
            return Err(ApplicationServiceError::InvalidRequestId);
        }
        if !is_digest_pinned_image_reference(&self.image_reference) {
            return Err(ApplicationServiceError::ImageReferenceNotDigestPinned);
        }
        if self.container_port == 0 {
            return Err(ApplicationServiceError::InvalidContainerPort);
        }
        if self.command.len() > MAX_COMMAND_ARGUMENTS {
            return Err(ApplicationServiceError::TooManyCommandArguments {
                maximum_arguments: MAX_COMMAND_ARGUMENTS,
            });
        }
        for (argument_index, argument) in self.command.iter().enumerate() {
            if argument.is_empty()
                || argument.len() > MAX_COMMAND_ARGUMENT_BYTES
                || argument.chars().any(char::is_control)
            {
                return Err(ApplicationServiceError::InvalidCommandArgument { argument_index });
            }
        }
        self.resources.validate_against(policy).map_err(Into::into)
    }
}

/// Loopback-only endpoint returned to an authorized consumer.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ServiceEndpoint {
    host: String,
    port: u16,
    protocol: ServiceProtocol,
}

impl ServiceEndpoint {
    pub(crate) fn loopback(port: u16, protocol: ServiceProtocol) -> Self {
        Self {
            host: "127.0.0.1".to_owned(),
            port,
            protocol,
        }
    }

    /// Return the loopback host name guaranteed by this profile.
    #[must_use]
    pub fn host(&self) -> &str {
        &self.host
    }

    /// Return the random host port published to the consumer.
    #[must_use]
    pub const fn port(&self) -> u16 {
        self.port
    }

    /// Return the declared service protocol.
    #[must_use]
    pub const fn protocol(&self) -> ServiceProtocol {
        self.protocol
    }
}

/// Security-boundary facts guaranteed by the P0 application-service profile.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct IsolationAttestation {
    rootless: bool,
    read_only_root_filesystem: bool,
    all_capabilities_dropped: bool,
    no_new_privileges: bool,
    isolated_user_namespace: bool,
    external_egress_denied: bool,
    loopback_only_publication: bool,
    credentials_available: bool,
}

impl IsolationAttestation {
    pub(crate) const fn p0() -> Self {
        Self {
            rootless: true,
            read_only_root_filesystem: true,
            all_capabilities_dropped: true,
            no_new_privileges: true,
            isolated_user_namespace: true,
            external_egress_denied: true,
            loopback_only_publication: true,
            credentials_available: false,
        }
    }

    /// Whether the container backend was verified to run rootless.
    #[must_use]
    pub const fn rootless(self) -> bool {
        self.rootless
    }

    /// Whether the application root filesystem was mounted read-only.
    #[must_use]
    pub const fn read_only_root_filesystem(self) -> bool {
        self.read_only_root_filesystem
    }

    /// Whether all Linux capabilities were dropped from the application.
    #[must_use]
    pub const fn all_capabilities_dropped(self) -> bool {
        self.all_capabilities_dropped
    }

    /// Whether the no-new-privileges security option was enforced.
    #[must_use]
    pub const fn no_new_privileges(self) -> bool {
        self.no_new_privileges
    }

    /// Whether the application uses an isolated user namespace.
    #[must_use]
    pub const fn isolated_user_namespace(self) -> bool {
        self.isolated_user_namespace
    }

    /// Whether the standard profile denies externally routed application egress.
    #[must_use]
    pub const fn external_egress_denied(self) -> bool {
        self.external_egress_denied
    }

    /// Whether the service was published exclusively on host loopback.
    #[must_use]
    pub const fn loopback_only_publication(self) -> bool {
        self.loopback_only_publication
    }

    /// Whether consumer/provider credentials were made available to the application.
    #[must_use]
    pub const fn credentials_available(self) -> bool {
        self.credentials_available
    }
}

/// Attested lease for one ready isolated application service.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ApplicationServiceLease {
    schema_version: String,
    request_id: String,
    image_reference: String,
    backend_id: String,
    sandbox_id: String,
    network_id: String,
    policy_id: String,
    policy_sha256: String,
    endpoint: ServiceEndpoint,
    started_at_epoch_seconds: u64,
    expires_at_epoch_seconds: u64,
    shutdown_grace_seconds: u32,
    isolation_attestation: IsolationAttestation,
}

impl ApplicationServiceLease {
    pub(crate) fn new(
        request: &ApplicationServiceRequest,
        metadata: RuntimeLeaseMetadata,
        endpoint: ServiceEndpoint,
    ) -> Self {
        Self {
            schema_version: APPLICATION_SERVICE_LEASE_SCHEMA_VERSION.to_owned(),
            request_id: request.request_id.clone(),
            image_reference: request.image_reference.clone(),
            backend_id: metadata.backend_id.to_owned(),
            sandbox_id: metadata.sandbox_id,
            network_id: metadata.network_id,
            policy_id: metadata.policy_id,
            policy_sha256: metadata.policy_sha256,
            endpoint,
            started_at_epoch_seconds: metadata.started_at_epoch_seconds,
            expires_at_epoch_seconds: metadata.expires_at_epoch_seconds,
            shutdown_grace_seconds: metadata.shutdown_grace_seconds,
            isolation_attestation: IsolationAttestation::p0(),
        }
    }

    /// Return the wire-contract version of this lease.
    #[must_use]
    pub fn schema_version(&self) -> &str {
        &self.schema_version
    }

    /// Return the originating consumer request identifier.
    #[must_use]
    pub fn request_id(&self) -> &str {
        &self.request_id
    }

    /// Return the immutable OCI image reference used to create the service.
    #[must_use]
    pub fn image_reference(&self) -> &str {
        &self.image_reference
    }

    /// Return the infrastructure backend identity.
    #[must_use]
    pub fn backend_id(&self) -> &str {
        &self.backend_id
    }

    /// Return the opaque sandbox identifier.
    #[must_use]
    pub fn sandbox_id(&self) -> &str {
        &self.sandbox_id
    }

    /// Return the opaque per-sandbox network identifier.
    #[must_use]
    pub fn network_id(&self) -> &str {
        &self.network_id
    }

    /// Return the operator isolation-policy identifier.
    #[must_use]
    pub fn policy_id(&self) -> &str {
        &self.policy_id
    }

    /// Return the canonical SHA-256 identity of the full effective isolation policy.
    #[must_use]
    pub fn policy_sha256(&self) -> &str {
        &self.policy_sha256
    }

    /// Return the loopback-scoped service endpoint.
    #[must_use]
    pub const fn endpoint(&self) -> &ServiceEndpoint {
        &self.endpoint
    }

    /// Return the start timestamp recorded by the launcher.
    #[must_use]
    pub const fn started_at_epoch_seconds(&self) -> u64 {
        self.started_at_epoch_seconds
    }

    /// Return the absolute lease expiry timestamp.
    #[must_use]
    pub const fn expires_at_epoch_seconds(&self) -> u64 {
        self.expires_at_epoch_seconds
    }

    pub(crate) const fn shutdown_grace_seconds(&self) -> u32 {
        self.shutdown_grace_seconds
    }

    /// Return the P0 isolation attestation.
    #[must_use]
    pub const fn isolation_attestation(&self) -> IsolationAttestation {
        self.isolation_attestation
    }
}

/// Evidence that runtime-owned isolation resources were removed.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CleanupReceipt {
    schema_version: String,
    sandbox_id: String,
    network_id: String,
    container_removed: bool,
    network_removed: bool,
    terminated_at_epoch_seconds: u64,
}

impl CleanupReceipt {
    pub(crate) fn complete(
        lease: &ApplicationServiceLease,
        terminated_at_epoch_seconds: u64,
    ) -> Self {
        Self {
            schema_version: CONTRACT_SCHEMA_VERSION.to_owned(),
            sandbox_id: lease.sandbox_id.clone(),
            network_id: lease.network_id.clone(),
            container_removed: true,
            network_removed: true,
            terminated_at_epoch_seconds,
        }
    }

    /// Return the wire-contract version of this cleanup receipt.
    #[must_use]
    pub fn schema_version(&self) -> &str {
        &self.schema_version
    }

    /// Return the sandbox identifier whose container was removed.
    #[must_use]
    pub fn sandbox_id(&self) -> &str {
        &self.sandbox_id
    }

    /// Return the network identifier removed with the sandbox.
    #[must_use]
    pub fn network_id(&self) -> &str {
        &self.network_id
    }

    /// Whether the application container was removed.
    #[must_use]
    pub const fn container_removed(&self) -> bool {
        self.container_removed
    }

    /// Whether the per-sandbox network was removed.
    #[must_use]
    pub const fn network_removed(&self) -> bool {
        self.network_removed
    }

    /// Return the termination timestamp supplied by the supervisor.
    #[must_use]
    pub const fn terminated_at_epoch_seconds(&self) -> u64 {
        self.terminated_at_epoch_seconds
    }
}

/// Fail-closed application-service validation or runtime error.
#[derive(Clone, Debug, Error, PartialEq, Eq)]
pub enum ApplicationServiceError {
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
    /// The configured Podman executable could not be invoked.
    #[error("Podman invocation failed during {operation}")]
    BackendInvocationFailed {
        /// Stable operation code.
        operation: &'static str,
    },
    /// Podman exceeded the bounded wall-clock budget for a required operation.
    #[error("Podman command timed out during {operation}")]
    BackendCommandTimedOut {
        /// Stable operation code.
        operation: &'static str,
    },
    /// Podman exceeded the bounded retained-output budget for a required operation.
    #[error("Podman command exceeded output limit during {operation}")]
    BackendOutputLimitExceeded {
        /// Stable operation code.
        operation: &'static str,
    },
    /// Podman returned a nonzero exit status for a required operation.
    #[error("Podman command failed during {operation}")]
    BackendCommandFailed {
        /// Stable operation code.
        operation: &'static str,
    },
    /// The backend did not attest that it is running rootless.
    #[error("Podman backend is not rootless")]
    BackendNotRootless,
    /// A required effective isolation control was not positively verified.
    #[error("effective isolation verification failed for {control_name}")]
    IsolationVerificationFailed {
        /// Stable isolation-control code.
        control_name: &'static str,
    },
    /// Podman inspection output was malformed, contradictory, or bound to another sandbox.
    #[error("malformed Podman isolation inspection during {operation}")]
    MalformedIsolationInspection {
        /// Stable inspection operation code.
        operation: &'static str,
    },
    /// Podman returned a service publication other than one IPv4 loopback port.
    #[error("invalid Podman loopback port mapping")]
    InvalidPortMapping,
    /// The service did not become reachable before the bounded readiness deadline.
    #[error("application service readiness timed out")]
    ReadinessTimeout,
    /// Cleanup could not prove removal of all runtime-owned resources.
    #[error("sandbox cleanup failed")]
    CleanupFailed,
}

fn is_digest_pinned_image_reference(value: &str) -> bool {
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
    let Some(first_component) = components.next() else {
        return false;
    };
    if !registry_authority_or_name_is_safe(first_component) {
        return false;
    }
    components.all(|component| {
        !component.is_empty() && component != "." && component != ".." && !component.contains(':')
    })
}

fn registry_authority_or_name_is_safe(component: &str) -> bool {
    let Some((host, port)) = component.rsplit_once(':') else {
        return !component.is_empty();
    };
    !host.is_empty() && !port.is_empty() && port.bytes().all(|byte| byte.is_ascii_digit())
}
