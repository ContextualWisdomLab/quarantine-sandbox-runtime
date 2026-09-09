//! Regressions for partially successful `podman create` invocations.
//!
//! A container runtime can persist a container and still return a nonzero CLI
//! status. Cleanup after that failure is safe only when this invocation has an
//! exact container identity. The command profile therefore requests a private
//! `--cidfile` receipt and removes that acquired ID, never the requested name.

#![cfg(target_os = "linux")]

use std::{
    fs,
    os::unix::fs::symlink,
    path::{Path, PathBuf},
    sync::atomic::{AtomicU64, Ordering},
    time::{SystemTime, UNIX_EPOCH},
};

use quarantine_sandbox_runtime::{
    ApplicationServiceError, CommandExecutionError, CommandExecutionRequest, IsolationPolicy,
    ResourceRequest, RootlessPodmanAdapter,
};

static NEXT_PATH_ID: AtomicU64 = AtomicU64::new(0);
const OWNED_CONTAINER_ID: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";

fn temporary_path(name: &str) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock should be after the Unix epoch")
        .as_nanos();
    let unique_id = NEXT_PATH_ID.fetch_add(1, Ordering::Relaxed);
    std::env::temp_dir().join(format!(
        "quarantine-sandbox-runtime-create-failure-cleanup-red-{name}-{}-{nanos}-{unique_id}",
        std::process::id()
    ))
}

fn fixture_executable() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/fake_podman.sh")
}

fn config_path(program: &Path) -> PathBuf {
    PathBuf::from(format!("{}.config", program.display()))
}

fn source_script_program(name: &str, call_log: &Path, script: &str) -> (PathBuf, PathBuf, PathBuf) {
    let program = temporary_path(name);
    symlink(fixture_executable(), &program).expect("fake Podman symlink should be creatable");
    let script_path = temporary_path(&format!("{name}-script"));
    fs::write(&script_path, script).expect("fake Podman data script should be writable");
    let config = config_path(&program);
    fs::write(
        &config,
        format!(
            "MODE='source_script'\nLOG='{}'\nSCRIPT='{}'\n",
            call_log.display(),
            script_path.display(),
        ),
    )
    .expect("fake Podman data config should be writable");
    (program, config, script_path)
}

fn cleanup_fixture(program: PathBuf, config: PathBuf, script: PathBuf, call_log: PathBuf) {
    let _ = fs::remove_file(program);
    let _ = fs::remove_file(config);
    let _ = fs::remove_file(script);
    let _ = fs::remove_file(call_log);
}

