//! Contract and fake-Podman coverage for exact-revision PR source inputs.

#![cfg(target_os = "linux")]

use std::{
    fs,
    os::unix::fs::{MetadataExt, PermissionsExt},
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
        .expect("system clock should be valid")
        .as_nanos();
    let unique = NEXT_PATH_ID.fetch_add(1, Ordering::Relaxed);
    std::env::temp_dir().join(format!(
        "qsr-pr-source-{name}-{}-{nanos}-{unique}",
        std::process::id()
    ))
}

fn tree_digest(entries: &[(&str, &[u8])]) -> String {
    let mut hasher = Sha256::new();
    for (path, bytes) in entries {
        hasher.update((path.len() as u64).to_be_bytes());
        hasher.update(path.as_bytes());
        hasher.update((bytes.len() as u64).to_be_bytes());
        hasher.update(bytes);
    }
    format!("{:x}", hasher.finalize())
}

#[test]
fn stages_exact_revision_source_read_only_and_strips_every_executable_bit() {
    let source = temporary_path("source");
    fs::create_dir_all(source.join("nested")).expect("source directory");
    fs::write(source.join("run.sh"), b"#!/bin/sh\necho unsafe\n").expect("script");
    fs::write(source.join("nested/data.txt"), b"bounded\n").expect("data");
    fs::set_permissions(source.join("run.sh"), fs::Permissions::from_mode(0o755))
        .expect("executable source mode");
    let expected = tree_digest(&[
        ("nested/data.txt", b"bounded\n"),
        ("run.sh", b"#!/bin/sh\necho unsafe\n"),
    ]);
    let input = PrSourceArtifactInput {
        host_path: source.clone(),
        revision_sha: "a".repeat(40),
        expected_tree_sha256: expected.clone(),
    };

    let staged = stage_pr_source_artifact(&input).expect("valid exact source should stage");

    assert_eq!(staged.receipt().revision_sha(), "a".repeat(40));
    assert_eq!(staged.receipt().tree_sha256(), expected);
    assert_eq!(staged.receipt().executable_files_stripped(), 1);
    assert_eq!(staged.receipt().regular_file_count(), 2);
    assert_eq!(staged.receipt().total_bytes(), 30);
    assert_eq!(
        fs::metadata(staged.path().join("run.sh")).unwrap().mode() & 0o777,
        0o444
    );
    assert_eq!(
        fs::metadata(staged.path().join("nested")).unwrap().mode() & 0o777,
        0o755
    );
    let staged_path = staged.path().to_path_buf();
    drop(staged);
    assert!(
        !staged_path.exists(),
        "sanitized staging tree must be removed"
    );

    let _ = fs::remove_dir_all(source);
}

#[test]
fn rejects_symlinks_and_digest_mismatches_fail_closed() {
    let source = temporary_path("invalid-source");
    fs::create_dir_all(&source).expect("source directory");
    fs::write(source.join("data.txt"), b"actual").expect("data");
    std::os::unix::fs::symlink("data.txt", source.join("link")).expect("source symlink");
    let input = PrSourceArtifactInput {
        host_path: source.clone(),
        revision_sha: "b".repeat(40),
        expected_tree_sha256: "c".repeat(64),
    };

    let error = stage_pr_source_artifact(&input).expect_err("symlink must be rejected");
    assert!(error.to_string().contains("unsupported source entry"));

    fs::remove_file(source.join("link")).expect("remove symlink");
    let error = stage_pr_source_artifact(&input).expect_err("digest mismatch must be rejected");
    assert!(error.to_string().contains("digest mismatch"));
    let _ = fs::remove_dir_all(source);
}
