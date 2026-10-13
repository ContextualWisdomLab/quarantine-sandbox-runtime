//! Offline artifact-binding verification (ADR 0012).
//!
//! Confirms that a serialized [`EvidenceBundle`] describes artifact bytes the
//! consumer still holds. The derivation reuses ingestion's SHA-256 digest, byte
//! length, and non-executing format classification. Artifact bytes are never
//! executed, copied, or written anywhere.

use serde::Serialize;
use sha2::{Digest, Sha256};
use thiserror::Error;

use super::ingestion::detect_artifact_kind;
use crate::{ArtifactKind, ContractError, EvidenceBundle};

/// Overall binding between a bundle descriptor and supplied artifact bytes.
///
/// `Matched` is not a verdict and not an analysis-completeness statement.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ArtifactBindingOutcome {
    /// Digest, byte length, and format classification all match.
    Matched,
    /// At least one recomputed identity field differs from the descriptor.
    Mismatched,
}

impl ArtifactBindingOutcome {
    /// Return the stable snake-case wire code.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Matched => "matched",
            Self::Mismatched => "mismatched",
        }
    }
}

/// Comparison result for one descriptor field.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum FieldBinding {
    /// The recomputed value equals the descriptor value.
    Matched,
    /// The recomputed value differs from the descriptor value.
    Mismatched,
}

impl FieldBinding {
    /// Return the stable snake-case wire code.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Matched => "matched",
            Self::Mismatched => "mismatched",
        }
    }

    fn compare<T: PartialEq>(declared: &T, computed: &T) -> Self {
        if declared == computed {
            Self::Matched
        } else {
            Self::Mismatched
        }
    }
}

/// Runtime-derived comparison of a bundle descriptor with supplied bytes.
///
/// The report can only be produced by [`verify_artifact_binding`]; it is
/// serializable for transport but intentionally not deserializable.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct ArtifactBindingReport {
    outcome: ArtifactBindingOutcome,
    artifact_sha256: FieldBinding,
    artifact_size_bytes: FieldBinding,
    artifact_kind: FieldBinding,
    computed_artifact_sha256: String,
    computed_artifact_size_bytes: u64,
    computed_artifact_kind: ArtifactKind,
}

impl ArtifactBindingReport {
    /// Return the overall binding outcome.
    #[must_use]
    pub const fn outcome(&self) -> ArtifactBindingOutcome {
        self.outcome
    }

    /// Return whether every compared identity field matched.
    #[must_use]
    pub const fn is_matched(&self) -> bool {
        matches!(self.outcome, ArtifactBindingOutcome::Matched)
    }

    /// Return the SHA-256 digest comparison.
    #[must_use]
    pub const fn artifact_sha256(&self) -> FieldBinding {
        self.artifact_sha256
    }

    /// Return the byte-length comparison.
    #[must_use]
    pub const fn artifact_size_bytes(&self) -> FieldBinding {
        self.artifact_size_bytes
    }

    /// Return the format-classification comparison.
    #[must_use]
    pub const fn artifact_kind(&self) -> FieldBinding {
        self.artifact_kind
    }

    /// Return the lower-case SHA-256 digest recomputed from the supplied bytes.
    #[must_use]
    pub fn computed_artifact_sha256(&self) -> &str {
        &self.computed_artifact_sha256
    }

    /// Return the byte length of the supplied bytes.
    #[must_use]
    pub const fn computed_artifact_size_bytes(&self) -> u64 {
        self.computed_artifact_size_bytes
    }

    /// Return the non-executing classification of the supplied bytes.
    #[must_use]
    pub const fn computed_artifact_kind(&self) -> ArtifactKind {
        self.computed_artifact_kind
    }
}

/// Artifact-binding verification could not produce a report.
#[derive(Clone, Debug, Error, PartialEq, Eq)]
pub enum ArtifactBindingError {
    /// The bundle violates the published evidence contract.
    #[error(transparent)]
    Contract(#[from] ContractError),
    /// No artifact bytes were supplied.
    #[error("artifact bytes must not be empty")]
    EmptyArtifact,
}

/// Compare a bundle's artifact descriptor with artifact bytes the caller holds.
///
/// The bundle is validated first. The SHA-256 digest, byte length, and
/// non-executing format classification are then recomputed from `artifact_bytes`
/// exactly as ingestion derives them, using the descriptor's original file name.
/// A mismatch is returned as a report, not an error.
///
/// # Errors
///
/// Returns [`ArtifactBindingError::Contract`] when the bundle is invalid and
/// [`ArtifactBindingError::EmptyArtifact`] when `artifact_bytes` is empty.
pub fn verify_artifact_binding(
    bundle: &EvidenceBundle,
    artifact_bytes: &[u8],
) -> Result<ArtifactBindingReport, ArtifactBindingError> {
    bundle.validate()?;
    if artifact_bytes.is_empty() {
        return Err(ArtifactBindingError::EmptyArtifact);
    }

    let declared = &bundle.artifact;
    let computed_artifact_sha256 = format!("{:x}", Sha256::digest(artifact_bytes));
    let computed_artifact_size_bytes = artifact_bytes.len() as u64;
    let computed_artifact_kind = detect_artifact_kind(
        declared.original_file_name.as_deref().unwrap_or(""),
        artifact_bytes,
    );

    let artifact_sha256 =
        FieldBinding::compare(&declared.artifact_sha256, &computed_artifact_sha256);
    let artifact_size_bytes =
        FieldBinding::compare(&declared.artifact_size_bytes, &computed_artifact_size_bytes);
    let artifact_kind = FieldBinding::compare(&declared.artifact_kind, &computed_artifact_kind);
    let outcome = if [artifact_sha256, artifact_size_bytes, artifact_kind]
        .iter()
        .all(|binding| *binding == FieldBinding::Matched)
    {
        ArtifactBindingOutcome::Matched
    } else {
        ArtifactBindingOutcome::Mismatched
    };

    Ok(ArtifactBindingReport {
        outcome,
        artifact_sha256,
        artifact_size_bytes,
        artifact_kind,
        computed_artifact_sha256,
        computed_artifact_size_bytes,
        computed_artifact_kind,
    })
}
