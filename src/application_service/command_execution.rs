//! Bounded command-to-completion contract: a narrower sibling of the service lease.
//!
//! [`super::ApplicationServiceRequest`]/[`super::ApplicationServiceLease`] model a
//! long-lived, readiness-gated network service: the runtime returns a loopback
//! endpoint once the workload is reachable, and the caller polls or connects to
//! it. A CI-style consumer -- for example a review pipeline that must execute a
//! pull request's own test suite or a proof-of-concept command and report a
//! structured pass/fail -- needs a different shape: run one bounded command
//! inside an isolated sandbox *to completion* and receive its exit status and
//! bounded output. There is no service endpoint to become ready, and a nonzero
//! exit status is an expected, valid outcome the consumer decides how to treat,
//! not a runtime failure.
//!
//! This module adds [`CommandExecutionRequest`]/[`CommandExecutionResult`] and
//! the matching [`CommandExecutionBackend`] port for that narrower contract. It
//! reuses the existing digest-pinned image, identifier, and command-argument
//! validation plus the [`crate::IsolationPolicy`]/[`crate::ResourceRequest`]
//! budget contracts from [`super`] rather than duplicating them, and it does not
//! change the existing service-lease lifecycle at all.
//!
//! No production backend exists yet: see `docs/adr/0007-bounded-command-execution-contract.md`
//! for why this ships as a validated contract plus a fake/test backend only, and
//! `docs/product-technical-gap-baseline.md` for the follow-on work (a real
//! Podman-backed adapter and a CLI/HTTP entrypoint) this unblocks.

use serde::{Deserialize, Serialize};
use thiserror::Error;

use super::ApplicationServiceError;
use crate::{CONTRACT_SCHEMA_VERSION, IsolationPolicy, ResourceRequest, SandboxExecutionError};

#[cfg(test)]
const COMMAND_EXECUTION_RESULT_SCHEMA_VERSION: &str = "1.0.0";

/// Consumer request to run one bounded command to completion inside an isolated sandbox.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CommandExecutionRequest {
    /// Wire-contract version, currently `1.0.0`.
    pub schema_version: String,
    /// Opaque consumer correlation identifier.
    pub request_id: String,
    /// OCI image reference pinned to an immutable lower-case SHA-256 digest.
    pub image_reference: String,
    /// Direct argv executed inside the sandbox; no shell is used.
    pub command: Vec<String>,
    /// Requested resources, which must remain within the operator policy.
    pub resources: ResourceRequest,
}

impl CommandExecutionRequest {
    /// Validate all untrusted consumer fields against an operator policy.
    ///
    /// # Errors
    ///
    /// Returns [`CommandExecutionError`] when the policy, identifier, image,
    /// command, or resource bounds do not satisfy the command-execution
    /// contract.
    pub fn validate(&self, policy: &IsolationPolicy) -> Result<(), CommandExecutionError> {
        policy.validate()?;
        if self.schema_version != CONTRACT_SCHEMA_VERSION {
            return Err(CommandExecutionError::UnsupportedSchemaVersion {
                actual_version: self.schema_version.clone(),
            });
        }
        if self.request_id.is_empty()
            || self.request_id.len() > super::MAX_REQUEST_IDENTIFIER_BYTES
            || self.request_id.chars().any(char::is_control)
        {
            return Err(CommandExecutionError::InvalidRequestId);
        }
        if !super::is_digest_pinned_image_reference(&self.image_reference) {
            return Err(CommandExecutionError::ImageReferenceNotDigestPinned);
        }
        if self.command.is_empty() {
            return Err(CommandExecutionError::EmptyCommand);
        }
        if self.command.len() > super::MAX_COMMAND_ARGUMENTS {
            return Err(CommandExecutionError::TooManyCommandArguments {
                maximum_arguments: super::MAX_COMMAND_ARGUMENTS,
            });
        }
        for (argument_index, argument) in self.command.iter().enumerate() {
            if argument.is_empty()
                || argument.len() > super::MAX_COMMAND_ARGUMENT_BYTES
                || argument.chars().any(char::is_control)
            {
                return Err(CommandExecutionError::InvalidCommandArgument { argument_index });
            }
        }
        self.resources.validate_against(policy)?;
        Ok(())
    }
}

