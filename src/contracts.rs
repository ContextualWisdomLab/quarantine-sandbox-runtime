//! Versioned request, evidence, and runtime-attestation contracts.

use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Current wire-contract version.
pub const CONTRACT_SCHEMA_VERSION: &str = "1.0.0";

const MAX_IDENTIFIER_BYTES: usize = 128;
const MAX_SOURCE_REFERENCE_BYTES: usize = 1_024;
const MAX_ARTIFACT_NAME_BYTES: usize = 255;
const MAX_ATTRIBUTE_COUNT: usize = 32;
const MAX_ATTRIBUTE_KEY_BYTES: usize = 128;
const MAX_ATTRIBUTE_VALUE_BYTES: usize = 1_024;
const MAX_EVIDENCE_ID_BYTES: usize = 256;
const MAX_SUMMARY_BYTES: usize = 4_096;
const MAX_LIMITATION_BYTES: usize = 256;

/// Requested analysis depth.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AnalysisProfile {
    /// Run only non-executing static analyzers.
    StaticOnly,
    /// Request an isolated Linux dynamic worker.
    LinuxDynamic,
    /// Request an isolated Windows dynamic worker.
    WindowsDynamic,
}

impl AnalysisProfile {
    /// Return the stable snake-case wire code.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::StaticOnly => "static_only",
            Self::LinuxDynamic => "linux_dynamic",
            Self::WindowsDynamic => "windows_dynamic",
        }
    }
}

/// High-level artifact family inferred without executing content.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ArtifactKind {
    /// Microsoft Portable Executable input.
    PortableExecutable,
    /// Executable and Linkable Format input.
    ElfExecutable,
    /// Apple Mach-O or universal-binary input.
    MachOExecutable,
    /// ZIP-compatible archive input.
    ZipArchive,
    /// Portable Document Format input.
    PdfDocument,
    /// OLE compound-document input.
    OleCompoundDocument,
    /// UTF-8 script input identified by shebang or approved extension.
    Script,
    /// Bounded UTF-8 text input.
    Text,
    /// Input without a recognized foundation signature.
    Unknown,
}

impl ArtifactKind {
    /// Return the stable snake-case wire code.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::PortableExecutable => "portable_executable",
            Self::ElfExecutable => "elf_executable",
            Self::MachOExecutable => "mach_o_executable",
            Self::ZipArchive => "zip_archive",
            Self::PdfDocument => "pdf_document",
            Self::OleCompoundDocument => "ole_compound_document",
            Self::Script => "script",
            Self::Text => "text",
            Self::Unknown => "unknown",
        }
    }
}

/// Evidence category emitted by the runtime or an analyzer.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EvidenceKind {
    /// Cryptographic artifact identity and immutable byte metadata.
    ArtifactIdentity,
    /// Non-executing format classification.
    FileFormat,
    /// Declared security boundary and runtime policy evidence.
    PolicyBoundary,
    /// Capability inferred by a static analyzer.
    StaticCapability,
    /// Behavior observed by a future isolated execution worker.
    RuntimeBehavior,
    /// Network activity observed by a future controlled sinkhole.
    NetworkAttempt,
    /// Attributable analyzer or worker failure.
    ToolFailure,
}

impl EvidenceKind {
    /// Return the stable snake-case wire code.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::ArtifactIdentity => "artifact_identity",
            Self::FileFormat => "file_format",
            Self::PolicyBoundary => "policy_boundary",
            Self::StaticCapability => "static_capability",
            Self::RuntimeBehavior => "runtime_behavior",
            Self::NetworkAttempt => "network_attempt",
            Self::ToolFailure => "tool_failure",
        }
    }
}

/// Analysis execution-completeness state.
///
/// This is not a maliciousness verdict.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RuntimeDisposition {
    /// Every analyzer required by the requested profile completed.
    Completed,
    /// Available evidence was returned, but the requested profile was incomplete.
    Inconclusive,
    /// No trustworthy evidence bundle could be produced.
    Failed,
}

impl RuntimeDisposition {
    /// Return the stable snake-case wire code.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Completed => "completed",
            Self::Inconclusive => "inconclusive",
            Self::Failed => "failed",
        }
    }
}

