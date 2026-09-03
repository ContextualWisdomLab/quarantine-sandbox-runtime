use std::{path::Path, time::Duration};

use super::bounded_command::{BoundedCommandError, BoundedCommandRunner};

#[test]
fn concrete_child_success_preserves_bounded_output() {
    let args = vec![
        "-c".to_owned(),
        "printf 'safe-stdout'; printf 'safe-stderr' >&2".to_owned(),
    ];

    let output = BoundedCommandRunner::new(Duration::from_secs(1), 64)
        .run(Path::new("/bin/sh"), &args)
        .expect("bounded shell command should complete");

    assert!(output.status.success());
    assert_eq!(output.stdout, b"safe-stdout");
    assert_eq!(output.stderr, b"safe-stderr");
}

#[test]
fn concrete_child_spawn_failure_is_typed() {
    let error = BoundedCommandRunner::new(Duration::from_millis(10), 64)
        .run(Path::new("/definitely-missing-qsr-command"), &[])
        .expect_err("missing executable must fail closed");

    assert_eq!(error, BoundedCommandError::Spawn);
}

#[test]
fn concrete_child_timeout_is_killed_and_reaped() {
    // Keep the workload inside the shell process itself. Spawning `sleep` here
    // would create a grandchild that killing the supervised shell cannot reap.
    let args = vec!["-c".to_owned(), "while :; do :; done".to_owned()];

    let error = BoundedCommandRunner::new(Duration::from_millis(1), 64)
        .run(Path::new("/bin/sh"), &args)
        .expect_err("long-running command must be terminated at its deadline");

    assert_eq!(error, BoundedCommandError::Timeout);
}

#[test]
fn concrete_child_output_overflow_is_killed_and_reaped() {
    let args = vec!["-c".to_owned(), "printf 'overflow'".to_owned()];

    let error = BoundedCommandRunner::new(Duration::from_secs(1), 4)
        .run(Path::new("/bin/sh"), &args)
        .expect_err("output beyond the retained budget must fail closed");

    assert_eq!(error, BoundedCommandError::OutputLimit);
}