/// Infrastructure port required by the command-execution coordinator.
///
/// Implementations may reuse the same underlying container runtime as
/// [`super::ApplicationServiceBackend`]; the two ports are kept separate
/// because "run to completion" and "become ready as a network service" are
/// different consumer contracts with different terminal conditions.
pub trait CommandExecutionBackend: Send + Sync {
    /// Run one bounded, already-validated command to completion inside a
    /// fresh isolated sandbox and return its terminal outcome.
    ///
    /// # Errors
    ///
    /// Returns [`CommandExecutionError`] when the backend cannot establish
    /// the requested isolation contract or observe the command to
    /// completion. A nonzero exit status from the command itself is *not*
    /// an error: it is reported via [`CommandExecutionResult::exit_code`].
    fn run_to_completion_at(
        &self,
        request: &CommandExecutionRequest,
        policy: &IsolationPolicy,
        started_at_epoch_seconds: u64,
    ) -> Result<CommandExecutionResult, CommandExecutionError>;
}

/// Validate one command-execution request and delegate it to a backend.
///
/// # Errors
///
/// Returns [`CommandExecutionError`] when the request fails validation
/// against `policy`, or when the backend cannot run it to completion.
pub fn execute_command(
    backend: &dyn CommandExecutionBackend,
    request: &CommandExecutionRequest,
    policy: &IsolationPolicy,
    started_at_epoch_seconds: u64,
) -> Result<CommandExecutionResult, CommandExecutionError> {
    request.validate(policy)?;
    backend.run_to_completion_at(request, policy, started_at_epoch_seconds)
}

/// Backend-supplied fields describing how one command ran to completion.
///
/// Bundled into one type, rather than passed as separate constructor
/// arguments, to keep the constructor a small, clippy-clean call site for
/// every backend that builds a result. `#[cfg(test)]`-only for now: no
/// production backend exists yet (see the module docs), so today this type
/// is exercised only by [`tests::FakeCommandExecutionBackend`]. A real
/// backend adapter is expected to reintroduce (or un-gate) an equivalent
/// type when it lands.
#[cfg(test)]
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct CommandExecutionOutcome {
    pub(crate) backend_id: &'static str,
    pub(crate) backend_version: String,
    pub(crate) sandbox_id: String,
    pub(crate) exit_code: i32,
    pub(crate) timed_out: bool,
    pub(crate) stdout: String,
    pub(crate) stdout_truncated: bool,
    pub(crate) stderr: String,
    pub(crate) stderr_truncated: bool,
    pub(crate) started_at_epoch_seconds: u64,
    pub(crate) finished_at_epoch_seconds: u64,
}

/// Bounded, attributable outcome of one command executed to completion inside an isolated sandbox.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CommandExecutionResult {
    schema_version: String,
    request_id: String,
    image_reference: String,
    backend_id: String,
    backend_version: String,
    sandbox_id: String,
    exit_code: i32,
    timed_out: bool,
    stdout: String,
    stdout_truncated: bool,
    stderr: String,
    stderr_truncated: bool,
    started_at_epoch_seconds: u64,
    finished_at_epoch_seconds: u64,
}

