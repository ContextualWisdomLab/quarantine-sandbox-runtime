//! RED: network authority must be bound to the object created by this invocation.
//!
//! `podman network create` reports a correlation name, while the current owner acquires the
//! backend ID through a later name-based inspection. A same-name replacement between those
//! operations can therefore present a different canonical ID with otherwise-valid P0 state.
//! This witness requires that such later name resolution never become private attachment or
//! destructive authority merely because its fields are internally consistent.

#![cfg(target_os = "linux")]

use std::{
    fs,
    os::unix::fs::PermissionsExt,
    path::{Path, PathBuf},
    sync::atomic::{AtomicU64, Ordering},
    time::{SystemTime, UNIX_EPOCH},
};

use quarantine_sandbox_runtime::{
    ApplicationServiceRequest, IsolationPolicy, ResourceRequest, RootlessPodmanAdapter,
    ServiceProtocol,
};

const STARTED_AT_EPOCH_SECONDS: u64 = 1_780_004_500;
const REPLACEMENT_NETWORK_ID: &str =
    "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";
static NEXT_TEMP_PATH_ID: AtomicU64 = AtomicU64::new(0);

fn temporary_path(name: &str) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("test clock must be after the Unix epoch")
        .as_nanos();
    let unique_id = NEXT_TEMP_PATH_ID.fetch_add(1, Ordering::Relaxed);
    std::env::temp_dir().join(format!(
        "qsr-network-create-inspect-toctou-red-{name}-{}-{nanos}-{unique_id}",
        std::process::id()
    ))
}

fn policy() -> IsolationPolicy {
    IsolationPolicy {
        policy_id: "network_create_inspect_toctou_red_v1".to_owned(),
        maximum_memory_bytes: 256 * 1024 * 1024,
        maximum_cpu_millicores: 500,
        maximum_processes: 32,
        maximum_lease_seconds: 60,
        maximum_tmpfs_bytes: 32 * 1024 * 1024,
        readiness_timeout_millis: 500,
        readiness_poll_interval_millis: 10,
        shutdown_grace_seconds: 1,
        run_as_user_id: 65_532,
        run_as_group_id: 65_532,
    }
}

fn request() -> ApplicationServiceRequest {
    ApplicationServiceRequest {
        schema_version: "1.0.0".to_owned(),
        request_id: "network_create_inspect_toctou_red".to_owned(),
        image_reference: format!("localhost/cwl/tool@sha256:{}", "f".repeat(64)),
        container_port: 8_080,
        protocol: ServiceProtocol::Tcp,
        command: vec!["serve".to_owned()],
        resources: ResourceRequest {
            memory_bytes: 128 * 1024 * 1024,
            cpu_millicores: 250,
            maximum_processes: 16,
            lease_seconds: 30,
            tmpfs_bytes: 16 * 1024 * 1024,
        },
    }
}

fn write_fake_podman(log: &Path, container_create_marker: &Path) -> PathBuf {
    let program = temporary_path("fake-podman");
    let info = r#"{"host":{"security":{"rootless":true,"seccompEnabled":true,"seccompProfilePath":"/usr/share/containers/seccomp.json","apparmorEnabled":true,"selinuxEnabled":false}}}"#;
    let script = format!(
        r#"#!/bin/sh
set -eu
printf '%s\n' "$*" >> '{log}'
container_create_marker='{container_create_marker}'
replacement_network_id='{replacement_network_id}'
if [ "${{1:-}}" = info ]; then
  if [ "${{3:-}}" = json ]; then printf '%s\n' '{info}'; else printf 'true\n'; fi
  exit 0
fi
case "${{1:-}}:${{2:-}}" in
  network:create)
    # Podman's CLI creation result is the generated name. The originally created object is
    # treated as gone before the first inspect; a replacement now owns the same name.
    printf '%s\n' "${{5:-}}"
    ;;
  network:inspect)
    selector=${{5:-}}
    printf '[{{"name":"%s","id":"%s","internal":true,"dns_enabled":false,"containers":{{}}}}]\n' "$selector" "$replacement_network_id"
    ;;
  create:--name)
    printf 'foreign-network-promoted\n' > "$container_create_marker"
    exit 97
    ;;
  network:rm)
    :
    ;;
  *)
    exit 91
    ;;
esac
"#,
        log = log.display(),
        container_create_marker = container_create_marker.display(),
        replacement_network_id = REPLACEMENT_NETWORK_ID,
        info = info,
    );

    fs::write(&program, script).expect("fake Podman must be writable");
    let mut permissions = fs::metadata(&program)
        .expect("fake Podman metadata must exist")
        .permissions();
    permissions.set_mode(0o700);
    fs::set_permissions(&program, permissions).expect("fake Podman must be executable");
    program
}

#[test]
fn first_name_inspect_cannot_promote_a_same_name_replacement_network() {
    let log = temporary_path("calls");
    let container_create_marker = temporary_path("container-create-marker");
    let program = write_fake_podman(&log, &container_create_marker);
    let adapter = RootlessPodmanAdapter::new(program.clone());

    let result = adapter.launch_at(&request(), &policy(), STARTED_AT_EPOCH_SECONDS);
    let calls = fs::read_to_string(&log).expect("fake Podman calls must be recorded");
    let container_create_reached = container_create_marker.exists();

    let _ = fs::remove_file(program);
    let _ = fs::remove_file(log);
    let _ = fs::remove_file(container_create_marker);

    let lines: Vec<&str> = calls.lines().collect();
    let network_create_index = lines
        .iter()
        .position(|line| line.starts_with("network create --internal --disable-dns qsr-net-"))
        .expect("the witness must reach network creation");
    let created_name = lines[network_create_index]
        .split_whitespace()
        .last()
        .expect("network creation must include a generated correlation name");
    let expected_inspect = format!("network inspect --format json {created_name}");
    let inspect_indices: Vec<usize> = lines
        .iter()
        .enumerate()
        .filter_map(|(index, line)| (*line == expected_inspect).then_some(index))
        .collect();

    assert_eq!(
        inspect_indices.len(),
        1,
        "the causal witness must perform exactly one first name-based identity lookup; calls were:\n{calls}"
    );
    assert!(
        network_create_index < inspect_indices[0],
        "identity lookup must happen after network creation; calls were:\n{calls}"
    );
    assert!(
        result.is_err(),
        "a same-name replacement must never yield a successful application-service lease"
    );
    assert!(
        !container_create_reached,
        "a canonical ID learned only from the first post-create name lookup is not creation-bound authority"
    );
    assert!(
        !calls.lines().any(|line| line.starts_with("create --name ")),
        "container creation must not consume a replacement network ID learned only through later name resolution; calls were:\n{calls}"
    );
}
