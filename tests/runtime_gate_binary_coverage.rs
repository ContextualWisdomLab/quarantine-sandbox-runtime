//! Behavioral coverage for the runtime-owned pre-exec gate binary.
//!
//! These cases exercise the process boundary rather than duplicating the gate parser in test
//! code: invalid invocations must fail before readiness, release tokens are bounded and exact,
//! successful release emits its trusted acknowledgement before `exec`, and the consumer inherits
//! no controller stdin authority.

#![cfg(target_os = "linux")]

use std::{
    io::Write,
    process::{Command, Output, Stdio},
};

const GATE_BINARY: &str = env!("CARGO_BIN_EXE_qsr_runtime_gate");
const READY_MARKER: &str = "QSR_GATE_READY\n";
const RELEASED_MARKER: &str = "QSR_GATE_RELEASED\n";

fn run_gate(arguments: &[&str], release_input: &[u8]) -> Output {
    let mut child = Command::new(GATE_BINARY)
        .args(arguments)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("runtime gate should spawn");
    child
        .stdin
        .take()
        .expect("runtime gate stdin should be piped")
        .write_all(release_input)
        .expect("release input should be writable");
    child
        .wait_with_output()
        .expect("runtime gate should terminate")
}

#[test]
fn invalid_invocations_fail_before_the_ready_marker() {
    for arguments in [
        Vec::<&str>::new(),
        vec!["token-only"],
        vec!["", "/bin/true"],
        vec![&"x".repeat(129), "/bin/true"],
    ] {
        let output = run_gate(&arguments, b"");
        assert_eq!(output.status.code(), Some(64));
        assert!(output.stdout.is_empty());
    }
}

#[test]
fn eof_and_wrong_release_tokens_never_release_the_consumer() {
    for release_input in [&b""[..], &b"wrong\n"[..]] {
        let output = run_gate(&["expected", "/bin/true"], release_input);
        assert_eq!(output.status.code(), Some(77));
        assert_eq!(output.stdout, READY_MARKER.as_bytes());
    }
}

#[test]
fn oversized_release_input_fails_the_control_channel_closed() {
    let output = run_gate(&["expected", "/bin/true"], &vec![b'x'; 129]);
    assert_eq!(output.status.code(), Some(79));
    assert_eq!(output.stdout, READY_MARKER.as_bytes());
}

#[test]
fn exact_bounded_release_acknowledges_before_successful_exec() {
    let token = "x".repeat(128);
    let output = run_gate(&[&token, "/bin/true"], format!("{token}\n").as_bytes());
    assert_eq!(output.status.code(), Some(0));
    assert_eq!(
        output.stdout,
        format!("{READY_MARKER}{RELEASED_MARKER}").as_bytes()
    );
}

#[test]
fn released_consumer_cannot_read_the_controller_release_channel() {
    let output = run_gate(
        &[
            "release",
            "/bin/sh",
            "-c",
            "read value && exit 90 || exit 0",
        ],
        b"release\nconsumer-secret\n",
    );
    assert_eq!(output.status.code(), Some(0));
    assert_eq!(
        output.stdout,
        format!("{READY_MARKER}{RELEASED_MARKER}").as_bytes()
    );
}

#[test]
fn exec_failure_is_typed_by_exit_status_after_release_acknowledgement() {
    let output = run_gate(&["release", "/definitely-not-a-qsr-consumer"], b"release\n");
    assert_eq!(output.status.code(), Some(78));
    assert_eq!(
        output.stdout,
        format!("{READY_MARKER}{RELEASED_MARKER}").as_bytes()
    );
    assert!(String::from_utf8_lossy(&output.stderr).contains("qsr runtime gate exec failed"));
}
