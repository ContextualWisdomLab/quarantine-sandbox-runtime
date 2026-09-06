//! RED: command evidence must be bound to the requested immutable image digest.
//!
//! `--pull=never` and a digest-pinned request are necessary but not sufficient
//! attestation. The created container's applied `ImageDigest` is authoritative
//! runtime evidence and must match the request before command output is trusted.

#![cfg(target_os = "linux")]

use std::{fs, os::unix::fs::PermissionsExt, path::PathBuf};

use quarantine_sandbox_runtime::{
    ApplicationServiceError, CommandExecutionError, CommandExecutionRequest, IsolationPolicy,
    ResourceRequest, RootlessPodmanAdapter,
};
use tempfile::TempDir;

const OWNED_CONTAINER_ID: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
const EXPECTED_DIGEST_HEX: &str =
    "eeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeee";
const WRONG_DIGEST_HEX: &str = "ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff";

fn policy() -> IsolationPolicy {
    IsolationPolicy {
        policy_id: "command_image_digest_red_v1".to_owned(),
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
        request_id: "command-image-digest-red".to_owned(),
        image_reference: format!("localhost/cwl/tool@sha256:{EXPECTED_DIGEST_HEX}"),
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
        .prefix("qsr-command-image-digest-red-")
        .tempdir()
        .expect("isolated fake-Podman directory");
    let program = directory.path().join("podman");
    let calls = directory.path().join("calls");
    let info = r#"{"host":{"security":{"rootless":true,"seccompEnabled":true,"seccompProfilePath":"/usr/share/containers/seccomp.json","apparmorEnabled":true,"selinuxEnabled":false}},"version":{"Version":"6.1.0"}}"#;
    let inspect = format!(
        "[{{\"Id\":\"{OWNED_CONTAINER_ID}\",\"Image\":\"{}\",\"ImageDigest\":\"sha256:{WRONG_DIGEST_HEX}\",\"ImageName\":\"localhost/cwl/tool@sha256:{WRONG_DIGEST_HEX}\",\"AppArmorProfile\":\"containers-default\",\"ProcessLabel\":\"\",\"EffectiveCaps\":[],\"BoundingCaps\":[],\"Config\":{{\"User\":\"65532:65532\",\"Timeout\":20}},\"HostConfig\":{{\"ReadonlyRootfs\":true,\"Privileged\":false,\"SecurityOpt\":[\"no-new-privileges\"],\"UsernsMode\":\"auto\",\"PidMode\":\"private\",\"IpcMode\":\"none\",\"NetworkMode\":\"none\",\"Memory\":268435456,\"NanoCpus\":1000000000,\"PidsLimit\":16,\"Tmpfs\":{{\"/tmp\":\"rw,noexec,nosuid,nodev,size=16777216\"}}}},\"Mounts\":[]}}]",
        "b".repeat(64),
    );
    let top = "PID SECCOMP CAPEFF CAPBND CAPINH CAPPRM CAPAMB LABEL\n1 filter - - - - - containers-default (enforce)\n";
    let script = format!(
        "#!/bin/sh\nset -eu\nprintf '%s\\n' \"$*\" >> '{}'\ncase \"${{1:-}}:${{2:-}}\" in\n  info:--format) printf '%s\\n' '{}' ;;\n  create:--name) printf '%s\\n' '{}' ;;\n  start:*) : ;;\n  container:inspect) printf '%s\\n' '{}' ;;\n  top:*) printf '%s' '{}' ;;\n  wait:*) printf '0\\n' ;;\n  logs:*) printf 'must-not-be-trusted\\n' ;;\n  kill:*) : ;;\n  rm:--force) : ;;\n  *) exit 91 ;;\nesac\n",
        calls.display(),
        info,
        OWNED_CONTAINER_ID,
        inspect,
        top
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
fn mismatched_applied_image_digest_fails_closed_before_command_evidence_is_trusted() {
    let (_directory, program, calls_path) = fake_podman();
    let adapter = RootlessPodmanAdapter::new(program);

    let result = adapter.run_command_at(&request(), &policy(), 1_780_000_037);
    let calls = fs::read_to_string(calls_path).expect("fake Podman calls must be recorded");

    assert_eq!(
        result,
        Err(CommandExecutionError::Backend(
            ApplicationServiceError::IsolationVerificationFailed {
                control_name: "immutable_image_identity",
            },
        )),
        "a different applied image digest must not become trusted command evidence"
    );
    assert!(
        calls.lines().any(|line| line.starts_with("rm --force ")),
        "image-identity rejection must still clean up the invocation-owned container: {calls}"
    );
    assert!(
        !calls.lines().any(|line| line.starts_with("logs ")),
        "output from the wrong image must never be collected as trusted workload evidence: {calls}"
    );
}
