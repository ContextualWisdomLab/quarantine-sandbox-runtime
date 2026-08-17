//! Bounded immutable artifact ingestion and non-executing format detection.

use sha2::{Digest, Sha256};
use thiserror::Error;

use crate::{ArtifactDescriptor, ArtifactKind};

const DEFAULT_MAXIMUM_ARTIFACT_BYTES: usize = 64 * 1_024 * 1_024;
const DEFAULT_MAXIMUM_ARTIFACT_NAME_BYTES: usize = 255;

/// Hard limits applied before artifact bytes are cloned.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct IngestionPolicy {
    /// Maximum accepted artifact byte length.
    pub maximum_artifact_bytes: usize,
    /// Maximum accepted UTF-8 artifact-name byte length.
    pub maximum_artifact_name_bytes: usize,
}

impl Default for IngestionPolicy {
    fn default() -> Self {
        Self {
            maximum_artifact_bytes: DEFAULT_MAXIMUM_ARTIFACT_BYTES,
            maximum_artifact_name_bytes: DEFAULT_MAXIMUM_ARTIFACT_NAME_BYTES,
        }
    }
}

impl IngestionPolicy {
    pub(crate) fn validate(self) -> Result<(), IngestionError> {
        if self.maximum_artifact_bytes == 0 {
            return Err(IngestionError::InvalidPolicy {
                policy_field: "maximum_artifact_bytes",
            });
        }
        if self.maximum_artifact_name_bytes == 0 {
            return Err(IngestionError::InvalidPolicy {
                policy_field: "maximum_artifact_name_bytes",
            });
        }
        Ok(())
    }
}

/// Immutable in-process artifact representation.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct IngestedArtifact {
    descriptor: ArtifactDescriptor,
    bytes: Vec<u8>,
}

impl IngestedArtifact {
    /// Return immutable identity and classification metadata.
    #[must_use]
    pub const fn descriptor(&self) -> &ArtifactDescriptor {
        &self.descriptor
    }

    /// Return the original artifact bytes exactly as submitted.
    #[must_use]
    pub fn bytes(&self) -> &[u8] {
        &self.bytes
    }
}

/// Ingestion policy or input failure.
#[derive(Clone, Debug, Error, PartialEq, Eq)]
pub enum IngestionError {
    /// A configured hard limit is zero.
    #[error("invalid ingestion policy field: {policy_field}")]
    InvalidPolicy {
        /// Invalid policy field.
        policy_field: &'static str,
    },
    /// The submitted artifact has no bytes.
    #[error("artifact must not be empty")]
    EmptyArtifact,
    /// The untrusted artifact name is empty.
    #[error("artifact name must not be empty")]
    EmptyArtifactName,
    /// The artifact name contains a control character.
    #[error("artifact name contains a control character")]
    ArtifactNameControlCharacter,
    /// The artifact name exceeds its configured UTF-8 byte limit.
    #[error("artifact name is {actual_bytes} bytes; maximum is {maximum_bytes}")]
    ArtifactNameTooLong {
        /// Actual UTF-8 byte count.
        actual_bytes: usize,
        /// Configured maximum byte count.
        maximum_bytes: usize,
    },
    /// The artifact exceeds its configured byte limit.
    #[error("artifact is {actual_bytes} bytes; maximum is {maximum_bytes}")]
    ArtifactTooLarge {
        /// Actual artifact byte count.
        actual_bytes: usize,
        /// Configured maximum byte count.
        maximum_bytes: usize,
    },
}

/// Validate, identify, and preserve an artifact without executing it.
///
/// # Errors
///
/// Returns [`IngestionError`] when policy or untrusted input violates a hard
/// limit.
pub fn ingest_bytes(
    artifact_name: &str,
    bytes: &[u8],
    policy: &IngestionPolicy,
) -> Result<IngestedArtifact, IngestionError> {
    policy.validate()?;

    if artifact_name.is_empty() {
        return Err(IngestionError::EmptyArtifactName);
    }
    if artifact_name.chars().any(char::is_control) {
        return Err(IngestionError::ArtifactNameControlCharacter);
    }
    if artifact_name.len() > policy.maximum_artifact_name_bytes {
        return Err(IngestionError::ArtifactNameTooLong {
            actual_bytes: artifact_name.len(),
            maximum_bytes: policy.maximum_artifact_name_bytes,
        });
    }
    if bytes.is_empty() {
        return Err(IngestionError::EmptyArtifact);
    }
    if bytes.len() > policy.maximum_artifact_bytes {
        return Err(IngestionError::ArtifactTooLarge {
            actual_bytes: bytes.len(),
            maximum_bytes: policy.maximum_artifact_bytes,
        });
    }

    let artifact_sha256 = format!("{:x}", Sha256::digest(bytes));
    let descriptor = ArtifactDescriptor {
        artifact_name: artifact_name.to_owned(),
        artifact_size_bytes: bytes.len() as u64,
        artifact_sha256,
        artifact_kind: detect_artifact_kind(artifact_name, bytes),
    };

    Ok(IngestedArtifact {
        descriptor,
        bytes: bytes.to_vec(),
    })
}

fn detect_artifact_kind(artifact_name: &str, bytes: &[u8]) -> ArtifactKind {
    if bytes.starts_with(b"MZ") {
        return ArtifactKind::PortableExecutable;
    }
    if bytes.starts_with(b"\x7fELF") {
        return ArtifactKind::ElfExecutable;
    }
    if is_mach_o(bytes) {
        return ArtifactKind::MachOExecutable;
    }
    if is_zip(bytes) {
        return ArtifactKind::ZipArchive;
    }
    if bytes.starts_with(b"%PDF-") {
        return ArtifactKind::PdfDocument;
    }
    if bytes.starts_with(b"\xd0\xcf\x11\xe0\xa1\xb1\x1a\xe1") {
        return ArtifactKind::OleCompoundDocument;
    }
    if is_utf8_text(bytes) {
        if bytes.starts_with(b"#!") || has_script_extension(artifact_name) {
            return ArtifactKind::Script;
        }
        return ArtifactKind::Text;
    }
    ArtifactKind::Unknown
}

fn is_mach_o(bytes: &[u8]) -> bool {
    const MAGICS: [[u8; 4]; 6] = [
        [0xfe, 0xed, 0xfa, 0xce],
        [0xce, 0xfa, 0xed, 0xfe],
        [0xfe, 0xed, 0xfa, 0xcf],
        [0xcf, 0xfa, 0xed, 0xfe],
        [0xca, 0xfe, 0xba, 0xbe],
        [0xbe, 0xba, 0xfe, 0xca],
    ];
    MAGICS.iter().any(|magic| bytes.starts_with(magic))
}

fn is_zip(bytes: &[u8]) -> bool {
    const MAGICS: [[u8; 4]; 3] = [*b"PK\x03\x04", *b"PK\x05\x06", *b"PK\x07\x08"];
    MAGICS.iter().any(|magic| bytes.starts_with(magic))
}

fn has_script_extension(artifact_name: &str) -> bool {
    let Some((_, extension)) = artifact_name.rsplit_once('.') else {
        return false;
    };
    matches!(
        extension.to_ascii_lowercase().as_str(),
        "bat" | "bash" | "cmd" | "cjs" | "js" | "mjs" | "ps1" | "py" | "sh" | "vbs" | "wsf" | "zsh"
    )
}

fn is_utf8_text(bytes: &[u8]) -> bool {
    std::str::from_utf8(bytes).is_ok()
        && bytes
            .iter()
            .all(|byte| *byte >= b' ' || matches!(*byte, b'\t' | b'\n' | b'\r'))
}