/// Bounded context supplied by a trigger adapter.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct AnalysisContext {
    /// Source product or adapter identifier.
    pub source_system: String,
    /// Opaque source-local reference such as an upload or message identifier.
    pub source_reference: String,
    /// Bounded non-secret dimensions used by the consumer for correlation.
    pub attributes: BTreeMap<String, String>,
}

impl AnalysisContext {
    fn validate(&self) -> Result<(), ContractError> {
        validate_text(
            "source_system",
            &self.source_system,
            MAX_IDENTIFIER_BYTES,
        )?;
        validate_text(
            "source_reference",
            &self.source_reference,
            MAX_SOURCE_REFERENCE_BYTES,
        )?;

        if self.attributes.len() > MAX_ATTRIBUTE_COUNT {
            return Err(ContractError::TooManyAttributes {
                maximum_attributes: MAX_ATTRIBUTE_COUNT,
            });
        }

        for (key, value) in &self.attributes {
            if key.is_empty() {
                return Err(ContractError::EmptyAttributeKey);
            }
            if value.is_empty() {
                return Err(ContractError::EmptyAttributeValue {
                    attribute_key: key.clone(),
                });
            }
            validate_nonempty_text(
                "attribute_key",
                key,
                MAX_ATTRIBUTE_KEY_BYTES,
            )?;
            validate_nonempty_text(
                "attribute_value",
                value,
                MAX_ATTRIBUTE_VALUE_BYTES,
            )?;
        }

        Ok(())
    }
}

/// Source-agnostic analysis request.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct AnalysisRequest {
    /// Contract version, currently `1.0.0`.
    pub schema_version: String,
    /// Caller-supplied idempotency and correlation identifier.
    pub request_id: String,
    /// Requested static or dynamic analysis profile.
    pub profile: AnalysisProfile,
    /// Bounded, non-secret trigger context.
    pub context: AnalysisContext,
}

impl AnalysisRequest {
    /// Validate all untrusted request fields.
    ///
    /// # Errors
    ///
    /// Returns [`ContractError`] when the schema version or metadata violates
    /// a bounded contract.
    pub fn validate(&self) -> Result<(), ContractError> {
        validate_schema_version(&self.schema_version)?;
        validate_text("request_id", &self.request_id, MAX_IDENTIFIER_BYTES)?;
        self.context.validate()
    }
}

/// Immutable identity and foundation classification of an artifact.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ArtifactDescriptor {
    /// Untrusted display name after bounded validation.
    pub artifact_name: String,
    /// Original artifact byte length.
    pub artifact_size_bytes: u64,
    /// Lower-case SHA-256 hex digest of the original bytes.
    pub artifact_sha256: String,
    /// Non-executing foundation format classification.
    pub artifact_kind: ArtifactKind,
}

impl ArtifactDescriptor {
    /// Validate identity fields.
    ///
    /// # Errors
    ///
    /// Returns [`ContractError`] for an empty or malformed identity.
    pub fn validate(&self) -> Result<(), ContractError> {
        validate_text(
            "artifact_name",
            &self.artifact_name,
            MAX_ARTIFACT_NAME_BYTES,
        )?;
        if self.artifact_size_bytes == 0 {
            return Err(ContractError::ZeroArtifactSize);
        }
        if !is_lowercase_sha256(&self.artifact_sha256) {
            return Err(ContractError::InvalidSha256);
        }
        Ok(())
    }
}

/// One normalized and attributable evidence record.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct EvidenceRecord {
    /// Deterministic evidence identifier.
    pub evidence_id: String,
    /// One-based sequence within the bundle.
    pub sequence_number: usize,
    /// Evidence category.
    pub evidence_kind: EvidenceKind,
    /// Runtime, analyzer, or worker identifier.
    pub producer_id: String,
    /// Bounded human-readable explanation.
    pub summary: String,
    /// Deterministically ordered structured attributes.
    pub attributes: BTreeMap<String, String>,
}

