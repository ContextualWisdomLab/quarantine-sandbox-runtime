//! Exact create-receipt authority edge cases for the application-service Podman adapter.
//!
//! A backend-produced container identifier may authorize destructive cleanup only after exact
//! admission. These cases exercise reconciliation, cleanup precedence, and receipt I/O failures
//! without ever falling back to the generated `qsr-app-*` correlation name.

#![cfg(target_os = "linux")]

use std::{
    fs,
    os::unix::fs::PermissionsExt,
    path::{Path, PathBuf},
    sync::atomic::{AtomicU64, Ordering},
    time::{SystemTime, UNIX_EPOCH},
};

use quarantine_sandbox_runtime::{
    ApplicationServiceError, ApplicationServiceRequest, IsolationPolicy, ResourceRequest,
    RootlessPodmanAdapter, ServiceProtocol,
};

static NEXT_TEMP_PATH_ID: AtomicU64 = AtomicU64::new(0);

const STDOUT_ID: &str =
    "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";
const RECEIPT_ID: &str =
    "fedcba9876543210fedcba9876543210fedcba9876543210fedcba9876543210";

fn temporary_path(name: &str) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("test clock must be after the Unix epoch")
        .as_nanos();
    let unique_id = NEXT_TEMP_PATH_ID.fetch_add(1, Ordering::Relaxed);
    std::env::temp_dir().join(format!(
        "qsr-application-service-create-receipt-edges-{name}-{}-{nanos}-{unique_id}",
        std::process::id()
    ))
}

fn policy() -> IsolationPolicy {
    IsolationPolicy {
        policy_id: "application_service_create_receipt_edges_v1".to_owned(),
        maximum_memory_bytes: 256 * 1024 * 1024,
        maximum_cpu_millicores: 500,
        maximum_processes: 32,
        maximum_lease_seconds: 60,
        maximum_tmpfs_bytes: 32 * 1024 * 1024,
        readiness_timeout_millis: 250,
        readiness_poll_interval_millis: 10,
        shutdown_grace_seconds: 1,
        run_as_user_id: 65_532,
        run_as_group_id: 65_532,
    }
}

fn request(case_name: &str) -> ApplicationServiceRequest {
    ApplicationServiceRequest {
        schema_version: "1.0.0".to_owned(),
        request_id: format!("application_service_create_receipt_edges_{case_name}"),
        image_reference: format!("localhost/cwl/tool@sha256:{}", "a".repeat(64)),
        container_port: 8_080,
        protocol: ServiceProtocol::Http,
        command: vec!["serve".to_owned()],
        resources: ResourceRequest {
            memory_bytes: 128 * 1024 * 1024,
            cpu_millicores: 250,
            maximum_processes: 16,
            lease_seconds: 30,
            tmpfs_bytes: 16 * 1024 * 1024,
        },
    }
}

