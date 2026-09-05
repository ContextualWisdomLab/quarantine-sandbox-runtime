//! RED: caller-supplied wall-clock input must not become authoritative execution evidence.
//!
//! `run_command_at` currently copies `started_at_epoch_seconds` into the public
//! result while recording completion from the runtime host clock. An otherwise
//! successful invocation can therefore emit an impossible chronology when the
//! supplied start lies in the future. Runtime evidence must either fail closed
//! or replace the supplied value with a runtime-observed start time; it must not
//! publish the caller's contradictory timestamp as observed execution evidence.

#![cfg(target_os = "linux")]

use std::{fs, os::unix::fs::PermissionsExt, path::PathBuf, time::Duration};

use quarantine_sandbox_runtime::{
    CommandExecutionRequest, IsolationPolicy, ResourceRequest, RootlessPodmanAdapter,
};
use tempfile::TempDir;

const CALLER_SUPPLIED_FUTURE_START: u64 = 1_000_000_000_000;

fn policy() -> IsolationPolicy {
    IsolationPolicy {
        policy_id: "command_timestamp_authority_red_v1".to_owned(),
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
        request_id: "command-timestamp-authority-red".to_owned(),
        image_reference: format!("localhost/cwl/tool@sha256:{}", "f".repeat(64)),
        command: vec!["true".to_owned()],
        source_artifact: None,
        resources: ResourceRequest {
            memory_bytes: 256 * 1024 * 1024,
            cpu_millicores: 1_000,
            maximum_processes: 16,
            lease_seconds: 30,
            tmpfs_bytes: 16 * 1024 * 1024,
        },
    }
}

fn fake_podman() -> (TempDir, PathBuf, PathBuf) {
    let directory = tempfile::Builder::new()
        .prefix("qsr-command-timestamp-authority-red-")
        .tempdir()
        .expect("isolated fake-Podman directory");
    let program = directory.path().join("podman");
    let calls = directory.path().join("calls");
    let info = r#"{"host":{"security":{"rootless":true,"seccompEnabled":true,"seccompProfilePath":"/usr/share/containers/seccomp.json","apparmorEnabled":true,"selinuxEnabled":false}},"version":{"Version":"6.1.0"}}"#;
    let inspect = r#"[{"Id":"fake-command-container-id","AppArmorProfile":"containers-default","ProcessLabel":"","EffectiveCaps":[],"BoundingCaps":[],"Config":{"User":"65532:65532"},"HostConfig":{"ReadonlyRootfs":true,"Privileged":false,"SecurityOpt":["no-new-privileges"],"UsernsMode":"auto","PidMode":"private","IpcMode":"none","NetworkMode":"none","Memory":268435456,"NanoCpus":1000000000,"PidsLimit":16}}]"#;
    let top = "PID SECCOMP CAPEFF CAPBND CAPINH CAPPRM CAPAMB LABEL\n1 filter - - - - - containers-default (enforce)\n";
    let script = format!(
        "#!/bin/sh\nset -eu\nprintf '%s\\n' \"$*\" >> '{}'\ncase \"${{1:-}}:${{2:-}}\" in\n  info:--format) printf '%s\\n' '{}' ;;\n  create:--name) printf 'fake-command-container-id\\n' ;;\n  start:*) : ;;\n  container:inspect) printf '%s\\n' '{}' ;;\n  top:*) printf '%s' '{}' ;;\n  wait:*) printf '0\\n' ;;\n  logs:*) printf 'ok\\n' ;;\n  rm:--force) : ;;\n  *) exit 91 ;;\nesac\n",
        calls.display(), info, inspect, top
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
fn future_caller_timestamp_is_not_published_as_observed_runtime_chronology() {
    let (_directory, program, calls_path) = fake_podman();
    let adapter = RootlessPodmanAdapter::new(program).with_command_timeout(Duration::from_secs(2));

    let result = adapter.run_command_at(
        &request(),
        &policy(),
        CALLER_SUPPLIED_FUTURE_START,
    );
    let calls = fs::read_to_string(calls_path).expect("fake Podman calls must be recorded");

    if let Ok(result) = result {
        assert_ne!(
            result.started_at_epoch_seconds(),
            CALLER_SUPPLIED_FUTURE_START,
            "consumer/test-seam wall-clock input must not be emitted unchanged as runtime-observed start evidence"
        );
        assert!(
            result.finished_at_epoch_seconds() >= result.started_at_epoch_seconds(),
            "successful execution evidence must have a nondecreasing chronology: started={}, finished={}",
            result.started_at_epoch_seconds(),
            result.finished_at_epoch_seconds()
        );
    }

    assert!(
        calls
            .lines()
            .any(|line| line.starts_with("rm --force --ignore ")),
        "the success or fail-closed path must still clean the invocation container: {calls}"
    );
}
