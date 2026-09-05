#![cfg(unix)]

use std::{
    ffi::OsString,
    fs,
    os::unix::ffi::OsStringExt,
};

use quarantine_sandbox_runtime::{PrSourceArtifactInput, stage_pr_source_artifact};
use sha2::{Digest, Sha256};
use tempfile::tempdir;

fn canonical_single_file_digest(relative_path: &[u8], contents: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update((relative_path.len() as u64).to_be_bytes());
    hasher.update(relative_path);
    hasher.update((contents.len() as u64).to_be_bytes());
    hasher.update(contents);
    format!("{:x}", hasher.finalize())
}

#[test]
fn staging_preserves_non_utf8_git_pathname_bytes() {
    let source = tempdir().expect("create source fixture");
    let relative_bytes = b"evidence-\xff.txt".to_vec();
    let relative_name = OsString::from_vec(relative_bytes.clone());
    let contents = b"byte-faithful source identity";
    fs::write(source.path().join(&relative_name), contents).expect("write non-UTF-8 fixture");

    let expected_tree_sha256 = canonical_single_file_digest(&relative_bytes, contents);
    let input = PrSourceArtifactInput {
        host_path: source.path().to_path_buf(),
        revision_sha: "a".repeat(40),
        expected_tree_sha256: expected_tree_sha256.clone(),
    };

    let staged = stage_pr_source_artifact(&input)
        .expect("valid Git/Linux pathname bytes must not require UTF-8");

    assert!(
        staged.path().join(&relative_name).is_file(),
        "staging must preserve the exact raw pathname byte sequence"
    );
    assert_eq!(staged.receipt().tree_sha256(), expected_tree_sha256);
    assert_eq!(staged.receipt().regular_file_count(), 1);
    assert_eq!(staged.receipt().total_bytes(), contents.len() as u64);
}
