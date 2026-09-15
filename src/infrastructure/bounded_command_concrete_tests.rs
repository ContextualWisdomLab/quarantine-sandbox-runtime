use std::{
    io::ErrorKind,
    path::Path,
    time::{Duration, Instant},
};

use super::bounded_command::{BoundedCommandError, BoundedCommandRunner};

#[test]
fn concrete_child_success_preserves_bounded_output() {
    let args = vec![
        "-c".to_owned(),
        "printf 'safe-stdout'; printf 'safe-stderr' >&2".to_owned(),
    ];

    let result = BoundedCommandRunner::new(Duration::from_secs(1), 64)
        .run(Path::new("/bin/sh"), &args)
        .map(|output| (output.status.success(), output.stdout, output.stderr));

    assert_eq!(
        result,
        Ok((true, b"safe-stdout".to_vec(), b"safe-stderr".to_vec(),))
    );
}

#[test]
fn concrete_child_spawn_failure_is_typed() {
    let error = BoundedCommandRunner::new(Duration::from_millis(10), 64)
        .run(Path::new("/definitely-missing-qsr-command"), &[])
        .err();

    assert_eq!(error, Some(BoundedCommandError::Spawn(ErrorKind::NotFound)));
}

#[test]
fn concrete_child_timeout_is_killed_and_reaped() {
    // Keep the workload inside the shell process itself. Spawning `sleep` here
    // would create a grandchild that killing the supervised shell cannot reap.
    let args = vec!["-c".to_owned(), "while :; do :; done".to_owned()];

    let error = BoundedCommandRunner::new(Duration::from_millis(1), 64)
        .run(Path::new("/bin/sh"), &args)
        .err();

    assert_eq!(error, Some(BoundedCommandError::Timeout));
}

#[test]
fn descendant_holding_output_pipe_cannot_outlive_command_deadline() {
    // The direct shell exits immediately while the background descendant keeps
    // stdout/stderr open. The invocation deadline must cover the whole process
    // tree and capture lifecycle, not only the direct child wait.
    let args = vec!["-c".to_owned(), "sleep 3 & exit 0".to_owned()];
    let started_at = Instant::now();

    let error = BoundedCommandRunner::new(Duration::from_millis(100), 64)
        .run(Path::new("/bin/sh"), &args)
        .err();
    let elapsed = started_at.elapsed();

    assert_eq!(error, Some(BoundedCommandError::Timeout));
    assert!(
        elapsed < Duration::from_secs(2),
        "descendant-held capture exceeded the command deadline envelope: {elapsed:?}"
    );
}

#[test]
fn concrete_child_output_overflow_is_killed_and_reaped() {
    let args = vec!["-c".to_owned(), "printf 'overflow'".to_owned()];

    let error = BoundedCommandRunner::new(Duration::from_secs(1), 4)
        .run(Path::new("/bin/sh"), &args)
        .err();

    assert_eq!(error, Some(BoundedCommandError::OutputLimit));
}
