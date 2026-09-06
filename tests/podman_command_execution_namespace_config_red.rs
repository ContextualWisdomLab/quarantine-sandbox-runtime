//! RED: command execution must bind applied UTS and cgroup namespace modes.
//!
//! PR #14 requests private UTS and cgroup namespaces, and AGENTS.md treats
//! isolated namespaces / no host namespaces as fail-closed P0 invariants.
//! Current command attestation does not deserialize Podman `HostConfig.UTSMode`
//! or `HostConfig.CgroupMode`, so these otherwise-positive hostile fixtures can
//! report host namespace sharing without being rejected. Production behavior is
//! intentionally unchanged until this RED executes for that exact cause.

#![cfg(target_os = "linux")]

use std::{fs, os::unix::fs::PermissionsExt, path::PathBuf};

use quarantine_sandbox_runtime::{
    ApplicationServiceError, CommandExecutionError, CommandExecutionRequest, IsolationPolicy,
    ResourceRequest, RootlessPodmanAdapter,
};
use tempfile::TempDir;

fn policy() -> IsolationPolicy {
    IsolationPolicy {
        policy_id: "command_namespace_red_v1".to_owned(),
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
        request_id: "command-namespace-red".to_owned(),
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

fn fake_podman(container_inspect: &str) -> (TempDir, PathBuf, PathBuf) {
    let directory = tempfile::Builder::new()
        .prefix("qsr-command-namespace-red-")
        .tempdir()
        .expect("isolated fake-Podman directory");
    let program = directory.path().join("podman");
    let calls = directory.path().join("calls");
    let info = r#"{"host":{"security":{"rootless":true,"seccompEnabled":true,"seccompProfilePath":"/usr/share/containers/seccomp.json","apparmorEnabled":true,"selinuxEnabled":false}},"version":{"Version":"6.1.0"}}"#;
    let top = "PID SECCOMP CAPEFF CAPBND CAPINH CAPPRM CAPAMB LABEL\n1 filter - - - - - containers-default (enforce)\n";
    let script = format!(
        "#!/bin/sh\nset -eu\nprintf '%s\\n' \"$*\" >> '{}'\ncase \"${{1:-}}:${{2:-}}\" in\n  info:--format) printf '%s\\n' '{}' ;;\n  create:--name) printf 'fake-command-container-id\\n' ;;\n  start:*) : ;;\n  container:inspect) printf '%s\\n' '{}' ;;\n  top:*) printf '%s' '{}' ;;\n  wait:*) printf '0\\n' ;;\n  logs:*) : ;;\n  rm:--force) : ;;\n  *) exit 91 ;;\nesac\n",
        calls.display(),
        info,
        container_inspect,
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

fn assert_namespace_config_rejected(
    container_inspect: &str,
    control_name: &'static str,
    evidence_name: &str,
) {
    let (_directory, program, calls_path) = fake_podman(container_inspect);
    let adapter = RootlessPodmanAdapter::new(program);

    let result = adapter.run_command_at(&request(), &policy(), 1_780_000_100);
    let calls = fs::read_to_string(calls_path).expect("fake Podman calls must be recorded");
    let sandbox_name = calls
        .lines()
        .find_map(|line| {
            let mut parts = line.split_whitespace();
            (parts.next() == Some("create") && parts.next() == Some("--name"))
                .then(|| parts.next().map(str::to_owned))
                .flatten()
        })
        .expect("create must record the invocation-owned sandbox name");

    assert_eq!(
        result,
        Err(CommandExecutionError::Backend(
            ApplicationServiceError::IsolationVerificationFailed { control_name },
        )),
        "{evidence_name} must fail closed instead of becoming command evidence"
    );
    assert!(
        calls
            .lines()
            .any(|line| line == format!("rm --force --ignore {sandbox_name}")),
        "{evidence_name} must clean up the exact created sandbox: {calls}"
    );
    assert!(
        !calls.lines().any(|line| line.starts_with("logs ")),
        "{evidence_name} must be rejected before command output is trusted: {calls}"
    );
}

#[test]
fn host_uts_namespace_configuration_fails_closed_and_cleans_up() {
    let host_uts = r#"[{"Id":"fake-command-container-id","AppArmorProfile":"containers-default","ProcessLabel":"","EffectiveCaps":[],"BoundingCaps":[],"Config":{"User":"65532:65532","Timeout":20},"HostConfig":{"ReadonlyRootfs":true,"Privileged":false,"SecurityOpt":["no-new-privileges"],"UsernsMode":"auto","PidMode":"private","IpcMode":"none","NetworkMode":"none","UTSMode":"host","CgroupMode":"private","Memory":268435456,"NanoCpus":1000000000,"PidsLimit":16,"Tmpfs":{"/tmp":"rw,noexec,nosuid,nodev,size=16777216"}},"Mounts":[]}]"#;

    assert_namespace_config_rejected(
        host_uts,
        "isolated_uts_namespace",
        "host UTS namespace configuration",
    );
}

#[test]
fn host_cgroup_namespace_configuration_fails_closed_and_cleans_up() {
    let host_cgroup = r#"[{"Id":"fake-command-container-id","AppArmorProfile":"containers-default","ProcessLabel":"","EffectiveCaps":[],"BoundingCaps":[],"Config":{"User":"65532:65532","Timeout":20},"HostConfig":{"ReadonlyRootfs":true,"Privileged":false,"SecurityOpt":["no-new-privileges"],"UsernsMode":"auto","PidMode":"private","IpcMode":"none","NetworkMode":"none","UTSMode":"private","CgroupMode":"host","Memory":268435456,"NanoCpus":1000000000,"PidsLimit":16,"Tmpfs":{"/tmp":"rw,noexec,nosuid,nodev,size=16777216"}},"Mounts":[]}]"#;

    assert_namespace_config_rejected(
        host_cgroup,
        "isolated_cgroup_namespace",
        "host cgroup namespace configuration",
    );
}
