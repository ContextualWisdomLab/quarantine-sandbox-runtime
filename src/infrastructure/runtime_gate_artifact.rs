//! Immutable host-side staging for the runtime-owned command hold gate.

use std::{
    fs::{self, OpenOptions},
    io::Write,
    os::unix::fs::OpenOptionsExt,
    path::{Path, PathBuf},
};

use sha2::{Digest, Sha256};
use tempfile::TempDir;
use thiserror::Error;

const RUNTIME_GATE_FILE_NAME: &str = "qsr-runtime-gate";
const ELF_HEADER_MINIMUM_BYTES: usize = 20;

/// A digest-bound, architecture-matched runtime gate staged in a private directory.
///
/// The staging directory is owned by this value and is removed when the value is dropped. The
/// staged gate is created read-only and executable so a later container adapter can bind-mount the
/// exact verified bytes without trusting an artifact supplied by the hostile image.
#[derive(Debug)]
pub struct RuntimeGateArtifact {
    _staging_directory: TempDir,
    path: PathBuf,
    sha256: String,
    architecture: String,
}

impl RuntimeGateArtifact {
    /// Verify and stage a runtime gate from a trusted host release path.
    ///
    /// `expected_sha256` must be a canonical lowercase SHA-256 digest. The expected architecture
    /// must equal the current host architecture and the executable's ELF machine identity. Symlink
    /// and non-regular-file sources fail closed before bytes are staged.
    ///
    /// # Errors
    ///
    /// Returns [`RuntimeGateArtifactError`] when the expected identity is malformed, the source is
    /// not a regular file, the bytes do not match the expected digest or architecture, or private
    /// read-only staging cannot be completed.
    pub fn stage(
        source: &Path,
        expected_sha256: &str,
        expected_architecture: &str,
    ) -> Result<Self, RuntimeGateArtifactError> {
        validate_expected_digest(expected_sha256)?;

        let host_architecture = std::env::consts::ARCH;
        if expected_architecture != host_architecture {
            return Err(RuntimeGateArtifactError::ArchitectureMismatch {
                expected: expected_architecture.to_owned(),
                actual: host_architecture.to_owned(),
            });
        }

        let metadata = fs::symlink_metadata(source)
            .map_err(|_| RuntimeGateArtifactError::SourceNotRegularFile)?;
        if !metadata.file_type().is_file() {
            return Err(RuntimeGateArtifactError::SourceNotRegularFile);
        }

        let bytes = fs::read(source).map_err(|_| RuntimeGateArtifactError::SourceReadFailed)?;
        let actual_sha256 = format!("{:x}", Sha256::digest(&bytes));
        if actual_sha256 != expected_sha256 {
            return Err(RuntimeGateArtifactError::DigestMismatch);
        }

        let actual_architecture = executable_architecture(&bytes);
        if actual_architecture != expected_architecture {
            return Err(RuntimeGateArtifactError::ArchitectureMismatch {
                expected: expected_architecture.to_owned(),
                actual: actual_architecture.to_owned(),
            });
        }

        let staging_directory = tempfile::Builder::new()
            .prefix("qsr-runtime-gate-")
            .tempdir()
            .map_err(|_| RuntimeGateArtifactError::StagingFailed)?;
        let path = staging_directory.path().join(RUNTIME_GATE_FILE_NAME);
        let mut staged = OpenOptions::new()
            .write(true)
            .create_new(true)
            .mode(0o500)
            .open(&path)
            .map_err(|_| RuntimeGateArtifactError::StagingFailed)?;
        staged
            .write_all(&bytes)
            .map_err(|_| RuntimeGateArtifactError::StagingFailed)?;
        staged
            .sync_all()
            .map_err(|_| RuntimeGateArtifactError::StagingFailed)?;
        drop(staged);

        Ok(Self {
            _staging_directory: staging_directory,
            path,
            sha256: actual_sha256,
            architecture: actual_architecture,
        })
    }

    /// Return the private host path containing the verified gate bytes.
    #[must_use]
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Return the verified lowercase SHA-256 digest of the staged gate.
    #[must_use]
    pub fn sha256(&self) -> &str {
        &self.sha256
    }

