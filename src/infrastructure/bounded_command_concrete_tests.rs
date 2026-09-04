use std::{path::Path, time::Duration};

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
fn concrete_child_completion_preserves_terminal_facts() {
    let outcome = BoundedCommandRunner::new(Duration::from_secs(1), 64)
        .run_to_completion(
            Path::new("/bin/sh"),
            &["-c".to_owned(), "printf done".to_owned()],
        )
        .map(|outcome| {
            (
                outcome.status.is_some_and(|status| status.success()),
                outcome.timed_out,
                outcome.stdout,
                outcome.stdout_truncated,
                outcome.stderr,
                outcome.stderr_truncated,
            )
        });

    assert_eq!(
        outcome,
        Ok((true, false, b"done".to_vec(), false, vec![], false))
    );
}

#[test]
fn concrete_child_spawn_failure_is_typed() {
    let error = BoundedCommandRunner::new(Duration::from_millis(10), 64)
        .run(Path::new("/definitely-missing-qsr-command"), &[])
        .err();

    assert_eq!(error, Some(BoundedCommandError::Spawn));
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
fn concrete_child_output_overflow_is_killed_and_reaped() {
    let args = vec!["-c".to_owned(), "printf 'overflow'".to_owned()];

    let error = BoundedCommandRunner::new(Duration::from_secs(1), 4)
        .run(Path::new("/bin/sh"), &args)
        .err();

    assert_eq!(error, Some(BoundedCommandError::OutputLimit));
}