impl EvidenceRecord {
    fn validate(&self, expected_sequence: usize) -> Result<(), ContractError> {
        validate_text(
            "evidence_id",
            &self.evidence_id,
            MAX_EVIDENCE_ID_BYTES,
        )?;
        if self.sequence_number != expected_sequence {
            return Err(ContractError::InvalidEvidenceSequence {
                expected_sequence,
                actual_sequence: self.sequence_number,
            });
        }
        validate_text(
            "producer_id",
            &self.producer_id,
            MAX_IDENTIFIER_BYTES,
        )?;
        validate_text("summary", &self.summary, MAX_SUMMARY_BYTES)?;

        if self.attributes.len() > MAX_ATTRIBUTE_COUNT {
            return Err(ContractError::TooManyAttributes {
                maximum_attributes: MAX_ATTRIBUTE_COUNT,
            });
        }

        for (key, value) in &self.attributes {
            if key.is_empty() {
                return Err(ContractError::EmptyAttributeKey);
            }
            if value.is_empty() {
                return Err(ContractError::EmptyAttributeValue {
                    attribute_key: key.clone(),
                });
            }
            validate_nonempty_text(
                "attribute_key",
                key,
                MAX_ATTRIBUTE_KEY_BYTES,
            )?;
            validate_nonempty_text(
                "attribute_value",
                value,
                MAX_ATTRIBUTE_VALUE_BYTES,
            )?;
        }
        Ok(())
    }
}

/// Runtime identity and security-boundary attestation.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RuntimeManifest {
    /// Runtime product name.
    pub runtime_name: String,
    /// Runtime package version.
    pub runtime_version: String,
    /// Source revision or build reference.
    pub source_revision: String,
    /// Profile requested by the consumer.
    pub requested_profile: AnalysisProfile,
    /// Whether artifact content was executed.
    pub dynamic_execution_performed: bool,
    /// Whether the runtime made a network request for this analysis.
    pub network_access_performed: bool,
    /// Whether production or consumer credentials were available.
    pub credentials_available: bool,
}

impl RuntimeManifest {
    fn validate(&self) -> Result<(), ContractError> {
        validate_text("runtime_name", &self.runtime_name, MAX_IDENTIFIER_BYTES)?;
        validate_text(
            "runtime_version",
            &self.runtime_version,
            MAX_IDENTIFIER_BYTES,
        )?;
        validate_text(
            "source_revision",
            &self.source_revision,
            MAX_IDENTIFIER_BYTES,
        )?;

        for (boundary_name, violated) in [
            (
                "dynamic_execution_performed",
                self.dynamic_execution_performed,
            ),
            ("network_access_performed", self.network_access_performed),
            ("credentials_available", self.credentials_available),
        ] {
            if violated {
                return Err(ContractError::RuntimeBoundaryViolated {
                    boundary_name,
                });
            }
        }

        Ok(())
    }
}

/// Deterministic evidence output consumed by a verdict authority.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct EvidenceBundle {
    /// Contract version, currently `1.0.0`.
    pub schema_version: String,
    /// Deterministic analysis-job identifier.
    pub analysis_job_id: String,
    /// Original request identifier.
    pub request_id: String,
    /// Immutable artifact descriptor.
    pub artifact: ArtifactDescriptor,
    /// Runtime identity and boundary facts.
    pub runtime: RuntimeManifest,
    /// Execution completeness, not maliciousness.
    pub disposition: RuntimeDisposition,
    /// Always true because the consumer owns the verdict.
    pub consumer_verdict_required: bool,
    /// Ordered evidence records.
    pub evidence: Vec<EvidenceRecord>,
    /// Stable machine-readable limitations.
    pub limitations: Vec<String>,
}

impl EvidenceBundle {
    /// Validate bundle ordering, identity, and foundation security boundaries.
    ///
    /// # Errors
    ///
    /// Returns [`ContractError`] when the bundle is malformed or attempts to
    /// overstate the foundation runtime boundary.
    pub fn validate(&self) -> Result<(), ContractError> {
        validate_schema_version(&self.schema_version)?;
        validate_text(
            "analysis_job_id",
            &self.analysis_job_id,
            MAX_IDENTIFIER_BYTES,
        )?;
        validate_text("request_id", &self.request_id, MAX_IDENTIFIER_BYTES)?;
        self.artifact.validate()?;
        self.runtime.validate()?;

        if !self.consumer_verdict_required {
            return Err(ContractError::ConsumerVerdictMustBeRequired);
        }
        if self.evidence.is_empty() {
            return Err(ContractError::EmptyEvidence);
        }

        for (index, record) in self.evidence.iter().enumerate() {
            record.validate(index + 1)?;
        }

        let mut unique_limitations = BTreeSet::new();
        for limitation in &self.limitations {
            validate_text("limitation", limitation, MAX_LIMITATION_BYTES)?;
            if !unique_limitations.insert(limitation) {
                return Err(ContractError::DuplicateLimitation {
                    limitation: limitation.clone(),
                });
            }
        }

        Ok(())
    }
}

