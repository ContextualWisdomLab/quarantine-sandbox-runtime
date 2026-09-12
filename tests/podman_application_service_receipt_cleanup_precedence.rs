//! Cleanup-precedence coverage for application-service create receipts.
//!
//! Once create or receipt reconciliation fails, cleanup failure must take
//! precedence over the original error without ever promoting `qsr-app-*`
//! correlation names to destructive authority.

#![cfg(target_os = "linux")]

use std::{
    fs,
    os::unix::fs::symlink,
    path::{Path, PathBuf},
    sync::atomic::{AtomicU64, Ordering},
    time::{SystemTime, UNIX_EPOCH},
};

use quarantine_sandbox_runtime::{
    ApplicationServiceError, ApplicationServiceRequest, IsolationPolicy, ResourceRequest,
    RootlessPodmanAdapter, ServiceProtocol,
};

static NEXT_TEMP_PATH_ID: AtomicU64 = AtomicU64::new(0);
const STDOUT_ID: &str = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";
const RECEIPT_ID: &str = "fedcba9876543210fedcba9876543210fedcba9876543210fedcba9876543210";

fn temporary_path(name: &str) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("test clock must be after the Unix epoch")
        .as_nanos();
    let unique_id = NEXT_TEMP_PATH_ID.fetch_add(1, Ordering::Relaxed);
    std::env::temp_dir().join(format!(
        "qsr-application-service-receipt-cleanup-{name}-{}-{nanos}-{unique_id}",
        std::process::id()
    ))
}

fn policy() -> IsolationPolicy {
    IsolationPolicy {
        policy_id: "application_service_receipt_cleanup_precedence_v1".to_owned(),
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
        request_id: format!("application_service_receipt_cleanup_{case_name}"),
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
        "#!/bin/sh\nset -eu\nSCENARIO='{scenario}'\nLOG='{}'\nUNEXPECTED='{}'\nSTDOUT_ID='{STDOUT_ID}'\nRECEIPT_ID='{RECEIPT_ID}'\nINFO='{}'\nprintf '%s\\n' \"$*\" >> \"$LOG\"\nif [ \"${{1:-}}\" = info ]; then\n  if [ \"${{3:-}}\" = json ]; then printf '%s\\n' \"$INFO\"; else printf 'true\\n'; fi\n  exit 0\nfi\ncase \"${{1:-}}:${{2:-}}\" in\n  network:create) : ;;\n  network:rm)\n    case \"$SCENARIO\" in\n      create_error_read_error_network_cleanup_fail|malformed_read_error_network_cleanup_fail|malformed_no_receipt_network_cleanup_fail) exit 71 ;;\n      *) : ;;\n    esac\n    ;;\n  create:--name)\n    cidfile=''\n    for argument in \"$@\"; do\n      case \"$argument\" in --cidfile=*) cidfile=${{argument#--cidfile=}} ;; esac\n    done\n    [ -n \"$cidfile\" ] || exit 95\n    case \"$SCENARIO\" in\n      create_error_read_error_network_cleanup_fail) mkdir \"$cidfile\"; exit 22 ;;\n      create_error_receipt_container_cleanup_fail) printf '%s\\n' \"$RECEIPT_ID\" > \"$cidfile\"; exit 22 ;;\n      malformed_read_error_network_cleanup_fail) mkdir \"$cidfile\"; printf 'bad identifier with spaces\\n' ;;\n      malformed_receipt_container_cleanup_fail) printf '%s\\n' \"$RECEIPT_ID\" > \"$cidfile\"; printf 'bad identifier with spaces\\n' ;;\n      malformed_no_receipt_network_cleanup_fail) printf 'bad identifier with spaces\\n' ;;\n      mismatched_receipt_container_cleanup_fail) printf '%s\\n' \"$RECEIPT_ID\" > \"$cidfile\"; printf '%s\\n' \"$STDOUT_ID\" ;;\n      valid_stdout_read_error_container_cleanup_fail) mkdir \"$cidfile\"; printf '%s\\n' \"$STDOUT_ID\" ;;\n      *) exit 96 ;;\n    esac\n    ;;\n  rm:--force)\n    case \"$SCENARIO:${{3:-}}\" in\n      create_error_receipt_container_cleanup_fail:$RECEIPT_ID|malformed_receipt_container_cleanup_fail:$RECEIPT_ID|mismatched_receipt_container_cleanup_fail:$RECEIPT_ID|valid_stdout_read_error_container_cleanup_fail:$STDOUT_ID) exit 72 ;;\n      *) printf 'unexpected rm target: %s\\n' \"${{3:-}}\" > \"$UNEXPECTED\"; exit 98 ;;\n    esac\n    ;;\n  start:*|container:inspect|top:*|port:*) printf 'unexpected post-create lifecycle: %s\\n' \"$*\" > \"$UNEXPECTED\"; exit 99 ;;\n  *) printf 'unexpected backend call: %s\\n' \"$*\" > \"$UNEXPECTED\"; exit 91 ;;\nesac\n",
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
fn receipt_failure_paths_surface_cleanup_failure_without_name_fallback() {
    let scenarios = [
        "create_error_read_error_network_cleanup_fail",
        "create_error_receipt_container_cleanup_fail",
        "malformed_read_error_network_cleanup_fail",
        "malformed_receipt_container_cleanup_fail",
        "malformed_no_receipt_network_cleanup_fail",
        "mismatched_receipt_container_cleanup_fail",
        "valid_stdout_read_error_container_cleanup_fail",
    ];

    for (index, scenario) in scenarios.into_iter().enumerate() {
        let (program, log, unexpected) = write_fake_podman(scenario);
        let adapter = RootlessPodmanAdapter::new(program.clone());

        assert_eq!(
            adapter.launch_at(&request(scenario), &policy(), 1_780_001_500 + index as u64,),
            Err(ApplicationServiceError::CleanupFailed),
            "cleanup must take precedence for scenario {scenario}"
        );

        let calls = fs::read_to_string(&log).expect("fake Podman calls must be recorded");
        assert!(
            !calls
                .lines()
                .any(|line| line.starts_with("rm --force qsr-app-")),
            "generated correlation names must never become destructive authority: {calls}"
        );
        assert!(
            !unexpected.exists(),
            "unexpected backend call in {scenario}"
        );
        cleanup_fixture(&program, &log, &unexpected);
    }
}
