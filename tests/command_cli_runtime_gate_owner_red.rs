//! Causal RED for the production CLI owner path of issue #25.
//!
//! A caller must not be able to reach Podman command execution without first supplying an
//! independently trusted runtime-gate artifact identity. The low-level direct-consumer adapter
//! remains useful for migration tests, but the shipped CLI is a release-authorized owner path and
//! must fail before any backend process is spawned when no gate path/digest is configured.

#![cfg(target_os = "linux")]

use std::{fs, os::unix::fs::PermissionsExt, process::Command};

#[test]
fn cli_without_runtime_gate_identity_fails_before_spawning_podman() {
    let fixture = tempfile::tempdir().expect("isolated CLI owner-path fixture should exist");
    let podman = fixture.path().join("podman");
    let touched = fixture.path().join("podman-was-spawned");
    fs::write(
        &podman,
        format!("#!/bin/sh\nset -eu\n: > '{}'\nexit 91\n", touched.display()),
    )
    .expect("fake Podman should be writable");
    let mut permissions = fs::metadata(&podman)
        .expect("fake Podman metadata should exist")
        .permissions();
    permissions.set_mode(0o700);
    fs::set_permissions(&podman, permissions).expect("fake Podman should be executable");

    let image = format!("localhost/cwl/tool@sha256:{}", "a".repeat(64));
    let output = Command::new(env!("CARGO_BIN_EXE_quarantine-sandbox-runtime"))
        .args([
            "run",
            "--image",
            &image,
            "--podman",
            podman
                .to_str()
                .expect("temporary Podman path should be UTF-8"),
            "--",
            "true",
        ])
        .output()
        .expect("CLI process should execute");

    assert_eq!(
        output.status.code(),
        Some(2),
        "missing independent runtime-gate identity must be a CLI configuration error"
    );
    assert!(
        !touched.exists(),
        "production CLI spawned Podman before independently verifying a runtime-gate artifact"
    );
    let stderr = String::from_utf8(output.stderr).expect("CLI diagnostics should remain UTF-8");
    assert!(
        stderr.contains("runtime gate") || stderr.contains("runtime-gate"),
        "configuration error should identify the missing runtime-gate trust input: {stderr}"
    );
}
