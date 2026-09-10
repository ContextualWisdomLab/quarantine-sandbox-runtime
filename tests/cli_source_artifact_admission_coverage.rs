//! Process-boundary CLI coverage for complete PR-source input admission.

#![cfg(target_os = "linux")]

#[path = "support/runtime_gate_fixture.rs"]
mod runtime_gate_fixture;

use std::{fs, os::unix::fs::PermissionsExt, process::Command};

use runtime_gate_fixture::write_self_contained_gate;
use sha2::{Digest, Sha256};
use tempfile::tempdir;

fn binary() -> &'static str {
    env!("CARGO_BIN_EXE_quarantine-sandbox-runtime")
}

fn digest_pinned_image() -> String {
    format!("repo@sha256:{}", "a".repeat(64))
}

#[test]
fn complete_source_flags_with_malformed_revision_fail_before_podman() {
    let directory = tempdir().expect("CLI source fixture directory should exist");
    let (gate_path, gate_bytes) = write_self_contained_gate(directory.path());
    let gate_digest = format!("{:x}", Sha256::digest(&gate_bytes));

    let source_path = directory.path().join("source");
    fs::create_dir(&source_path).expect("source fixture directory should be creatable");
    fs::write(source_path.join("artifact.txt"), b"exact source bytes")
        .expect("source fixture should be writable");

    let invoked_marker = directory.path().join("podman-invoked");
    let fake_podman = directory.path().join("fake-podman");
    fs::write(
        &fake_podman,
        format!(
            "#!/bin/sh\nset -eu\ntouch '{}'\nexit 99\n",
            invoked_marker.display()
        ),
    )
    .expect("fake Podman should be writable");
    let mut permissions = fs::metadata(&fake_podman)
        .expect("fake Podman metadata should exist")
        .permissions();
    permissions.set_mode(0o700);
    fs::set_permissions(&fake_podman, permissions).expect("fake Podman should be executable");

    let image = digest_pinned_image();
    let tree_digest = "0".repeat(64);
    let output = Command::new(binary())
        .args([
            "run",
            "--image",
            image.as_str(),
            "--runtime-gate-path",
            gate_path
                .to_str()
                .expect("temporary gate path should be UTF-8"),
            "--runtime-gate-sha256",
            gate_digest.as_str(),
            "--source-path",
            source_path
                .to_str()
                .expect("temporary source path should be UTF-8"),
            "--source-revision",
            "not-a-git-object-id",
            "--source-tree-sha256",
            tree_digest.as_str(),
            "--podman",
            fake_podman
                .to_str()
                .expect("temporary fake-Podman path should be UTF-8"),
            "--",
            "true",
        ])
        .output()
        .expect("CLI binary should execute");

    assert_eq!(output.status.code(), Some(2));
    let stderr = String::from_utf8(output.stderr).expect("CLI stderr should be UTF-8");
    assert!(
        stderr.contains("PR source artifact rejected: invalid_input"),
        "malformed source revision must surface the source-artifact validation boundary: {stderr}"
    );
    assert!(
        !invoked_marker.exists(),
        "malformed source identity must fail before Podman receives execution authority"
    );
}
