#!/usr/bin/env python3
"""One-shot DDD repair restoring application-service error ownership.

The command-contract move temporarily aliased the Supporting application-service error vocabulary
to Core `SandboxRuntimeError`. That makes service request/readiness/cleanup concepts Core domain
truth and lets command infrastructure name Supporting-context errors. This exact-head fixer restores
an explicit ACL: Core owns generic sandbox/runtime failures, application_service owns its public
service error vocabulary, and infrastructure maps Core failures into the service contract.
"""

from pathlib import Path
import re


def replace_once(path: str, old: str, new: str) -> None:
    target = Path(path)
    text = target.read_text()
    count = text.count(old)
    if count != 1:
        raise SystemExit(f"{path}: expected exactly one literal match, found {count}")
    target.write_text(text.replace(old, new, 1))


def regex_once(path: str, pattern: str, replacement: str) -> None:
    target = Path(path)
    text = target.read_text()
    updated, count = re.subn(pattern, replacement, text, count=1, flags=re.S)
    if count != 1:
        raise SystemExit(f"{path}: expected exactly one regex match, found {count}")
    target.write_text(updated)


def replace_between(path: str, start: str, end: str, old: str, new: str, minimum: int = 1) -> None:
    target = Path(path)
    text = target.read_text()
    start_index = text.find(start)
    if start_index < 0:
        raise SystemExit(f"{path}: start marker not found: {start}")
    end_index = text.find(end, start_index + len(start))
    if end_index < 0:
        raise SystemExit(f"{path}: end marker not found: {end}")
    segment = text[start_index:end_index]
    count = segment.count(old)
    if count < minimum:
        raise SystemExit(
            f"{path}: expected at least {minimum} matches between markers, found {count}: {old}"
        )
    segment = segment.replace(old, new)
    target.write_text(text[:start_index] + segment + text[end_index:])


# Supporting context owns its request limits/validation vocabulary instead of importing Core-private
# command constants and an aliased Core runtime error.
replace_once(
    "src/application_service/mod.rs",
    "use serde::{Deserialize, Serialize};\n",
    "use serde::{Deserialize, Serialize};\nuse thiserror::Error;\n",
)

replace_once(
    "src/application_service/mod.rs",
    '''use crate::{\n    CONTRACT_SCHEMA_VERSION, IsolationPolicy, ResourceRequest,\n    sandbox_execution::{\n        MAX_COMMAND_ARGUMENT_BYTES, MAX_COMMAND_ARGUMENTS, MAX_REQUEST_IDENTIFIER_BYTES,\n        RuntimeLeaseMetadata, SandboxRuntimeError as ApplicationServiceError,\n        VerifiedIsolationState, is_digest_pinned_image_reference,\n    },\n};\n\nconst APPLICATION_SERVICE_LEASE_SCHEMA_VERSION: &str = "1.2.0";\n''',
    '''use crate::{\n    CONTRACT_SCHEMA_VERSION, IsolationPolicy, ResourceRequest,\n    sandbox_execution::{\n        BackendInvocationFailureKind, RuntimeLeaseMetadata, SandboxExecutionError,\n        SandboxRuntimeError, VerifiedIsolationState,\n    },\n};\n\nconst MAX_REQUEST_IDENTIFIER_BYTES: usize = 128;\nconst MAX_IMAGE_REFERENCE_BYTES: usize = 512;\nconst MAX_COMMAND_ARGUMENTS: usize = 64;\nconst MAX_COMMAND_ARGUMENT_BYTES: usize = 1_024;\nconst APPLICATION_SERVICE_LEASE_SCHEMA_VERSION: &str = "1.2.0";\n''',
)