fn write_fake_podman(
    scenario: &str,
    expected_remove_id: Option<&str>,
) -> (PathBuf, PathBuf, PathBuf) {
    let program = temporary_path("fake-podman");
    let log = temporary_path("calls");
    let unexpected_destructive = temporary_path("unexpected-destructive");
    let info = r#"{"host":{"security":{"rootless":true,"seccompEnabled":true,"seccompProfilePath":"/usr/share/containers/seccomp.json","apparmorEnabled":true,"selinuxEnabled":false}}}"#;
    let allowed_remove_id = expected_remove_id.unwrap_or("");
    let script = format!(
        "#!/bin/sh\nset -eu\nSCENARIO='{scenario}'\nLOG='{}'\nUNEXPECTED='{}'\nSTDOUT_ID='{STDOUT_ID}'\nRECEIPT_ID='{RECEIPT_ID}'\nALLOWED_REMOVE_ID='{allowed_remove_id}'\nINFO='{}'\nprintf '%s\\n' \"$*\" >> \"$LOG\"\nif [ \"${{1:-}}\" = info ]; then\n  if [ \"${{3:-}}\" = json ]; then printf '%s\\n' \"$INFO\"; else printf 'true\\n'; fi\n  exit 0\nfi\ncase \"${{1:-}}:${{2:-}}\" in\n  network:create) : ;;\n  network:rm) : ;;\n  create:--name)\n    cidfile=''\n    for argument in \"$@\"; do\n      case \"$argument\" in --cidfile=*) cidfile=${{argument#--cidfile=}} ;; esac\n    done\n    [ -n \"$cidfile\" ] || exit 95\n    case \"$SCENARIO\" in\n      create_fail_with_receipt) printf '%s\\n' \"$RECEIPT_ID\" > \"$cidfile\"; exit 22 ;;\n      matching_receipt_then_start_fail) printf '%s\\n' \"$STDOUT_ID\" > \"$cidfile\"; printf '%s\\n' \"$STDOUT_ID\" ;;\n      mismatched_receipt) printf '%s\\n' \"$RECEIPT_ID\" > \"$cidfile\"; printf '%s\\n' \"$STDOUT_ID\" ;;\n      malformed_stdout_receipt_read_error) mkdir \"$cidfile\"; printf 'bad identifier with spaces\\n' ;;\n      valid_stdout_receipt_read_error) mkdir \"$cidfile\"; printf '%s\\n' \"$STDOUT_ID\" ;;\n      invalid_receipt_length) printf 'abc\\n' > \"$cidfile\"; printf 'bad identifier with spaces\\n' ;;\n      invalid_receipt_character) printf '%064d\\n' 0 | tr '0' 'G' > \"$cidfile\"; printf 'bad identifier with spaces\\n' ;;\n      invalid_receipt_utf8) printf '\\377' > \"$cidfile\"; printf 'bad identifier with spaces\\n' ;;\n      *) exit 96 ;;\n    esac\n    ;;\n  start:*)\n    if [ \"$SCENARIO\" = matching_receipt_then_start_fail ] && [ \"${{2:-}}\" = \"$STDOUT_ID\" ]; then exit 24; fi\n    printf 'unexpected start target: %s\\n' \"${{2:-}}\" > \"$UNEXPECTED\"\n    exit 97\n    ;;\n  rm:--force)\n    if [ -n \"$ALLOWED_REMOVE_ID\" ] && [ \"${{3:-}}\" = \"$ALLOWED_REMOVE_ID\" ]; then exit 0; fi\n    printf 'unexpected rm target: %s\\n' \"${{3:-}}\" > \"$UNEXPECTED\"\n    exit 98\n    ;;\n  container:inspect|top:*|port:*) printf 'unexpected post-create lifecycle\\n' > \"$UNEXPECTED\"; exit 99 ;;\n  *) exit 91 ;;\nesac\n",
        log.display(),
        unexpected_destructive.display(),
        info,
    );
    fs::write(&program, script).expect("fake Podman must be writable");
    let mut permissions = fs::metadata(&program)
        .expect("fake Podman metadata must exist")
        .permissions();
    permissions.set_mode(0o700);
    fs::set_permissions(&program, permissions).expect("fake Podman must be executable");
    (program, log, unexpected_destructive)
}

fn calls(log: &Path) -> String {
    fs::read_to_string(log).expect("fake Podman calls must be recorded")
}

fn assert_no_generated_name_cleanup(calls: &str) {
    assert!(
        !calls
            .lines()
            .any(|line| line.starts_with("rm --force qsr-app-")),
        "generated correlation names must never become destructive authority; calls were:\n{calls}"
    );
}

fn cleanup_fixture(program: &Path, log: &Path, unexpected: &Path) {
    let _ = fs::remove_file(program);
    let _ = fs::remove_file(log);
    let _ = fs::remove_file(unexpected);
}

#[test]
fn failed_create_with_valid_receipt_cleans_only_the_receipt_id() {
    let (program, log, unexpected) = write_fake_podman("create_fail_with_receipt", Some(RECEIPT_ID));
    let adapter = RootlessPodmanAdapter::new(program.clone());

    let result = adapter.launch_at(&request("create_fail_with_receipt"), &policy(), 1_780_001_300);
    assert_eq!(
        result,
        Err(ApplicationServiceError::BackendCommandFailed {
            operation: "container_create",
        })
    );
    let calls = calls(&log);
    assert!(calls.lines().any(|line| line == format!("rm --force {RECEIPT_ID}")));
    assert_no_generated_name_cleanup(&calls);
    assert!(!unexpected.exists());
    cleanup_fixture(&program, &log, &unexpected);
}

