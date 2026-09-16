#![cfg(all(test, unix))]

use std::{
    panic::{AssertUnwindSafe, catch_unwind},
    path::Path,
    time::Duration,
};

use super::bounded_command::BoundedCommandRunner;

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
        outcome
            .expect("panic admission is checked above")
            .is_err(),
        "an unrepresentable command deadline must fail closed"
    );
}
