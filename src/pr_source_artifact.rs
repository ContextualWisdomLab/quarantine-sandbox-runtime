//! Fail-closed staging contract for untrusted pull-request source trees.

use std::{
    fs::{self, File},
    io::{self, Read, Write},
    os::unix::fs::{MetadataExt, PermissionsExt},
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
        /// UTF-8 relative path of the rejected entry.
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

fn collect_regular_files(
    root: &Path,
    directory: &Path,
    files: &mut Vec<(String, PathBuf, bool, u64)>,
) -> Result<(), PrSourceArtifactError> {
    let entries = fs::read_dir(directory).map_err(PrSourceArtifactError::Io)?;
    for entry in entries {
        let entry = entry.map_err(PrSourceArtifactError::Io)?;
        let path = entry.path();
        let relative =
            path.strip_prefix(root)
                .map_err(|_| PrSourceArtifactError::InvalidInput {
                    field_name: "host_path",
                })?;
        let relative_text = relative
            .to_str()
            .filter(|value| !value.is_empty())
            .ok_or(PrSourceArtifactError::InvalidInput {
                field_name: "host_path",
            })?
            .replace('\\', "/");
        let metadata = fs::symlink_metadata(&path).map_err(PrSourceArtifactError::Io)?;
        if metadata.is_dir() {
            collect_regular_files(root, &path, files)?;
        } else if metadata.is_file() {
            files.push((
                relative_text,
                path,
                metadata.mode() & 0o111 != 0,
                metadata.len(),
            ));
        } else {
            return Err(PrSourceArtifactError::UnsupportedEntry {
                relative_path: relative_text,
            });
        }
    }
    Ok(())
}

/// Copy, bound, hash, and de-executable an exact-revision source tree.
///
/// # Errors
///
/// Rejects malformed identity, digest mismatch, non-regular entries, oversized
/// trees, and every filesystem failure. The original tree is never mounted.
pub fn stage_pr_source_artifact(
    input: &PrSourceArtifactInput,
) -> Result<StagedPrSourceArtifact, PrSourceArtifactError> {
    input.validate_contract()?;
    let source = fs::canonicalize(&input.host_path).map_err(PrSourceArtifactError::Io)?;
    if !fs::metadata(&source)
        .map_err(PrSourceArtifactError::Io)?
        .is_dir()
    {
        return Err(PrSourceArtifactError::InvalidInput {
            field_name: "host_path",
        });
    }
    let mut files = Vec::new();
    collect_regular_files(&source, &source, &mut files)?;
    files.sort_by(|left, right| left.0.cmp(&right.0));
    if files.len() as u64 > MAX_SOURCE_FILES {
        return Err(PrSourceArtifactError::LimitExceeded {
            limit_name: "regular_file_count",
        });
    }
    let total_bytes = files
        .iter()
        .try_fold(0_u64, |total, entry| total.checked_add(entry.3))
        .ok_or(PrSourceArtifactError::LimitExceeded {
            limit_name: "total_bytes",
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
        if let Some(parent) = destination.parent() {
            fs::create_dir_all(parent).map_err(PrSourceArtifactError::Io)?;
        }
        let source_file = File::open(source_path).map_err(PrSourceArtifactError::Io)?;
        let mut bytes = Vec::new();
        source_file
            .take(MAX_SOURCE_BYTES + 1)
            .read_to_end(&mut bytes)
            .map_err(PrSourceArtifactError::Io)?;
        if bytes.len() as u64 != *declared_len {
            return Err(PrSourceArtifactError::DigestMismatch);
        }
        hasher.update((relative.len() as u64).to_be_bytes());
        hasher.update(relative.as_bytes());
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
        let mut parent = directory
            .path()
            .join(&entry.0)
            .parent()
            .map(Path::to_path_buf);
        while let Some(path) = parent {
            if path == directory.path() {
                break;
            }
            fs::set_permissions(&path, fs::Permissions::from_mode(0o755))
                .map_err(PrSourceArtifactError::Io)?;
            parent = path.parent().map(Path::to_path_buf);
        }
    }
    fs::set_permissions(directory.path(), fs::Permissions::from_mode(0o755))
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
