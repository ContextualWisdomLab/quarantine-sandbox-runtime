//! Backend-version evidence validation coverage for the rootless Podman adapter.

#![cfg(target_os = "linux")]

use std::{
    fs,
    path::{Path, PathBuf},
    sync::Arc,
};

use quarantine_sandbox_runtime::{
    ApplicationServiceError, CommandExecutionError, CommandExecutionRequest, IsolationPolicy,
    ResourceRequest, RootlessPodmanAdapter,
};
use tempfile::TempDir;

#[derive(Clone)]
struct FixturePath {
    _directory: Arc<TempDir>,
    path: PathBuf,
}

impl AsRef<Path> for FixturePath {
    fn as_ref(&self) -> &Path {
        &self.path
    }
}

impl From<FixturePath> for PathBuf {
    fn from(value: FixturePath) -> Self {
        value.path.clone()
    }
}

fn temporary_path(name: &str) -> FixturePath {
    let directory = Arc::new(
        tempfile::Builder::new()
            .prefix("qsr-backend-version-coverage-")
            .tempdir()
            .expect("isolated fixture directory"),
    );
    FixturePath {
        path: directory.path().join(name),
        _directory: directory,
    }
}

fn immutable_fixture_executable() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/fake_podman.sh")
}

fn fixture_sidecar(program: &Path, suffix: &str) -> PathBuf {
    PathBuf::from(format!("{}.{suffix}", program.display()))
}

fn write_executable(name: &str, script: &str) -> FixturePath {
    let program = temporary_path(name);
    std::os::unix::fs::symlink(immutable_fixture_executable(), &program)
        .expect("fake Podman immutable symlink should be creatable");
    let script_path = fixture_sidecar(&program, "script");
    let init_capable_script = format!("if [ \"${{1:-}}\" = init ]; then exit 0; fi\n{script}");
    fs::write(&script_path, init_capable_script)
        .expect("fake Podman scenario data should be writable");
    fs::write(
        fixture_sidecar(&program, "config"),
        format!(
            "MODE='source_script'\nLOG='/dev/null'\nSCRIPT='{}'\n",
            script_path.display()
        ),
    )
    .expect("fake Podman dispatcher config should be writable");
    program
}

fn policy() -> IsolationPolicy {
    IsolationPolicy {
        policy_id: "backend_version_coverage".to_owned(),
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
        request_id: "backend-version-coverage".to_owned(),
        image_reference: format!("localhost/cwl/tool@sha256:{}", "e".repeat(64)),
        command: vec!["payload-sentinel".to_owned()],
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

fn security_info_json(version_json: &str) -> String {
    format!(
        "{{\"host\":{{\"security\":{{\"rootless\":true,\"seccompEnabled\":true,\"seccompProfilePath\":\"/usr/share/containers/seccomp.json\",\"apparmorEnabled\":true,\"selinuxEnabled\":false}}}},\"version\":{{\"Version\":{version_json}}}}}"
    )
}

fn assert_version_is_rejected(name: &str, version_json: &str) {
    let backend_info = security_info_json(version_json);
    let script = format!(
        "#!/bin/sh\nset -eu\ncase \"${{1:-}}:${{2:-}}\" in\n  info:--format) printf '%s\\n' '{backend_info}' ;;\n  *) exit 91 ;;\nesac\n"
    );
    let program = write_executable(name, &script);
    let adapter = RootlessPodmanAdapter::new(program.clone());

    assert_eq!(
        adapter.run_legacy_command_at_for_test(&request(), &policy(), 1_780_000_000),
        Err(CommandExecutionError::Backend(
            ApplicationServiceError::MalformedIsolationInspection {
                operation: "backend_security_info",
            },
        ))
    );

    let _ = fs::remove_file(program);
}

#[test]
fn backend_security_version_rejects_each_malformed_text_class() {
    assert_version_is_rejected("empty", "\"\"");
    assert_version_is_rejected("oversized", &format!("\"{}\"", "x".repeat(129)));
    assert_version_is_rejected("control", "\"6.1.0\\n\"");
}
