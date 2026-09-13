//! Fail-closed coverage for host permission failures during exact-source staging.
//!
//! These witnesses exercise real Linux discretionary-access-control failures rather
//! than synthetic filesystem doubles. Privileged environments that bypass mode
//! bits cannot supply this evidence and therefore skip only the affected witness.

#![cfg(target_os = "linux")]

use std::{
    fs,
    os::unix::fs::PermissionsExt,
    path::{Path, PathBuf},
    sync::atomic::{AtomicU64, Ordering},
    time::{SystemTime, UNIX_EPOCH},
};

use quarantine_sandbox_runtime::{
    PrSourceArtifactError, PrSourceArtifactInput, stage_pr_source_artifact,
};

static NEXT_PATH_ID: AtomicU64 = AtomicU64::new(0);

fn temporary_path(name: &str) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock should be after the Unix epoch")
        .as_nanos();
    let unique = NEXT_PATH_ID.fetch_add(1, Ordering::Relaxed);
    std::env::temp_dir().join(format!(
        "qsr-pr-source-host-io-{name}-{}-{nanos}-{unique}",
        std::process::id()
    ))
}

fn valid_input(host_path: &Path) -> PrSourceArtifactInput {
    PrSourceArtifactInput {
        host_path: host_path.to_path_buf(),
        revision_sha: "a".repeat(40),
        expected_tree_sha256: "b".repeat(64),
    }
}

#[test]
fn unreadable_source_root_fails_closed_as_host_io() {
    let source = temporary_path("unreadable-root");
    fs::create_dir_all(&source).expect("source directory");
    fs::write(source.join("payload.txt"), b"host permission boundary\n")
        .expect("source payload");
    fs::set_permissions(&source, fs::Permissions::from_mode(0o000))
        .expect("deny source traversal");

    if fs::read_dir(&source).is_ok() {
        fs::set_permissions(&source, fs::Permissions::from_mode(0o700))
            .expect("restore source permissions");
        fs::remove_dir_all(source).expect("source cleanup");
        return;
    }

    let error = stage_pr_source_artifact(&valid_input(&source))
        .expect_err("an unreadable admitted source root must fail closed");
    assert!(
        matches!(error, PrSourceArtifactError::Io(_)),
        "host permission denial must remain a typed staging I/O failure: {error:?}"
    );

    fs::set_permissions(&source, fs::Permissions::from_mode(0o700))
        .expect("restore source permissions");
    fs::remove_dir_all(source).expect("source cleanup");
}

#[test]
fn unreadable_regular_file_fails_closed_as_host_io() {
    let source = temporary_path("unreadable-file");
    fs::create_dir_all(&source).expect("source directory");
    let payload = source.join("payload.txt");
    fs::write(&payload, b"host permission boundary\n").expect("source payload");
    fs::set_permissions(&payload, fs::Permissions::from_mode(0o000))
        .expect("deny source file read");

    if fs::File::open(&payload).is_ok() {
        fs::set_permissions(&payload, fs::Permissions::from_mode(0o600))
            .expect("restore source file permissions");
        fs::remove_dir_all(source).expect("source cleanup");
        return;
    }

    let error = stage_pr_source_artifact(&valid_input(&source))
        .expect_err("an unreadable regular source file must fail closed");
    assert!(
        matches!(error, PrSourceArtifactError::Io(_)),
        "host permission denial must remain a typed staging I/O failure: {error:?}"
    );

    fs::set_permissions(&payload, fs::Permissions::from_mode(0o600))
        .expect("restore source file permissions");
    fs::remove_dir_all(source).expect("source cleanup");
}
