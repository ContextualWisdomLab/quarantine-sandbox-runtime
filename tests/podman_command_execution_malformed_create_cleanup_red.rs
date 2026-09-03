//! RED regression for cleanup precedence after malformed container creation evidence.
//!
//! A successful `podman create` process can still return malformed/empty stdout.
//! Once that command has succeeded, the runtime owns a possible container resource
//! and must surface `CleanupFailed` when removing it cannot be proven. Returning
//! only the earlier malformed-inspection error would hide the stronger leak risk.

#![cfg(target_os = "linux")]

use std::{
    fs,
    os::unix::fs::PermissionsExt,
    path::PathBuf,
    sync::atomic::{AtomicU64, Ordering},
    time::{SystemTime, UNIX_EPOCH},
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
        "quarantine-sandbox-runtime-malformed-create-cleanup-red-{name}-{}-{nanos}-{unique_id}",
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
        policy_id: "malformed_create_cleanup_red_policy_v1".to_owned(),
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
        request_id: "malformed-create-cleanup-red-request".to_owned(),
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

#[test]
fn cleanup_failure_is_not_hidden_behind_malformed_container_identifier() {
    let call_log = temporary_path("call-log");
    let script = format!(
        "#!/bin/sh\nset -eu\nprintf '%s\\n' \"$*\" >> '{}'\ncase \"${{1:-}}:${{2:-}}\" in\n  info:--format) printf '%s\\n' '{{\"host\":{{\"security\":{{\"rootless\":true,\"seccompEnabled\":true,\"seccompProfilePath\":\"/usr/share/containers/seccomp.json\",\"apparmorEnabled\":true,\"selinuxEnabled\":false}}}},\"version\":{{\"Version\":\"6.1.0\"}}}}' ;;\n  create:--name) printf '\\n' ;;\n  rm:--force) exit 88 ;;\n  *) exit 91 ;;\nesac\n",
        call_log.display(),
    );
    let program = write_executable("fake-podman", &script);
    let adapter = RootlessPodmanAdapter::new(program.clone());

    let error = adapter
        .run_command_at(&request(), &policy(), 1_780_000_000)
        .unwrap_err();

    assert_eq!(
        error,
        CommandExecutionError::Backend(ApplicationServiceError::CleanupFailed)
    );
    let calls = fs::read_to_string(&call_log).expect("fake Podman calls should be recorded");
    assert!(calls.lines().any(|line| line.starts_with("create --name ")));
    assert!(calls.lines().any(|line| line.starts_with("rm --force ")));

    let _ = fs::remove_file(program);
    let _ = fs::remove_file(call_log);
}