/// Constructs a [`CommandExecutionResult`] from backend-supplied fields.
///
/// `#[cfg(test)]`-only alongside [`CommandExecutionOutcome`]: see that type's
/// docs for why. A real backend adapter builds a `CommandExecutionResult` the
/// same way once it exists.
#[cfg(test)]
impl CommandExecutionResult {
    pub(crate) fn new(request: &CommandExecutionRequest, outcome: CommandExecutionOutcome) -> Self {
        Self {
            schema_version: COMMAND_EXECUTION_RESULT_SCHEMA_VERSION.to_owned(),
            request_id: request.request_id.clone(),
            image_reference: request.image_reference.clone(),
            backend_id: outcome.backend_id.to_owned(),
            backend_version: outcome.backend_version,
            sandbox_id: outcome.sandbox_id,
            exit_code: outcome.exit_code,
            timed_out: outcome.timed_out,
            stdout: outcome.stdout,
            stdout_truncated: outcome.stdout_truncated,
            stderr: outcome.stderr,
            stderr_truncated: outcome.stderr_truncated,
            started_at_epoch_seconds: outcome.started_at_epoch_seconds,
            finished_at_epoch_seconds: outcome.finished_at_epoch_seconds,
        }
    }
}

impl CommandExecutionResult {
    /// Return the wire-contract version of this result.
    #[must_use]
    pub fn schema_version(&self) -> &str {
        &self.schema_version
    }

    /// Return the originating consumer request identifier.
    #[must_use]
    pub fn request_id(&self) -> &str {
        &self.request_id
    }

    /// Return the immutable OCI image reference the command ran inside.
    #[must_use]
    pub fn image_reference(&self) -> &str {
        &self.image_reference
    }

    /// Return the infrastructure backend identity.
    #[must_use]
    pub fn backend_id(&self) -> &str {
        &self.backend_id
    }

    /// Return the inspected backend version bound to this result.
    #[must_use]
    pub fn backend_version(&self) -> &str {
        &self.backend_version
    }

    /// Return the opaque sandbox identifier the command ran inside.
    #[must_use]
    pub fn sandbox_id(&self) -> &str {
        &self.sandbox_id
    }

    /// Return the command's process exit status.
    ///
    /// A nonzero value is an observed fact about the command, not a runtime
    /// error; the consumer decides how a nonzero status affects its own
    /// pass/fail verdict.
    #[must_use]
    pub const fn exit_code(&self) -> i32 {
        self.exit_code
    }

    /// Return whether the command was killed for exceeding its bounded wall-clock budget.
    ///
    /// When `true`, [`Self::exit_code`] reflects the forced termination, not
    /// a status the command itself chose.
    #[must_use]
    pub const fn timed_out(&self) -> bool {
        self.timed_out
    }

    /// Return the command's retained standard-output text.
    #[must_use]
    pub fn stdout(&self) -> &str {
        &self.stdout
    }

    /// Return whether retained standard output was truncated to the backend's output budget.
    #[must_use]
    pub const fn stdout_truncated(&self) -> bool {
        self.stdout_truncated
    }

    /// Return the command's retained standard-error text.
    #[must_use]
    pub fn stderr(&self) -> &str {
        &self.stderr
    }

    /// Return whether retained standard error was truncated to the backend's output budget.
    #[must_use]
    pub const fn stderr_truncated(&self) -> bool {
        self.stderr_truncated
    }

    /// Return the start timestamp recorded by the backend.
    #[must_use]
    pub const fn started_at_epoch_seconds(&self) -> u64 {
        self.started_at_epoch_seconds
    }

    /// Return the completion timestamp recorded by the backend.
    #[must_use]
    pub const fn finished_at_epoch_seconds(&self) -> u64 {
        self.finished_at_epoch_seconds
    }
}

