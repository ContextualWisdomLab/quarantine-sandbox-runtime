//! OS-boundary coverage for application-service create receipts.
//!
//! Receipt setup and read failures must stay typed and fail closed before any
//! generated correlation name can become destructive backend authority.

#![cfg(target_os = "linux")]

use std::{
    env,
    ffi::OsString,
    fs,
    os::unix::{
        ffi::{OsStrExt, OsStringExt},
        fs::symlink,
    },
    path::{Path, PathBuf},
    process::Command,
    sync::atomic::{AtomicU64, Ordering},
    time::{SystemTime, UNIX_EPOCH},
};

use quarantine_sandbox_runtime::{
    ApplicationServiceError, ApplicationServiceRequest, IsolationPolicy, ResourceRequest,
    RootlessPodmanAdapter, ServiceProtocol,
};

static NEXT_TEMP_PATH_ID: AtomicU64 = AtomicU64::new(0);
const CHILD_MODE_ENV: &str = "QSR_RECEIPT_OS_EDGE_CHILD";
const CHILD_PODMAN_ENV: &str = "QSR_RECEIPT_OS_EDGE_PODMAN";
const CHILD_LOG_ENV: &str = "QSR_RECEIPT_OS_EDGE_LOG";

fn temporary_path(name: &str) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("test clock must be after the Unix epoch")
        .as_nanos();
    let unique_id = NEXT_TEMP_PATH_ID.fetch_add(1, Ordering::Relaxed);
    env::temp_dir().join(format!(
        "qsr-application-service-receipt-os-{name}-{}-{nanos}-{unique_id}",
        std::process::id()
    ))
}

fn policy() -> IsolationPolicy {
    IsolationPolicy {
        policy_id: "application_service_receipt_os_edges_v1".to_owned(),
        maximum_memory_bytes: 256 * 1024 * 1024,
        maximum_cpu_millicores: 500,
        maximum_processes: 32,
        maximum_lease_seconds: 60,
        maximum_tmpfs_bytes: 32 * 1024 * 1024,
        readiness_timeout_millis: 100,
        readiness_poll_interval_millis: 10,
        shutdown_grace_seconds: 1,
        run_as_user_id: 65_532,
        run_as_group_id: 65_532,
    }
}