    /// Return the verified executable architecture of the staged gate.
    #[must_use]
    pub fn architecture(&self) -> &str {
        &self.architecture
    }
}

/// Failure to establish an immutable runtime gate artifact identity.
#[derive(Debug, Error)]
pub enum RuntimeGateArtifactError {
    /// The configured release digest is not canonical lowercase SHA-256.
    #[error("runtime gate expected digest is not canonical SHA-256")]
    InvalidExpectedDigest,
    /// The configured gate source is absent, a symlink, or not a regular file.
    #[error("runtime gate source is not a regular file")]
    SourceNotRegularFile,
    /// The regular source could not be read after its file type was verified.
    #[error("runtime gate source could not be read")]
    SourceReadFailed,
    /// The source bytes do not match the configured immutable release digest.
    #[error("runtime gate digest does not match the configured release digest")]
    DigestMismatch,
    /// The configured or executable architecture is incompatible with the current runtime host.
    #[error("runtime gate architecture mismatch: expected {expected}, actual {actual}")]
    ArchitectureMismatch {
        /// Required architecture.
        expected: String,
        /// Observed architecture.
        actual: String,
    },
    /// The verified bytes could not be materialized into a private read-only executable staging area.
    #[error("runtime gate private staging failed")]
    StagingFailed,
}

fn validate_expected_digest(expected_sha256: &str) -> Result<(), RuntimeGateArtifactError> {
    if expected_sha256.len() != 64
        || !expected_sha256
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    {
        return Err(RuntimeGateArtifactError::InvalidExpectedDigest);
    }
    Ok(())
}

fn executable_architecture(bytes: &[u8]) -> String {
    if bytes.len() < ELF_HEADER_MINIMUM_BYTES || &bytes[..4] != b"\x7fELF" {
        return "non-elf".to_owned();
    }

    let machine = match bytes[5] {
        1 => u16::from_le_bytes([bytes[18], bytes[19]]),
        2 => u16::from_be_bytes([bytes[18], bytes[19]]),
        _ => return "elf-unknown-data-encoding".to_owned(),
    };
    match machine {
        62 => "x86_64".to_owned(),
        183 => "aarch64".to_owned(),
        other => format!("elf-machine-{other}"),
    }
}

#[cfg(test)]
mod tests {
    use super::{executable_architecture, validate_expected_digest, RuntimeGateArtifactError};

    #[test]
    fn expected_digest_requires_exact_lowercase_sha256_shape() {
        assert!(validate_expected_digest(&"0".repeat(64)).is_ok());
        assert!(matches!(
            validate_expected_digest(&"A".repeat(64)),
            Err(RuntimeGateArtifactError::InvalidExpectedDigest)
        ));
        assert!(matches!(
            validate_expected_digest(&"0".repeat(63)),
            Err(RuntimeGateArtifactError::InvalidExpectedDigest)
        ));
    }

    #[test]
    fn elf_machine_identity_is_bounded_and_endian_aware() {
        let mut little_x86_64 = vec![0_u8; 20];
        little_x86_64[..4].copy_from_slice(b"\x7fELF");
        little_x86_64[5] = 1;
        little_x86_64[18..20].copy_from_slice(&62_u16.to_le_bytes());
        assert_eq!(executable_architecture(&little_x86_64), "x86_64");

        let mut big_aarch64 = vec![0_u8; 20];
        big_aarch64[..4].copy_from_slice(b"\x7fELF");
        big_aarch64[5] = 2;
        big_aarch64[18..20].copy_from_slice(&183_u16.to_be_bytes());
        assert_eq!(executable_architecture(&big_aarch64), "aarch64");

        let mut unknown_machine = little_x86_64.clone();
        unknown_machine[18..20].copy_from_slice(&7_u16.to_le_bytes());
        assert_eq!(executable_architecture(&unknown_machine), "elf-machine-7");

        let mut unknown_encoding = little_x86_64;
        unknown_encoding[5] = 0;
        assert_eq!(
            executable_architecture(&unknown_encoding),
            "elf-unknown-data-encoding"
        );
        assert_eq!(executable_architecture(b"not-elf"), "non-elf");
    }
}
