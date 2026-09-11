//! Process-boundary CLI coverage for fail-closed runtime-gate admission.

#![cfg(target_os = "linux")]

use std::{fs, process::Command};

use sha2::{Digest, Sha256};
use tempfile::tempdir;

fn binary() -> &'static str {
    env!("CARGO_BIN_EXE_quarantine-sandbox-runtime")
}

fn digest_pinned_image() -> String {
    format!("repo@sha256:{}", "a".repeat(64))
}

#[test]
fn cli_rejects_command_execution_without_a_complete_runtime_gate_identity() {
    let image = digest_pinned_image();
    let output = Command::new(binary())
        .args(["run", "--image", image.as_str(), "--", "true"])
        .output()
        .expect("CLI binary should execute");

    assert_eq!(output.status.code(), Some(2));
    let stderr = String::from_utf8(output.stderr).expect("CLI stderr should be UTF-8");
    assert!(stderr.contains("runtime gate requires --runtime-gate-path and --runtime-gate-sha256"));
}

#[test]
fn cli_rejects_a_digest_matched_but_non_elf_runtime_gate_before_podman() {
    let directory = tempdir().expect("runtime-gate fixture directory");
    let gate_path = directory.path().join("invalid-runtime-gate");
    let bytes = b"not an ELF runtime gate";
    fs::write(&gate_path, bytes).expect("runtime-gate fixture should be writable");
    let digest = format!("{:x}", Sha256::digest(bytes));
    let image = digest_pinned_image();

    let output = Command::new(binary())
        .args([
            "run",
            "--image",
            image.as_str(),
            "--runtime-gate-path",
            gate_path.to_str().expect("temporary path should be UTF-8"),
            "--runtime-gate-sha256",
            digest.as_str(),
            "--podman",
            "/definitely/not/a/podman/binary",
            "--",
            "true",
        ])
        .output()
        .expect("CLI binary should execute");

    assert_eq!(output.status.code(), Some(2));
    let stderr = String::from_utf8(output.stderr).expect("CLI stderr should be UTF-8");
    assert!(stderr.contains("runtime gate admission failed"));
    assert!(
        !stderr.contains("backend process spawn failed"),
        "runtime-gate admission must fail before Podman is invoked"
    );
}
