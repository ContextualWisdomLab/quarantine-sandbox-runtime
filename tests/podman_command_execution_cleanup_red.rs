//! RED regressions for command-output evidence and cleanup error handling.
//!
//! These tests intentionally exercise the production `RootlessPodmanAdapter`
//! through a fake Podman executable. They pin the review finding that backend
//! `podman logs` failures are infrastructure errors, not workload output, and
//! that a failed cleanup attempt must never be discarded behind an earlier
//! backend error.

#![cfg(target_os = "linux")]

use std::{
    fs,
    os::unix::fs::PermissionsExt,
    path::PathBuf,
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

fn write_executable(name: &str, script: &str) -> PathBuf {
    let program = temporary_path(name);
    fs::write(&program, script).expect("fake Podman should be writable");
    let mut permissions = fs::metadata(&program)
        .expect("fake Podman metadata should exist")
        .permissions();
    permissions.set_mode(0o700);
    fs::set_permissions(&program, permissions).expect("fake Podman should be executable");
    program
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
        resources: ResourceRequest {
            memory_bytes: 256 * 1024 * 1024,
            cpu_millicores: 1_000,
            maximum_processes: 16,
            lease_seconds: 20,
            tmpfs_bytes: 16 * 1024 * 1024,
        },
    }
}

fn security_info_json() -> &'static str {
    r#"{"host":{"security":{"rootless":true,"seccompEnabled":true,"seccompProfilePath":"/usr/share/containers/seccomp.json","apparmorEnabled":true,"selinuxEnabled":false}},"version":{"Version":"6.1.0"}}"#
}

fn container_inspect_json() -> &'static str {
    r#"[{"Id":"fake-command-container-id","AppArmorProfile":"containers-default","ProcessLabel":"","EffectiveCaps":null,"BoundingCaps":null,"Config":{"User":"65532:65532"},"HostConfig":{"ReadonlyRootfs":true,"Privileged":false,"SecurityOpt":["no-new-privileges"],"UsernsMode":"","Annotations":{"io.podman.annotations.userns":"auto"},"PidMode":"private","IpcMode":"none","Memory":268435456,"NanoCpus":1000000000,"PidsLimit":16}}]"#
}

fn top_output() -> &'static str {
    "PID SECCOMP CAPEFF CAPBND CAPINH CAPPRM CAPAMB LABEL\n1 filter - - - - - containers-default (enforce)\n"
}

#[test]
fn nonzero_container_logs_status_is_backend_failure_and_cleanup_is_attempted() {
    let call_log = temporary_path("nonzero-logs-call-log");
    let script = format!(
        "#!/bin/sh\nset -eu\nprintf '%s\\n' \"$*\" >> '{}'\ncase \"${{1:-}}:${{2:-}}\" in\n  \
         info:--format) printf '%s\\n' '{}' ;;\n  \
         create:--name) printf 'fake-command-container-id\\n' ;;\n  \
         start:*) : ;;\n  \
         container:inspect) printf '%s\\n' '{}' ;;\n  \
         top:*) printf '%s' '{}' ;;\n  \
         wait:*) printf '0\\n' ;;\n  \
         logs:*) printf 'podman logs backend failure\\n' >&2; exit 42 ;;\n  \
         rm:--force) : ;;\n  \
         *) exit 91 ;;\nesac\n",
        call_log.display(),
        security_info_json(),
        container_inspect_json(),
        top_output(),
    );
    let program = write_executable("nonzero-logs", &script);
    let adapter = RootlessPodmanAdapter::new(program.clone());

    let error = adapter
        .run_command_at(&request(), &policy(), 1_780_000_000)
        .unwrap_err();

    assert_eq!(
        error,
        CommandExecutionError::Backend(ApplicationServiceError::BackendCommandFailed {
            operation: "container_logs",
        })
    );
    let calls = fs::read_to_string(&call_log).expect("fake Podman calls should be recorded");
    assert!(calls.lines().any(|line| line.starts_with("logs ")));
    assert!(calls.lines().any(|line| line.starts_with("rm --force ")));

    let _ = fs::remove_file(program);
    let _ = fs::remove_file(call_log);
}

#[test]
fn cleanup_failure_is_not_hidden_behind_container_start_failure() {
    let call_log = temporary_path("start-and-cleanup-fail-call-log");
    let script = format!(
        "#!/bin/sh\nset -eu\nprintf '%s\\n' \"$*\" >> '{}'\ncase \"${{1:-}}:${{2:-}}\" in\n  \
         info:--format) printf '%s\\n' '{}' ;;\n  \
         create:--name) printf 'fake-command-container-id\\n' ;;\n  \
         start:*) exit 17 ;;\n  \
         rm:--force) exit 88 ;;\n  \
         *) exit 91 ;;\nesac\n",
        call_log.display(),
        security_info_json(),
    );
    let program = write_executable("start-and-cleanup-fail", &script);
    let adapter = RootlessPodmanAdapter::new(program.clone());

    let error = adapter
        .run_command_at(&request(), &policy(), 1_780_000_000)
        .unwrap_err();

    assert_eq!(
        error,
        CommandExecutionError::Backend(ApplicationServiceError::CleanupFailed)
    );
    let calls = fs::read_to_string(&call_log).expect("fake Podman calls should be recorded");
    assert!(calls.lines().any(|line| line.starts_with("start ")));
    assert!(calls.lines().any(|line| line.starts_with("rm --force ")));

    let _ = fs::remove_file(program);
    let _ = fs::remove_file(call_log);
}

