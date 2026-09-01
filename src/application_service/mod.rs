//! Supporting bounded context for launching an approved application as an isolated service.

use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::{CONTRACT_SCHEMA_VERSION, IsolationPolicy, ResourceRequest};

const MAX_REQUEST_IDENTIFIER_BYTES: usize = 128;
const MAX_IMAGE_REFERENCE_BYTES: usize = 512;
const MAX_COMMAND_ARGUMENTS: usize = 64;
const MAX_COMMAND_ARGUMENT_BYTES: usize = 1_024;

/// Service protocol exposed on the consumer-visible loopback endpoint.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ServiceProtocol {
    /// Plain TCP service whose application protocol is consumer-defined.
    Tcp,
    /// HTTP service; readiness still uses a bounded TCP connect in the P0 adapter.
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
    /// OCI image reference pinned to an immutable lower-case SHA-256 digest.
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
        self.resources.validate_against(policy)
    }
}

/// Fail-closed application-service validation or launch-plan error.
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
}

fn is_digest_pinned_image_reference(value: &str) -> bool {
    if value.is_empty()
        || value.len() > MAX_IMAGE_REFERENCE_BYTES
        || value.chars().any(|character| character.is_control() || character.is_whitespace())
    {
        return false;
    }
    let Some((repository, digest)) = value.rsplit_once("@sha256:") else {
        return false;
    };
    !repository.is_empty()
        && digest.len() == 64
        && digest
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}
