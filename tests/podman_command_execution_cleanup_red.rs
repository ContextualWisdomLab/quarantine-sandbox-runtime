//! RED regressions for command-output evidence and cleanup error handling.
//!
//! These tests exercise the production `RootlessPodmanAdapter` through a
//! checked-in immutable fake Podman executable. Runtime-written executable
//! scripts are deliberately avoided because parallel Unix process creation can
//! race with a newly written executable and surface ETXTBSY before the intended
//! backend scenario is reached.

#![cfg(target_os = "linux")]

use std::{
    fs,
    os::unix::fs::symlink,
    path::{Path, PathBuf},
    sync::atomic::{AtomicU64, Ordering},
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use quarantine_sandbox_runtime::{
    ApplicationServiceError, CommandExecutionError, CommandExecutionRequest, IsolationPolicy,
    ResourceRequest, RootlessPodmanAdapter,
};

static NEXT_PATH_ID: AtomicU64 = AtomicU64::new(0);

fn temporary_path(name: &str) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock should be after the Unix epoch")
        .as_nanos();
    let unique_id = NEXT_PATH_ID.fetch_add(1, Ordering::Relaxed);
    std::env::temp_dir().join(format!(
        "quarantine-sandbox-runtime-command-cleanup-red-{name}-{}-{nanos}-{unique_id}",
        std::process::id()
    ))
}

fn fixture_executable() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/fake_podman.sh")
}

fn config_path(program: &Path) -> PathBuf {
    PathBuf::from(format!("{}.config", program.display()))
}

fn create_fake_podman(name: &str, mode: &str) -> (PathBuf, PathBuf) {
    let program = temporary_path(name);
    let call_log = temporary_path(&format!("{name}-call-log"));
    symlink(fixture_executable(), &program).expect("fake Podman symlink should be creatable");
    fs::write(
        config_path(&program),
        format!(
            "MODE='{mode}'\nREADY_PORT='0'\nLOG='{}'\n",
            call_log.display()
        ),
    )
    .expect("fake Podman data config should be writable");
    (program, call_log)
}

fn remove_fixture(program: &Path, call_log: &Path) {
    let _ = fs::remove_file(program);
    let _ = fs::remove_file(config_path(program));
    let _ = fs::remove_file(call_log);
}

fn policy() -> IsolationPolicy {
    IsolationPolicy {
        policy_id: "command_cleanup_red_policy_v1".to_owned(),
        maximum_memory_bytes: 512 * 1024 * 1024,
        maximum_cpu_millicores: 2_000,
        maximum_processes: 64,
        maximum_lease_seconds: 300,
        maximum_tmpfs_bytes: 64 * 1024 * 1024,
        readiness_timeout_millis: 1_000,
        readiness_poll_interval_millis: 10,
        shutdown_grace_seconds: 2,
        run_as_user_id: 65_532,
        run_as_group_id: 65_532,
    }
}

fn request() -> CommandExecutionRequest {
    CommandExecutionRequest {
        schema_version: "1.0.0".to_owned(),
        request_id: "command-cleanup-red-request".to_owned(),
        image_reference: format!("localhost/cwl/tool@sha256:{}", "e".repeat(64)),
        command: vec!["pytest".to_owned(), "-q".to_owned()],
        source_artifact: None,
        resources: ResourceRequest {
            memory_bytes: 256 * 1024 * 1024,
            cpu_millicores: 1_000,
            maximum_processes: 16,
            lease_seconds: 20,
            tmpfs_bytes: 16 * 1024 * 1024,
        },
    }
}

fn assert_exact_log_and_cleanup_calls(call_log: &Path) {
    let calls = fs::read_to_string(call_log).expect("fake Podman calls should be recorded");
    assert!(
        calls
            .lines()
            .any(|line| line == "logs fake-command-container-id"),
        "log retrieval must target only the acquired container ID: {calls}"
    );
    assert!(
        calls
            .lines()
            .any(|line| line == "rm --force --ignore fake-command-container-id"),
        "cleanup must target only the acquired container ID: {calls}"
    );
}

#[test]
fn nonzero_container_logs_status_is_backend_failure_and_cleanup_is_attempted() {
    let (program, call_log) = create_fake_podman("nonzero-logs", "command_nonzero_logs");
    let adapter = RootlessPodmanAdapter::new(program.clone());

    let error = adapter
        .run_legacy_command_at_for_test(&request(), &policy(), 1_780_000_000)
        .unwrap_err();

    assert_eq!(
        error,
        CommandExecutionError::Backend(ApplicationServiceError::BackendCommandFailed {
            operation: "container_logs",
        })
    );
    assert_exact_log_and_cleanup_calls(&call_log);
    remove_fixture(&program, &call_log);
}

