//! Hostile RED for command payload execution before effective isolation attestation.
//!
//! `podman create IMAGE COMMAND...` binds the consumer command as the container's
//! OCI process and `podman start` makes that process runnable. A fail-closed error
//! after `start` cannot undo hostile payload execution that happened before live
//! seccomp/LSM/capability evidence was sampled. The command runtime must therefore
//! use a two-phase hold/attest/release primitive (or equivalent) so the consumer
//! payload is not runnable until positive effective isolation is established.

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
        "quarantine-sandbox-runtime-pre-attestation-{name}-{}-{nanos}-{unique_id}",
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
        policy_id: "pre_attestation_hold_policy_v1".to_owned(),
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
        request_id: "pre-attestation-payload-request".to_owned(),
        image_reference: format!("localhost/cwl/tool@sha256:{}", "a".repeat(64)),
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

#[test]
fn command_payload_is_not_runnable_before_effective_process_attestation() {
    let calls = temporary_path("calls");
    let created_payload = temporary_path("created-payload");
    let payload_side_effect = temporary_path("payload-side-effect");
    let security_info = r#"{"host":{"security":{"rootless":true,"seccompEnabled":true,"seccompProfilePath":"/usr/share/containers/seccomp.json","apparmorEnabled":true,"selinuxEnabled":false}},"version":{"Version":"6.1.0"}}"#;
    let inspect = r#"[{"Id":"fake-command-container-id","AppArmorProfile":"containers-default","ProcessLabel":"","EffectiveCaps":null,"BoundingCaps":null,"Config":{"User":"65532:65532"},"HostConfig":{"ReadonlyRootfs":true,"Privileged":false,"SecurityOpt":["no-new-privileges"],"UsernsMode":"","Annotations":{"io.podman.annotations.userns":"auto"},"PidMode":"private","IpcMode":"none","NetworkMode":"none","Memory":268435456,"NanoCpus":1000000000,"PidsLimit":16}}]"#;
    let script = format!(
        "#!/bin/sh\nset -eu\nprintf '%s\\n' \"$*\" >> '{}'\ncase \"${{1:-}}:${{2:-}}\" in\n  info:--format) printf '%s\\n' '{}' ;;\n  create:--name) case \"$*\" in *payload-sentinel*) : > '{}' ;; esac; printf 'fake-command-container-id\\n' ;;\n  start:*) if [ -f '{}' ]; then : > '{}'; fi ;;\n  container:inspect) printf '%s\\n' '{}' ;;\n  top:*) exit 1 ;;\n  wait:*) printf '0\\n' ;;\n  logs:*) : ;;\n  rm:--force) : ;;\n  *) exit 91 ;;\nesac\n",
        calls.display(),
        security_info,
        created_payload.display(),
        created_payload.display(),
        payload_side_effect.display(),
        inspect,
    );
    let program = write_executable("payload-before-attestation", &script);
    let adapter = RootlessPodmanAdapter::new(program.clone());

    let result = adapter.run_command_at(&request(), &policy(), 1_780_000_001);

    assert!(
        result.is_err(),
        "missing live process attestation must fail the command execution"
    );
    let recorded_calls = fs::read_to_string(&calls).expect("backend calls should be recorded");
    assert!(
        recorded_calls.contains("rm --force --ignore"),
        "attestation failure must clean up the exact command sandbox"
    );
    assert!(
        !payload_side_effect.exists(),
        "consumer payload became runnable before effective isolation attestation; use a trusted hold/attest/release phase rather than starting the consumer OCI command directly"
    );

    let _ = fs::remove_file(program);
    let _ = fs::remove_file(calls);
    let _ = fs::remove_file(created_payload);
    let _ = fs::remove_file(payload_side_effect);
}
