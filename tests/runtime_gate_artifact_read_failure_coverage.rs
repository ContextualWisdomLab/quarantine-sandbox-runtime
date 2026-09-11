//! Fail-closed coverage for a regular-looking source that cannot be read as release bytes.

#![cfg(target_os = "linux")]

use std::{fs, path::Path};

use quarantine_sandbox_runtime::{RuntimeGateArtifact, RuntimeGateArtifactError};

#[test]
fn runtime_gate_artifact_rejects_unreadable_regular_proc_source() {
    let source = Path::new("/proc/self/mem");
    let metadata = fs::symlink_metadata(source).expect("Linux procfs should expose /proc/self/mem");
    assert!(
        metadata.file_type().is_file(),
        "the witness must pass the no-follow regular-file admission check before read failure"
    );

    let error = RuntimeGateArtifact::stage(source, &"0".repeat(64), std::env::consts::ARCH)
        .expect_err("a regular-looking source that cannot be read as bytes must fail closed");

    assert!(matches!(error, RuntimeGateArtifactError::SourceReadFailed));
}