#[test]
fn cleanup_failure_is_not_hidden_behind_container_logs_failure() {
    let call_log = temporary_path("logs-and-cleanup-fail-call-log");
    let script = format!(
        "#!/bin/sh\nset -eu\nprintf '%s\\n' \"$*\" >> '{}'\ncase \"${{1:-}}:${{2:-}}\" in\n  \
         info:--format) printf '%s\\n' '{}' ;;\n  \
         create:--name) printf 'fake-command-container-id\\n' ;;\n  \
         start:*) : ;;\n  \
         container:inspect) printf '%s\\n' '{}' ;;\n  \
         top:*) printf '%s' '{}' ;;\n  \
         wait:*) printf '0\\n' ;;\n  \
         logs:*) printf 'podman logs backend failure\\n' >&2; exit 42 ;;\n  \
         rm:--force) exit 88 ;;\n  \
         *) exit 91 ;;\nesac\n",
        call_log.display(),
        security_info_json(),
        container_inspect_json(),
        top_output(),
    );
    let program = write_executable("logs-and-cleanup-fail", &script);
    let adapter = RootlessPodmanAdapter::new(program.clone());

    let error = adapter
        .run_command_at(&request(), &policy(), 1_780_000_000)
        .unwrap_err();

    assert_eq!(
        error,
        CommandExecutionError::Backend(ApplicationServiceError::CleanupFailed)
    );
    let calls = fs::read_to_string(&call_log).expect("fake Podman calls should be recorded");
    assert!(calls.lines().any(|line| line.starts_with("logs ")));
    assert!(calls.lines().any(|line| line.starts_with("rm --force ")));

    let _ = fs::remove_file(program);
    let _ = fs::remove_file(call_log);
}

#[test]
fn cleanup_failure_is_not_hidden_behind_container_logs_timeout() {
    let call_log = temporary_path("logs-timeout-and-cleanup-fail-call-log");
    let script = format!(
        "#!/bin/sh\nset -eu\nprintf '%s\\n' \"$*\" >> '{}'\ncase \"${{1:-}}:${{2:-}}\" in\n  \
         info:--format) printf '%s\\n' '{}' ;;\n  \
         create:--name) printf 'fake-command-container-id\\n' ;;\n  \
         start:*) : ;;\n  \
         container:inspect) printf '%s\\n' '{}' ;;\n  \
         top:*) printf '%s' '{}' ;;\n  \
         wait:*) printf '0\\n' ;;\n  \
         logs:*) sleep 1 ;;\n  \
         rm:--force) exit 88 ;;\n  \
         *) exit 91 ;;\nesac\n",
        call_log.display(),
        security_info_json(),
        container_inspect_json(),
        top_output(),
    );
    let program = write_executable("logs-timeout-and-cleanup-fail", &script);
    let adapter = RootlessPodmanAdapter::new(program.clone())
        .with_command_timeout(Duration::from_millis(200));

    let error = adapter
        .run_command_at(&request(), &policy(), 1_780_000_000)
        .unwrap_err();

    assert_eq!(
        error,
        CommandExecutionError::Backend(ApplicationServiceError::CleanupFailed)
    );
    let calls = fs::read_to_string(&call_log).expect("fake Podman calls should be recorded");
    assert!(calls.lines().any(|line| line.starts_with("logs ")));
    assert!(calls.lines().any(|line| line.starts_with("rm --force ")));

    let _ = fs::remove_file(program);
    let _ = fs::remove_file(call_log);
}

#[test]
fn cleanup_failure_is_not_hidden_behind_effective_isolation_failure() {
    let call_log = temporary_path("isolation-and-cleanup-fail-call-log");
    let invalid_container = r#"[{"Id":"fake-command-container-id","AppArmorProfile":"containers-default","ProcessLabel":"","EffectiveCaps":null,"BoundingCaps":null,"Config":{"User":"65532:65532"},"HostConfig":{"ReadonlyRootfs":false,"Privileged":false,"SecurityOpt":["no-new-privileges"],"UsernsMode":"","Annotations":{"io.podman.annotations.userns":"auto"},"PidMode":"private","IpcMode":"none","Memory":268435456,"NanoCpus":1000000000,"PidsLimit":16}}]"#;
    let script = format!(
        "#!/bin/sh\nset -eu\nprintf '%s\\n' \"$*\" >> '{}'\ncase \"${{1:-}}:${{2:-}}\" in\n  \
         info:--format) printf '%s\\n' '{}' ;;\n  \
         create:--name) printf 'fake-command-container-id\\n' ;;\n  \
         start:*) : ;;\n  \
         container:inspect) printf '%s\\n' '{}' ;;\n  \
         rm:--force) exit 88 ;;\n  \
         *) exit 91 ;;\nesac\n",
        call_log.display(),
        security_info_json(),
        invalid_container,
    );
    let program = write_executable("isolation-and-cleanup-fail", &script);
    let adapter = RootlessPodmanAdapter::new(program.clone());

    let error = adapter
        .run_command_at(&request(), &policy(), 1_780_000_000)
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

    let _ = fs::remove_file(program);
    let _ = fs::remove_file(call_log);
}
