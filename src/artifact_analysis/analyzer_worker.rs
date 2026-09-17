//! Backend-neutral analyzer-worker execution contract.
//!
//! This module defines the artifact-analysis port and value objects used to
//! cross the hostile analyzer isolation boundary. It deliberately contains no
//! Podman, gVisor, containerd, VM, or other infrastructure-specific types.

use std::collections::BTreeMap;

use thiserror::Error;

use super::{EvidenceKind, IngestedArtifact};
use crate::sandbox_execution::{
    SandboxWorkerBudget, SandboxWorkerIsolationEvidence, SandboxWorkerTerminationState,
};

const MAX_IDENTIFIER_BYTES: usize = 128;
const MAX_ATTRIBUTE_COUNT: usize = 32;
const MAX_ATTRIBUTE_KEY_BYTES: usize = 128;
const MAX_ATTRIBUTE_VALUE_BYTES: usize = 1_024;
const MAX_SUMMARY_BYTES: usize = 4_096;
const SHA256_HEX_BYTES: usize = 64;

/// Immutable analyzer implementation identity supplied to an isolated worker.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AnalyzerWorkerIdentity {
    /// Stable logical analyzer identifier.
    pub analyzer_id: String,
    /// Stable analyzer implementation version.
    pub implementation_version: String,
    /// Lower-case SHA-256 digest of the immutable analyzer implementation.
    pub implementation_sha256: String,
}

impl AnalyzerWorkerIdentity {
    /// Build and validate one immutable analyzer identity.
    ///
    /// # Errors
    ///
    /// Returns [`AnalyzerWorkerContractError::InvalidIdentity`] when an
    /// identifier is empty, oversized, contains control text, or the digest is
    /// not exactly 64 lower-case hexadecimal characters.
    pub fn new(
        analyzer_id: &str,
        implementation_version: &str,
        implementation_sha256: &str,
    ) -> Result<Self, AnalyzerWorkerContractError> {
        let identity = Self {
            analyzer_id: analyzer_id.to_owned(),
            implementation_version: implementation_version.to_owned(),
            implementation_sha256: implementation_sha256.to_owned(),
        };
        identity.validate()?;
        Ok(identity)
    }

    /// Revalidate stored identity fields before they cross a trust boundary.
    fn validate(&self) -> Result<(), AnalyzerWorkerContractError> {
        validate_identity_text("analyzer_id", &self.analyzer_id)?;
        validate_identity_text("implementation_version", &self.implementation_version)?;
        if !is_lowercase_sha256(&self.implementation_sha256) {
            return Err(AnalyzerWorkerContractError::InvalidIdentity {
                field_name: "implementation_sha256",
            });
        }
        Ok(())
    }
}

/// Controller-owned request passed through the analyzer-worker execution port.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AnalyzerWorkerRequest<'a> {
    /// Immutable analyzer implementation identity.
    pub analyzer: AnalyzerWorkerIdentity,
    /// Exact immutable artifact admitted by artifact ingestion.
    pub artifact: &'a IngestedArtifact,
    /// Stable operator-owned worker policy identifier.
    pub policy_id: String,
    /// Expected lower-case SHA-256 digest of the immutable isolation policy.
    pub isolation_policy_sha256: String,
    /// Core-owned resource and worker-output ceilings.
    pub budget: SandboxWorkerBudget,
}

impl<'a> AnalyzerWorkerRequest<'a> {
    /// Construct a validated analyzer-worker request.
    ///
    /// # Errors
    ///
    /// Returns [`AnalyzerWorkerContractError`] when analyzer identity, policy
    /// identity, isolation-policy digest, or a resource budget is malformed.
    pub fn new(
        analyzer: &AnalyzerWorkerIdentity,
        artifact: &'a IngestedArtifact,
        policy_id: &str,
        isolation_policy_sha256: &str,
        budget: SandboxWorkerBudget,
    ) -> Result<Self, AnalyzerWorkerContractError> {
        let request = Self {
            analyzer: analyzer.clone(),
            artifact,
            policy_id: policy_id.to_owned(),
            isolation_policy_sha256: isolation_policy_sha256.to_owned(),
            budget,
        };
        request.validate()?;
        Ok(request)
    }

    /// Revalidate controller-owned request fields before backend execution.
    fn validate(&self) -> Result<(), AnalyzerWorkerContractError> {
        self.analyzer.validate()?;
        validate_identity_text("policy_id", &self.policy_id)?;
        if !is_lowercase_sha256(&self.isolation_policy_sha256) {
            return Err(AnalyzerWorkerContractError::InvalidIdentity {
                field_name: "isolation_policy_sha256",
            });
        }
        if let Some(field_name) = self.budget.invalid_field() {
            return Err(AnalyzerWorkerContractError::InvalidBudget { field_name });
        }
        Ok(())
    }
}