fn policy() -> IsolationPolicy {
    IsolationPolicy {
        policy_id: "command_create_failure_cleanup_red_policy_v1".to_owned(),
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
        request_id: "command-create-failure-cleanup-red-request".to_owned(),
        image_reference: format!("localhost/cwl/tool@sha256:{}", "a".repeat(64)),
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

fn security_info_json() -> &'static str {
    r#"{"host":{"security":{"rootless":true,"seccompEnabled":true,"seccompProfilePath":"/usr/share/containers/seccomp.json","apparmorEnabled":true,"selinuxEnabled":false}},"version":{"Version":"6.1.0"}}"#
}

fn cidfile_parser_prefix() -> String {
    format!(
        "cidfile=''\nfor arg in \"$@\"; do\n  case \"$arg\" in\n    --cidfile=*) cidfile=${{arg#--cidfile=}} ;;\n  esac\ndone\ncase \"${{1:-}}:${{2:-}}\" in\n  info:--format) printf '%s\\n' '{}' ;;\n",
        security_info_json(),
    )
}

#[test]
fn failed_create_with_owned_cidfile_receipt_is_cleaned_by_exact_id() {
    let call_log = temporary_path("call-log");
    let script = format!(
        "{}  create:--name) test -n \"$cidfile\"; printf '%s\\n' '{}' > \"$cidfile\"; exit 42 ;;\n  rm:--force) : ;;\n  *) exit 91 ;;\nesac\n",
        cidfile_parser_prefix(),
        OWNED_CONTAINER_ID,
    );
    let (program, config, script_path) =
        source_script_program("fake-podman", &call_log, &script);
    let adapter = RootlessPodmanAdapter::new(program.clone());

    let error = adapter
        .run_command_at(&request(), &policy(), 1_780_000_002)
        .unwrap_err();

    assert_eq!(
        error,
        CommandExecutionError::Backend(ApplicationServiceError::BackendCommandFailed {
            operation: "container_create",
        })
    );
    let calls = fs::read_to_string(&call_log).expect("fake Podman calls should be recorded");
    assert!(calls.lines().any(|line| {
        line.starts_with("create --name ")
            && line
                .split_whitespace()
                .any(|arg| arg.starts_with("--cidfile="))
    }));
    assert!(
        calls
            .lines()
            .any(|line| line == format!("rm --force --ignore {OWNED_CONTAINER_ID}"))
    );
    assert!(!calls.lines().any(|line| line.starts_with("start ")));

    cleanup_fixture(program, config, script_path, call_log);
}

#[test]
fn cleanup_failure_for_owned_failed_create_surfaces_leak_risk() {
    let call_log = temporary_path("cleanup-fail-call-log");
    let script = format!(
        "{}  create:--name) test -n \"$cidfile\"; printf '%s\\n' '{}' > \"$cidfile\"; exit 42 ;;\n  rm:--force) exit 88 ;;\n  *) exit 91 ;;\nesac\n",
        cidfile_parser_prefix(),
        OWNED_CONTAINER_ID,
    );
    let (program, config, script_path) =
        source_script_program("fake-podman-cleanup-fail", &call_log, &script);
    let adapter = RootlessPodmanAdapter::new(program.clone());

    let error = adapter
        .run_command_at(&request(), &policy(), 1_780_000_003)
        .unwrap_err();

    assert_eq!(
        error,
        CommandExecutionError::Backend(ApplicationServiceError::CleanupFailed)
    );
    let calls = fs::read_to_string(&call_log).expect("fake Podman calls should be recorded");
    assert!(calls.lines().any(|line| {
        line.starts_with("create --name ")
            && line
                .split_whitespace()
                .any(|arg| arg.starts_with("--cidfile="))
    }));
    assert!(
        calls
            .lines()
            .any(|line| line == format!("rm --force --ignore {OWNED_CONTAINER_ID}"))
    );
    assert!(!calls.lines().any(|line| line.starts_with("start ")));

    cleanup_fixture(program, config, script_path, call_log);
}

#[test]
fn malformed_failed_create_receipt_fails_closed_without_name_cleanup() {
    let call_log = temporary_path("malformed-receipt-call-log");
    let script = format!(
        "{}  create:--name) test -n \"$cidfile\"; printf '%s\\n' 'not-an-owned-container-id' > \"$cidfile\"; exit 42 ;;\n  rm:--force) exit 99 ;;\n  *) exit 91 ;;\nesac\n",
        cidfile_parser_prefix(),
    );
    let (program, config, script_path) =
        source_script_program("fake-podman-malformed-receipt", &call_log, &script);
    let adapter = RootlessPodmanAdapter::new(program.clone());

    let error = adapter
        .run_command_at(&request(), &policy(), 1_780_000_004)
        .unwrap_err();

    assert_eq!(
        error,
        CommandExecutionError::Backend(ApplicationServiceError::MalformedIsolationInspection {
            operation: "container_create_receipt",
        })
    );
    let calls = fs::read_to_string(&call_log).expect("fake Podman calls should be recorded");
    assert!(calls.lines().any(|line| line.starts_with("create --name ")));
    assert!(!calls.lines().any(|line| line.starts_with("rm --force ")));
    assert!(!calls.lines().any(|line| line.starts_with("start ")));

    cleanup_fixture(program, config, script_path, call_log);
}

#[test]
fn unreadable_failed_create_receipt_fails_closed_without_name_cleanup() {
    let call_log = temporary_path("unreadable-receipt-call-log");
    let script = format!(
        "{}  create:--name) test -n \"$cidfile\"; mkdir \"$cidfile\"; exit 42 ;;\n  rm:--force) exit 99 ;;\n  *) exit 91 ;;\nesac\n",
        cidfile_parser_prefix(),
    );
    let (program, config, script_path) =
        source_script_program("fake-podman-unreadable-receipt", &call_log, &script);
    let adapter = RootlessPodmanAdapter::new(program.clone());

    let error = adapter
        .run_command_at(&request(), &policy(), 1_780_000_005)
        .unwrap_err();

    assert_eq!(
        error,
        CommandExecutionError::Backend(ApplicationServiceError::BackendInvocationFailed {
            operation: "container_create_receipt",
        })
    );
    let calls = fs::read_to_string(&call_log).expect("fake Podman calls should be recorded");
    assert!(calls.lines().any(|line| line.starts_with("create --name ")));
    assert!(!calls.lines().any(|line| line.starts_with("rm --force ")));
    assert!(!calls.lines().any(|line| line.starts_with("start ")));

    cleanup_fixture(program, config, script_path, call_log);
}