#[test]
fn matching_stdout_and_receipt_select_the_exact_id_before_start_cleanup() {
    let (program, log, unexpected) =
        write_fake_podman("matching_receipt_then_start_fail", Some(STDOUT_ID));
    let adapter = RootlessPodmanAdapter::new(program.clone());

    let result = adapter.launch_at(
        &request("matching_receipt_then_start_fail"),
        &policy(),
        1_780_001_301,
    );
    assert_eq!(
        result,
        Err(ApplicationServiceError::BackendCommandFailed {
            operation: "container_start",
        })
    );
    let calls = calls(&log);
    assert!(calls.lines().any(|line| line == format!("start {STDOUT_ID}")));
    assert!(calls.lines().any(|line| line == format!("rm --force {STDOUT_ID}")));
    assert_no_generated_name_cleanup(&calls);
    assert!(!unexpected.exists());
    cleanup_fixture(&program, &log, &unexpected);
}

#[test]
fn mismatched_stdout_and_receipt_fail_closed_and_cleanup_the_receipt_id() {
    let (program, log, unexpected) = write_fake_podman("mismatched_receipt", Some(RECEIPT_ID));
    let adapter = RootlessPodmanAdapter::new(program.clone());

    let result = adapter.launch_at(&request("mismatched_receipt"), &policy(), 1_780_001_302);
    assert_eq!(
        result,
        Err(ApplicationServiceError::MalformedIsolationInspection {
            operation: "container_create_receipt",
        })
    );
    let calls = calls(&log);
    assert!(calls.lines().any(|line| line == format!("rm --force {RECEIPT_ID}")));
    assert!(!calls.lines().any(|line| line.starts_with("start ")));
    assert_no_generated_name_cleanup(&calls);
    assert!(!unexpected.exists());
    cleanup_fixture(&program, &log, &unexpected);
}

#[test]
fn malformed_stdout_with_unreadable_receipt_reports_receipt_io_failure_without_container_guessing() {
    let (program, log, unexpected) =
        write_fake_podman("malformed_stdout_receipt_read_error", None);
    let adapter = RootlessPodmanAdapter::new(program.clone());

    let result = adapter.launch_at(
        &request("malformed_stdout_receipt_read_error"),
        &policy(),
        1_780_001_303,
    );
    assert_eq!(
        result,
        Err(ApplicationServiceError::BackendInvocationFailed {
            operation: "container_create_receipt",
        })
    );
    let calls = calls(&log);
    assert!(!calls.lines().any(|line| line.starts_with("rm --force ")));
    assert_no_generated_name_cleanup(&calls);
    assert!(!unexpected.exists());
    cleanup_fixture(&program, &log, &unexpected);
}

#[test]
fn valid_stdout_with_unreadable_receipt_cleans_the_admitted_stdout_id() {
    let (program, log, unexpected) =
        write_fake_podman("valid_stdout_receipt_read_error", Some(STDOUT_ID));
    let adapter = RootlessPodmanAdapter::new(program.clone());

    let result = adapter.launch_at(
        &request("valid_stdout_receipt_read_error"),
        &policy(),
        1_780_001_304,
    );
    assert_eq!(
        result,
        Err(ApplicationServiceError::BackendInvocationFailed {
            operation: "container_create_receipt",
        })
    );
    let calls = calls(&log);
    assert!(calls.lines().any(|line| line == format!("rm --force {STDOUT_ID}")));
    assert_no_generated_name_cleanup(&calls);
    assert!(!unexpected.exists());
    cleanup_fixture(&program, &log, &unexpected);
}

fn assert_invalid_receipt_fails_without_container_cleanup(scenario: &str, started_at: u64) {
    let (program, log, unexpected) = write_fake_podman(scenario, None);
    let adapter = RootlessPodmanAdapter::new(program.clone());

    let result = adapter.launch_at(&request(scenario), &policy(), started_at);
    assert_eq!(
        result,
        Err(ApplicationServiceError::MalformedIsolationInspection {
            operation: "container_create_receipt",
        })
    );
    let calls = calls(&log);
    assert!(!calls.lines().any(|line| line.starts_with("rm --force ")));
    assert_no_generated_name_cleanup(&calls);
    assert!(!unexpected.exists());
    cleanup_fixture(&program, &log, &unexpected);
}

#[test]
fn invalid_receipt_length_is_not_admitted() {
    assert_invalid_receipt_fails_without_container_cleanup("invalid_receipt_length", 1_780_001_305);
}

#[test]
fn invalid_receipt_character_is_not_admitted() {
    assert_invalid_receipt_fails_without_container_cleanup(
        "invalid_receipt_character",
        1_780_001_306,
    );
}

#[test]
fn invalid_receipt_utf8_is_not_admitted() {
    assert_invalid_receipt_fails_without_container_cleanup("invalid_receipt_utf8", 1_780_001_307);
}
