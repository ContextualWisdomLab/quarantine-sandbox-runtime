//! Succession regression for the executed create-to-public-name-inspect TOCTOU RED.
//!
//! The historical RED proved that a later lookup of the generated `qsr-net-*` name could
//! promote a same-name replacement into private attachment and cleanup authority. The owner
//! now acquires the immutable ID from bounded creation history and performs P0 inspection by
//! that exact ID. This fixture preserves the original replacement threat while exercising the
//! repaired call shape so the old witness cannot mask later owner-path evidence checks.

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
const CREATED_NETWORK_ID: &str =
    "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
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

fn write_fake_podman(log: &Path, network_selector_marker: &Path) -> PathBuf {
    let program = temporary_path("fake-podman");
    let info = r#"{"host":{"security":{"rootless":true,"seccompEnabled":true,"seccompProfilePath":"/usr/share/containers/seccomp.json","apparmorEnabled":true,"selinuxEnabled":false}}}"#;
    let script = format!(
        r#"#!/bin/sh
set -eu
printf '%s\n' "$*" >> '{log}'
network_selector_marker='{network_selector_marker}'
created_network_id='{created_network_id}'
replacement_network_id='{replacement_network_id}'
if [ "${{1:-}}" = info ]; then
  if [ "${{3:-}}" = json ]; then printf '%s\n' '{info}'; else printf 'true\n'; fi
  exit 0
fi
case "${{1:-}}:${{2:-}}" in
  network:create)
    printf '%s\n' "${{5:-}}"
    ;;
  events:--stream=false)
    created_name=$(awk '$1 == "network" && $2 == "create" {{ name=$NF }} END {{ print name }}' '{log}')
    printf '{{"ID":"%s","Network":"%s","Status":"create","Type":"network"}}\n' "$created_network_id" "$created_name"
    ;;
  network:inspect)
    selector=${{5:-}}
    created_name=$(awk '$1 == "network" && $2 == "create" {{ name=$NF }} END {{ print name }}' '{log}')
    if [ "$selector" = "$created_network_id" ]; then
      printf '[{{"name":"%s","id":"%s","internal":true,"dns_enabled":false,"containers":{{}}}}]\n' "$created_name" "$created_network_id"
    else
      printf '[{{"name":"%s","id":"%s","internal":true,"dns_enabled":false,"containers":{{}}}}]\n' "$selector" "$replacement_network_id"
    fi
    ;;
  create:--name)
    previous=''
    selected=''
    for argument in "$@"; do
      if [ "$previous" = '--network' ]; then selected=$argument; break; fi
      previous=$argument
    done
    printf '%s\n' "$selected" > "$network_selector_marker"
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
        network_selector_marker = network_selector_marker.display(),
        created_network_id = CREATED_NETWORK_ID,
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
fn creation_bound_id_prevents_same_name_replacement_promotion() {
    let log = temporary_path("calls");
    let network_selector_marker = temporary_path("network-selector");
    let program = write_fake_podman(&log, &network_selector_marker);
    let adapter = RootlessPodmanAdapter::new(program.clone());

    let result = adapter.launch_at(&request(), &policy(), STARTED_AT_EPOCH_SECONDS);
    let calls = fs::read_to_string(&log).expect("fake Podman calls must be recorded");
    let selected_network = fs::read_to_string(&network_selector_marker).ok();

    let _ = fs::remove_file(program);
    let _ = fs::remove_file(log);
    let _ = fs::remove_file(network_selector_marker);

    let lines: Vec<&str> = calls.lines().collect();
    let network_create_index = lines
        .iter()
        .position(|line| line.starts_with("network create --internal --disable-dns qsr-net-"))
        .expect("the witness must reach network creation");
    let created_name = lines[network_create_index]
        .split_whitespace()
        .last()
        .expect("network creation must include a generated correlation name");
    let event_index = lines
        .iter()
        .position(|line| line.starts_with("events --stream=false "))
        .expect("creation-bound authority must be acquired from bounded event history");
    let exact_id_inspect = format!("network inspect --format json {CREATED_NETWORK_ID}");
    let exact_id_inspect_index = lines
        .iter()
        .position(|line| *line == exact_id_inspect)
        .expect("P0 network state must be inspected through the creation-bound ID");
    let public_name_lookup = format!("network inspect --format json {created_name}");

    assert!(
        network_create_index < event_index && event_index < exact_id_inspect_index,
        "authority must flow create -> creation receipt -> exact-ID P0 inspection; calls were:\n{calls}"
    );
    assert!(
        !lines.iter().any(|line| *line == public_name_lookup),
        "the mutable public correlation must not be re-resolved into private authority; calls were:\n{calls}"
    );
    assert!(result.is_err(), "the controlled container-create failure must surface");
    assert_eq!(
        selected_network.as_deref().map(str::trim),
        Some(CREATED_NETWORK_ID),
        "container creation must bind the creation-bound network ID"
    );
    assert!(
        lines
            .iter()
            .any(|line| *line == format!("network rm {CREATED_NETWORK_ID}")),
        "partial cleanup must retain creation-bound exact-ID authority; calls were:\n{calls}"
    );
    assert!(
        !lines
            .iter()
            .any(|line| *line == format!("network rm {REPLACEMENT_NETWORK_ID}")),
        "a same-name replacement must never become destructive authority; calls were:\n{calls}"
    );
    assert!(
        !lines
            .iter()
            .any(|line| *line == format!("network rm {created_name}")),
        "public correlation must never become destructive authority; calls were:\n{calls}"
    );
}