#[test]
fn container_logs_timeout_is_backend_timeout_and_cleanup_is_attempted() {
    let (program, call_log) = create_fake_podman("logs-timeout", "command_logs_timeout");
    let adapter = RootlessPodmanAdapter::new(program.clone())
        .with_command_timeout(Duration::from_millis(250));

    let error = adapter
        .run_legacy_command_at_for_test(&request(), &policy(), 1_780_000_000)
        .unwrap_err();

    assert_eq!(
        error,
        CommandExecutionError::Backend(ApplicationServiceError::BackendCommandTimedOut {
            operation: "container_logs",
        })
    );
    assert_exact_log_and_cleanup_calls(&call_log);
    remove_fixture(&program, &call_log);
}

#[test]
fn cleanup_failure_is_not_hidden_behind_container_start_failure() {
    let (program, call_log) =
        create_fake_podman("start-and-cleanup-fail", "command_start_cleanup_fail");
    let adapter = RootlessPodmanAdapter::new(program.clone());

    let error = adapter
        .run_legacy_command_at_for_test(&request(), &policy(), 1_780_000_000)
        .unwrap_err();

    assert_eq!(
        error,
        CommandExecutionError::Backend(ApplicationServiceError::CleanupFailed)
    );
    let calls = fs::read_to_string(&call_log).expect("fake Podman calls should be recorded");
    assert!(calls.lines().any(|line| line.starts_with("start ")));
    assert!(calls.lines().any(|line| line.starts_with("rm --force ")));
    remove_fixture(&program, &call_log);
}

#[test]
fn cleanup_failure_is_not_hidden_behind_container_logs_failure() {
    let (program, call_log) =
        create_fake_podman("logs-and-cleanup-fail", "command_logs_cleanup_fail");
    let adapter = RootlessPodmanAdapter::new(program.clone());

    let error = adapter
        .run_legacy_command_at_for_test(&request(), &policy(), 1_780_000_000)
        .unwrap_err();

    assert_eq!(
        error,
        CommandExecutionError::Backend(ApplicationServiceError::CleanupFailed)
    );
    let calls = fs::read_to_string(&call_log).expect("fake Podman calls should be recorded");
    assert!(calls.lines().any(|line| line.starts_with("logs ")));
    assert!(calls.lines().any(|line| line.starts_with("rm --force ")));
    remove_fixture(&program, &call_log);
}

#[test]
fn cleanup_failure_is_not_hidden_behind_container_logs_timeout() {
    let (program, call_log) = create_fake_podman(
        "logs-timeout-and-cleanup-fail",
        "command_logs_timeout_cleanup_fail",
    );
    let adapter =
        RootlessPodmanAdapter::new(program.clone()).with_command_timeout(Duration::from_secs(2));

    let error = adapter
        .run_legacy_command_at_for_test(&request(), &policy(), 1_780_000_000)
        .unwrap_err();

    assert_eq!(
        error,
        CommandExecutionError::Backend(ApplicationServiceError::CleanupFailed)
    );
    let calls = fs::read_to_string(&call_log).expect("fake Podman calls should be recorded");
    assert!(calls.lines().any(|line| line.starts_with("logs ")));
    assert!(calls.lines().any(|line| line.starts_with("rm --force ")));
    remove_fixture(&program, &call_log);
}

#[test]
fn cleanup_failure_is_not_hidden_behind_effective_isolation_failure() {
    let (program, call_log) = create_fake_podman(
        "isolation-and-cleanup-fail",
        "command_isolation_cleanup_fail",
    );
    let adapter = RootlessPodmanAdapter::new(program.clone());

    let error = adapter
        .run_legacy_command_at_for_test(&request(), &policy(), 1_780_000_000)
        .unwrap_err();

    assert_eq!(
        error,
        CommandExecutionError::Backend(ApplicationServiceError::CleanupFailed)
    );
    let calls = fs::read_to_string(&call_log).expect("fake Podman calls should be recorded");
    assert!(
        calls
            .lines()
            .any(|line| line.starts_with("container inspect "))
    );
    assert!(calls.lines().any(|line| line.starts_with("rm --force ")));
    remove_fixture(&program, &call_log);
}
