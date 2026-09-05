//! RED regression for post-create command-container lifecycle ownership.
//!
//! Once `podman create` returns a concrete container ID, every later lifecycle
//! operation must use that acquired identity. The requested `qsr-cmd-*` name is
//! correlation metadata, not destructive authority: name reuse or rebinding
//! must not let this invocation start, inspect, wait on, log, kill, or remove a
//! different same-principal container.

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
const OWNED_CONTAINER_ID: &str =
    "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";

fn temporary_path(name: &str) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock should be after the Unix epoch")
        .as_nanos();
    let unique_id = NEXT_PATH_ID.fetch_add(1, Ordering::Relaxed);
    std::env::temp_dir().join(format!(
        "quarantine-sandbox-runtime-post-create-ownership-red-{name}-{}-{nanos}-{unique_id}",
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
        policy_id: "command_post_create_ownership_red_policy_v1".to_owned(),
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
        request_id: "command-post-create-ownership-red-request".to_owned(),
        image_reference: format!("localhost/cwl/tool@sha256:{}", "8".repeat(64)),
        command: vec!["pytest".to_owned(), "-q".to_owned()],
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

fn container_inspect_json() -> String {
    format!(
        "[{{\"Id\":\"{OWNED_CONTAINER_ID}\",\"AppArmorProfile\":\"containers-default\",\"ProcessLabel\":\"\",\
         \"EffectiveCaps\":null,\"BoundingCaps\":null,\"Config\":{{\"User\":\"65532:65532\",\"Timeout\":20}},\
         \"HostConfig\":{{\"ReadonlyRootfs\":true,\"Privileged\":false,\
         \"SecurityOpt\":[\"no-new-privileges\"],\"UsernsMode\":\"\",\
         \"Annotations\":{{\"io.podman.annotations.userns\":\"auto\"}},\
         \"PidMode\":\"private\",\"IpcMode\":\"none\",\"NetworkMode\":\"none\",\"Memory\":268435456,\
         \"NanoCpus\":1000000000,\"PidsLimit\":16,\"Tmpfs\":{{\"/tmp\":\"rw,noexec,nosuid,nodev,size=16777216\"}}}},\
         \"Mounts\":[]}}]"
    )
}

#[test]
fn successful_create_binds_every_lifecycle_operation_to_the_acquired_container_id() {
    let call_log = temporary_path("call-log");
    let owned_marker = temporary_path("owned-container-marker");
    let foreign_marker = temporary_path("foreign-container-marker");
    fs::write(&foreign_marker, b"foreign-container-untouched")
        .expect("foreign resource marker should be writable");

    let script = format!(
        "#!/bin/sh\nset -eu\nprintf '%s\\n' \"$*\" >> '{}'\nowned='{}'\nforeign='{}'\nowned_marker='{}'\nmark_foreign() {{ printf 'foreign-container-touched' > \"$foreign\"; }}\nrequire_owned() {{ [ \"${{1:-}}\" = \"$owned\" ] || {{ mark_foreign; exit 97; }}; }}\ncase \"${{1:-}}:${{2:-}}\" in\n  info:--format) printf '%s\\n' '{}' ;;\n  create:--name) : > \"$owned_marker\"; printf '%s\\n' \"$owned\" ;;\n  start:*) require_owned \"${{2:-}}\" ;;\n  container:inspect) require_owned \"${{5:-}}\"; printf '%s\\n' '{}' ;;\n  top:*) require_owned \"${{2:-}}\"; printf '%s\\n' 'PID SECCOMP CAPEFF CAPBND CAPINH CAPPRM CAPAMB LABEL'; printf '%s\\n' '1 filter - - - - - containers-default (enforce)' ;;\n  wait:*) require_owned \"${{2:-}}\"; printf '0\\n' ;;\n  logs:*) require_owned \"${{2:-}}\"; printf 'owned stdout\\n' ;;\n  kill:*) require_owned \"${{2:-}}\" ;;\n  rm:--force) require_owned \"${{3:-}}\"; rm -f \"$owned_marker\" ;;\n  *) exit 91 ;;\nesac\n",
        call_log.display(),
        OWNED_CONTAINER_ID,
        foreign_marker.display(),
        owned_marker.display(),
        security_info_json(),
        container_inspect_json(),
    );
    let program = write_executable("fake-podman", &script);
    let adapter = RootlessPodmanAdapter::new(program.clone());

    let result = adapter
        .run_command_at(&request(), &policy(), 1_780_000_036)
        .expect("an owned container ID should remain lifecycle authority after create");

    assert_eq!(result.exit_code(), 0);
    assert_eq!(result.stdout(), "owned stdout\n");
    assert_eq!(
        fs::read(&foreign_marker).expect("foreign marker should remain readable"),
        b"foreign-container-untouched",
        "post-create lifecycle or cleanup must never resolve the mutable qsr-cmd name onto a foreign container"
    );
    assert!(
        !owned_marker.exists(),
        "the invocation-owned container must still be removed after successful completion"
    );

    let calls = fs::read_to_string(&call_log).expect("fake Podman calls should be recorded");
    for operation in ["start ", "top ", "wait ", "logs "] {
        assert!(
            calls
                .lines()
                .any(|line| line.starts_with(operation) && line.contains(OWNED_CONTAINER_ID)),
            "{operation} must target the acquired container ID: {calls}"
        );
    }
    assert!(
        calls.lines().any(|line| {
            line.starts_with("container inspect ") && line.ends_with(OWNED_CONTAINER_ID)
        }),
        "container inspect must target the acquired container ID: {calls}"
    );
    assert!(
        calls.lines().any(|line| {
            line.starts_with("rm --force ") && line.ends_with(OWNED_CONTAINER_ID)
        }),
        "destructive removal must target the acquired container ID: {calls}"
    );

    let _ = fs::remove_file(program);
    let _ = fs::remove_file(call_log);
    let _ = fs::remove_file(owned_marker);
    let _ = fs::remove_file(foreign_marker);
}
