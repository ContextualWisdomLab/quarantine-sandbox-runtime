//! Versioned request, evidence, and runtime-attestation contracts.

use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Current wire-contract version.
pub const CONTRACT_SCHEMA_VERSION: &str = "1.0.0";

const MAX_IDENTIFIER_BYTES: usize = 128;
const MAX_ARTIFACT_NAME_BYTES: usize = 255;
const MAX_SOURCE_CHANNEL_CODE_BYTES: usize = 64;
const MAX_DECLARED_MEDIA_TYPE_BYTES: usize = 255;
const MAX_HOST_ARTIFACT_REFERENCE_BYTES: usize = 128;
const MAX_SUBMITTED_AT_BYTES: usize = 30;
const MAX_BOUNDED_SOURCE_CONTEXT_BYTES: usize = 1_024;
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

/// Optional, bounded, non-secret source metadata supplied by a trigger adapter.
///
/// This context is correlation metadata only. It must never carry message bodies,
/// credentials, raw URLs, or other unbounded source payloads.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BoundedSourceContext {
    /// Stable lower-case source-channel code, for example `direct_api`.
    pub source_channel_code: Option<String>,
    /// Original leaf file name when the trigger supplied one.
    pub original_file_name: Option<String>,
    /// Untrusted declared media type; detection still comes from artifact bytes.
    pub declared_media_type: Option<String>,
    /// Opaque host-side artifact identifier, never a credential or URL.
    pub host_artifact_reference: Option<String>,
    /// UTC RFC 3339 submission timestamp using an upper-case `T` and `Z`.
    pub submitted_at: Option<String>,
}

impl BoundedSourceContext {
    fn validate(&self) -> Result<(), ContractError> {
        if self.source_channel_code.is_none()
            && self.original_file_name.is_none()
            && self.declared_media_type.is_none()
            && self.host_artifact_reference.is_none()
            && self.submitted_at.is_none()
        {
            return Err(ContractError::EmptyBoundedSourceContext);
        }

        if self
            .source_channel_code
            .as_deref()
            .is_some_and(|value| !is_valid_source_channel_code(value))
        {
            return Err(ContractError::InvalidSourceChannelCode);
        }
        if self
            .original_file_name
            .as_deref()
            .is_some_and(|value| !is_valid_original_file_name(value))
        {
            return Err(ContractError::InvalidOriginalFileName);
        }
        if self
            .declared_media_type
            .as_deref()
            .is_some_and(|value| !is_valid_declared_media_type(value))
        {
            return Err(ContractError::InvalidDeclaredMediaType);
        }
        if self
            .host_artifact_reference
            .as_deref()
            .is_some_and(|value| !is_valid_host_artifact_reference(value))
        {
            return Err(ContractError::InvalidHostArtifactReference);
        }
        if self
            .submitted_at
            .as_deref()
            .is_some_and(|value| !is_valid_submitted_at(value))
        {
            return Err(ContractError::InvalidSubmittedAt);
        }

        let serialized_bytes = serde_json::to_vec(self).map_or(usize::MAX, |value| value.len());
        if serialized_bytes > MAX_BOUNDED_SOURCE_CONTEXT_BYTES {
            return Err(ContractError::BoundedSourceContextTooLarge {
                maximum_bytes: MAX_BOUNDED_SOURCE_CONTEXT_BYTES,
            });
        }
        Ok(())
    }
}

/// Source-agnostic analysis request.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AnalysisRequest {
    /// Contract version, currently `1.0.0`.
    pub schema_version: String,
    /// Caller-supplied idempotency and correlation identifier.
    pub request_id: String,
    /// Requested static or dynamic analysis profile.
    pub profile: AnalysisProfile,
    /// Optional bounded, non-secret trigger context.
    #[serde(default)]
    pub bounded_source_context: Option<BoundedSourceContext>,
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
        if let Some(context) = &self.bounded_source_context {
            context.validate()?;
        }
        Ok(())
    }
}

/// Immutable identity and foundation classification of an artifact.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ArtifactDescriptor {
    /// Stable display name used by the foundation ingestion API.
    pub artifact_name: String,
    /// Original leaf file name when one was supplied by source context.
    pub original_file_name: Option<String>,
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
        if self
            .original_file_name
            .as_deref()
            .is_some_and(|value| !is_valid_original_file_name(value))
        {
            return Err(ContractError::InvalidOriginalFileName);
        }
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
        validate_text("evidence_id", &self.evidence_id, MAX_EVIDENCE_ID_BYTES)?;
        if self.sequence_number != expected_sequence {
            return Err(ContractError::InvalidEvidenceSequence {
                expected_sequence,
                actual_sequence: self.sequence_number,
            });
        }
        validate_text("producer_id", &self.producer_id, MAX_IDENTIFIER_BYTES)?;
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
            validate_nonempty_text("attribute_key", key, MAX_ATTRIBUTE_KEY_BYTES)?;
            validate_nonempty_text("attribute_value", value, MAX_ATTRIBUTE_VALUE_BYTES)?;
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
                return Err(ContractError::RuntimeBoundaryViolated { boundary_name });
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
    /// An explicitly present bounded source context contains no fields.
    #[error("bounded source context must contain at least one field")]
    EmptyBoundedSourceContext,
    /// Source-channel code is not a bounded lower-case code.
    #[error("invalid source channel code")]
    InvalidSourceChannelCode,
    /// Original file name is not a safe leaf name.
    #[error("invalid original file name")]
    InvalidOriginalFileName,
    /// Declared media type is not bounded media-type syntax.
    #[error("invalid declared media type")]
    InvalidDeclaredMediaType,
    /// Host artifact reference is not an opaque non-secret identifier.
    #[error("invalid host artifact reference")]
    InvalidHostArtifactReference,
    /// Submission timestamp is not the accepted UTC RFC 3339 subset.
    #[error("invalid submitted_at timestamp")]
    InvalidSubmittedAt,
    /// Serialized bounded source context exceeds the aggregate byte budget.
    #[error("bounded source context exceeds {maximum_bytes} serialized bytes")]
    BoundedSourceContextTooLarge {
        /// Maximum serialized JSON byte length.
        maximum_bytes: usize,
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
    #[error("invalid evidence sequence: expected {expected_sequence}, got {actual_sequence}")]
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

fn is_valid_source_channel_code(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= MAX_SOURCE_CHANNEL_CODE_BYTES
        && value
            .bytes()
            .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'_')
}