application_error = r'''

/// Fail-closed application-service validation or runtime error owned by this Supporting context.
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
    /// The runtime could not obtain operating-system entropy for an invocation identity.
    #[error("application service runtime identity entropy is unavailable")]
    RuntimeIdentityUnavailable,
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
    /// The backend exceeded the bounded wall-clock budget for a required operation.
    #[error("backend command timed out during {operation}")]
    BackendCommandTimedOut {
        /// Stable operation code.
        operation: &'static str,
    },
    /// The backend exceeded the bounded retained-output budget for a required operation.
    #[error("backend command exceeded output limit during {operation}")]
    BackendOutputLimitExceeded {
        /// Stable operation code.
        operation: &'static str,
    },
    /// The backend returned a nonzero exit status for a required runtime operation.
    #[error("backend command failed during {operation}")]
    BackendCommandFailed {
        /// Stable operation code.
        operation: &'static str,
    },
    /// The backend did not attest that it is running rootless.
    #[error("application-service backend is not rootless")]
    BackendNotRootless,
    /// A required effective isolation control was not positively verified.
    #[error("effective isolation verification failed for {control_name}")]
    IsolationVerificationFailed {
        /// Stable isolation-control code.
        control_name: &'static str,
    },
    /// Backend inspection output was malformed, contradictory, or bound to another sandbox.
    #[error("malformed application-service isolation inspection during {operation}")]
    MalformedIsolationInspection {
        /// Stable inspection operation code.
        operation: &'static str,
    },
    /// The backend returned a service publication other than one IPv4 loopback port.
    #[error("invalid application-service loopback port mapping")]
    InvalidPortMapping,
    /// The service did not become reachable before the bounded readiness deadline.
    #[error("application service readiness timed out")]
    ReadinessTimeout,
    /// Consumer-visible lease evidence lacks non-serializable runtime cleanup authority.
    #[error("application service cleanup authority is unavailable")]
    CleanupAuthorityUnavailable,
    /// Cleanup could not prove removal of all runtime-owned resources.
    #[error("application-service cleanup failed")]
    CleanupFailed,
}

impl From<SandboxExecutionError> for ApplicationServiceError {
    fn from(error: SandboxExecutionError) -> Self {
        match error {
            SandboxExecutionError::InvalidPolicy { field_name } => Self::InvalidPolicy { field_name },
            SandboxExecutionError::ResourceLimitExceeded { resource_name } => {
                Self::ResourceLimitExceeded { resource_name }
            }
        }
    }
}

impl From<SandboxRuntimeError> for ApplicationServiceError {
    fn from(error: SandboxRuntimeError) -> Self {
        match error {
            SandboxRuntimeError::InvalidPolicy { field_name } => Self::InvalidPolicy { field_name },
            SandboxRuntimeError::ResourceLimitExceeded { resource_name } => {
                Self::ResourceLimitExceeded { resource_name }
            }
            SandboxRuntimeError::BackendSpawnFailed { operation, failure_kind } => {
                Self::BackendSpawnFailed { operation, failure_kind }
            }
            SandboxRuntimeError::BackendInvocationFailed { operation } => {
                Self::BackendInvocationFailed { operation }
            }
            SandboxRuntimeError::BackendCommandTimedOut { operation } => {
                Self::BackendCommandTimedOut { operation }
            }
            SandboxRuntimeError::BackendOutputLimitExceeded { operation } => {
                Self::BackendOutputLimitExceeded { operation }
            }
            SandboxRuntimeError::BackendCommandFailed { operation } => {
                Self::BackendCommandFailed { operation }
            }
            SandboxRuntimeError::BackendNotRootless => Self::BackendNotRootless,
            SandboxRuntimeError::IsolationVerificationFailed { control_name } => {
                Self::IsolationVerificationFailed { control_name }
            }
            SandboxRuntimeError::MalformedIsolationInspection { operation } => {
                Self::MalformedIsolationInspection { operation }
            }
            SandboxRuntimeError::CleanupFailed => Self::CleanupFailed,
        }
    }
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

    let first_component = repository
        .split_once('/')
        .map_or(repository, |(component, _)| component);
    if !registry_authority_or_name_is_safe(first_component) {
        return false;
    }
    repository
        .split('/')
        .skip(1)
        .all(|component| component != "." && component != ".." && !component.contains(':'))
}

fn registry_authority_or_name_is_safe(component: &str) -> bool {
    let Some((host, port)) = component.rsplit_once(':') else {
        return true;
    };
    !host.is_empty() && !port.is_empty() && port.bytes().all(|byte| byte.is_ascii_digit())
}
'''

