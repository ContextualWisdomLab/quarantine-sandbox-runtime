//! Fail-closed source-artifact admission coverage for malformed identity and host inputs.

#![cfg(target_os = "linux")]

use std::{
    fs,
    path::PathBuf,
    sync::atomic::{AtomicU64, Ordering},
    time::{SystemTime, UNIX_EPOCH},
};

use quarantine_sandbox_runtime::{
    PrSourceArtifactError, PrSourceArtifactInput, stage_pr_source_artifact,
};

static NEXT_PATH_ID: AtomicU64 = AtomicU64::new(0);

fn temporary_file_path() -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock should be valid")
        .as_nanos();
    let unique = NEXT_PATH_ID.fetch_add(1, Ordering::Relaxed);
    std::env::temp_dir().join(format!(
        "qsr-pr-source-file-{}-{nanos}-{unique}",
        std::process::id()
    ))
}

fn valid_input(host_path: PathBuf) -> PrSourceArtifactInput {
    PrSourceArtifactInput {
        host_path,
        revision_sha: "a".repeat(40),
        expected_tree_sha256: "b".repeat(64),
    }
}

fn assert_invalid_field(input: &PrSourceArtifactInput, expected_field: &'static str) {
    match stage_pr_source_artifact(input).expect_err("malformed source input must fail closed") {
        PrSourceArtifactError::InvalidInput { field_name } => {
            assert_eq!(field_name, expected_field);
        }
        other => panic!("expected InvalidInput for {expected_field}, got {other:?}"),
    }
}

#[test]
fn rejects_malformed_source_identities_and_relative_host_path_before_staging() {
    let mut input = valid_input(PathBuf::from("relative-source"));
    input.revision_sha = "A".repeat(40);
    assert_invalid_field(&input, "revision_sha");

    input.revision_sha = "a".repeat(39);
    assert_invalid_field(&input, "revision_sha");

    input.revision_sha = "a".repeat(40);
    input.expected_tree_sha256 = "B".repeat(64);
    assert_invalid_field(&input, "expected_tree_sha256");

    input.expected_tree_sha256 = "b".repeat(64);
    assert_invalid_field(&input, "host_path");
}

#[test]
fn rejects_an_absolute_regular_file_as_a_source_tree() {
    let path = temporary_file_path();
    fs::write(&path, b"not a source tree").expect("temporary source file");
    let input = valid_input(path.clone());

    assert_invalid_field(&input, "host_path");

    fs::remove_file(path).expect("temporary source file cleanup");
}

#[test]
fn rejects_a_missing_absolute_source_before_tree_inspection() {
    let path = temporary_file_path();
    let input = valid_input(path);

    assert!(matches!(
        stage_pr_source_artifact(&input),
        Err(PrSourceArtifactError::Io(_))
    ));
}

#[test]
fn rejects_source_trees_above_the_regular_file_budget() {
    const MAX_SOURCE_FILES: usize = 10_000;

    let source = temporary_file_path();
    fs::create_dir_all(&source).expect("temporary source directory");
    for index in 0..=MAX_SOURCE_FILES {
        fs::File::create(source.join(format!("entry-{index:05}")))
            .expect("bounded empty source fixture");
    }
    let input = valid_input(source.clone());

    assert!(matches!(
        stage_pr_source_artifact(&input),
        Err(PrSourceArtifactError::LimitExceeded {
            limit_name: "regular_file_count"
        })
    ));

    fs::remove_dir_all(source).expect("temporary source directory cleanup");
}

#[test]
fn rejects_source_trees_above_the_total_byte_budget_before_copying() {
    const MAX_SOURCE_BYTES: u64 = 64 * 1024 * 1024;

    let source = temporary_file_path();
    fs::create_dir_all(&source).expect("temporary source directory");
    let oversized = fs::File::create(source.join("oversized.bin"))
        .expect("oversized sparse source fixture should be creatable");
    oversized
        .set_len(MAX_SOURCE_BYTES + 1)
        .expect("sparse source fixture should expose its declared size");
    let input = valid_input(source.clone());

    assert!(matches!(
        stage_pr_source_artifact(&input),
        Err(PrSourceArtifactError::LimitExceeded {
            limit_name: "total_bytes"
        })
    ));

    fs::remove_dir_all(source).expect("temporary source directory cleanup");
}