fn request(case_name: &str) -> ApplicationServiceRequest {
    ApplicationServiceRequest {
        schema_version: "1.0.0".to_owned(),
        request_id: format!("application_service_receipt_os_{case_name}"),
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

fn immutable_fixture_executable() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/fake_podman.sh")
}

fn fixture_sidecar(program: &Path, suffix: &str) -> PathBuf {
    PathBuf::from(format!("{}.{suffix}", program.display()))
}

fn write_fake_podman(scenario: &str) -> (PathBuf, PathBuf, PathBuf) {
    let program = temporary_path("fake-podman");
    let log = temporary_path("calls");
    let unexpected = temporary_path("unexpected");
    let info = r#"{"host":{"security":{"rootless":true,"seccompEnabled":true,"seccompProfilePath":"/usr/share/containers/seccomp.json","apparmorEnabled":true,"selinuxEnabled":false}}}"#;
    let script = format!(
        "#!/bin/sh\nset -eu\nSCENARIO='{scenario}'\nLOG='{}'\nUNEXPECTED='{}'\nINFO='{}'\nprintf '%s\\n' \"$*\" >> \"$LOG\"\nif [ \"${{1:-}}\" = info ]; then\n  if [ \"${{3:-}}\" = json ]; then printf '%s\\n' \"$INFO\"; else printf 'true\\n'; fi\n  exit 0\nfi\ncase \"${{1:-}}:${{2:-}}\" in\n  network:create) : ;;\n  network:rm) : ;;\n  create:--name)\n    if [ \"$SCENARIO\" != create_fail_unreadable_receipt ]; then\n      printf 'unexpected create\\n' > \"$UNEXPECTED\"\n      exit 96\n    fi\n    cidfile=''\n    for argument in \"$@\"; do\n      case \"$argument\" in --cidfile=*) cidfile=${{argument#--cidfile=}} ;; esac\n    done\n    [ -n \"$cidfile\" ] || exit 95\n    mkdir \"$cidfile\"\n    exit 22\n    ;;\n  *) printf 'unexpected backend call: %s\\n' \"$*\" > \"$UNEXPECTED\"; exit 91 ;;\nesac\n",
        log.display(),
        unexpected.display(),
        info,
    );
    symlink(immutable_fixture_executable(), &program)
        .expect("fake Podman immutable symlink should be creatable");
    let script_path = fixture_sidecar(&program, "script");
    fs::write(&script_path, script).expect("fake Podman scenario data should be writable");
    fs::write(
        fixture_sidecar(&program, "config"),
        format!(
            "MODE='source_script'\nLOG='{}'\nSCRIPT='{}'\n",
            log.display(),
            script_path.display()
        ),
    )
    .expect("fake Podman dispatcher config should be writable");
    (program, log, unexpected)
}

fn cleanup_fixture(program: &Path, log: &Path, unexpected: &Path) {
    let _ = fs::remove_file(fixture_sidecar(program, "config"));
    let _ = fs::remove_file(fixture_sidecar(program, "script"));
    let _ = fs::remove_file(program);
    let _ = fs::remove_file(log);
    let _ = fs::remove_file(unexpected);
}

#[test]
fn failed_create_with_unreadable_receipt_preserves_receipt_io_failure() {
    let (program, log, unexpected) = write_fake_podman("create_fail_unreadable_receipt");
    let adapter = RootlessPodmanAdapter::new(program.clone());

    let result = adapter.launch_at(
        &request("create_fail_unreadable_receipt"),
        &policy(),
        1_780_001_400,
    );
    assert_eq!(
        result,
        Err(ApplicationServiceError::BackendInvocationFailed {
            operation: "container_create_receipt",
        })
    );
    let calls = fs::read_to_string(&log).expect("fake Podman calls must be recorded");
    assert!(calls.lines().any(|line| line.starts_with("network rm ")));
    assert!(!calls.lines().any(|line| line.starts_with("rm --force ")));
    assert!(!unexpected.exists());

    cleanup_fixture(&program, &log, &unexpected);
}

#[test]
fn receipt_path_failure_child() {
    let Some(mode) = env::var_os(CHILD_MODE_ENV) else {
        return;
    };
    let program = PathBuf::from(
        env::var_os(CHILD_PODMAN_ENV).expect("child Podman fixture path must be provided"),
    );
    let log = PathBuf::from(env::var_os(CHILD_LOG_ENV).expect("child log path must be provided"));
    let adapter = RootlessPodmanAdapter::new(program);

    let result = adapter.launch_at(&request("receipt_path_failure"), &policy(), 1_780_001_401);
    assert_eq!(
        result,
        Err(ApplicationServiceError::BackendInvocationFailed {
            operation: "container_create_receipt",
        }),
        "receipt OS edge {mode:?} must remain a typed pre-resource failure"
    );
    let calls = fs::read_to_string(log).expect("rootless/security probes must be recorded");
    assert!(calls.lines().any(|line| line.starts_with("info --format ")));
    assert!(!calls.lines().any(|line| line.starts_with("network create")));
}

fn run_receipt_path_child(mode: &str, temp_directory: &Path) {
    let (program, log, unexpected) = write_fake_podman("pre_create_only");
    let status = Command::new(env::current_exe().expect("current test executable must resolve"))
        .arg("--exact")
        .arg("receipt_path_failure_child")
        .arg("--nocapture")
        .arg("--test-threads=1")
        .env(CHILD_MODE_ENV, mode)
        .env(CHILD_PODMAN_ENV, &program)
        .env(CHILD_LOG_ENV, &log)
        .env("TMPDIR", temp_directory)
        .status()
        .expect("receipt OS edge child must spawn");
    assert!(status.success(), "receipt OS edge child {mode} failed");
    assert!(!unexpected.exists());
    cleanup_fixture(&program, &log, &unexpected);
}

#[test]
fn receipt_temp_directory_creation_failure_is_typed_before_network_creation() {
    let missing_temp_directory = temporary_path("missing-temp-directory");
    let _ = fs::remove_dir_all(&missing_temp_directory);
    run_receipt_path_child("tempdir_creation_failure", &missing_temp_directory);
}

#[test]
fn non_utf8_receipt_path_is_rejected_before_network_creation() {
    let mut bytes = env::temp_dir().as_os_str().as_bytes().to_vec();
    bytes.extend_from_slice(b"/qsr-receipt-non-utf8-");
    bytes.push(0xff);
    bytes.extend_from_slice(
        format!(
            "-{}-{}",
            std::process::id(),
            NEXT_TEMP_PATH_ID.fetch_add(1, Ordering::Relaxed)
        )
        .as_bytes(),
    );
    let non_utf8_directory = PathBuf::from(OsString::from_vec(bytes));
    fs::create_dir(&non_utf8_directory).expect("non-UTF8 temporary directory must be creatable");
    run_receipt_path_child("non_utf8_receipt_path", &non_utf8_directory);
    fs::remove_dir(&non_utf8_directory).expect("non-UTF8 temporary directory must be removable");
}
