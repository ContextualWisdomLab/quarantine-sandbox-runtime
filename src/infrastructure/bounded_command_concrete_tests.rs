use std::{path::Path, time::Duration};

use super::bounded_command::{BoundedCommandError, BoundedCommandRunner};

#[test]
fn concrete_child_success_preserves_bounded_output() {
    let args = vec![
        "-c".to_owned(),
        "printf 'safe-stdout'; printf 'safe-stderr' >&2".to_owned(),
    ];

    let result = BoundedCommandRunner::new(Duration::from_secs(1), 64)
        .run(Path::new("/bin/sh"), &args);
    assert!(result.is_ok(), "bounded shell command should complete: {result:?}");
    if let Ok(output) = result {
        assert!(output.status.success());
        assert_eq!(output.stdout, b"safe-stdout");
        assert_eq!(output.stderr, b"safe-stderr");
    }
}

#[test]
fn concrete_child_spawn_failure_is_typed() {
    let result = BoundedCommandRunner::new(Duration::from_millis(10), 64)
        .run(Path::new("/definitely-missing-qsr-command"), &[]);

    assert!(matches!(result, Err(BoundedCommandError::Spawn)));
}

#[test]
fn concrete_child_timeout_is_killed_and_reaped() {
    // Keep the workload inside the shell process itself. Spawning `sleep` here
    // would create a grandchild that killing the supervised shell cannot reap.
    let args = vec!["-c".to_owned(), "while :; do :; done".to_owned()];

    let result = BoundedCommandRunner::new(Duration::from_millis(1), 64)
        .run(Path::new("/bin/sh"), &args);

    assert!(matches!(result, Err(BoundedCommandError::Timeout)));
}

#[test]
fn concrete_child_output_overflow_is_killed_and_reaped() {
    let args = vec!["-c".to_owned(), "printf 'overflow'".to_owned()];

    let result = BoundedCommandRunner::new(Duration::from_secs(1), 4)
        .run(Path::new("/bin/sh"), &args);

    assert!(matches!(result, Err(BoundedCommandError::OutputLimit)));
}
