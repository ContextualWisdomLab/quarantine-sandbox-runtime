//! RED for runtime-owned hold-gate delivery and OCI process binding.
//!
//! A held-gate design is not real until the production Podman create request both delivers the
//! runtime-owned gate into the container read-only and makes that gate, rather than the hostile
//! consumer argv, the initial OCI process. This regression intentionally stops before proving the
//! later attestation/release channel; issue #25 keeps that evidence independent.

#![cfg(target_os = "linux")]

use std::{
    fs,
    os::unix::fs::PermissionsExt,
    path::PathBuf,
    sync::atomic::{AtomicU64, Ordering},
    time::{SystemTime, UNIX_EPOCH},
};

use quarantine_sandbox_runtime::{
    CommandExecutionRequest, IsolationPolicy, ResourceRequest, RootlessPodmanAdapter,
};

static NEXT_PATH_ID: AtomicU64 = AtomicU64::new(0);

fn temporary_path(name: &str) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock should be after the Unix epoch")
        .as_nanos();
    let unique_id = NEXT_PATH_ID.fetch_add(1, Ordering::Relaxed);
    std::env::temp_dir().join(format!(
        "quarantine-sandbox-runtime-hold-gate-binding-{name}-{}-{nanos}-{unique_id}",
        std::process::id()
    ))
}

fn write_executable(name: &str, script: &str) -> PathBuf {
    let program = temporary_path(name);
    fs::write(&program, script).expect("fake Podman should be writable");
    let mut permissions = fs::metadata(&program)
        .expect("fake Podman metadata should exist")
        .permissions();
    permissions.set_mode(0o700);
    fs::set_permissions(&program, permissions).expect("fake Podman should be executable");
    program
}

fn policy() -> IsolationPolicy {
    IsolationPolicy {
        policy_id: "hold_gate_binding_policy_v1".to_owned(),
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
        request_id: "hold-gate-binding-request".to_owned(),
        image_reference: format!("localhost/cwl/tool@sha256:{}", "a".repeat(64)),
        command: vec!["payload-sentinel".to_owned(), "argument with spaces".to_owned()],
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
fn podman_create_binds_runtime_gate_as_initial_process_before_consumer_argv() {
    let calls = temporary_path("calls");
    let security_info = r#"{"host":{"security":{"rootless":true,"seccompEnabled":true,"seccompProfilePath":"/usr/share/containers/seccomp.json","apparmorEnabled":true,"selinuxEnabled":false}},"version":{"Version":"6.1.0"}}"#;
    let inspect = r#"[{"Id":"fake-command-container-id","AppArmorProfile":"containers-default","ProcessLabel":"","EffectiveCaps":null,"BoundingCaps":null,"Config":{"User":"65532:65532"},"HostConfig":{"ReadonlyRootfs":true,"Privileged":false,"SecurityOpt":["no-new-privileges"],"UsernsMode":"","Annotations":{"io.podman.annotations.userns":"auto"},"PidMode":"private","IpcMode":"none","NetworkMode":"none","Memory":268435456,"NanoCpus":1000000000,"PidsLimit":16}}]"#;
    let script = format!(
        "#!/bin/sh\nset -eu\nprintf '%s\\n' \"$*\" >> '{}'\ncase \"${{1:-}}:${{2:-}}\" in\n  info:--format) printf '%s\\n' '{}' ;;\n  create:--name) printf 'fake-command-container-id\\n' ;;\n  init:*) : ;;\n  container:inspect) printf '%s\\n' '{}' ;;\n  start:*) : ;;\n  top:*) exit 1 ;;\n  rm:--force) : ;;\n  *) exit 91 ;;\nesac\n",
        calls.display(), security_info, inspect,
    );
    let program = write_executable("hold-gate-binding", &script);
    let adapter = RootlessPodmanAdapter::new(program.clone());

    let result = adapter.run_command_at(&request(), &policy(), 1_780_000_101);

    assert!(
        result.is_err(),
        "the fake backend intentionally lacks effective process evidence after start"
    );
    let recorded_calls = fs::read_to_string(&calls).expect("backend calls should be recorded");
    let create_call = recorded_calls
        .lines()
        .find(|line| line.starts_with("create --name "))
        .expect("the command runtime must issue one Podman create request");

    assert!(
        create_call.contains("--entrypoint=/qsr-runtime-gate"),
        "the initial OCI process must be the runtime-owned hold gate, not the hostile consumer argv: {create_call}"
    );
    assert!(
        create_call.matches("/qsr-runtime-gate").count() >= 2,
        "the gate must be both delivered into the container and selected as its initial process: {create_call}"
    );
    assert!(
        !create_call.contains("--entrypoint=[\"payload-sentinel\""),
        "consumer argv must remain held behind the runtime-owned gate rather than becoming the OCI entrypoint: {create_call}"
    );
    assert!(
        create_call.contains("payload-sentinel") && create_call.contains("argument with spaces"),
        "the exact consumer argv must still be preserved for release by the gate: {create_call}"
    );
    assert!(
        recorded_calls.contains("rm --force --ignore fake-command-container-id"),
        "the test must preserve exact-ID cleanup after the intentional post-start attestation failure"
    );

    let _ = fs::remove_file(program);
    let _ = fs::remove_file(calls);
}
