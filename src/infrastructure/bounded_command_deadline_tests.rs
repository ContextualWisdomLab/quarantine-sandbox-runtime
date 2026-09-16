#![cfg(all(test, unix))]

use std::{
    panic::{AssertUnwindSafe, catch_unwind},
    path::Path,
    time::Duration,
};

use super::bounded_command::{BoundedCommandError, BoundedCommandRunner};

#[test]
fn unrepresentable_command_deadline_fails_closed_without_panicking() {
    let args = vec!["-c".to_owned(), "exit 0".to_owned()];
    let outcome = catch_unwind(AssertUnwindSafe(|| {
        BoundedCommandRunner::new(Duration::MAX, 64).run(Path::new("/bin/sh"), &args)
    }));

    assert!(
        outcome.is_ok(),
        "an unrepresentable monotonic deadline must return a typed error instead of panicking"
    );
    assert!(
        outcome.expect("panic admission is checked above").is_err(),
        "an unrepresentable command deadline must fail closed"
    );
}

#[cfg(target_os = "linux")]
#[test]
fn detached_descendant_output_limit_preserves_cleanup_failure() {
    let args = vec![
        "-c".to_owned(),
        "/usr/bin/setsid /bin/sh -c 'sleep 0.1; printf \"%0128d\" 0; sleep 0.3' & exit 0"
            .to_owned(),
    ];

    assert_eq!(
        BoundedCommandRunner::new(Duration::from_secs(1), 64)
            .run(Path::new("/bin/sh"), &args)
            .map(|_| ()),
        Err(BoundedCommandError::Wait),
        "a detached descendant that overflows a retained pipe after the supervised child exits must preserve cleanup failure when the original process group no longer exists"
    );
}
