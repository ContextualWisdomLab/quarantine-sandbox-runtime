//! RED: Podman v6 JSON event receipts use a lowercase `network` key.
//!
//! Go-template placeholders such as `.Network` are not JSON field names. Podman v6.0.0's
//! `cmd/podman/system/events.go` serializes the network event field as `json:"network,omitempty"`.
//! The runtime must accept that exact upstream JSON surface before promoting a creation-bound ID.

#![cfg(target_os = "linux")]

use std::{
    fs,
    os::unix::fs::PermissionsExt,
    path::{Path, PathBuf},
    sync::atomic::{AtomicU64, Ordering},
    time::{SystemTime, UNIX_EPOCH},
};

use quarantine_sandbox_runtime::{
    ApplicationServiceError, ApplicationServiceRequest, IsolationPolicy, ResourceRequest,
    RootlessPodmanAdapter, ServiceProtocol,
};

const STARTED_AT_EPOCH_SECONDS: u64 = 1_780_006_100;
const NETWORK_ID: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
static NEXT_TEMP_PATH_ID: AtomicU64 = AtomicU64::new(0);

fn temporary_path(name: &str) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("test clock must be after the Unix epoch")
        .as_nanos();
    let unique_id = NEXT_TEMP_PATH_ID.fetch_add(1, Ordering::Relaxed);
    std::env::temp_dir().join(format!(
        "qsr-podman-event-json-red-{name}-{}-{nanos}-{unique_id}",
        std::process::id()
    ))
}

fn policy() -> IsolationPolicy {
    IsolationPolicy {
        policy_id: "podman_event_json_contract_red_v1".to_owned(),
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
        request_id: "podman_event_json_contract_red".to_owned(),
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

fn write_fake_podman(log: &Path, network_name_file: &Path) -> PathBuf {
    let program = temporary_path("fake-podman");
    let info = r#"{"host":{"security":{"rootless":true,"seccompEnabled":true,"seccompProfilePath":"/usr/share/containers/seccomp.json","apparmorEnabled":true,"selinuxEnabled":false}}}"#;
    let script = format!(
        r#"#!/bin/sh
set -eu
printf '%s\n' "$*" >> '{log}'
network_name_file='{network_name_file}'
network_id='{network_id}'
if [ "${{1:-}}" = info ]; then
  if [ "${{3:-}}" = json ]; then printf '%s\n' '{info}'; else printf 'true\n'; fi
  exit 0
fi
case "${{1:-}}:${{2:-}}" in
  network:create)
    network_name=${{5:-}}
    [ -n "$network_name" ] || exit 94
    printf '%s\n' "$network_name" > "$network_name_file"
    printf '%s\n' "$network_name"
    ;;
  events:--stream=false)
    network_name=$(cat "$network_name_file")
    printf '{{"ID":"%s","network":"%s","Status":"create","Type":"network"}}\n' "$network_id" "$network_name"
    ;;
  network:inspect)
    network_name=$(cat "$network_name_file")
    [ "${{5:-}}" = "$network_id" ] || exit 95
    printf '[{{"name":"%s","id":"%s","internal":true,"dns_enabled":false}}]\n' "$network_name" "$network_id"
    ;;
  create:--name)
    exit 41
    ;;
  network:rm)
    [ "${{3:-}}" = "$network_id" ] || exit 96
    ;;
  *) exit 91 ;;
esac
"#,
        log = log.display(),
        network_name_file = network_name_file.display(),
        network_id = NETWORK_ID,
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
fn podman_v6_lowercase_network_json_reaches_exact_id_boundary() {
    let log = temporary_path("calls");
    let network_name_file = temporary_path("network-name");
    let program = write_fake_podman(&log, &network_name_file);
    let adapter = RootlessPodmanAdapter::new(program.clone());

    let result = adapter.launch_at(&request(), &policy(), STARTED_AT_EPOCH_SECONDS);
    let calls = fs::read_to_string(&log).expect("fake Podman calls must be recorded");

    let _ = fs::remove_file(program);
    let _ = fs::remove_file(log);
    let _ = fs::remove_file(network_name_file);

    assert_eq!(
        result,
        Err(ApplicationServiceError::BackendCommandFailed {
            operation: "container_create",
        }),
        "real Podman v6 event JSON must be admitted before the controlled container-create failure"
    );

    let lines: Vec<&str> = calls.lines().collect();
    let create_index = lines
        .iter()
        .position(|line| line.starts_with("network create --internal --disable-dns qsr-net-"))
        .expect("network creation must be exercised");
    let created_name = lines[create_index]
        .split_whitespace()
        .last()
        .expect("network creation must carry a generated correlation name");
    let receipt_index = lines
        .iter()
        .position(|line| line.starts_with("events --stream=false "))
        .expect("creation history must be queried");
    let exact_id_inspect = format!("network inspect --format json {NETWORK_ID}");
    let inspect_index = lines
        .iter()
        .position(|line| *line == exact_id_inspect)
        .expect("lowercase network JSON must admit the exact creation-bound ID");
    let container_create_index = lines
        .iter()
        .position(|line| line.starts_with("create --name "))
        .expect("the controlled downstream failure must be reached");
    let public_name_inspect = format!("network inspect --format json {created_name}");

    assert!(
        create_index < receipt_index
            && receipt_index < inspect_index
            && inspect_index < container_create_index,
        "authority must flow create -> real JSON receipt -> exact-ID P0 -> container create; calls were:\n{calls}"
    );
    assert!(
        !lines.iter().any(|line| **line == public_name_inspect),
        "real JSON compatibility must not restore public-name identity authority; calls were:\n{calls}"
    );
    assert!(
        lines
            .iter()
            .any(|line| **line == format!("network rm {NETWORK_ID}")),
        "controlled downstream failure must retain exact-ID non-force cleanup; calls were:\n{calls}"
    );
}
