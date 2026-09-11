//! Runtime-gate artifact value-semantics and admission coverage.

#![cfg(target_os = "linux")]

#[path = "support/runtime_gate_fixture.rs"]
mod runtime_gate_fixture;

use std::fs;

use quarantine_sandbox_runtime::{RuntimeGateArtifact, RuntimeGateArtifactError};
use runtime_gate_fixture::write_self_contained_gate;
use sha2::{Digest, Sha256};

fn big_endian_gate_bytes(mut bytes: Vec<u8>) -> Vec<u8> {
    let machine = match std::env::consts::ARCH {
        "x86_64" => 62_u16,
        "aarch64" => 183_u16,
        other => panic!("runtime-gate test fixture does not support architecture {other}"),
    };
    let file_bytes = bytes.len() as u64;

    bytes[5] = 2;
    bytes[16..18].copy_from_slice(&2_u16.to_be_bytes());
    bytes[18..20].copy_from_slice(&machine.to_be_bytes());
    bytes[20..24].copy_from_slice(&1_u32.to_be_bytes());
    bytes[24..32].copy_from_slice(&0x400100_u64.to_be_bytes());
    bytes[32..40].copy_from_slice(&64_u64.to_be_bytes());
    bytes[52..54].copy_from_slice(&64_u16.to_be_bytes());
    bytes[54..56].copy_from_slice(&56_u16.to_be_bytes());
    bytes[56..58].copy_from_slice(&1_u16.to_be_bytes());
    bytes[64..68].copy_from_slice(&1_u32.to_be_bytes());
    bytes[68..72].copy_from_slice(&5_u32.to_be_bytes());
    bytes[80..88].copy_from_slice(&0x400000_u64.to_be_bytes());
    bytes[96..104].copy_from_slice(&file_bytes.to_be_bytes());
    bytes[104..112].copy_from_slice(&file_bytes.to_be_bytes());
    bytes[112..120].copy_from_slice(&4096_u64.to_be_bytes());
    bytes
}

#[test]
fn cloned_runtime_gate_artifact_preserves_verified_authority_identity() {
    let directory = tempfile::tempdir().expect("fixture directory should exist");
    let (source, bytes) = write_self_contained_gate(directory.path());
    let expected_sha256 = format!("{:x}", Sha256::digest(&bytes));
    let artifact = RuntimeGateArtifact::stage(&source, &expected_sha256, std::env::consts::ARCH)
        .expect("matching self-contained gate should stage");
    let cloned = artifact.clone();

    assert_eq!(cloned, artifact);
}

#[test]
fn separately_staged_identical_gate_bytes_do_not_alias_runtime_authority() {
    let first_directory = tempfile::tempdir().expect("first fixture directory should exist");
    let second_directory = tempfile::tempdir().expect("second fixture directory should exist");
    let (first_source, bytes) = write_self_contained_gate(first_directory.path());
    let (second_source, second_bytes) = write_self_contained_gate(second_directory.path());
    assert_eq!(second_bytes, bytes);
    let expected_sha256 = format!("{:x}", Sha256::digest(&bytes));

    let first = RuntimeGateArtifact::stage(&first_source, &expected_sha256, std::env::consts::ARCH)
        .expect("first matching gate should stage");
    let second =
        RuntimeGateArtifact::stage(&second_source, &expected_sha256, std::env::consts::ARCH)
            .expect("second matching gate should stage");

    assert_eq!(first.sha256(), second.sha256());
    assert_eq!(first.architecture(), second.architecture());
    assert_ne!(
        first, second,
        "separate private staging lifetimes must not compare as the same runtime authority"
    );
}

#[test]
fn staging_fails_closed_for_digest_architecture_and_source_authority_mismatches() {
    let directory = tempfile::tempdir().expect("fixture directory should exist");
    let (source, bytes) = write_self_contained_gate(directory.path());
    let expected_sha256 = format!("{:x}", Sha256::digest(&bytes));

    assert!(matches!(
        RuntimeGateArtifact::stage(&source, &"0".repeat(64), std::env::consts::ARCH),
        Err(RuntimeGateArtifactError::DigestMismatch)
    ));
    assert!(matches!(
        RuntimeGateArtifact::stage(&source, &expected_sha256, "not-this-host"),
        Err(RuntimeGateArtifactError::ArchitectureMismatch { .. })
    ));

    let source_link = directory.path().join("gate-link");
    std::os::unix::fs::symlink(&source, &source_link).expect("fixture symlink should be creatable");
    assert!(matches!(
        RuntimeGateArtifact::stage(&source_link, &expected_sha256, std::env::consts::ARCH),
        Err(RuntimeGateArtifactError::SourceNotRegularFile)
    ));
}

#[test]
fn staging_classifies_digest_matching_loading_and_machine_mismatches_fail_closed() {
    let directory = tempfile::tempdir().expect("fixture directory should exist");
    let (source, mut bytes) = write_self_contained_gate(directory.path());

    let non_elf_bytes = vec![0_u8; bytes.len()];
    fs::write(&source, &non_elf_bytes).expect("non-ELF fixture should be writable");
    let non_elf_digest = format!("{:x}", Sha256::digest(&non_elf_bytes));
    assert!(matches!(
        RuntimeGateArtifact::stage(&source, &non_elf_digest, std::env::consts::ARCH),
        Err(RuntimeGateArtifactError::UnsafeExecutableLoadingBoundary)
    ));

    bytes[20..24].copy_from_slice(&0_u32.to_le_bytes());
    fs::write(&source, &bytes).expect("malformed ELF fixture should be writable");
    let malformed_digest = format!("{:x}", Sha256::digest(&bytes));
    assert!(matches!(
        RuntimeGateArtifact::stage(&source, &malformed_digest, std::env::consts::ARCH),
        Err(RuntimeGateArtifactError::UnsafeExecutableLoadingBoundary)
    ));

    let (_, mut machine_bytes) = write_self_contained_gate(directory.path());
    machine_bytes[18..20].copy_from_slice(&7_u16.to_le_bytes());
    fs::write(&source, &machine_bytes).expect("machine mismatch fixture should be writable");
    let machine_digest = format!("{:x}", Sha256::digest(&machine_bytes));
    assert!(matches!(
        RuntimeGateArtifact::stage(&source, &machine_digest, std::env::consts::ARCH),
        Err(RuntimeGateArtifactError::ArchitectureMismatch { .. })
    ));
}

#[test]
fn staging_rejects_digest_matching_gate_with_host_incompatible_elf_data_encoding() {
    let directory = tempfile::tempdir().expect("fixture directory should exist");
    let (source, bytes) = write_self_contained_gate(directory.path());
    let big_endian_bytes = big_endian_gate_bytes(bytes);
    fs::write(&source, &big_endian_bytes).expect("big-endian ELF fixture should be writable");
    let expected_sha256 = format!("{:x}", Sha256::digest(&big_endian_bytes));

    assert!(matches!(
        RuntimeGateArtifact::stage(&source, &expected_sha256, std::env::consts::ARCH),
        Err(RuntimeGateArtifactError::UnsafeExecutableLoadingBoundary)
    ));
}
