//! RED contract for bounded runtime-owned command log storage.
//!
//! A bounded `podman logs` reader does not cap the container runtime's own
//! host-side log file. Command creation must therefore configure a positive,
//! finite storage cap for the `k8s-file` log driver instead of relying only on
//! downstream output truncation.

#![cfg(target_os = "linux")]

use std::{fs, os::unix::fs::PermissionsExt};

use quarantine_sandbox_runtime::{
    CommandExecutionRequest, IsolationPolicy, ResourceRequest, RootlessPodmanAdapter,
};

fn policy() -> IsolationPolicy {
    IsolationPolicy {
        policy_id: "command_log_storage_red_v1".to_owned(),
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
        request_id: "command-log-storage-red".to_owned(),
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

#[test]
fn command_container_configures_a_positive_finite_runtime_log_storage_cap() {
    let directory = tempfile::tempdir().expect("isolated fake Podman directory");
    let program = directory.path().join("podman");
    let create_args = directory.path().join("create-args");
    let inspect = r#"[{"Id":"fake-command-container-id","AppArmorProfile":"containers-default","ProcessLabel":"","EffectiveCaps":[],"BoundingCaps":[],"Config":{"User":"65532:65532","Timeout":20},"HostConfig":{"ReadonlyRootfs":true,"Privileged":false,"SecurityOpt":["no-new-privileges"],"UsernsMode":"auto","PidMode":"private","IpcMode":"none","NetworkMode":"none","UTSMode":"private","CgroupMode":"private","Memory":268435456,"NanoCpus":1000000000,"PidsLimit":16,"Tmpfs":{"/tmp":"rw,noexec,nosuid,nodev,size=16777216"}}}]"#;
    let script = format!(
        "#!/bin/sh\nset -eu\ncase \"${{1:-}}:${{2:-}}\" in\n  info:--format) printf '%s\\n' '{{\"host\":{{\"security\":{{\"rootless\":true,\"seccompEnabled\":true,\"seccompProfilePath\":\"/usr/share/containers/seccomp.json\",\"apparmorEnabled\":true,\"selinuxEnabled\":false}}}},\"version\":{{\"Version\":\"6.1.0\"}}}}' ;;\n  create:--name) printf '%s\\n' \"$@\" > '{}'; printf 'fake-command-container-id\\n' ;;\n  init:*) : ;;\n  start:*) : ;;\n  container:inspect) printf '%s\\n' '{}' ;;\n  top:*) printf 'PID SECCOMP CAPEFF CAPBND CAPINH CAPPRM CAPAMB LABEL\\n1 filter - - - - - containers-default (enforce)\\n' ;;\n  wait:*) printf '0\\n' ;;\n  logs:*) : ;;\n  rm:--force) : ;;\n  *) exit 91 ;;\nesac\n",
        create_args.display(),
        inspect,
    );
    fs::write(&program, script).expect("fake Podman should be writable");
    let mut permissions = fs::metadata(&program)
        .expect("fake Podman metadata should exist")
        .permissions();
    permissions.set_mode(0o700);
    fs::set_permissions(&program, permissions).expect("fake Podman should be executable");
    let adapter = RootlessPodmanAdapter::new(&program);

    let result = adapter.run_legacy_command_at_for_test(&request(), &policy(), 1_780_000_000);
    assert!(
        result.is_ok(),
        "otherwise-positive fake Podman must complete: {result:?}"
    );

    let args = fs::read_to_string(create_args).expect("create argv must be recorded");
    let values = args.lines().collect::<Vec<_>>();
    let log_driver = values
        .windows(2)
        .find_map(|pair| (pair[0] == "--log-driver=k8s-file").then_some(pair[0]));
    assert_eq!(log_driver, Some("--log-driver=k8s-file"));
    let max_size = values.windows(2).find_map(|pair| {
        (pair[0] == "--log-opt" && pair[1].starts_with("max-size=")).then_some(pair[1])
    });
    let max_size = max_size.expect("command creation must bound runtime-owned log storage");
    let bytes = max_size
        .strip_prefix("max-size=")
        .and_then(|value| value.strip_suffix('b'))
        .and_then(|value| value.parse::<u64>().ok())
        .expect("max-size must be a positive byte count");
    assert!(bytes > 0, "runtime log storage cap must be positive");
}