app_path = Path("src/application_service/mod.rs")
app_text = app_path.read_text()
if "pub enum ApplicationServiceError" in app_text:
    raise SystemExit("src/application_service/mod.rs: application-service error already has a local owner")
app_path.write_text(app_text.rstrip() + application_error + "\n")

# Core keeps only profile-neutral policy/resource/backend/isolation failures used by Core command
# execution. Service request/readiness/cleanup-authority vocabulary is no longer Core truth.
core_runtime_error = r'''/// Profile-neutral fail-closed runtime failures owned by sandbox_execution Core.
#[derive(Clone, Debug, Error, PartialEq, Eq)]
pub enum SandboxRuntimeError {
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
    /// Cleanup could not prove removal of all runtime-owned resources.
    #[error("sandbox cleanup failed")]
    CleanupFailed,
}

impl From<SandboxExecutionError> for SandboxRuntimeError {'''
regex_once(
    "src/sandbox_execution/mod.rs",
    r"/// Shared fail-closed runtime failure taxonomy for sandbox execution adapters\..*?pub enum SandboxRuntimeError \{.*?\n\}\n\nimpl From<SandboxExecutionError> for SandboxRuntimeError \{",
    core_runtime_error,
)

replace_once(
    "src/lib.rs",
    '''    ApplicationServiceBackend, ApplicationServiceCoordinator, ApplicationServiceCoordinatorError,\n    ApplicationServiceLease, ApplicationServiceRequest, CleanupReceipt, ExpiredLeaseCleanupResult,\n    IsolationAttestation, LeaseOwnerId, ServiceEndpoint, ServiceProtocol,\n''',
    '''    ApplicationServiceBackend, ApplicationServiceCoordinator, ApplicationServiceCoordinatorError,\n    ApplicationServiceError, ApplicationServiceLease, ApplicationServiceRequest, CleanupReceipt,\n    ExpiredLeaseCleanupResult, IsolationAttestation, LeaseOwnerId, ServiceEndpoint, ServiceProtocol,\n''',
)
replace_once(
    "src/lib.rs",
    '''/// Compatibility name retained for the application-service public error contract.\npub use sandbox_execution::SandboxRuntimeError as ApplicationServiceError;\n''',
    "",
)

# Command infrastructure speaks only Core runtime failures. Supporting service methods may still
# use ApplicationServiceError, with `From<SandboxRuntimeError>` as their ACL.
replace_once(
    "src/infrastructure/podman.rs",
    '''use crate::sandbox_execution::{CommandExecutionOutcome, RuntimeLeaseMetadata};\n''',
    '''use crate::sandbox_execution::{\n    CommandExecutionOutcome, RuntimeLeaseMetadata, SandboxRuntimeError,\n};\n''',
)
replace_between(
    "src/infrastructure/podman.rs",
    "    fn run_command_with_binding_at",
    "    fn verify_effective_isolation",
    "ApplicationServiceError",
    "SandboxRuntimeError",
    1,
)
replace_between(
    "src/infrastructure/podman.rs",
    "    fn checked_output(",
    "    fn command_succeeded(",
    "ApplicationServiceError",
    "SandboxRuntimeError",
    1,
)
replace_between(
    "src/infrastructure/podman.rs",
    "fn verify_command_container_configuration(",
    "fn map_bounded_command_error(",
    "ApplicationServiceError",
    "SandboxRuntimeError",
    1,
)
replace_between(
    "src/infrastructure/podman.rs",
    "fn map_bounded_command_error(",
    "pub(super) fn classify_spawn_failure",
    "ApplicationServiceError",
    "SandboxRuntimeError",
    1,
)
replace_between(
    "src/infrastructure/podman.rs",
    "fn validate_backend_security(",
    "fn effective_lsm_verified(",
    "ApplicationServiceError",
    "SandboxRuntimeError",
    1,
)
replace_between(
    "src/infrastructure/podman.rs",
    "fn parse_process_security_top(",
    "/// Confirm the container was created with an isolated",
    "ApplicationServiceError",
    "SandboxRuntimeError",
    1,
)
replace_between(
    "src/infrastructure/podman.rs",
    "fn require_control(",
    "fn parse_backend_identifier(",
    "ApplicationServiceError",
    "SandboxRuntimeError",
    1,
)
replace_between(
    "src/infrastructure/podman.rs",
    "fn read_command_create_receipt(",
    "fn runtime_epoch_seconds(",
    "ApplicationServiceError",
    "SandboxRuntimeError",
    1,
)

