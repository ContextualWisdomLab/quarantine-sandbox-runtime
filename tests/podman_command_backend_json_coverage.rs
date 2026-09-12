//! Command-runtime coverage for malformed backend security JSON.
//!
//! Podman `info --format json` is external evidence. Invalid JSON must fail
//! closed before container creation rather than being interpreted as partial
//! isolation evidence.

#![cfg(target_os = "linux")]

use std::{fs, os::unix::fs::symlink, path::PathBuf};

use quarantine_sandbox_runtime::{
    ApplicationServiceError, CommandExecutionError, CommandExecutionRequest, IsolationPolicy,
    ResourceRequest, RootlessPodmanAdapter,
};

fn policy() -> IsolationPolicy {
    IsolationPolicy {
        policy_id: "command_backend_json_coverage".to_owned(),
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
        request_id: "command-backend-json-coverage".to_owned(),
        image_reference: format!("localhost/cwl/tool@sha256:{}", "b".repeat(64)),
        command: vec!["payload-must-not-start".to_owned()],
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

#[test]
fn malformed_backend_security_json_fails_closed_before_container_create() {
    let fixture = tempfile::tempdir().expect("isolated fake Podman directory");
    let program = fixture.path().join("podman");
    let scenario = fixture.path().join("scenario.sh");
    let invocation_log = fixture.path().join("invocations.log");
    let immutable_fixture = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/fake_podman.sh");

    symlink(&immutable_fixture, &program).expect("immutable fake Podman symlink should be created");
    fs::write(
        &scenario,
        r#"case "${1:-}:${2:-}" in
  info:--format) printf '%s\n' '{"host":' ;;
  *) exit 91 ;;
esac
"#,
    )
    .expect("scenario data should be writable");
    fs::write(
        PathBuf::from(format!("{}.config", program.display())),
        format!(
            "MODE='source_script'\nLOG='{}'\nSCRIPT='{}'\n",
            invocation_log.display(),
            scenario.display()
        ),
    )
    .expect("fake Podman configuration should be writable");

    let adapter = RootlessPodmanAdapter::new(&program);
    assert_eq!(
        adapter.run_legacy_command_at_for_test(&request(), &policy(), 1_780_000_000),
        Err(CommandExecutionError::Backend(
            ApplicationServiceError::MalformedIsolationInspection {
                operation: "backend_security_info",
            },
        )),
        "malformed backend JSON must be rejected as typed isolation evidence",
    );

    let invocations = fs::read_to_string(&invocation_log).expect("invocation log should exist");
    assert_eq!(
        invocations.lines().collect::<Vec<_>>(),
        vec!["info --format json"],
        "container creation must not begin after malformed backend security evidence",
    );
}
