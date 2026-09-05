//! RED coverage for source-backed command-container mount-set attestation.
//!
//! The effective mount set must contain exactly the runtime-owned `/workspace`
//! bind when an exact source artifact is requested. Finding one valid bind is
//! insufficient if an additional or duplicate host-backed mount is also present.

#![cfg(target_os = "linux")]

use std::{
    fs,
    os::unix::fs::PermissionsExt,
    path::PathBuf,
    sync::atomic::{AtomicU64, Ordering},
    time::{SystemTime, UNIX_EPOCH},
};

use quarantine_sandbox_runtime::{
    ApplicationServiceError, CommandExecutionError, CommandExecutionRequest, IsolationPolicy,
    PrSourceArtifactInput, ResourceRequest, RootlessPodmanAdapter,
};
use sha2::{Digest, Sha256};

static NEXT_PATH_ID: AtomicU64 = AtomicU64::new(0);

fn temporary_path(name: &str) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock should be after the Unix epoch")
        .as_nanos();
    let unique_id = NEXT_PATH_ID.fetch_add(1, Ordering::Relaxed);
    std::env::temp_dir().join(format!(
        "quarantine-sandbox-runtime-source-mount-set-{name}-{}-{nanos}-{unique_id}",
        std::process::id()
    ))
}

fn tree_digest(path: &str, bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update((path.len() as u64).to_be_bytes());
    hasher.update(path.as_bytes());
    hasher.update((bytes.len() as u64).to_be_bytes());
    hasher.update(bytes);
    format!("{:x}", hasher.finalize())
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
        policy_id: "command_source_mount_set_policy_v1".to_owned(),
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

fn source_request(source: PathBuf) -> CommandExecutionRequest {
    let bytes = b"bounded source\n";
    fs::write(source.join("data.txt"), bytes).expect("source file should be writable");
    CommandExecutionRequest {
        schema_version: "1.0.0".to_owned(),
        request_id: "source-command-mount-set-request".to_owned(),
        image_reference: format!("localhost/cwl/tool@sha256:{}", "b".repeat(64)),
        command: vec!["true".to_owned()],
        source_artifact: Some(PrSourceArtifactInput {
            host_path: source,
            revision_sha: "a".repeat(40),
            expected_tree_sha256: tree_digest("data.txt", bytes),
        }),
        resources: ResourceRequest {
            memory_bytes: 256 * 1024 * 1024,
            cpu_millicores: 1_000,
            maximum_processes: 16,
            lease_seconds: 20,
            tmpfs_bytes: 16 * 1024 * 1024,
        },
    }
}

fn run_with_effective_mounts(
    name: &str,
    unexpected_mount_json: &str,
) -> (
    Result<quarantine_sandbox_runtime::CommandExecutionResult, CommandExecutionError>,
    String,
    PathBuf,
    PathBuf,
) {
    let source = temporary_path(&format!("{name}-source"));
    fs::create_dir_all(&source).expect("source directory should be creatable");
    let request = source_request(source.clone());
    let calls = temporary_path(&format!("{name}-calls"));
    let volume_record = temporary_path(&format!("{name}-volume"));
    let security_info = r#"{"host":{"security":{"rootless":true,"seccompEnabled":true,"seccompProfilePath":"/usr/share/containers/seccomp.json","apparmorEnabled":true,"selinuxEnabled":false}},"version":{"Version":"6.1.0"}}"#;
    let top = "PID SECCOMP CAPEFF CAPBND CAPINH CAPPRM CAPAMB LABEL\n1 filter - - - - - containers-default (enforce)\n";
    let script = format!(
        "#!/bin/sh\nset -eu\nprintf '%s\\n' \"$*\" >> '{}'\ncase \"${{1:-}}:${{2:-}}\" in\n  info:--format) printf '%s\\n' '{}' ;;\n  create:--name)\n    previous=''\n    volume=''\n    for argument in \"$@\"; do\n      if [ \"$previous\" = '--volume' ]; then volume=\"$argument\"; break; fi\n      previous=\"$argument\"\n    done\n    [ -n \"$volume\" ] || exit 92\n    printf '%s\\n' \"$volume\" > '{}'\n    printf 'fake-command-container-id\\n'\n    ;;\n  start:*) : ;;\n  container:inspect)\n    volume=$(cat '{}')\n    source_path=${{volume%%:/workspace:*}}\n    printf '[{{\"Id\":\"fake-command-container-id\",\"AppArmorProfile\":\"containers-default\",\"ProcessLabel\":\"\",\"EffectiveCaps\":null,\"BoundingCaps\":null,\"Config\":{{\"User\":\"65532:65532\"}},\"HostConfig\":{{\"ReadonlyRootfs\":true,\"Privileged\":false,\"SecurityOpt\":[\"no-new-privileges\"],\"UsernsMode\":\"\",\"Annotations\":{{\"io.podman.annotations.userns\":\"auto\"}},\"PidMode\":\"private\",\"IpcMode\":\"none\",\"NetworkMode\":\"none\",\"Memory\":268435456,\"NanoCpus\":1000000000,\"PidsLimit\":16}},\"Mounts\":[{{\"Source\":\"%s\",\"Destination\":\"/workspace\",\"Type\":\"bind\",\"Options\":[\"ro\",\"noexec\",\"nosuid\",\"nodev\"],\"RW\":false}},{}]}}]\\n' \"$source_path\"\n    ;;\n  top:*) printf '%s' '{}' ;;\n  wait:*) printf '0\\n' ;;\n  logs:*) : ;;\n  rm:--force) : ;;\n  *) exit 91 ;;\nesac\n",
        calls.display(),
        security_info,
        volume_record.display(),
        volume_record.display(),
        unexpected_mount_json,
        top,
    );
    let program = write_executable(name, &script);
    let adapter = RootlessPodmanAdapter::new(program.clone());
    let result = adapter.run_command_at(&request, &policy(), 1_780_000_033);
    let recorded_calls = fs::read_to_string(&calls).expect("backend calls should be recorded");
    let _ = fs::remove_file(calls);
    let _ = fs::remove_file(volume_record);
    (result, recorded_calls, program, source)
}

fn assert_mount_set_rejected(name: &str, unexpected_mount_json: &str) {
    let (result, recorded_calls, program, source) =
        run_with_effective_mounts(name, unexpected_mount_json);
    assert_eq!(
        result,
        Err(CommandExecutionError::Backend(
            ApplicationServiceError::IsolationVerificationFailed {
                control_name: "command_mount_set",
            },
        )),
        "the complete effective mount set must reject unrequested host filesystem authority"
    );
    assert!(
        recorded_calls.contains("rm --force --ignore"),
        "mount-set attestation failure must clean up the exact command sandbox"
    );
    let _ = fs::remove_file(program);
    let _ = fs::remove_dir_all(source);
}

#[test]
fn source_command_rejects_an_additional_host_bind_even_when_workspace_is_exact() {
    assert_mount_set_rejected(
        "additional-host-bind",
        r#"{"Source":"/etc","Destination":"/unexpected-host","Type":"bind","Options":["ro","noexec","nosuid","nodev"],"RW":false}"#,
    );
}

#[test]
fn source_command_rejects_a_duplicate_workspace_mount_after_the_exact_runtime_bind() {
    assert_mount_set_rejected(
        "duplicate-workspace-bind",
        r#"{"Source":"/etc","Destination":"/workspace","Type":"bind","Options":["ro","noexec","nosuid","nodev"],"RW":false}"#,
    );
}
