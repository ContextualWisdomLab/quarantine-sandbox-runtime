//! Process-level acceptance for `quarantine-sandbox-runtime validate-analysis-request`.
//!
//! A schema consumer that cannot embed the Rust crate can still validate one
//! request against the executable contract through the shipped binary. This
//! witness drives the real executable over stdin and checks its exit codes and
//! one-line JSON verdict.

use std::{
    io::Write,
    process::{Command, Stdio},
};

#[cfg(unix)]
use std::{ffi::OsStr, os::unix::ffi::OsStrExt};

use serde_json::Value;

fn validate(input: &[u8], extra_args: &[&str]) -> (i32, Value, String) {
    let mut child = Command::new(env!("CARGO_BIN_EXE_quarantine-sandbox-runtime"))
        .arg("validate-analysis-request")
        .args(extra_args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("binary must start");
    child
        .stdin
        .take()
        .expect("stdin")
        .write_all(input)
        .expect("write request");
    let output = child.wait_with_output().expect("binary must finish");
    let stdout = String::from_utf8(output.stdout).expect("stdout is UTF-8");
    assert_eq!(stdout.lines().count(), 1, "exactly one verdict line");
    (
        output.status.code().expect("exit code"),
        serde_json::from_str(&stdout).expect("verdict is JSON"),
        stdout,
    )
}

#[test]
fn binary_accepts_a_gregorian_century_leap_day() {
    let (code, verdict, _) = validate(
        br#"{"schema_version":"1.0.0","request_id":"cli-1","profile":"static_only","bounded_source_context":{"submitted_at":"2000-02-29T00:00:00Z"}}"#,
        &[],
    );
    assert_eq!(code, 0);
    assert_eq!(verdict["valid"], true);
}

#[test]
fn binary_rejects_a_non_400_century_leap_day_with_exit_one() {
    let (code, verdict, _) = validate(
        br#"{"schema_version":"1.0.0","request_id":"cli-2","profile":"static_only","bounded_source_context":{"submitted_at":"1900-02-29T00:00:00Z"}}"#,
        &[],
    );
    assert_eq!(code, 1);
    assert_eq!(verdict["valid"], false);
    assert_eq!(verdict["error"], "invalid submitted_at timestamp");
}

#[test]
fn binary_refuses_unusable_input_with_exit_two_without_echoing_it() {
    let (code, verdict, stdout) = validate(b"\xffSECRET", &[]);
    assert_eq!(code, 2);
    assert_eq!(verdict["valid"], false);
    assert!(!stdout.contains("SECRET"));
    let output = Command::new(env!("CARGO_BIN_EXE_quarantine-sandbox-runtime"))
        .args(["validate-analysis-request", "--unexpected"])
        .stdin(Stdio::null())
        .output()
        .expect("binary must start");
    assert_eq!(output.status.code(), Some(2));
    let verdict: Value = serde_json::from_slice(&output.stdout).expect("verdict is JSON");
    assert_eq!(verdict["error"], "unexpected argument");
}

#[cfg(unix)]
#[test]
fn binary_maps_a_non_utf8_argument_to_exit_two_instead_of_panicking() {
    let output = Command::new(env!("CARGO_BIN_EXE_quarantine-sandbox-runtime"))
        .arg("validate-analysis-request")
        .arg(OsStr::from_bytes(b"\xff"))
        .stdin(Stdio::null())
        .output()
        .expect("binary must start");
    assert_eq!(output.status.code(), Some(2));
    let verdict: Value = serde_json::from_slice(&output.stdout).expect("verdict is JSON");
    assert_eq!(verdict["valid"], false);
}

#[test]
fn binary_rejects_non_i_json_lone_surrogate_with_exit_one() {
    let (code, verdict, _) = validate(
        br#"{"schema_version":"1.0.0","request_id":"\ud800","profile":"static_only"}"#,
        &[],
    );
    assert_eq!(
        code, 1,
        "a lone surrogate is not I-JSON and must not validate"
    );
    assert_eq!(verdict["valid"], false);
}

#[test]
fn binary_rejects_a_nested_duplicate_submitted_at_with_exit_one() {
    let (code, verdict, _) = validate(
        br#"{"schema_version":"1.0.0","request_id":"dup","profile":"static_only","bounded_source_context":{"submitted_at":"1900-02-29T00:00:00Z","submitted_at":"2000-02-29T00:00:00Z"}}"#,
        &[],
    );
    assert_eq!(
        code, 1,
        "a last-wins parse must not hide the invalid first member"
    );
    assert_eq!(verdict["valid"], false);
    assert_eq!(
        verdict["error"], "malformed JSON or unknown, duplicate, or mistyped field",
        "the rejection must come from the I-JSON parse, not from the first value"
    );
}

#[test]
fn binary_rejects_a_lone_surrogate_in_submitted_at_with_exit_one() {
    let (code, verdict, _) = validate(
        br#"{"schema_version":"1.0.0","request_id":"sur","profile":"static_only","bounded_source_context":{"submitted_at":"\ud800024-02-29T00:00:00Z"}}"#,
        &[],
    );
    assert_eq!(code, 1);
    assert_eq!(verdict["valid"], false);
    assert_eq!(
        verdict["error"], "malformed JSON or unknown, duplicate, or mistyped field",
        "an unpaired surrogate must be refused by the parser, not replaced"
    );
}

#[test]
fn binary_without_a_subcommand_keeps_the_run_usage_contract() {
    let output = Command::new(env!("CARGO_BIN_EXE_quarantine-sandbox-runtime"))
        .stdin(Stdio::null())
        .output()
        .expect("binary must start");
    assert_eq!(output.status.code(), Some(2));
    assert!(
        output.stdout.is_empty(),
        "usage errors never print a verdict"
    );
    assert!(String::from_utf8_lossy(&output.stderr).contains("missing subcommand"));
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("validate-analysis-request"),
        "usage text must advertise the validation subcommand"
    );
}

#[cfg(unix)]
#[test]
fn binary_maps_a_non_utf8_run_argument_to_exit_two_instead_of_panicking() {
    let output = Command::new(env!("CARGO_BIN_EXE_quarantine-sandbox-runtime"))
        .arg("run")
        .arg(OsStr::from_bytes(b"\xff"))
        .stdin(Stdio::null())
        .output()
        .expect("binary must start");
    assert_eq!(output.status.code(), Some(2));
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("arguments must be valid UTF-8"));
    assert!(!stderr.contains("panicked"));
}

#[test]
fn binary_does_not_panic_when_stdout_is_closed() {
    let mut child = Command::new(env!("CARGO_BIN_EXE_quarantine-sandbox-runtime"))
        .arg("validate-analysis-request")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("binary must start");
    drop(child.stdout.take());
    child
        .stdin
        .take()
        .expect("stdin")
        .write_all(br#"{"schema_version":"1.0.0","request_id":"r","profile":"static_only"}"#)
        .expect("write request");
    let output = child.wait_with_output().expect("binary must finish");
    assert_eq!(
        output.status.code(),
        Some(2),
        "closed stdout is an unusable output channel"
    );
    assert!(
        !String::from_utf8_lossy(&output.stderr).contains("panicked"),
        "a closed stdout must not panic"
    );
}