/// One normalized analyzer finding returned across the worker boundary.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AnalyzerWorkerFinding {
    /// Evidence category represented by this finding.
    pub evidence_kind: EvidenceKind,
    /// Bounded human-readable finding summary.
    pub summary: String,
    /// Deterministically ordered bounded finding attributes.
    pub attributes: BTreeMap<String, String>,
}

impl AnalyzerWorkerFinding {
    /// Validate worker-owned finding payload bounds before controller ingestion.
    fn validate(&self) -> Result<(), AnalyzerWorkerContractError> {
        if !is_bounded_text(&self.summary, MAX_SUMMARY_BYTES) {
            return Err(AnalyzerWorkerContractError::InvalidOutcome {
                field_name: "summary",
            });
        }
        if self.attributes.len() > MAX_ATTRIBUTE_COUNT {
            return Err(AnalyzerWorkerContractError::InvalidOutcome {
                field_name: "attributes",
            });
        }
        for (key, value) in &self.attributes {
            if !is_bounded_text(key, MAX_ATTRIBUTE_KEY_BYTES) {
                return Err(AnalyzerWorkerContractError::InvalidOutcome {
                    field_name: "attribute_key",
                });
            }
            if !is_bounded_text(value, MAX_ATTRIBUTE_VALUE_BYTES) {
                return Err(AnalyzerWorkerContractError::InvalidOutcome {
                    field_name: "attribute_value",
                });
            }
        }
        Ok(())
    }
}

/// Bounded semantic result returned by an analyzer worker.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum AnalyzerWorkerOutcome {
    /// Worker completed and returned zero or more normalized findings.
    Completed {
        /// Normalized findings produced by the analyzer.
        findings: Vec<AnalyzerWorkerFinding>,
    },
    /// Worker failed before producing a trustworthy completed result.
    Failed {
        /// Stable bounded machine-readable failure code.
        failure_code: String,
    },
}

impl AnalyzerWorkerOutcome {
    /// Validate the semantic result without treating process success as authority.
    fn validate(&self) -> Result<(), AnalyzerWorkerContractError> {
        match self {
            Self::Completed { findings } => {
                for finding in findings {
                    finding.validate()?;
                }
            }
            Self::Failed { failure_code } => {
                if !is_bounded_text(failure_code, MAX_IDENTIFIER_BYTES) {
                    return Err(AnalyzerWorkerContractError::InvalidOutcome {
                        field_name: "failure_code",
                    });
                }
            }
        }
        Ok(())
    }
}

/// Worker result plus controller-verifiable request-binding evidence.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AnalyzerWorkerReceipt {
    /// Analyzer identity reported for the completed worker invocation.
    pub analyzer: AnalyzerWorkerIdentity,
    /// SHA-256 digest of the artifact processed by the worker.
    pub artifact_sha256: String,
    /// Worker policy identifier reported by the backend.
    pub policy_id: String,
    /// Core-owned runtime isolation, lifecycle, and cleanup evidence.
    pub isolation: SandboxWorkerIsolationEvidence,
    /// Bounded analyzer result.
    pub outcome: AnalyzerWorkerOutcome,
}

impl AnalyzerWorkerReceipt {
    /// Validate this worker receipt against the exact controller request.
    ///
    /// This verifies internal consistency and normalization only. It does not
    /// prove that a backend actually enforced the reported isolation controls.
    ///
    /// # Errors
    ///
    /// Returns [`AnalyzerWorkerContractError`] when the receipt contradicts the
    /// request, contains malformed runtime-owned identity, reports an unavailable
    /// required control or denied capability, lacks exact terminal cleanup
    /// evidence, or contains an unbounded outcome.
    pub fn validate_against(
        &self,
        request: &AnalyzerWorkerRequest<'_>,
    ) -> Result<(), AnalyzerWorkerContractError> {
        request.validate()?;
        self.analyzer.validate()?;
        if self.analyzer != request.analyzer {
            return Err(AnalyzerWorkerContractError::ReceiptMismatch {
                field_name: "analyzer",
            });
        }
        if self.artifact_sha256 != request.artifact.descriptor().artifact_sha256 {
            return Err(AnalyzerWorkerContractError::ReceiptMismatch {
                field_name: "artifact_sha256",
            });
        }
        if self.policy_id != request.policy_id {
            return Err(AnalyzerWorkerContractError::ReceiptMismatch {
                field_name: "policy_id",
            });
        }
        if let Some(field_name) = self.isolation.invalid_identity_field() {
            return Err(AnalyzerWorkerContractError::InvalidIdentity { field_name });
        }
        if self.isolation.isolation_policy_sha256 != request.isolation_policy_sha256 {
            return Err(AnalyzerWorkerContractError::ReceiptMismatch {
                field_name: "isolation_policy_sha256",
            });
        }
        if self.isolation.applied_budget != request.budget {
            return Err(AnalyzerWorkerContractError::ReceiptMismatch {
                field_name: "applied_budget",
            });
        }
        if let Some(field_name) = self.isolation.boundary_violation() {
            return Err(AnalyzerWorkerContractError::IsolationBoundaryViolated { field_name });
        }
        if matches!(&self.outcome, AnalyzerWorkerOutcome::Completed { .. })
            && !matches!(
                self.isolation.termination.state,
                SandboxWorkerTerminationState::Exited { exit_code: 0 }
            )
        {
            return Err(AnalyzerWorkerContractError::InvalidOutcome {
                field_name: "worker_exit_code",
            });
        }
        self.outcome.validate()
    }
}

