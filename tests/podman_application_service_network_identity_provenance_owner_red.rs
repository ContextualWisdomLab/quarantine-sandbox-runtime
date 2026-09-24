//! RED: network identity admission must bind creation authority to P0 state.
//!
//! A canonical Podman network ID is necessary but not sufficient ownership evidence. The ID must
//! come from the invocation-local creation receipt, then the exact ID must identify the expected
//! generated network and preserve the deny-by-default state requested at creation before container
//! creation. Public `qsr-net-*` correlation is never private attachment or cleanup authority.

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

const STARTED_AT_EPOCH_SECONDS: u64 = 1_780_004_000;
const OWNED_NETWORK_ID: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";

static NEXT_TEMP_PATH_ID: AtomicU64 = AtomicU64::new(0);

#[derive(Clone, Copy)]
enum IdentityContradiction {
    WrongName,
    ExternalNetwork,
    DnsEnabled,
}

fn temporary_path(name: &str) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("test clock must be after the Unix epoch")
        .as_nanos();
    let unique_id = NEXT_TEMP_PATH_ID.fetch_add(1, Ordering::Relaxed);
    std::env::temp_dir().join(format!(
        "qsr-network-identity-provenance-red-{name}-{}-{nanos}-{unique_id}",
        std::process::id()
    ))
}

fn policy() -> IsolationPolicy {
    IsolationPolicy {
        policy_id: "network_identity_provenance_red_v1".to_owned(),
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
        request_id: "network_identity_provenance_red".to_owned(),
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

fn write_fake_podman(
    contradiction: IdentityContradiction,
    log: &Path,
    created_network_name: &Path,
    container_create_marker: &Path,
) -> PathBuf {
    let program = temporary_path("fake-podman");
    let info = r#"{"host":{"security":{"rootless":true,"seccompEnabled":true,"seccompProfilePath":"/usr/share/containers/seccomp.json","apparmorEnabled":true,"selinuxEnabled":false}}}"#;
    let (reported_name_assignment, internal, dns_enabled) = match contradiction {
        IdentityContradiction::WrongName => ("reported_name='qsr-net-foreign'", "true", "false"),
        IdentityContradiction::ExternalNetwork => {
            ("reported_name=\"$network_name\"", "false", "false")
        }
        IdentityContradiction::DnsEnabled => ("reported_name=\"$network_name\"", "true", "true"),
    };
    let script = format!(
        r#"#!/bin/sh
set -eu
printf '%s\n' "$*" >> '{log}'
network_name_file='{network_name_file}'
container_create_marker='{container_create_marker}'
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
  events:*)
    network_name=$(cat "$network_name_file")
    printf '{{"ID":"%s","Network":"%s","Status":"create","Time":"2026-09-22T13:30:00Z","Type":"network"}}\n' "$network_id" "$network_name"
    ;;
  network:inspect)
    network_name=$(cat "$network_name_file")
    selector=${{5:-}}
    [ "$selector" = "$network_id" ] || exit 95
    {reported_name_assignment}
    internal='{internal}'
    dns_enabled='{dns_enabled}'
    printf '[{{"name":"%s","id":"%s","internal":%s,"dns_enabled":%s,"containers":{{}}}}]\n' "$reported_name" "$network_id" "$internal" "$dns_enabled"
    ;;
  create:--name)
    printf 'consumer-container-create-reached\n' > "$container_create_marker"
    exit 97
    ;;
  network:rm) : ;;
  *) exit 91 ;;
esac
"#,
        log = log.display(),
        network_name_file = created_network_name.display(),
        container_create_marker = container_create_marker.display(),
        network_id = OWNED_NETWORK_ID,
        info = info,
        reported_name_assignment = reported_name_assignment,
        internal = internal,
        dns_enabled = dns_enabled,
    );

    fs::write(&program, script).expect("fake Podman must be writable");
    let mut permissions = fs::metadata(&program)
        .expect("fake Podman metadata must exist")
        .permissions();
    permissions.set_mode(0o700);
    fs::set_permissions(&program, permissions).expect("fake Podman must be executable");
    program
}

fn assert_identity_contradiction_stops_before_create(contradiction: IdentityContradiction) {
    let log = temporary_path("calls");
    let created_network_name = temporary_path("network-name");
    let container_create_marker = temporary_path("container-create-marker");
    let program = write_fake_podman(
        contradiction,
        &log,
        &created_network_name,
        &container_create_marker,
    );
    let adapter = RootlessPodmanAdapter::new(program.clone());

    let result = adapter.launch_at(&request(), &policy(), STARTED_AT_EPOCH_SECONDS);
    let calls = fs::read_to_string(&log).expect("fake Podman calls must be recorded");
    let container_create_reached = container_create_marker.exists();

    let _ = fs::remove_file(program);
    let _ = fs::remove_file(log);
    let _ = fs::remove_file(created_network_name);
    let _ = fs::remove_file(container_create_marker);

    let lines: Vec<&str> = calls.lines().collect();
    let network_create_index = lines
        .iter()
        .position(|line| line.starts_with("network create --internal --disable-dns qsr-net-"))
        .expect("the provenance witness must reach runtime-owned network creation");
    let network_name = lines[network_create_index]
        .split_whitespace()
        .last()
        .expect("network creation must include the exact correlation name");
    let public_name_inspect = format!("network inspect --format json {network_name}");
    let exact_id_inspect = format!("network inspect --format json {OWNED_NETWORK_ID}");

    let event_indices: Vec<usize> = lines
        .iter()
        .enumerate()
        .filter_map(|(index, line)| line.starts_with("events ").then_some(index))
        .collect();
    assert_eq!(
        event_indices.len(),
        1,
        "network identity provenance must consume exactly one creation receipt before P0 inspection; calls were:\n{calls}"
    );
    assert!(
        network_create_index < event_indices[0],
        "creation receipt must be queried only after network creation; calls were:\n{calls}"
    );

    let exact_id_inspect_indices: Vec<usize> = lines
        .iter()
        .enumerate()
        .filter_map(|(index, line)| (*line == exact_id_inspect).then_some(index))
        .collect();
    assert_eq!(
        exact_id_inspect_indices.len(),
        1,
        "P0 provenance must inspect exactly the immutable ID admitted by the creation receipt; calls were:\n{calls}"
    );
    assert!(
        event_indices[0] < exact_id_inspect_indices[0],
        "creation receipt must establish immutable identity before exact-ID P0 inspection; calls were:\n{calls}"
    );
    assert!(
        !lines.iter().any(|line| **line == public_name_inspect),
        "public network correlation must not mint or re-resolve private authority; calls were:\n{calls}"
    );
    assert!(
        result.is_err(),
        "contradictory network provenance/P0 evidence must fail closed"
    );
    assert!(
        !container_create_reached,
        "a creation-bound ID must not reach container creation when name/P0 evidence contradicts the created object; calls were:\n{calls}"
    );
    assert!(
        !calls.lines().any(|line| line.starts_with("create --name ")),
        "container creation must not be reached before exact-ID network provenance is admitted; calls were:\n{calls}"
    );
}

#[test]
fn network_identity_requires_expected_name_before_container_create() {
    assert_identity_contradiction_stops_before_create(IdentityContradiction::WrongName);
}

#[test]
fn network_identity_requires_internal_state_before_container_create() {
    assert_identity_contradiction_stops_before_create(IdentityContradiction::ExternalNetwork);
}

#[test]
fn network_identity_requires_dns_disabled_before_container_create() {
    assert_identity_contradiction_stops_before_create(IdentityContradiction::DnsEnabled);
}