fn is_valid_original_file_name(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= MAX_ARTIFACT_NAME_BYTES
        && value != "."
        && value != ".."
        && !value.chars().any(char::is_control)
        && !value.bytes().any(|byte| matches!(byte, b'/' | b'\\'))
}

fn is_valid_declared_media_type(value: &str) -> bool {
    if value.is_empty() || value.len() > MAX_DECLARED_MEDIA_TYPE_BYTES || !value.is_ascii() {
        return false;
    }
    let Some((top_level, remainder)) = value.split_once('/') else {
        return false;
    };
    let (subtype, parameters) = remainder
        .split_once(';')
        .map_or((remainder, None), |parts| (parts.0, Some(parts.1)));
    if !is_media_type_token(top_level) || !is_media_type_token(subtype) {
        return false;
    }
    parameters.is_none_or(|tail| {
        !tail.is_empty()
            && tail
                .bytes()
                .all(|byte| byte == b'\t' || (b' '..=b'~').contains(&byte))
    })
}

fn is_media_type_token(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 127
        && value.bytes().all(|byte| {
            byte.is_ascii_alphanumeric()
                || matches!(
                    byte,
                    b'!' | b'#' | b'$' | b'&' | b'-' | b'^' | b'_' | b'.' | b'+'
                )
        })
}

fn is_valid_host_artifact_reference(value: &str) -> bool {
    if value.is_empty() || value.len() > MAX_HOST_ARTIFACT_REFERENCE_BYTES || !value.is_ascii() {
        return false;
    }
    let lowercase = value.to_ascii_lowercase();
    if lowercase.starts_with("github_pat_") || lowercase.starts_with("bearer_") {
        return false;
    }
    value
        .bytes()
        .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'-' | b'.' | b':'))
}

fn is_valid_submitted_at(value: &str) -> bool {
    let bytes = value.as_bytes();
    if bytes.len() < 20 || bytes.len() > MAX_SUBMITTED_AT_BYTES || bytes.last() != Some(&b'Z') {
        return false;
    }
    if bytes.get(4) != Some(&b'-')
        || bytes.get(7) != Some(&b'-')
        || bytes.get(10) != Some(&b'T')
        || bytes.get(13) != Some(&b':')
        || bytes.get(16) != Some(&b':')
    {
        return false;
    }

    let Some(year) = parse_ascii_u32(&bytes[0..4]) else {
        return false;
    };
    let Some(month) = parse_ascii_u32(&bytes[5..7]) else {
        return false;
    };
    let Some(day) = parse_ascii_u32(&bytes[8..10]) else {
        return false;
    };
    let Some(hour) = parse_ascii_u32(&bytes[11..13]) else {
        return false;
    };
    let Some(minute) = parse_ascii_u32(&bytes[14..16]) else {
        return false;
    };
    let Some(second) = parse_ascii_u32(&bytes[17..19]) else {
        return false;
    };

    if !(1..=12).contains(&month)
        || hour > 23
        || minute > 59
        || second > 60
        || day == 0
        || day > days_in_month(year, month)
    {
        return false;
    }

    if bytes.len() == 20 {
        return true;
    }
    bytes.get(19) == Some(&b'.')
        && bytes.len() > 21
        && bytes[20..bytes.len() - 1].iter().all(u8::is_ascii_digit)
}

fn parse_ascii_u32(bytes: &[u8]) -> Option<u32> {
    let mut value = 0_u32;
    for byte in bytes {
        if !byte.is_ascii_digit() {
            return None;
        }
        value = value * 10 + u32::from(*byte - b'0');
    }
    Some(value)
}

const fn days_in_month(year: u32, month: u32) -> u32 {
    match month {
        2 if is_leap_year(year) => 29,
        2 => 28,
        4 | 6 | 9 | 11 => 30,
        _ => 31,
    }
}

const fn is_leap_year(year: u32) -> bool {
    year.is_multiple_of(4) && (!year.is_multiple_of(100) || year.is_multiple_of(400))
}
