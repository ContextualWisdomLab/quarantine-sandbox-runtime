//! Fail-closed staging contract for untrusted pull-request source trees.

use std::{
    fs::{self, File},
    io::{self, Read, Write},
    os::unix::{
        ffi::OsStrExt,
        fs::{MetadataExt, PermissionsExt},
    },
    path::{Path, PathBuf},
};

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use tempfile::TempDir;
use thiserror::Error;

const MAX_SOURCE_FILES: u64 = 10_000;
const MAX_SOURCE_BYTES: u64 = 64 * 1024 * 1024;

/// Host-owned exact-revision source tree supplied to one command execution.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PrSourceArtifactInput {
    /// Existing host directory containing an already-materialized exact revision.
    pub host_path: PathBuf,
    /// Exact lower-case Git object id, supporting SHA-1 and SHA-256 repositories.
    pub revision_sha: String,
    /// Expected digest of the canonical sorted regular-file tree manifest.
    pub expected_tree_sha256: String,
}

/// Evidence binding the sandbox input to an exact revision and sanitized tree.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PrSourceArtifactReceipt {
    revision_sha: String,
    tree_sha256: String,
    executable_files_stripped: u64,
    regular_file_count: u64,
    total_bytes: u64,
    mounted_read_only: bool,
    mounted_noexec: bool,
}

impl PrSourceArtifactReceipt {
    /// Return the exact Git revision asserted by the trusted caller.
    #[must_use]
    pub fn revision_sha(&self) -> &str {
        &self.revision_sha
    }
    /// Return the verified canonical source-tree digest.
    #[must_use]
    pub fn tree_sha256(&self) -> &str {
        &self.tree_sha256
    }
    /// Return the number of regular files whose executable bits were removed.
    #[must_use]
    pub const fn executable_files_stripped(&self) -> u64 {
        self.executable_files_stripped
    }
    /// Return the number of regular files included in the digest.
    #[must_use]
    pub const fn regular_file_count(&self) -> u64 {
        self.regular_file_count
    }
    /// Return the total regular-file bytes staged.
    #[must_use]
    pub const fn total_bytes(&self) -> u64 {
        self.total_bytes
    }
    /// Return whether the backend contract mounts the staged tree read-only.
    #[must_use]
    pub const fn mounted_read_only(&self) -> bool {
        self.mounted_read_only
    }
    /// Return whether the backend contract mounts the staged tree with `noexec`.
    #[must_use]
    pub const fn mounted_noexec(&self) -> bool {
        self.mounted_noexec
    }
}

impl PrSourceArtifactInput {
    pub(crate) fn validate_contract(&self) -> Result<(), PrSourceArtifactError> {
        if !valid_lower_hex(&self.revision_sha, &[40, 64]) {
            return Err(PrSourceArtifactError::InvalidInput {
                field_name: "revision_sha",
            });
        }
        if !valid_lower_hex(&self.expected_tree_sha256, &[64]) {
            return Err(PrSourceArtifactError::InvalidInput {
                field_name: "expected_tree_sha256",
            });
        }
        if !self.host_path.is_absolute() {
            return Err(PrSourceArtifactError::InvalidInput {
                field_name: "host_path",
            });
        }
        Ok(())
    }
}

/// A sanitized source tree whose temporary lifetime covers the sandbox invocation.
#[derive(Debug)]
pub struct StagedPrSourceArtifact {
    directory: TempDir,
    receipt: PrSourceArtifactReceipt,
}

impl StagedPrSourceArtifact {
    /// Return the host path safe to bind-mount into the sandbox.
    #[must_use]
    pub fn path(&self) -> &Path {
        self.directory.path()
    }
    /// Return exact-revision and sanitization evidence for this staged tree.
    #[must_use]
    pub const fn receipt(&self) -> &PrSourceArtifactReceipt {
        &self.receipt
    }
}

