//! RED for the runtime-owned gate artifact identity prerequisite of issue #25.
//!
//! The Podman adapter cannot safely bind a held gate until the host artifact has a bounded,
//! immutable staging contract: the expected release digest must match, the executable architecture
//! must match the current host, symlink sources must be rejected, and the staged bind source must be
//! read-only. Static-linking and real rootless execution remain separate acceptance evidence.

#![cfg(target_os = "linux")]

use std::{fs, os::unix::fs::PermissionsExt};

use quarantine_sandbox_runtime::{RuntimeGateArtifact, RuntimeGateArtifactError};
use sha2::{Digest, Sha256};
use tempfile::tempdir;

fn current_executable_digest() -> String {
    let executable = std::env::current_exe().expect("current test executable should exist");
    let bytes = fs::read(executable).expect("current test executable should be readable");
    format!("{:x}", Sha256::digest(bytes))
}

#[test]
fn runtime_gate_artifact_is_digest_bound_architecture_matched_and_staged_read_only() {
    let source = std::env::current_exe().expect("current test executable should exist");
    let expected_sha256 = current_executable_digest();

    let artifact = RuntimeGateArtifact::stage(&source, &expected_sha256, std::env::consts::ARCH)
        .expect("a matching executable artifact should stage");

    assert_ne!(artifact.path(), source);
    assert_eq!(
        artifact.path().file_name().and_then(|name| name.to_str()),
        Some("qsr-runtime-gate")
    );
    assert_eq!(artifact.sha256(), expected_sha256);
    assert_eq!(artifact.architecture(), std::env::consts::ARCH);
    assert_eq!(
        fs::read(artifact.path()).expect("staged gate should be readable"),
        fs::read(source).expect("source gate should remain readable")
    );
    let mode = fs::metadata(artifact.path())
        .expect("staged gate metadata should exist")
        .permissions()
        .mode();
    assert_eq!(mode & 0o222, 0, "staged gate must not be writable");
    assert_ne!(mode & 0o111, 0, "staged gate must remain executable");
}

#[test]
fn runtime_gate_artifact_rejects_digest_architecture_and_symlink_authority_mismatches() {
    let source = std::env::current_exe().expect("current test executable should exist");
    let expected_sha256 = current_executable_digest();

    let digest_error = RuntimeGateArtifact::stage(&source, &"0".repeat(64), std::env::consts::ARCH)
        .expect_err("a different release digest must fail closed");
    assert!(matches!(
        digest_error,
        RuntimeGateArtifactError::DigestMismatch
    ));

    let architecture_error = RuntimeGateArtifact::stage(&source, &expected_sha256, "not-this-host")
        .expect_err("a mismatched architecture declaration must fail closed");
    assert!(matches!(
        architecture_error,
        RuntimeGateArtifactError::ArchitectureMismatch { .. }
    ));

    let invalid_digest = RuntimeGateArtifact::stage(&source, "ABC", std::env::consts::ARCH)
        .expect_err("a non-canonical expected digest must fail closed");
    assert!(matches!(
        invalid_digest,
        RuntimeGateArtifactError::InvalidExpectedDigest
    ));

    let directory = tempdir().expect("temporary directory should be available");
    let link = directory.path().join("gate-link");
    std::os::unix::fs::symlink(&source, &link).expect("test symlink should be creatable");
    let symlink_error = RuntimeGateArtifact::stage(&link, &expected_sha256, std::env::consts::ARCH)
        .expect_err("a symlink must never become runtime gate authority");
    assert!(matches!(
        symlink_error,
        RuntimeGateArtifactError::SourceNotRegularFile
    ));
}
