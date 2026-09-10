//! Root source paths must not cross a symlink before exact-revision staging.

#![cfg(unix)]

use std::{fs, os::unix::fs::symlink};

use quarantine_sandbox_runtime::{
    PrSourceArtifactError, PrSourceArtifactInput, stage_pr_source_artifact,
};

#[test]
fn root_symlink_is_rejected_before_source_tree_staging() {
    let directory = tempfile::tempdir().expect("fixture directory should exist");
    let source = directory.path().join("exact-revision");
    fs::create_dir(&source).expect("source directory should be created");
    let linked_source = directory.path().join("linked-revision");
    symlink(&source, &linked_source).expect("source symlink should be created");

    let input = PrSourceArtifactInput {
        host_path: linked_source,
        revision_sha: "a".repeat(40),
        expected_tree_sha256: "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
            .to_owned(),
    };

    assert!(matches!(
        stage_pr_source_artifact(&input),
        Err(PrSourceArtifactError::InvalidInput {
            field_name: "host_path"
        })
    ));
}