/// Fail-closed command-execution validation or runtime error.
#[derive(Clone, Debug, Error, PartialEq, Eq)]
pub enum CommandExecutionError {
    /// The wire schema is not supported by this runtime.
    #[error("unsupported command-execution schema version: {actual_version}")]
    UnsupportedSchemaVersion {
        /// Version supplied by the consumer.
        actual_version: String,
    },
    /// The request identifier is empty, oversized, or contains control text.
    #[error("invalid command-execution request identifier")]
    InvalidRequestId,
    /// The image is not pinned to a lower-case SHA-256 digest.
    #[error("command-execution image must be pinned by sha256 digest")]
    ImageReferenceNotDigestPinned,
    /// The request supplied no argv to execute.
    #[error("command-execution request must include at least one argv entry")]
    EmptyCommand,
    /// The request contains more direct argv entries than the contract permits.
    #[error("too many command-execution argv entries; maximum {maximum_arguments}")]
    TooManyCommandArguments {
        /// Maximum accepted number of direct argv entries.
        maximum_arguments: usize,
    },
    /// A direct argv entry is empty, oversized, or contains control text.
    #[error("invalid command-execution argv entry at index {argument_index}")]
    InvalidCommandArgument {
        /// Zero-based argument position.
        argument_index: usize,
    },
    /// The operator isolation policy was invalid, a resource request exceeded
    /// it, or the backend could not establish or observe the sandbox.
    #[error(transparent)]
    Backend(#[from] ApplicationServiceError),
}

impl From<SandboxExecutionError> for CommandExecutionError {
    fn from(error: SandboxExecutionError) -> Self {
        Self::Backend(ApplicationServiceError::from(error))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sandbox_execution::IsolationPolicy;
    use std::sync::atomic::{AtomicUsize, Ordering};

    fn policy() -> IsolationPolicy {
        IsolationPolicy {
            policy_id: "ci_default_policy".to_owned(),
            maximum_memory_bytes: 512 * 1024 * 1024,
            maximum_cpu_millicores: 2_000,
            maximum_processes: 64,
            maximum_lease_seconds: 600,
            maximum_tmpfs_bytes: 64 * 1024 * 1024,
            readiness_timeout_millis: 5_000,
            readiness_poll_interval_millis: 100,
            shutdown_grace_seconds: 5,
            run_as_user_id: 65_534,
            run_as_group_id: 65_534,
        }
    }

    fn resources() -> ResourceRequest {
        ResourceRequest {
            memory_bytes: 256 * 1024 * 1024,
            cpu_millicores: 1_000,
            maximum_processes: 16,
            lease_seconds: 120,
            tmpfs_bytes: 16 * 1024 * 1024,
        }
    }

    fn valid_request() -> CommandExecutionRequest {
        CommandExecutionRequest {
            schema_version: CONTRACT_SCHEMA_VERSION.to_owned(),
            request_id: "pr-1234-verify".to_owned(),
            image_reference: format!("ci/verify-runner@sha256:{}", "a".repeat(64)),
            command: vec!["pytest".to_owned(), "-q".to_owned()],
            resources: resources(),
        }
    }

    /// The fixed backend outcome `FakeCommandExecutionBackend::succeeding` returns.
    ///
    /// Exposed as a standalone fixture (rather than only inline inside the fake
    /// backend) so a test can build the exact expected [`CommandExecutionResult`]
    /// independently and compare it with `assert_eq!` -- this codebase's
    /// established pattern (see `infrastructure::bounded_command::tests` for
    /// precedent) for asserting a `Result` without ever needing to unwrap or
    /// pattern-match one whose `Err` arm a test can never actually reach.
    fn sample_outcome(exit_code: i32) -> CommandExecutionOutcome {
        CommandExecutionOutcome {
            backend_id: "fake",
            backend_version: "0.0.0-test".to_owned(),
            sandbox_id: "sandbox-1".to_owned(),
            exit_code,
            timed_out: false,
            stdout: "collected 3 items\n".to_owned(),
            stdout_truncated: false,
            stderr: String::new(),
            stderr_truncated: false,
            started_at_epoch_seconds: 1,
            finished_at_epoch_seconds: 2,
        }
    }

    /// A fixed-outcome backend for contract tests; never invokes a real container runtime.
    struct FakeCommandExecutionBackend {
        outcome: Result<CommandExecutionOutcome, CommandExecutionError>,
        invocations: AtomicUsize,
    }

    impl FakeCommandExecutionBackend {
        fn succeeding(exit_code: i32) -> Self {
            Self {
                outcome: Ok(sample_outcome(exit_code)),
                invocations: AtomicUsize::new(0),
            }
        }

        fn failing(error: CommandExecutionError) -> Self {
            Self {
                outcome: Err(error),
                invocations: AtomicUsize::new(0),
            }
        }
    }

    impl CommandExecutionBackend for FakeCommandExecutionBackend {
        fn run_to_completion_at(
            &self,
            request: &CommandExecutionRequest,
            _policy: &IsolationPolicy,
            _started_at_epoch_seconds: u64,
        ) -> Result<CommandExecutionResult, CommandExecutionError> {
            self.invocations.fetch_add(1, Ordering::SeqCst);
            match &self.outcome {
                Ok(outcome) => Ok(CommandExecutionResult::new(request, outcome.clone())),
                Err(error) => Err(error.clone()),
            }
        }
    }

    #[test]
    fn execute_command_returns_backend_result_for_a_valid_request() {
        let backend = FakeCommandExecutionBackend::succeeding(0);
        let request = valid_request();
        let expected = CommandExecutionResult::new(&request, sample_outcome(0));

        assert_eq!(
            execute_command(&backend, &request, &policy(), 1),
            Ok(expected.clone())
        );
        assert_eq!(backend.invocations.load(Ordering::SeqCst), 1);

        assert_eq!(expected.schema_version(), "1.0.0");
        assert_eq!(expected.request_id(), request.request_id);
        assert_eq!(expected.image_reference(), request.image_reference);
        assert_eq!(expected.backend_id(), "fake");
        assert_eq!(expected.backend_version(), "0.0.0-test");
        assert_eq!(expected.sandbox_id(), "sandbox-1");
        assert_eq!(expected.exit_code(), 0);
        assert!(!expected.timed_out());
        assert_eq!(expected.stdout(), "collected 3 items\n");
        assert!(!expected.stdout_truncated());
        assert_eq!(expected.stderr(), "");
        assert!(!expected.stderr_truncated());
        assert_eq!(expected.started_at_epoch_seconds(), 1);
        assert_eq!(expected.finished_at_epoch_seconds(), 2);
    }

    #[test]
    fn execute_command_reports_a_nonzero_exit_status_as_a_successful_result() {
        let backend = FakeCommandExecutionBackend::succeeding(1);
        let request = valid_request();
        let expected = CommandExecutionResult::new(&request, sample_outcome(1));

        assert_eq!(
            execute_command(&backend, &request, &policy(), 1),
            Ok(expected)
        );
    }

    #[test]
    fn execute_command_rejects_an_unsupported_schema_version_before_calling_the_backend() {
        let backend = FakeCommandExecutionBackend::succeeding(0);
        let mut request = valid_request();
        request.schema_version = "9.9.9".to_owned();

        let error = execute_command(&backend, &request, &policy(), 1).unwrap_err();

        assert_eq!(
            error,
            CommandExecutionError::UnsupportedSchemaVersion {
                actual_version: "9.9.9".to_owned()
            }
        );
        assert_eq!(backend.invocations.load(Ordering::SeqCst), 0);
    }

    #[test]
    fn validate_rejects_an_empty_request_identifier() {
        let mut request = valid_request();
        request.request_id = String::new();
        assert_eq!(
            request.validate(&policy()).unwrap_err(),
            CommandExecutionError::InvalidRequestId
        );
    }

    #[test]
    fn validate_rejects_an_oversized_request_identifier() {
        let mut request = valid_request();
        request.request_id = "a".repeat(super::super::MAX_REQUEST_IDENTIFIER_BYTES + 1);
        assert_eq!(
            request.validate(&policy()).unwrap_err(),
            CommandExecutionError::InvalidRequestId
        );
    }

    #[test]
    fn validate_rejects_a_control_character_in_the_request_identifier() {
        let mut request = valid_request();
        request.request_id = "pr-\u{0007}".to_owned();
        assert_eq!(
            request.validate(&policy()).unwrap_err(),
            CommandExecutionError::InvalidRequestId
        );
    }

    #[test]
    fn validate_rejects_a_mutable_tag_image_reference() {
        let mut request = valid_request();
        request.image_reference = "ci/verify-runner:latest".to_owned();
        assert_eq!(
            request.validate(&policy()).unwrap_err(),
            CommandExecutionError::ImageReferenceNotDigestPinned
        );
    }

    #[test]
    fn validate_rejects_an_empty_command() {
        let mut request = valid_request();
        request.command = Vec::new();
        assert_eq!(
            request.validate(&policy()).unwrap_err(),
            CommandExecutionError::EmptyCommand
        );
    }

    #[test]
    fn validate_rejects_too_many_command_arguments() {
        let mut request = valid_request();
        request.command = (0..super::super::MAX_COMMAND_ARGUMENTS + 1)
            .map(|index| format!("arg{index}"))
            .collect();
        assert_eq!(
            request.validate(&policy()).unwrap_err(),
            CommandExecutionError::TooManyCommandArguments {
                maximum_arguments: super::super::MAX_COMMAND_ARGUMENTS
            }
        );
    }

    #[test]
    fn validate_rejects_a_control_character_in_a_command_argument() {
        let mut request = valid_request();
        request.command = vec!["pytest\u{0007}".to_owned()];
        assert_eq!(
            request.validate(&policy()).unwrap_err(),
            CommandExecutionError::InvalidCommandArgument { argument_index: 0 }
        );
    }

    #[test]
    fn validate_rejects_an_empty_command_argument() {
        let mut request = valid_request();
        request.command = vec![String::new()];
        assert_eq!(
            request.validate(&policy()).unwrap_err(),
            CommandExecutionError::InvalidCommandArgument { argument_index: 0 }
        );
    }

    #[test]
    fn validate_rejects_a_resource_request_above_policy() {
        let mut request = valid_request();
        request.resources.memory_bytes = policy().maximum_memory_bytes + 1;
        assert_eq!(
            request.validate(&policy()).unwrap_err(),
            CommandExecutionError::Backend(ApplicationServiceError::ResourceLimitExceeded {
                resource_name: "memory_bytes"
            })
        );
    }

    #[test]
    fn validate_rejects_an_invalid_policy() {
        let mut invalid_policy = policy();
        invalid_policy.policy_id = String::new();
        assert_eq!(
            valid_request().validate(&invalid_policy).unwrap_err(),
            CommandExecutionError::Backend(ApplicationServiceError::InvalidPolicy {
                field_name: "policy_id"
            })
        );
    }

    #[test]
    fn execute_command_propagates_a_backend_error() {
        let backend = FakeCommandExecutionBackend::failing(CommandExecutionError::Backend(
            ApplicationServiceError::BackendInvocationFailed {
                operation: "run_to_completion",
            },
        ));

        let error = execute_command(&backend, &valid_request(), &policy(), 1).unwrap_err();

        assert_eq!(
            error,
            CommandExecutionError::Backend(ApplicationServiceError::BackendInvocationFailed {
                operation: "run_to_completion",
            })
        );
    }

    #[test]
    fn request_and_result_round_trip_through_json() {
        let request = valid_request();
        let request_json = serde_json::to_string(&request).unwrap_or_default();
        assert_eq!(
            serde_json::from_str::<CommandExecutionRequest>(&request_json).ok(),
            Some(request.clone())
        );

        let expected_result = CommandExecutionResult::new(&request, sample_outcome(0));
        let result_json = serde_json::to_string(&expected_result).unwrap_or_default();
        assert_eq!(
            serde_json::from_str::<CommandExecutionResult>(&result_json).ok(),
            Some(expected_result)
        );
    }

    #[test]
    fn request_json_rejects_unknown_fields() {
        let request_json = serde_json::to_string(&valid_request()).unwrap_or_default();
        let mutated_json = request_json.replacen('{', "{\"unexpected_field\":true,", 1);

        let error = serde_json::from_str::<CommandExecutionRequest>(&mutated_json).unwrap_err();

        assert!(error.to_string().contains("unknown field"));
    }
}
