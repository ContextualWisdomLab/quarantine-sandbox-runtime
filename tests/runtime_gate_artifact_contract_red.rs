//! RED for the runtime-owned gate artifact identity prerequisite of issue #25.
//!
//! The Podman adapter cannot safely bind a held gate until the host artifact has a bounded,
//! immutable staging contract: the expected release digest must match, the executable architecture
//! must match the current host, symlink sources must be rejected, and the staged bind source must be
//! read-only while remaining readable/executable after Podman's user-namespace mapping. Static
//! linking and real rootless execution remain separate acceptance evidence.

#![cfg(target_os = "linux")]

#[path = "support/runtime_gate_fixture.rs"]
mod runtime_gate_fixture;

use std::{fs, os::unix::fs::PermissionsExt};

use quarantine_sandbox_runtime::{RuntimeGateArtifact, RuntimeGateArtifactError};
use runtime_gate_fixture::write_self_contained_gate;
use sha2::{Digest, Sha256};
use tempfile::tempdir;

#[test]
fn runtime_gate_artifact_is_digest_bound_architecture_matched_and_staged_read_only() {
    let directory = tempdir().expect("temporary directory should be available");
    let (source, gate_bytes) = write_self_contained_gate(directory.path());
    let expected_sha256 = format!("{:x}", Sha256::digest(&gate_bytes));

    let artifact = RuntimeGateArtifact::stage(&source, &expected_sha256, std::env::consts::ARCH)
        .expect("a matching self-contained executable artifact should stage");

    assert_ne!(artifact.path(), source);
    assert_eq!(
        artifact.path().file_name().and_then(|name| name.to_str()),
        Some("qsr-runtime-gate")
    );
    assert_eq!(artifact.sha256(), expected_sha256);
    assert_eq!(artifact.architecture(), std::env::consts::ARCH);
    assert_eq!(
        fs::read(artifact.path()).expect("staged gate should be readable"),
        gate_bytes
    );
    let mode = fs::metadata(artifact.path())
        .expect("staged gate metadata should exist")
        .permissions()
        .mode();
    assert_eq!(mode & 0o222, 0, "staged gate must not be writable");
    assert_eq!(
        mode & 0o555,
        0o555,
        "the read-only gate must remain readable and executable for the remapped container user"
    );
}

#[test]
fn runtime_gate_artifact_rejects_digest_architecture_and_symlink_authority_mismatches() {
    let directory = tempdir().expect("temporary directory should be available");
    let (source, gate_bytes) = write_self_contained_gate(directory.path());
    let expected_sha256 = format!("{:x}", Sha256::digest(&gate_bytes));

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

    let link = directory.path().join("gate-link");
    std::os::unix::fs::symlink(&source, &link).expect("test symlink should be creatable");
    let symlink_error = RuntimeGateArtifact::stage(&link, &expected_sha256, std::env::consts::ARCH)
        .expect_err("a symlink must never become runtime gate authority");
    assert!(matches!(
        symlink_error,
        RuntimeGateArtifactError::SourceNotRegularFile
    ));
}

#[test]
fn runtime_gate_artifact_rejects_missing_source_and_executable_machine_mismatch() {
    let directory = tempdir().expect("temporary directory should be available");
    let missing = directory.path().join("missing-runtime-gate");
    let missing_error = RuntimeGateArtifact::stage(
        &missing,
        &"0".repeat(64),
        std::env::consts::ARCH,
    )
    .expect_err("a missing release artifact must fail before staging");
    assert!(matches!(
        missing_error,
        RuntimeGateArtifactError::SourceNotRegularFile
    ));

    let (source, mut gate_bytes) = write_self_contained_gate(directory.path());
    gate_bytes[18..20].copy_from_slice(&7_u16.to_le_bytes());
    fs::write(&source, &gate_bytes).expect("mismatched-machine fixture should be writable");
    let expected_sha256 = format!("{:x}", Sha256::digest(&gate_bytes));
    let machine_error = RuntimeGateArtifact::stage(&source, &expected_sha256, std::env::consts::ARCH)
        .expect_err("a digest-matching gate for another ELF machine must fail closed");
    match machine_error {
        RuntimeGateArtifactError::ArchitectureMismatch { expected, actual } => {
            assert_eq!(expected, std::env::consts::ARCH);
            assert_eq!(actual, "elf-machine-7");
        }
        other => panic!("expected executable architecture mismatch, got {other:?}"),
    }
}
