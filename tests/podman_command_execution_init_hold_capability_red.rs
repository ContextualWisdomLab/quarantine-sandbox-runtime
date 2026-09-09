//! Issue #25 backend-capability RED for a pre-payload Podman init boundary.
//!
//! A configured container is not sufficient evidence. The backend must first
//! initialize the exact acquired container without releasing its consumer
//! payload, evaluate pre-start evidence, and fail closed when initialization or
//! that evidence is unavailable. Real effective-isolation acceptance remains a
//! separate rootless/LSM-capable E2E requirement.

#![cfg(target_os = "linux")]

use std::{
    fs,
    os::unix::fs::symlink,
    path::{Path, PathBuf},
};

use quarantine_sandbox_runtime::{
    CommandExecutionBackend, CommandExecutionRequest, IsolationPolicy, ResourceRequest,
    RootlessPodmanAdapter,
};
use tempfile::TempDir;

fn immutable_fixture_executable() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/fake_podman.sh")
}

fn fixture_sidecar(program: &Path, suffix: &str) -> PathBuf {
    PathBuf::from(format!("{}.{suffix}", program.display()))
}

fn write_fake_podman(directory: &TempDir, name: &str, script: &str) -> PathBuf {
    let program = directory.path().join(name);
    symlink(immutable_fixture_executable(), &program)
        .expect("fake Podman immutable symlink should be creatable");
    let script_path = fixture_sidecar(&program, "script");
    fs::write(&script_path, script).expect("fake Podman scenario data should be writable");
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
        policy_id: "cmdexec_init_hold_policy_v1".to_owned(),
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
        request_id: "cmdexec-init-hold-red".to_owned(),
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

fn security_info_json() -> &'static str {
    r#"{"host":{"security":{"rootless":true,"seccompEnabled":true,"seccompProfilePath":"/usr/share/containers/seccomp.json","apparmorEnabled":true,"selinuxEnabled":false}},"version":{"Version":"6.1.0"}}"#
}

fn invalid_prestart_inspect_json(id: &str) -> String {
    format!(
        "[{{\"Id\":\"{id}\",\"State\":{{\"Status\":\"created\",\"Pid\":4242}},\
         \"AppArmorProfile\":\"containers-default\",\"ProcessLabel\":\"\",\
         \"EffectiveCaps\":null,\"BoundingCaps\":null,\"Config\":{{\"User\":\"65532:65532\"}},\
         \"HostConfig\":{{\"ReadonlyRootfs\":false,\"Privileged\":false,\
         \"SecurityOpt\":[\"no-new-privileges\"],\"UsernsMode\":\"\",\
         \"Annotations\":{{\"io.podman.annotations.userns\":\"auto\"}},\
         \"PidMode\":\"private\",\"IpcMode\":\"none\",\"NetworkMode\":\"none\",\
         \"Memory\":268435456,\"NanoCpus\":1000000000,\"PidsLimit\":16}}}}]"
    )
}

#[test]
fn invalid_prestart_evidence_is_rejected_after_init_without_releasing_payload() {
    let directory = TempDir::new().expect("isolated fixture directory");
    let invocation_log = directory.path().join("invocations.log");
    let payload_marker = directory.path().join("payload-ran");
    let initialized_marker = directory.path().join("initialized");
    let container_id = "a".repeat(64);
    let script = format!(
        "#!/bin/sh\nset -eu\nprintf '%s\\n' \"$*\" >> '{}'\ncase \"${{1:-}}:${{2:-}}\" in\n  \
         info:--format) printf '%s\\n' '{}' ;;\n  \
         create:--name) printf '%s\\n' '{}' ;;\n  \
         init:*) : > '{}' ;;\n  \
         container:inspect) printf '%s\\n' '{}' ;;\n  \
         start:*) : > '{}'; : ;;\n  \
         rm:--force) : ;;\n  \
         *) exit 91 ;;\nesac\n",
        invocation_log.display(),
        security_info_json(),
        container_id,
        initialized_marker.display(),
        invalid_prestart_inspect_json(&container_id),
        payload_marker.display(),
    );
    let program = write_fake_podman(&directory, "init-hold-invalid-evidence", &script);
    let adapter = RootlessPodmanAdapter::new(program);

    let result = adapter.run_command_at(&request(), &policy(), 1_780_000_000);
    assert!(
        result.is_err(),
        "invalid pre-start isolation evidence must fail closed"
    );

    let invocations = fs::read_to_string(&invocation_log).expect("invocations should be recorded");
    let lines: Vec<&str> = invocations.lines().collect();
    let init_position = lines
        .iter()
        .position(|line| line.starts_with(&format!("init {container_id}")));
    let inspect_position = lines
        .iter()
        .position(|line| line.starts_with(&format!("container inspect {container_id}")));
    let start_seen = lines
        .iter()
        .any(|line| line.starts_with(&format!("start {container_id}")));
    let init_before_inspect = matches!(
        (init_position, inspect_position),
        (Some(init), Some(inspect)) if init < inspect
    );

    assert!(
        init_before_inspect && !start_seen && !payload_marker.exists(),
        "backend must hold consumer execution as create -> init -> pre-start evidence -> rejection; invocations={invocations:?}, payload_exists={}",
        payload_marker.exists()
    );
    assert!(
        initialized_marker.exists(),
        "the RED must distinguish a successful init hold from static configured state"
    );
}

#[test]
fn unavailable_init_capability_fails_closed_without_releasing_payload() {
    let directory = TempDir::new().expect("isolated fixture directory");
    let invocation_log = directory.path().join("invocations.log");
    let payload_marker = directory.path().join("payload-ran");
    let container_id = "b".repeat(64);
    let script = format!(
        "#!/bin/sh\nset -eu\nprintf '%s\\n' \"$*\" >> '{}'\ncase \"${{1:-}}:${{2:-}}\" in\n  \
         info:--format) printf '%s\\n' '{}' ;;\n  \
         create:--name) printf '%s\\n' '{}' ;;\n  \
         init:*) exit 125 ;;\n  \
         container:inspect) printf '%s\\n' '{}' ;;\n  \
         start:*) : > '{}'; : ;;\n  \
         rm:--force) : ;;\n  \
         *) exit 91 ;;\nesac\n",
        invocation_log.display(),
        security_info_json(),
        container_id,
        invalid_prestart_inspect_json(&container_id),
        payload_marker.display(),
    );
    let program = write_fake_podman(&directory, "init-hold-unavailable", &script);
    let adapter = RootlessPodmanAdapter::new(program);

    let result = adapter.run_command_at(&request(), &policy(), 1_780_000_001);
    assert!(
        result.is_err(),
        "an unavailable pre-start init capability must fail closed"
    );

    let invocations = fs::read_to_string(&invocation_log).expect("invocations should be recorded");
    let init_seen = invocations
        .lines()
        .any(|line| line.starts_with(&format!("init {container_id}")));
    let start_seen = invocations
        .lines()
        .any(|line| line.starts_with(&format!("start {container_id}")));

    assert!(
        init_seen && !start_seen && !payload_marker.exists(),
        "missing init capability must terminate before payload release; invocations={invocations:?}, payload_exists={}",
        payload_marker.exists()
    );
}
