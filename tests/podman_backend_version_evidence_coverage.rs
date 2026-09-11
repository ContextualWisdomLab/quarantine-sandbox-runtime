//! Exercise hostile Podman version evidence through the public command adapter boundary.

#![cfg(target_os = "linux")]

use std::{
    fs,
    os::unix::fs::symlink,
    path::{Path, PathBuf},
};

use quarantine_sandbox_runtime::{
    ApplicationServiceError, CommandExecutionError, CommandExecutionRequest, IsolationPolicy,
    ResourceRequest, RootlessPodmanAdapter,
};

fn policy() -> IsolationPolicy {
    IsolationPolicy {
        policy_id: "backend_version_evidence_v1".to_owned(),
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
        request_id: "backend-version-evidence".to_owned(),
        image_reference: format!("localhost/cwl/tool@sha256:{}", "d".repeat(64)),
        command: vec!["true".to_owned()],
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

fn fixture_executable() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/fake_podman.sh")
}

fn config_path(program: &Path) -> PathBuf {
    PathBuf::from(format!("{}.config", program.display()))
}

fn execute_with_backend_version(
    name: &str,
    version: &str,
) -> Result<quarantine_sandbox_runtime::CommandExecutionResult, CommandExecutionError> {
    let directory = tempfile::Builder::new()
        .prefix("qsr-backend-version-evidence-")
        .tempdir()
        .expect("isolated fake-Podman directory");
    let program = directory.path().join(name);
    let scenario = directory.path().join("scenario.sh");
    symlink(fixture_executable(), &program).expect("fake Podman symlink must be creatable");

    let backend_info = serde_json::json!({
        "host": {
            "security": {
                "rootless": true,
                "seccompEnabled": true,
                "seccompProfilePath": "/usr/share/containers/seccomp.json",
                "apparmorEnabled": true,
                "selinuxEnabled": false
            }
        },
        "version": { "Version": version }
    })
    .to_string();

    fs::write(
        &scenario,
        format!(
            r#"#!/bin/sh
set -eu
case "${{1:-}}:${{2:-}}" in
  info:--format) printf '%s\n' '{backend_info}' ;;
  create:*) exit 91 ;;
  *) exit 92 ;;
esac
"#,
        ),
    )
    .expect("fake Podman scenario must be writable");
    fs::write(
        config_path(&program),
        format!(
            "MODE='source_script'\nLOG='/dev/null'\nSCRIPT='{}'\n",
            scenario.display()
        ),
    )
    .expect("fake Podman config must be writable");

    RootlessPodmanAdapter::new(program).run_legacy_command_at_for_test(
        &request(),
        &policy(),
        1_780_001_000,
    )
}

fn malformed_backend_info() -> CommandExecutionError {
    CommandExecutionError::Backend(ApplicationServiceError::MalformedIsolationInspection {
        operation: "backend_security_info",
    })
}

#[test]
fn backend_version_evidence_rejects_empty_oversized_and_control_text() {
    for (name, version) in [
        ("empty-version", String::new()),
        ("oversized-version", "v".repeat(129)),
        ("control-version", "6.1.0\nforged".to_owned()),
    ] {
        assert_eq!(
            execute_with_backend_version(name, &version),
            Err(malformed_backend_info()),
            "untrusted backend version evidence must remain bounded printable text"
        );
    }
}

#[test]
fn backend_version_evidence_accepts_the_exact_128_byte_boundary() {
    assert_eq!(
        execute_with_backend_version("maximum-version", &"v".repeat(128)),
        Err(CommandExecutionError::Backend(
            ApplicationServiceError::BackendCommandFailed {
                operation: "container_create",
            },
        )),
        "an exact 128-byte printable backend version must pass evidence admission and reach create"
    );
}