/// Fail-closed source staging error.
#[derive(Debug, Error)]
pub enum PrSourceArtifactError {
    /// The source input metadata is malformed.
    #[error("invalid PR source artifact field: {field_name}")]
    InvalidInput {
        /// Contract field that failed validation.
        field_name: &'static str,
    },
    /// The source tree contains a symlink, device, socket, or other unsupported entry.
    #[error("unsupported source entry: {relative_path}")]
    UnsupportedEntry {
        /// Human-readable relative path of the rejected entry; not an identity encoding.
        relative_path: String,
    },
    /// The source tree exceeded a fixed staging budget.
    #[error("PR source artifact exceeds staging limit: {limit_name}")]
    LimitExceeded {
        /// Fixed staging bound that was exceeded.
        limit_name: &'static str,
    },
    /// The staged tree did not match the caller's expected digest.
    #[error("PR source artifact digest mismatch")]
    DigestMismatch,
    /// Host filesystem staging failed.
    #[error("PR source artifact staging failed")]
    Io(#[source] io::Error),
}

fn valid_lower_hex(value: &str, lengths: &[usize]) -> bool {
    lengths.contains(&value.len())
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

fn validate_canonical_root_identity(
    canonical_is_directory: bool,
    source_device: u64,
    source_inode: u64,
    canonical_device: u64,
    canonical_inode: u64,
) -> Result<(), PrSourceArtifactError> {
    if !canonical_is_directory
        || source_device != canonical_device
        || source_inode != canonical_inode
    {
        return Err(PrSourceArtifactError::InvalidInput {
            field_name: "host_path",
        });
    }
    Ok(())
}

fn validate_declared_file_length(
    observed_bytes: usize,
    declared_bytes: u64,
) -> Result<(), PrSourceArtifactError> {
    if observed_bytes as u64 != declared_bytes {
        return Err(PrSourceArtifactError::DigestMismatch);
    }
    Ok(())
}

/// Add one declared file length without allowing the source-byte budget to wrap.
fn checked_source_total_bytes(
    total_bytes: u64,
    file_bytes: u64,
) -> Result<u64, PrSourceArtifactError> {
    total_bytes
        .checked_add(file_bytes)
        .ok_or(PrSourceArtifactError::LimitExceeded {
            limit_name: "total_bytes",
        })
}

fn collect_regular_files(
    directory: &Path,
    relative_directory: &Path,
    files: &mut Vec<(PathBuf, PathBuf, bool, u64)>,
) -> Result<(), PrSourceArtifactError> {
    let entries = fs::read_dir(directory).map_err(PrSourceArtifactError::Io)?;
    for entry in entries {
        let entry = entry.map_err(PrSourceArtifactError::Io)?;
        let path = entry.path();
        // `read_dir` yields a child name without leading path components. Carry
        // the already-proven relative traversal state forward instead of
        // re-deriving it with `strip_prefix` and manufacturing an impossible
        // error branch after successful rooted traversal.
        let relative = relative_directory.join(entry.file_name());
        let metadata = fs::symlink_metadata(&path).map_err(PrSourceArtifactError::Io)?;
        if metadata.is_dir() {
            collect_regular_files(&path, &relative, files)?;
        } else if metadata.is_file() {
            files.push((relative, path, metadata.mode() & 0o111 != 0, metadata.len()));
        } else {
            return Err(PrSourceArtifactError::UnsupportedEntry {
                relative_path: relative.to_string_lossy().into_owned(),
            });
        }
    }
    Ok(())
}

/// Copy, bound, hash, and de-executable an exact-revision source tree.
///
/// Manifest identity is computed from the exact Unix pathname bytes returned by
/// the filesystem. Literal backslashes remain ordinary filename bytes and valid
/// non-UTF-8 Git pathnames do not cross a lossy text conversion boundary.
///
/// # Errors
///
/// Rejects malformed identity, digest mismatch, non-regular entries, oversized
/// trees, and every filesystem failure. The original tree is never mounted.
pub fn stage_pr_source_artifact(
    input: &PrSourceArtifactInput,
) -> Result<StagedPrSourceArtifact, PrSourceArtifactError> {
    input.validate_contract()?;
    // `canonicalize` follows a final symlink, so establish the caller-supplied
    // root object's no-follow type and Unix identity before resolving its path.
    let source_root = fs::symlink_metadata(&input.host_path).map_err(PrSourceArtifactError::Io)?;
    if !source_root.file_type().is_dir() {
        return Err(PrSourceArtifactError::InvalidInput {
            field_name: "host_path",
        });
    }
    let source = fs::canonicalize(&input.host_path).map_err(PrSourceArtifactError::Io)?;
    let canonical_root = fs::metadata(&source).map_err(PrSourceArtifactError::Io)?;
    validate_canonical_root_identity(
        canonical_root.is_dir(),
        source_root.dev(),
        source_root.ino(),
        canonical_root.dev(),
        canonical_root.ino(),
    )?;
    let mut files = Vec::new();
    collect_regular_files(&source, Path::new(""), &mut files)?;
    files.sort_by(|left, right| {
        left.0
            .as_os_str()
            .as_bytes()
            .cmp(right.0.as_os_str().as_bytes())
    });
    if files.len() as u64 > MAX_SOURCE_FILES {
        return Err(PrSourceArtifactError::LimitExceeded {
            limit_name: "regular_file_count",
        });
    }
    let total_bytes = files.iter().try_fold(0_u64, |total, entry| {
        checked_source_total_bytes(total, entry.3)
    })?;
    if total_bytes > MAX_SOURCE_BYTES {
        return Err(PrSourceArtifactError::LimitExceeded {
            limit_name: "total_bytes",
        });
    }

    let directory = tempfile::Builder::new()
        .prefix("qsr-pr-source-")
        .tempdir()
        .map_err(PrSourceArtifactError::Io)?;
    let mut hasher = Sha256::new();
    let mut stripped = 0_u64;
    for (relative, source_path, was_executable, declared_len) in &files {
        let destination = directory.path().join(relative);
        // `destination` is the staging root joined with one non-empty relative
        // file path gathered above. Popping the file name therefore always
        // leaves its staging parent, including the root for a top-level file.
        let mut destination_parent = destination.clone();
        let _ = destination_parent.pop();
        fs::create_dir_all(&destination_parent).map_err(PrSourceArtifactError::Io)?;
        let source_file = File::open(source_path).map_err(PrSourceArtifactError::Io)?;
        let mut bytes = Vec::new();
        source_file
            .take(MAX_SOURCE_BYTES + 1)
            .read_to_end(&mut bytes)
            .map_err(PrSourceArtifactError::Io)?;
        validate_declared_file_length(bytes.len(), *declared_len)?;
        let relative_bytes = relative.as_os_str().as_bytes();
        hasher.update((relative_bytes.len() as u64).to_be_bytes());
        hasher.update(relative_bytes);
        hasher.update((bytes.len() as u64).to_be_bytes());
        hasher.update(&bytes);
        let mut output = File::create(&destination).map_err(PrSourceArtifactError::Io)?;
        output
            .write_all(&bytes)
            .map_err(PrSourceArtifactError::Io)?;
        fs::set_permissions(&destination, fs::Permissions::from_mode(0o444))
            .map_err(PrSourceArtifactError::Io)?;
        stripped += u64::from(*was_executable);
    }
    for entry in files.iter().rev() {
        // The staged file path is guaranteed to be beneath `directory`; walk
        // upward by mutation and stop at that known root instead of carrying an
        // unreachable `Option::None` branch before the root can be reached.
        let mut parent = directory.path().join(&entry.0);
        let _ = parent.pop();
        while parent != directory.path() {
            fs::set_permissions(&parent, fs::Permissions::from_mode(0o755))
                .map_err(PrSourceArtifactError::Io)?;
            let _ = parent.pop();
        }
    }
    // The staging root is host-side security state. Nested directories remain
    // readable for the bind-mounted sandbox tree, while the temporary root
    // itself never grants unrelated host users traversal authority.
    fs::set_permissions(directory.path(), fs::Permissions::from_mode(0o700))
        .map_err(PrSourceArtifactError::Io)?;
    let actual = format!("{:x}", hasher.finalize());
    if actual != input.expected_tree_sha256 {
        return Err(PrSourceArtifactError::DigestMismatch);
    }
    Ok(StagedPrSourceArtifact {
        directory,
        receipt: PrSourceArtifactReceipt {
            revision_sha: input.revision_sha.clone(),
            tree_sha256: actual,
            executable_files_stripped: stripped,
            regular_file_count: files.len() as u64,
            total_bytes,
            mounted_read_only: true,
            mounted_noexec: true,
        },
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn canonical_root_identity_guard_rejects_each_observed_contradiction() {
        assert!(validate_canonical_root_identity(true, 7, 11, 7, 11).is_ok());
        assert!(matches!(
            validate_canonical_root_identity(false, 7, 11, 7, 11),
            Err(PrSourceArtifactError::InvalidInput {
                field_name: "host_path"
            })
        ));
        assert!(matches!(
            validate_canonical_root_identity(true, 7, 11, 8, 11),
            Err(PrSourceArtifactError::InvalidInput {
                field_name: "host_path"
            })
        ));
        assert!(matches!(
            validate_canonical_root_identity(true, 7, 11, 7, 12),
            Err(PrSourceArtifactError::InvalidInput {
                field_name: "host_path"
            })
        ));
    }

    #[test]
    fn declared_file_length_guard_rejects_observed_mutation() {
        assert!(validate_declared_file_length(17, 17).is_ok());
        assert!(matches!(
            validate_declared_file_length(16, 17),
            Err(PrSourceArtifactError::DigestMismatch)
        ));
    }

    #[test]
    fn source_byte_budget_overflow_fails_closed() {
        assert_eq!(checked_source_total_bytes(7, 11).unwrap(), 18);
        assert!(matches!(
            checked_source_total_bytes(u64::MAX, 1),
            Err(PrSourceArtifactError::LimitExceeded {
                limit_name: "total_bytes"
            })
        ));
    }
}