# All command-specific explicit wrappers in helper/tests must use the Core error taxonomy.
podman = Path("src/infrastructure/podman.rs")
podman_text = podman.read_text()
podman_text = podman_text.replace(
    "CommandExecutionError::Backend(ApplicationServiceError::",
    "CommandExecutionError::Backend(SandboxRuntimeError::",
)
# The bounded-command mapper's direct equality tests are outside the command helper segment.
podman_text = podman_text.replace(
    "map_bounded_command_error(operation, BoundedCommandError::Timeout),\n            ApplicationServiceError::",
    "map_bounded_command_error(operation, BoundedCommandError::Timeout),\n            SandboxRuntimeError::",
)
podman_text = podman_text.replace(
    "map_bounded_command_error(operation, BoundedCommandError::OutputLimit),\n            ApplicationServiceError::",
    "map_bounded_command_error(operation, BoundedCommandError::OutputLimit),\n            SandboxRuntimeError::",
)
podman_text = podman_text.replace(
    "map_bounded_command_error(operation, BoundedCommandError::Spawn(ErrorKind::NotFound)),\n            ApplicationServiceError::",
    "map_bounded_command_error(operation, BoundedCommandError::Spawn(ErrorKind::NotFound)),\n            SandboxRuntimeError::",
)
podman_text = podman_text.replace(
    "map_bounded_command_error(operation, error),\n                ApplicationServiceError::",
    "map_bounded_command_error(operation, error),\n                SandboxRuntimeError::",
)
# Test module needs the Core runtime error name for the assertions above.
podman_text = podman_text.replace(
    "use crate::{ApplicationServiceError, BackendInvocationFailureKind, CommandExecutionError};",
    "use crate::{\n        ApplicationServiceError, BackendInvocationFailureKind, CommandExecutionError,\n        SandboxRuntimeError,\n    };",
)
podman.write_text(podman_text)

# Runtime-gate binding is a Core bounded-command adapter and must not name the Supporting context's
# public error vocabulary at all.
gate = Path("src/infrastructure/podman_runtime_gate_binding.rs")
gate_text = gate.read_text()
if "ApplicationServiceError" not in gate_text:
    raise SystemExit("podman_runtime_gate_binding.rs: expected Supporting error reach-through")
gate_text = gate_text.replace("ApplicationServiceError", "SandboxRuntimeError")
gate.write_text(gate_text)

# Record the repaired context boundary in the live architecture document.
replace_once(
    "docs/ARCHITECTURE.md",
    '''- `artifact_analysis` and `application_service` may depend on public Core sandbox concepts; they do not depend on each other.\n''',
    '''- `artifact_analysis` and `application_service` may depend on public Core sandbox concepts; they do not depend on each other. Supporting-context request/error vocabularies stay locally owned and map Core runtime failures through explicit ACLs.\n''',
)
