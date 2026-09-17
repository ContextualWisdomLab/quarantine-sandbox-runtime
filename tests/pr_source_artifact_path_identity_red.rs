//! RED for exact Linux pathname identity during PR source staging.
//!
//! Linux uses `/` as the pathname-component separator. A literal backslash is
//! an ordinary filename byte and must remain part of the exact source identity;
//! rewriting it to `/` changes both the manifest digest and staged tree shape.

#![cfg(target_os = "linux")]

use std::{
    fs,
    path::PathBuf,
    sync::atomic::{AtomicU64, Ordering},
    time::{SystemTime, UNIX_EPOCH},
};

use quarantine_sandbox_runtime::{PrSourceArtifactInput, stage_pr_source_artifact};
use sha2::{Digest, Sha256};

static NEXT_PATH_ID: AtomicU64 = AtomicU64::new(0);

fn temporary_path(name: &str) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock should be after the Unix epoch")
        .as_nanos();
    let unique_id = NEXT_PATH_ID.fetch_add(1, Ordering::Relaxed);
    std::env::temp_dir().join(format!(
        "quarantine-sandbox-runtime-path-identity-{name}-{}-{nanos}-{unique_id}",
        std::process::id()
    ))
}

fn tree_digest_entries(entries: &[(&str, &[u8])]) -> String {
    let mut sorted = entries.to_vec();
    sorted.sort_by(|left, right| left.0.cmp(right.0));

    let mut hasher = Sha256::new();
    for (path, bytes) in sorted {
        hasher.update((path.len() as u64).to_be_bytes());
        hasher.update(path.as_bytes());
        hasher.update((bytes.len() as u64).to_be_bytes());
        hasher.update(bytes);
    }
    format!("{:x}", hasher.finalize())
}

fn tree_digest(path: &str, bytes: &[u8]) -> String {
    tree_digest_entries(&[(path, bytes)])
}

#[test]
fn literal_backslash_filename_keeps_exact_linux_path_identity() {
    let source = temporary_path("source");
    fs::create_dir_all(&source).expect("source directory should be created");
    let relative_path = r"literal\name.txt";
    let bytes = b"exact-source-bytes\n";
    fs::write(source.join(relative_path), bytes).expect("source file should be written");
    let input = PrSourceArtifactInput {
        host_path: source.clone(),
        revision_sha: "a".repeat(40),
        expected_tree_sha256: tree_digest(relative_path, bytes),
    };

    let staged = stage_pr_source_artifact(&input)
        .expect("a literal Linux backslash filename must preserve exact tree identity");

    assert!(
        staged.path().join(relative_path).is_file(),
        "staging must preserve the literal backslash filename"
    );
    assert!(
        !staged.path().join("literal/name.txt").exists(),
        "staging must not reinterpret backslash as a pathname separator"
    );
    assert_eq!(staged.receipt().regular_file_count(), 1);
    assert_eq!(staged.receipt().tree_sha256(), input.expected_tree_sha256);

    drop(staged);
    let _ = fs::remove_dir_all(source);
}

#[test]
fn literal_backslash_path_and_nested_slash_path_remain_distinct() {
    let source = temporary_path("collision-source");
    fs::create_dir_all(source.join("a")).expect("nested source directory should be created");

    let literal_backslash_path = r"a\b";
    let nested_slash_path = "a/b";
    let literal_bytes = b"literal-backslash\n";
    let nested_bytes = b"nested-slash\n";

    fs::write(source.join(literal_backslash_path), literal_bytes)
        .expect("literal backslash source file should be written");
    fs::write(source.join(nested_slash_path), nested_bytes)
        .expect("nested slash source file should be written");

    let input = PrSourceArtifactInput {
        host_path: source.clone(),
        revision_sha: "b".repeat(40),
        expected_tree_sha256: tree_digest_entries(&[
            (literal_backslash_path, literal_bytes),
            (nested_slash_path, nested_bytes),
        ]),
    };

    let staged = stage_pr_source_artifact(&input)
        .expect("distinct Linux source pathnames must not collapse during staging");

    assert_eq!(
        fs::read(staged.path().join(literal_backslash_path))
            .expect("literal backslash staged file should exist"),
        literal_bytes
    );
    assert_eq!(
        fs::read(staged.path().join(nested_slash_path))
            .expect("nested slash staged file should exist"),
        nested_bytes
    );
    assert_eq!(staged.receipt().regular_file_count(), 2);
    assert_eq!(staged.receipt().tree_sha256(), input.expected_tree_sha256);

    drop(staged);
    let _ = fs::remove_dir_all(source);
}
