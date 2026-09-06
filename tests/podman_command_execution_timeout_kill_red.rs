//! RED: a command lease timeout is not enforced unless runtime termination succeeds.
//!
//! `wait_for_command` currently ignores the result of `podman kill` after the
//! request lease expires. A later natural exit can therefore be reported as a
//! successful timed-out command even though runtime-owned termination was never
//! proven. This fixture keeps every currently-attested isolation control positive,
//! makes the first wait exceed the lease, makes `kill` fail, and makes the second
//! wait succeed. The runtime must fail closed before reading workload logs.

#![cfg(target_os = "linux")]

use std::{fs, os::unix::fs::PermissionsExt, path::PathBuf, time::Duration};

use quarantine_sandbox_runtime::{
    ApplicationServiceError, CommandExecutionError, CommandExecutionRequest, IsolationPolicy,
    ResourceRequest, RootlessPodmanAdapter,
};
use tempfile::TempDir;

fn policy() -> IsolationPolicy {
    IsolationPolicy {
        policy_id: "command_timeout_kill_red_v1".to_owned(),
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
        request_id: "command-timeout-kill-red".to_owned(),
        image_reference: format!("localhost/cwl/tool@sha256:{}", "e".repeat(64)),
        command: vec!["sleep".to_owned(), "60".to_owned()],
        source_artifact: None,
        resources: ResourceRequest {
            memory_bytes: 256 * 1024 * 1024,
            cpu_millicores: 1_000,
            maximum_processes: 16,
            lease_seconds: 1,
            tmpfs_bytes: 16 * 1024 * 1024,
        },
    }
}

fn fake_podman() -> (TempDir, PathBuf, PathBuf) {
    let directory = tempfile::Builder::new()
        .prefix("qsr-command-timeout-kill-red-")
        .tempdir()
        .expect("isolated fake-Podman directory");
    let program = directory.path().join("podman");
    let calls = directory.path().join("calls");
    let wait_marker = directory.path().join("first-wait-seen");
    let info = r#"{"host":{"security":{"rootless":true,"seccompEnabled":true,"seccompProfilePath":"/usr/share/containers/seccomp.json","apparmorEnabled":true,"selinuxEnabled":false}},"version":{"Version":"6.1.0"}}"#;
    let inspect = r#"[{"Id":"fake-command-container-id","AppArmorProfile":"containers-default","ProcessLabel":"","EffectiveCaps":[],"BoundingCaps":[],"Config":{"User":"65532:65532"},"HostConfig":{"ReadonlyRootfs":true,"Privileged":false,"SecurityOpt":["no-new-privileges"],"UsernsMode":"auto","PidMode":"private","IpcMode":"none","NetworkMode":"none","Memory":268435456,"NanoCpus":1000000000,"PidsLimit":16}}]"#;
    let top = "PID SECCOMP CAPEFF CAPBND CAPINH CAPPRM CAPAMB LABEL\n1 filter - - - - - containers-default (enforce)\n";
    let script = format!(
        "#!/bin/sh\nset -eu\nprintf '%s\\n' \"$*\" >> '{}'\ncase \"${{1:-}}:${{2:-}}\" in\n  info:--format) printf '%s\\n' '{}' ;;\n  create:--name) printf 'fake-command-container-id\\n' ;;\n  start:*) : ;;\n  container:inspect) printf '%s\\n' '{}' ;;\n  top:*) printf '%s' '{}' ;;\n  wait:*) if [ -f '{}' ]; then printf '0\\n'; else touch '{}'; sleep 5; fi ;;\n  kill:*) exit 42 ;;\n  logs:*) printf 'must-not-be-trusted\\n' ;;\n  rm:--force) : ;;\n  *) exit 91 ;;\nesac\n",
        calls.display(),
        info,
        inspect,
        top,
        wait_marker.display(),
        wait_marker.display()
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
fn failed_timeout_kill_fails_closed_before_logs_and_still_cleans_up() {
    let (_directory, program, calls_path) = fake_podman();
    let adapter = RootlessPodmanAdapter::new(program).with_command_timeout(Duration::from_secs(2));

    let result = adapter.run_command_at(&request(), &policy(), 1_780_000_000);
    let calls = fs::read_to_string(calls_path).expect("fake Podman calls must be recorded");

    assert_eq!(
        result,
        Err(CommandExecutionError::Backend(
            ApplicationServiceError::BackendCommandFailed {
                operation: "command_kill",
            },
        )),
        "a lease timeout must not become successful evidence when runtime termination fails"
    );
    assert!(
        calls.lines().any(|line| line.starts_with("kill ")),
        "the runtime must attempt termination when the request lease expires: {calls}"
    );
    assert!(
        !calls.lines().any(|line| line.starts_with("logs ")),
        "workload logs must not be trusted after unproven timeout termination: {calls}"
    );
    assert!(
        calls
            .lines()
            .any(|line| line.starts_with("rm --force --ignore ")),
        "failed termination must still attempt invocation cleanup: {calls}"
    );
}
