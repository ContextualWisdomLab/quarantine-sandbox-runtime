//! RED coverage for the one-shot command Podman option/data boundary.
//!
//! `CommandExecutionRequest::image_reference` is domain data. A digest-pinned repository name may
//! still begin with `-`, so the production command path must terminate Podman option parsing before
//! passing the image operand. This is independent of issue #25's hold/attest/release boundary: both
//! controls must survive the eventual runtime-gate integration.

#![cfg(target_os = "linux")]

use std::{fs, os::unix::fs::PermissionsExt, path::PathBuf};

use quarantine_sandbox_runtime::{
    CommandExecutionRequest, IsolationPolicy, ResourceRequest, RootlessPodmanAdapter,
};
use tempfile::TempDir;

const OWNED_CONTAINER_ID: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
const EXPECTED_DIGEST_HEX: &str =
    "eeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeee";

fn policy() -> IsolationPolicy {
    IsolationPolicy {
        policy_id: "command_option_terminator_red_v1".to_owned(),
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
        request_id: "command-option-terminator-red".to_owned(),
        image_reference: format!("-consumer/tool@sha256:{EXPECTED_DIGEST_HEX}"),
        command: vec![
            "/usr/bin/tool".to_owned(),
            "argument with spaces".to_owned(),
            "--flag=value".to_owned(),
        ],
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
        .prefix("qsr-command-option-terminator-red-")
        .tempdir()
        .expect("isolated fake-Podman directory");
    let program = directory.path().join("podman");
    let calls = directory.path().join("calls");
    let info = r#"{"host":{"security":{"rootless":true,"seccompEnabled":true,"seccompProfilePath":"/usr/share/containers/seccomp.json","apparmorEnabled":true,"selinuxEnabled":false}},"version":{"Version":"6.1.0"}}"#;
    let inspect = format!(
        "[{{\"Id\":\"{OWNED_CONTAINER_ID}\",\"ImageDigest\":\"sha256:{EXPECTED_DIGEST_HEX}\",\"AppArmorProfile\":\"containers-default\",\"ProcessLabel\":\"\",\"EffectiveCaps\":[],\"BoundingCaps\":[],\"Config\":{{\"User\":\"65532:65532\",\"Timeout\":20}},\"HostConfig\":{{\"ReadonlyRootfs\":true,\"Privileged\":false,\"SecurityOpt\":[\"no-new-privileges\"],\"UsernsMode\":\"auto\",\"PidMode\":\"private\",\"IpcMode\":\"none\",\"NetworkMode\":\"none\",\"UTSMode\":\"private\",\"CgroupMode\":\"private\",\"Memory\":268435456,\"NanoCpus\":1000000000,\"PidsLimit\":16,\"Tmpfs\":{{\"/tmp\":\"rw,noexec,nosuid,nodev,size=16777216\"}}}},\"Mounts\":[]}}]"
    );
    let top = "PID SECCOMP CAPEFF CAPBND CAPINH CAPPRM CAPAMB LABEL\n1 filter - - - - - containers-default (enforce)\n";
    let script = format!(
        "#!/bin/sh\nset -eu\nprintf '%s\\n' \"$*\" >> '{}'\ncase \"${{1:-}}:${{2:-}}\" in\n  info:--format) printf '%s\\n' '{}' ;;\n  create:--name) printf '%s\\n' '{}' ;;\n  init:*) : ;;\n  start:*) : ;;\n  container:inspect) printf '%s\\n' '{}' ;;\n  top:*) printf '%s' '{}' ;;\n  wait:*) printf '0\\n' ;;\n  logs:*) printf 'ok\\n' ;;\n  kill:*) : ;;\n  rm:--force) : ;;\n  *) exit 91 ;;\nesac\n",
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
fn command_runtime_terminates_podman_options_before_consumer_image() {
    let (_directory, program, calls_path) = fake_podman();
    let adapter = RootlessPodmanAdapter::new(program);
    let request = request();

    let result = adapter.run_legacy_command_at_for_test(&request, &policy(), 1_780_000_109);
    assert!(
        result.is_ok(),
        "fixture must stay positive through isolation so the RED is only the provider option/data boundary: {result:?}"
    );

    let calls = fs::read_to_string(calls_path).expect("fake Podman calls must be recorded");
    let create_call = calls
        .lines()
        .find(|line| line.starts_with("create --name "))
        .expect("container create call must be recorded");
    let argv = create_call.split_whitespace().collect::<Vec<_>>();
    let image_index = argv
        .iter()
        .position(|argument| *argument == request.image_reference)
        .expect("the exact consumer image must remain a positional create operand");

    assert!(
        image_index > 0,
        "the image must not be the first create argument"
    );
    assert_eq!(
        argv[image_index - 1],
        "--",
        "Podman option parsing must terminate immediately before consumer-controlled image data: {create_call}"
    );
}
