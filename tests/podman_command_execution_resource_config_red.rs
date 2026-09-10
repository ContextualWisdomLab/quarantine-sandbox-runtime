//! RED/GREEN contract: command execution binds applied tmpfs and wall-time configuration.
//!
//! PR #14 requests a bounded hardened `/tmp` and a finite Podman container
//! timeout. These hostile fixtures deliberately report otherwise-positive
//! isolation evidence with widened, missing, contradictory, or unbounded
//! resource configuration. They must fail closed before command evidence is
//! trusted and clean up the exact invocation-owned container.

#![cfg(target_os = "linux")]

use std::{fs, os::unix::fs::symlink, path::PathBuf};

use quarantine_sandbox_runtime::{
    ApplicationServiceError, CommandExecutionError, CommandExecutionRequest, IsolationPolicy,
    ResourceRequest, RootlessPodmanAdapter,
};
use tempfile::TempDir;

fn policy() -> IsolationPolicy {
    IsolationPolicy {
        policy_id: "command_resource_red_v1".to_owned(),
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
        request_id: "command-resource-red".to_owned(),
        image_reference: format!("localhost/cwl/tool@sha256:{}", "e".repeat(64)),
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
        .prefix("qsr-command-resource-red-")
        .tempdir()
        .expect("isolated fake-Podman directory");
    let program = directory.path().join("podman");
    let calls = directory.path().join("calls");
    let scenario = directory.path().join("scenario.sh");
    let config = PathBuf::from(format!("{}.config", program.display()));
    let dispatcher = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/fake_podman.sh");
    fs::write(&calls, "").expect("fake Podman call log should be initialized");
    let info = r#"{"host":{"security":{"rootless":true,"seccompEnabled":true,"seccompProfilePath":"/usr/share/containers/seccomp.json","apparmorEnabled":true,"selinuxEnabled":false}},"version":{"Version":"6.1.0"}}"#;
    let top = "PID SECCOMP CAPEFF CAPBND CAPINH CAPPRM CAPAMB LABEL\n1 filter - - - - - containers-default (enforce)\n";
    let script = format!(
        "set -eu\ncase \"${{1:-}}:${{2:-}}\" in\n  info:--format) printf '%s\\n' '{}' ;;\n  create:--name) printf 'fake-command-container-id\\n' ;;\n  init:*) : ;;\n  start:*) : ;;\n  container:inspect) printf '%s\\n' '{}' ;;\n  top:*) printf '%s' '{}' ;;\n  wait:*) printf '0\\n' ;;\n  logs:*) : ;;\n  rm:--force) : ;;\n  *) exit 91 ;;\nesac\n",
        info,
        container_inspect,
        top
    );
    fs::write(&scenario, script).expect("fake Podman scenario should be writable");
    fs::write(
        &config,
        format!(
            "MODE=source_script\nLOG='{}'\nSCRIPT='{}'\n",
            calls.display(),
            scenario.display()
        ),
    )
    .expect("fake Podman dispatcher config should be writable");
    symlink(&dispatcher, &program).expect("immutable fake Podman dispatcher should be linkable");
    (directory, program, calls)
}

fn assert_resource_config_rejected(container_inspect: &str, evidence_name: &str) {
    let (_directory, program, calls_path) = fake_podman(container_inspect);
    let adapter = RootlessPodmanAdapter::new(program);

    let result = adapter.run_legacy_command_at_for_test(&request(), &policy(), 1_780_000_000);
    let calls = fs::read_to_string(calls_path).expect("fake Podman calls must be recorded");
    let sandbox_name = calls
        .lines()
        .find_map(|line| {
            let mut parts = line.split_whitespace();
            (parts.next() == Some("create") && parts.next() == Some("--name"))
                .then(|| parts.next().map(str::to_owned))
                .flatten()
        })
        .expect("create must record the invocation correlation name");

    assert_eq!(
        result,
        Err(CommandExecutionError::Backend(
            ApplicationServiceError::IsolationVerificationFailed {
                control_name: "resource_limits",
            },
        )),
        "{evidence_name} must fail closed instead of becoming command evidence"
    );
    assert!(
        calls
            .lines()
            .any(|line| line == "rm --force --ignore fake-command-container-id"),
        "{evidence_name} must clean up the exact acquired container ID: {calls}"
    );
    assert!(
        !calls
            .lines()
            .any(|line| line == format!("rm --force --ignore {sandbox_name}")),
        "{evidence_name} must not regain generated-name cleanup authority: {calls}"
    );
}

#[test]
fn widened_command_tmpfs_configuration_fails_closed_and_cleans_up() {
    let widened_tmpfs = r#"[{"Id":"fake-command-container-id","AppArmorProfile":"containers-default","ProcessLabel":"","EffectiveCaps":[],"BoundingCaps":[],"Config":{"User":"65532:65532","Timeout":20},"HostConfig":{"ReadonlyRootfs":true,"Privileged":false,"SecurityOpt":["no-new-privileges"],"UsernsMode":"auto","PidMode":"private","IpcMode":"none","NetworkMode":"none","UTSMode":"private","CgroupMode":"private","Memory":268435456,"NanoCpus":1000000000,"PidsLimit":16,"Tmpfs":{"/tmp":"rw,exec,nosuid,nodev,size=33554432"}}}]"#;

    assert_resource_config_rejected(widened_tmpfs, "widened command tmpfs configuration");
}

#[test]
fn unbounded_command_timeout_configuration_fails_closed_and_cleans_up() {
    let unbounded_timeout = r#"[{"Id":"fake-command-container-id","AppArmorProfile":"containers-default","ProcessLabel":"","EffectiveCaps":[],"BoundingCaps":[],"Config":{"User":"65532:65532","Timeout":0},"HostConfig":{"ReadonlyRootfs":true,"Privileged":false,"SecurityOpt":["no-new-privileges"],"UsernsMode":"auto","PidMode":"private","IpcMode":"none","NetworkMode":"none","UTSMode":"private","CgroupMode":"private","Memory":268435456,"NanoCpus":1000000000,"PidsLimit":16,"Tmpfs":{"/tmp":"rw,noexec,nosuid,nodev,size=16777216"}}}]"#;

    assert_resource_config_rejected(
        unbounded_timeout,
        "unbounded command wall-time configuration",
    );
}

#[test]
fn mismatched_command_timeout_configuration_fails_closed_and_cleans_up() {
    let mismatched_timeout = r#"[{"Id":"fake-command-container-id","AppArmorProfile":"containers-default","ProcessLabel":"","EffectiveCaps":[],"BoundingCaps":[],"Config":{"User":"65532:65532","Timeout":21},"HostConfig":{"ReadonlyRootfs":true,"Privileged":false,"SecurityOpt":["no-new-privileges"],"UsernsMode":"auto","PidMode":"private","IpcMode":"none","NetworkMode":"none","UTSMode":"private","CgroupMode":"private","Memory":268435456,"NanoCpus":1000000000,"PidsLimit":16,"Tmpfs":{"/tmp":"rw,noexec,nosuid,nodev,size=16777216"}}}]"#;

    assert_resource_config_rejected(
        mismatched_timeout,
        "request-mismatched command wall-time configuration",
    );
}

#[test]
fn missing_command_timeout_configuration_fails_closed_and_cleans_up() {
    let missing_timeout = r#"[{"Id":"fake-command-container-id","AppArmorProfile":"containers-default","ProcessLabel":"","EffectiveCaps":[],"BoundingCaps":[],"Config":{"User":"65532:65532"},"HostConfig":{"ReadonlyRootfs":true,"Privileged":false,"SecurityOpt":["no-new-privileges"],"UsernsMode":"auto","PidMode":"private","IpcMode":"none","NetworkMode":"none","UTSMode":"private","CgroupMode":"private","Memory":268435456,"NanoCpus":1000000000,"PidsLimit":16,"Tmpfs":{"/tmp":"rw,noexec,nosuid,nodev,size=16777216"}}}]"#;

    assert_resource_config_rejected(missing_timeout, "missing command wall-time configuration");
}

#[test]
fn missing_command_tmpfs_configuration_fails_closed_and_cleans_up() {
    let missing_tmpfs = r#"[{"Id":"fake-command-container-id","AppArmorProfile":"containers-default","ProcessLabel":"","EffectiveCaps":[],"BoundingCaps":[],"Config":{"User":"65532:65532","Timeout":20},"HostConfig":{"ReadonlyRootfs":true,"Privileged":false,"SecurityOpt":["no-new-privileges"],"UsernsMode":"auto","PidMode":"private","IpcMode":"none","NetworkMode":"none","UTSMode":"private","CgroupMode":"private","Memory":268435456,"NanoCpus":1000000000,"PidsLimit":16}}]"#;

    assert_resource_config_rejected(missing_tmpfs, "missing command tmpfs configuration");
}

#[test]
fn additional_command_tmpfs_destination_fails_closed_and_cleans_up() {
    let additional_tmpfs = r#"[{"Id":"fake-command-container-id","AppArmorProfile":"containers-default","ProcessLabel":"","EffectiveCaps":[],"BoundingCaps":[],"Config":{"User":"65532:65532","Timeout":20},"HostConfig":{"ReadonlyRootfs":true,"Privileged":false,"SecurityOpt":["no-new-privileges"],"UsernsMode":"auto","PidMode":"private","IpcMode":"none","NetworkMode":"none","UTSMode":"private","CgroupMode":"private","Memory":268435456,"NanoCpus":1000000000,"PidsLimit":16,"Tmpfs":{"/tmp":"rw,noexec,nosuid,nodev,size=16777216","/cache":"rw,noexec,nosuid,nodev,size=4096"}}}]"#;

    assert_resource_config_rejected(additional_tmpfs, "additional command tmpfs destination");
}

#[test]
fn duplicate_command_tmpfs_option_fails_closed_and_cleans_up() {
    let duplicate_option = r#"[{"Id":"fake-command-container-id","AppArmorProfile":"containers-default","ProcessLabel":"","EffectiveCaps":[],"BoundingCaps":[],"Config":{"User":"65532:65532","Timeout":20},"HostConfig":{"ReadonlyRootfs":true,"Privileged":false,"SecurityOpt":["no-new-privileges"],"UsernsMode":"auto","PidMode":"private","IpcMode":"none","NetworkMode":"none","UTSMode":"private","CgroupMode":"private","Memory":268435456,"NanoCpus":1000000000,"PidsLimit":16,"Tmpfs":{"/tmp":"rw,noexec,nosuid,nodev,nodev,size=16777216"}}}]"#;

    assert_resource_config_rejected(duplicate_option, "duplicate command tmpfs option");
}

#[test]
fn contradictory_command_tmpfs_option_fails_closed_and_cleans_up() {
    let contradictory_option = r#"[{"Id":"fake-command-container-id","AppArmorProfile":"containers-default","ProcessLabel":"","EffectiveCaps":[],"BoundingCaps":[],"Config":{"User":"65532:65532","Timeout":20},"HostConfig":{"ReadonlyRootfs":true,"Privileged":false,"SecurityOpt":["no-new-privileges"],"UsernsMode":"auto","PidMode":"private","IpcMode":"none","NetworkMode":"none","UTSMode":"private","CgroupMode":"private","Memory":268435456,"NanoCpus":1000000000,"PidsLimit":16,"Tmpfs":{"/tmp":"rw,ro,noexec,nosuid,nodev,size=16777216"}}}]"#;

    assert_resource_config_rejected(contradictory_option, "contradictory command tmpfs option");
}