/// Contract validation failure.
#[derive(Clone, Debug, Error, PartialEq, Eq)]
pub enum ContractError {
    /// A required text field is empty.
    #[error("{field_name} must not be empty")]
    EmptyField {
        /// Stable field name.
        field_name: &'static str,
    },
    /// A text field contains a control character.
    #[error("{field_name} contains a control character")]
    ControlCharacter {
        /// Stable field name.
        field_name: &'static str,
    },
    /// A text field exceeds its byte limit.
    #[error("{field_name} exceeds {maximum_bytes} bytes")]
    FieldTooLong {
        /// Stable field name.
        field_name: &'static str,
        /// Maximum UTF-8 byte length.
        maximum_bytes: usize,
    },
    /// The wire schema version is unsupported.
    #[error("unsupported schema version: {actual_version}")]
    UnsupportedSchemaVersion {
        /// Supplied schema version.
        actual_version: String,
    },
    /// Context or evidence contains too many attributes.
    #[error("attribute count exceeds {maximum_attributes}")]
    TooManyAttributes {
        /// Maximum attribute count.
        maximum_attributes: usize,
    },
    /// An attribute key is empty.
    #[error("attribute key must not be empty")]
    EmptyAttributeKey,
    /// An attribute value is empty.
    #[error("attribute value for {attribute_key} must not be empty")]
    EmptyAttributeValue {
        /// Attribute key whose value was empty.
        attribute_key: String,
    },
    /// The artifact byte count is zero.
    #[error("artifact size must be greater than zero")]
    ZeroArtifactSize,
    /// The SHA-256 digest is not 64 lower-case hexadecimal characters.
    #[error("artifact SHA-256 must be lower-case hexadecimal")]
    InvalidSha256,
    /// The evidence vector is empty.
    #[error("evidence bundle must contain at least one record")]
    EmptyEvidence,
    /// Evidence sequence numbers are not contiguous and one-based.
    #[error(
        "invalid evidence sequence: expected {expected_sequence}, got {actual_sequence}"
    )]
    InvalidEvidenceSequence {
        /// Expected one-based sequence.
        expected_sequence: usize,
        /// Actual sequence.
        actual_sequence: usize,
    },
    /// The foundation boundary claims artifact execution, network, or credentials.
    #[error("foundation runtime boundary violated: {boundary_name}")]
    RuntimeBoundaryViolated {
        /// Machine-readable boundary field.
        boundary_name: &'static str,
    },
    /// The bundle attempted to suppress the consumer verdict requirement.
    #[error("consumer verdict must remain required")]
    ConsumerVerdictMustBeRequired,
    /// A limitation code is repeated.
    #[error("duplicate limitation: {limitation}")]
    DuplicateLimitation {
        /// Repeated limitation code.
        limitation: String,
    },
}

fn validate_schema_version(value: &str) -> Result<(), ContractError> {
    if value != CONTRACT_SCHEMA_VERSION {
        return Err(ContractError::UnsupportedSchemaVersion {
            actual_version: value.to_owned(),
        });
    }
    Ok(())
}

fn validate_text(
    field_name: &'static str,
    value: &str,
    maximum_bytes: usize,
) -> Result<(), ContractError> {
    if value.is_empty() {
        return Err(ContractError::EmptyField { field_name });
    }
    validate_nonempty_text(field_name, value, maximum_bytes)
}

fn validate_nonempty_text(
    field_name: &'static str,
    value: &str,
    maximum_bytes: usize,
) -> Result<(), ContractError> {
    if value.len() > maximum_bytes {
        return Err(ContractError::FieldTooLong {
            field_name,
            maximum_bytes,
        });
    }
    if value.chars().any(char::is_control) {
        return Err(ContractError::ControlCharacter { field_name });
    }
    Ok(())
}

fn is_lowercase_sha256(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
}
