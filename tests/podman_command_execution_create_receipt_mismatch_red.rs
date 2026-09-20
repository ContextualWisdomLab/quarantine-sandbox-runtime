//! RED regression for contradictory successful-create identity evidence.
//!
//! A successful Podman create writes the acquired 64-hex container ID to the
//! runtime-owned `--cidfile`. Stdout is independent backend output and must not
//! be allowed to redirect lifecycle or destructive authority when it names a
//! different syntactically valid container ID.

#![cfg(target_os = "linux")]

use std::{fs, os::unix::fs::PermissionsExt, path::PathBuf};

use quarantine_sandbox_runtime::{
    ApplicationServiceError, CommandExecutionError, CommandExecutionRequest, IsolationPolicy,
    ResourceRequest, RootlessPodmanAdapter,
};
use tempfile::TempDir;

const RECEIPT_CONTAINER_ID: &str =
    "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
const STDOUT_CONTAINER_ID: &str =
    "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";

fn policy() -> IsolationPolicy {
    IsolationPolicy {
        policy_id: "create_receipt_mismatch_red_v1".to_owned(),
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
        request_id: "create-receipt-mismatch-red".to_owned(),
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

fn fake_podman() -> (TempDir, PathBuf, PathBuf) {
    let directory = tempfile::Builder::new()
        .prefix("qsr-create-receipt-mismatch-red-")
        .tempdir()
        .expect("isolated fake-Podman directory");
    let program = directory.path().join("podman");
    let calls = directory.path().join("calls");
    let info = r#"{"host":{"security":{"rootless":true,"seccompEnabled":true,"seccompProfilePath":"/usr/share/containers/seccomp.json","apparmorEnabled":true,"selinuxEnabled":false}},"version":{"Version":"6.1.0"}}"#;
    let script = format!(
        "#!/bin/sh\nset -eu\nprintf '%s\\n' \"$*\" >> '{}'\ncase \"${{1:-}}:${{2:-}}\" in\n  info:--format) printf '%s\\n' '{}' ;;\n  create:--name)\n    cidfile=''\n    for arg in \"$@\"; do\n      case \"$arg\" in\n        --cidfile=*) cidfile=${{arg#--cidfile=}} ;;\n      esac\n    done\n    test -n \"$cidfile\"\n    printf '%s\\n' '{}' > \"$cidfile\"\n    printf '%s\\n' '{}'\n    ;;\n  init:*) exit 42 ;;\n  rm:--force) : ;;\n  *) exit 91 ;;\nesac\n",
        calls.display(),
        info,
        RECEIPT_CONTAINER_ID,
        STDOUT_CONTAINER_ID,
    );
    fs::write(&program, script).expect("fake Podman must be writable");
    let mut permissions = fs::metadata(&program)
        .expect("fake Podman metadata must exist")
        .permissions();
    permissions.set_mode(0o700);
    fs::set_permissions(&program, permissions).expect("fake Podman must be executable");
    (directory, program, calls)
}

#[test]
fn contradictory_create_stdout_cannot_override_runtime_owned_cidfile_identity() {
    let (_directory, program, calls_path) = fake_podman();
    let adapter = RootlessPodmanAdapter::new(program);

    let result = adapter.run_legacy_command_at_for_test(&request(), &policy(), 1_780_000_100);
    let calls = fs::read_to_string(calls_path).expect("fake Podman calls must be recorded");

    assert_eq!(
        result,
        Err(CommandExecutionError::Backend(
            ApplicationServiceError::MalformedIsolationInspection {
                operation: "container_create_receipt",
            },
        )),
        "contradictory successful-create identity evidence must fail closed before lifecycle authority is used"
    );
    assert!(
        !calls.lines().any(|line| line.starts_with("init ")),
        "contradictory create identity must be rejected before container init: {calls}"
    );
    assert!(
        calls
            .lines()
            .any(|line| line == format!("rm --force --ignore {RECEIPT_CONTAINER_ID}")),
        "cleanup must target the runtime-owned cidfile identity: {calls}"
    );
    assert!(
        !calls
            .lines()
            .any(|line| line == format!("rm --force --ignore {STDOUT_CONTAINER_ID}")),
        "stdout contradiction must never become destructive authority: {calls}"
    );
}