/// Analyzer-worker contract validation failure.
#[derive(Clone, Debug, Error, PartialEq, Eq)]
pub enum AnalyzerWorkerContractError {
    /// Analyzer, policy, worker, backend, version, or SHA-256 identity is invalid.
    #[error("invalid analyzer-worker identity field: {field_name}")]
    InvalidIdentity {
        /// Stable field name.
        field_name: &'static str,
    },
    /// A required worker resource or output budget is zero.
    #[error("invalid analyzer-worker budget field: {field_name}")]
    InvalidBudget {
        /// Stable field name.
        field_name: &'static str,
    },
    /// Runtime-owned receipt evidence contradicts the controller request.
    #[error("analyzer-worker receipt does not match request field: {field_name}")]
    ReceiptMismatch {
        /// Stable field name.
        field_name: &'static str,
    },
    /// Runtime-owned receipt evidence reports a denied capability or invalid lifecycle state.
    #[error("analyzer-worker isolation boundary violated: {field_name}")]
    IsolationBoundaryViolated {
        /// Stable field name.
        field_name: &'static str,
    },
    /// Worker finding or failure output violates bounded normalization.
    #[error("invalid analyzer-worker outcome field: {field_name}")]
    InvalidOutcome {
        /// Stable field name.
        field_name: &'static str,
    },
}

/// Infrastructure-adapter execution failure at the analyzer-worker port.
#[derive(Clone, Debug, Error, PartialEq, Eq)]
pub enum AnalyzerWorkerExecutionError {
    /// No eligible analyzer-worker backend is currently available.
    #[error("analyzer-worker backend is unavailable")]
    BackendUnavailable,
    /// The selected backend failed to complete the worker invocation.
    #[error("analyzer-worker execution failed")]
    ExecutionFailed,
}

/// Backend-neutral port for executing one analyzer behind an isolation boundary.
pub trait AnalyzerWorkerExecutionPort {
    /// Execute one validated worker request and return runtime-owned evidence.
    ///
    /// Receipt consistency remains controller-owned; callers should invoke
    /// [`AnalyzerWorkerReceipt::validate_against`] before ingesting the outcome.
    ///
    /// # Errors
    ///
    /// Returns [`AnalyzerWorkerExecutionError`] when the infrastructure adapter
    /// cannot execute or complete the worker invocation.
    fn execute(
        &self,
        request: &AnalyzerWorkerRequest<'_>,
    ) -> Result<AnalyzerWorkerReceipt, AnalyzerWorkerExecutionError>;
}

/// Validate one bounded identity field using the shared text invariant.
fn validate_identity_text(
    field_name: &'static str,
    value: &str,
) -> Result<(), AnalyzerWorkerContractError> {
    if is_bounded_text(value, MAX_IDENTIFIER_BYTES) {
        Ok(())
    } else {
        Err(AnalyzerWorkerContractError::InvalidIdentity { field_name })
    }
}

/// Return whether text is non-empty, bounded in bytes, and free of control characters.
fn is_bounded_text(value: &str, maximum_bytes: usize) -> bool {
    !value.is_empty() && value.len() <= maximum_bytes && !value.chars().any(char::is_control)
}

/// Return whether a value is exactly one lower-case hexadecimal SHA-256 digest.
fn is_lowercase_sha256(value: &str) -> bool {
    value.len() == SHA256_HEX_BYTES
        && value
            .bytes()
            .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
}
