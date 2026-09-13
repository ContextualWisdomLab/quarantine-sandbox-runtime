//! Public CLI argument-boundary regression coverage.
//!
//! These tests execute the shipped binary so every value-bearing flag retains its own
//! fail-closed missing-value boundary and every numeric resource flag retains its own parse error.

use std::process::Command;

fn run_cli(arguments: &[&str]) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_quarantine-sandbox-runtime"))
        .args(arguments)
        .output()
        .expect("CLI process should start")
}

#[test]
fn every_value_flag_fails_at_its_own_missing_value_boundary() {
    let value_flags = [
        "--request-id",
        "--memory-bytes",
        "--cpu-millicores",
        "--max-processes",
        "--timeout-seconds",
        "--tmpfs-bytes",
        "--podman",
        "--runtime-gate-path",
        "--runtime-gate-sha256",
        "--source-path",
        "--source-revision",
        "--source-tree-sha256",
    ];

    for flag in value_flags {
        let output = run_cli(&["run", "--image", "repo@sha256:deadbeef", flag]);
        assert_eq!(output.status.code(), Some(2), "flag={flag}");
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(
            stderr.contains(&format!("error: {flag} requires a value")),
            "flag={flag}, stderr={stderr:?}"
        );
    }
}

#[test]
fn every_numeric_resource_flag_fails_at_its_own_parse_boundary() {
    let numeric_flags = [
        "--memory-bytes",
        "--cpu-millicores",
        "--max-processes",
        "--timeout-seconds",
        "--tmpfs-bytes",
    ];

    for flag in numeric_flags {
        let output = run_cli(&[
            "run",
            "--image",
            "repo@sha256:deadbeef",
            flag,
            "not-a-number",
        ]);
        assert_eq!(output.status.code(), Some(2), "flag={flag}");
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(
            stderr.contains(&format!(
                "error: {flag} must be a non-negative integer, got \"not-a-number\""
            )),
            "flag={flag}, stderr={stderr:?}"
        );
    }
}
