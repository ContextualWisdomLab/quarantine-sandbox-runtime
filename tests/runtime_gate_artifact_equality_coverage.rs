//! Runtime-gate artifact value-semantics coverage.

#![cfg(target_os = "linux")]

#[path = "support/runtime_gate_fixture.rs"]
mod runtime_gate_fixture;

use quarantine_sandbox_runtime::RuntimeGateArtifact;
use runtime_gate_fixture::write_self_contained_gate;
use sha2::{Digest, Sha256};

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
