//! Public CLI transport coverage for explicit resource, identity, and source overrides.
//!
//! The witness stops at runtime-gate admission so argument parsing is exercised through the real
//! binary without invoking Podman or weakening the runtime isolation contract.

use std::process::Command;

#[test]
fn cli_parses_every_override_before_failing_closed_on_missing_runtime_gate() {
    let temporary_directory = tempfile::tempdir().expect("temporary directory should be available");
    let missing_runtime_gate = temporary_directory.path().join("missing-runtime-gate");
    let source_path = temporary_directory.path().join("source-tree");
    let runtime_gate_sha256 = "a".repeat(64);
    let source_tree_sha256 = "b".repeat(64);
    let source_revision = "c".repeat(40);

    let output = Command::new(env!("CARGO_BIN_EXE_quarantine-sandbox-runtime"))
        .args([
            "run",
            "--image",
            "registry.example.invalid/runtime@sha256:dddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddd",
            "--request-id",
            "cli-override-coverage",
            "--memory-bytes",
            "1048576",
            "--cpu-millicores",
            "250",
            "--max-processes",
            "8",
            "--timeout-seconds",
            "30",
            "--tmpfs-bytes",
            "2097152",
            "--podman",
            "/definitely/missing/podman",
            "--runtime-gate-path",
        ])
        .arg(&missing_runtime_gate)
        .args([
            "--runtime-gate-sha256",
            &runtime_gate_sha256,
            "--source-path",
        ])
        .arg(&source_path)
        .args([
            "--source-revision",
            &source_revision,
            "--source-tree-sha256",
            &source_tree_sha256,
            "--",
            "true",
        ])
        .output()
        .expect("CLI process should start");

    assert_eq!(output.status.code(), Some(2));
    let stderr = String::from_utf8(output.stderr).expect("CLI diagnostics should be UTF-8");
    assert!(stderr.contains("runtime gate admission failed"));
    assert!(!stderr.contains("must be a non-negative integer"));
    assert!(!stderr.contains("requires a value"));
}
