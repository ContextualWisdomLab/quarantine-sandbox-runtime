//! RED: partial application-service launch cleanup must retain acquired network authority.
//!
//! Once the runtime has admitted the Podman network ID from this invocation's creation receipt,
//! later failure cleanup must not fall back to the generated `qsr-net-*` correlation name. A name
//! can be rebound after admission; destructive cleanup must target the admitted ID without
//! network-level `--force`, preserving an unrelated network that later owns the correlation name.

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

const STARTED_AT_EPOCH_SECONDS: u64 = 1_780_005_000;
const OWNED_NETWORK_ID: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";

static NEXT_TEMP_PATH_ID: AtomicU64 = AtomicU64::new(0);

fn temporary_path(name: &str) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("test clock must be after the Unix epoch")
        .as_nanos();
    let unique_id = NEXT_TEMP_PATH_ID.fetch_add(1, Ordering::Relaxed);
    std::env::temp_dir().join(format!(
        "qsr-network-partial-cleanup-owner-red-{name}-{}-{nanos}-{unique_id}",
        std::process::id()
    ))
}

fn policy() -> IsolationPolicy {
    IsolationPolicy {
        policy_id: "network_partial_cleanup_owner_red_v1".to_owned(),
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
        request_id: "network_partial_cleanup_owner_red".to_owned(),
        image_reference: format!("localhost/cwl/tool@sha256:{}", "e".repeat(64)),
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

fn write_fake_podman(log: &Path, created_network_name: &Path, foreign_marker: &Path) -> PathBuf {
    let program = temporary_path("fake-podman");
    let info = r#"{"host":{"security":{"rootless":true,"seccompEnabled":true,"seccompProfilePath":"/usr/share/containers/seccomp.json","apparmorEnabled":true,"selinuxEnabled":false}}}"#;
    let script = format!(
        r#"#!/bin/sh
set -eu
printf '%s\n' "$*" >> '{log}'
network_name_file='{network_name_file}'
foreign_marker='{foreign_marker}'
network_id='{network_id}'
if [ "${{1:-}}" = info ]; then
  if [ "${{3:-}}" = json ]; then printf '%s\n' '{info}'; else printf 'true\n'; fi
  exit 0
fi
case "${{1:-}}:${{2:-}}" in
  network:create)
    network_name=''
    for argument in "$@"; do network_name="$argument"; done
    [ -n "$network_name" ] || exit 94
    printf '%s\n' "$network_name" > "$network_name_file"
    printf '%s\n' "$network_name"
    ;;
  events:--stream=false)
    network_name=$(cat "$network_name_file")
    printf '{{"ID":"%s","Network":"%s","Status":"create","Type":"network"}}\n' "$network_id" "$network_name"
    ;;
  network:inspect)
    network_name=$(cat "$network_name_file")
    [ "${{5:-}}" = "$network_id" ] || exit 95
    printf '[{{"name":"%s","id":"%s","internal":true,"dns_enabled":false,"containers":{{}}}}]\n' "$network_name" "$network_id"
    ;;
  create:--name)
    network=''
    previous=''
    for argument in "$@"; do
      if [ "$previous" = '--network' ]; then network="$argument"; fi
      previous="$argument"
    done
    [ "$network" = "$network_id" ] || exit 92
    printf 'safe\n' > "$foreign_marker"
    exit 41
    ;;
  network:rm)
    network_name=$(cat "$network_name_file")
    if [ "${{3:-}}" = '--force' ] && [ "${{4:-}}" = "$network_name" ]; then
      printf 'deleted-by-generated-name-force\n' > "$foreign_marker"
      exit 0
    fi
    if [ "${{3:-}}" = "$network_id" ] && [ "${{4:-}}" = '' ]; then
      exit 0
    fi
    exit 98
    ;;
  *) exit 91 ;;
esac
"#,
        log = log.display(),
        network_name_file = created_network_name.display(),
        foreign_marker = foreign_marker.display(),
        network_id = OWNED_NETWORK_ID,
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
fn failed_container_create_cleans_network_by_acquired_id_without_force() {
    let log = temporary_path("calls");
    let created_network_name = temporary_path("network-name");
    let foreign_marker = temporary_path("foreign-network");
    let program = write_fake_podman(&log, &created_network_name, &foreign_marker);
    let adapter = RootlessPodmanAdapter::new(program.clone());

    let result = adapter.launch_at(&request(), &policy(), STARTED_AT_EPOCH_SECONDS);
    let calls = fs::read_to_string(&log).expect("fake Podman calls must be recorded");
    let foreign_state = fs::read_to_string(&foreign_marker)
        .expect("foreign network marker must survive partial-launch cleanup");

    let _ = fs::remove_file(program);
    let _ = fs::remove_file(log);
    let _ = fs::remove_file(created_network_name);
    let _ = fs::remove_file(foreign_marker);

    assert_eq!(
        result,
        Err(ApplicationServiceError::BackendCommandFailed {
            operation: "container_create",
        }),
        "successful exact-authority cleanup must preserve the original container-create failure"
    );
    assert_eq!(
        foreign_state, "safe\n",
        "partial-launch cleanup must not delete a network that later owns the generated correlation name"
    );

    let lines: Vec<&str> = calls.lines().collect();
    let network_create_index = lines
        .iter()
        .position(|line| line.starts_with("network create --internal --disable-dns qsr-net-"))
        .expect("runtime-owned network must be created");
    let created_name = lines[network_create_index]
        .split_whitespace()
        .last()
        .expect("network create must include the generated correlation name");
    let receipt_index = lines
        .iter()
        .position(|line| line.starts_with("events --stream=false "))
        .expect("partial-cleanup witness must admit creation-bound network authority");
    let exact_id_inspect = format!("network inspect --format json {OWNED_NETWORK_ID}");
    let identity_inspect_index = lines
        .iter()
        .position(|line| *line == exact_id_inspect)
        .expect("P0 state must be inspected by exact admitted network ID");
    let public_name_inspect = format!("network inspect --format json {created_name}");
    let container_create_index = lines
        .iter()
        .position(|line| line.starts_with("create --name "))
        .expect("container create must be attempted");

    assert!(
        network_create_index < receipt_index
            && receipt_index < identity_inspect_index
            && identity_inspect_index < container_create_index,
        "admitted network authority must precede the failing container create; calls were:\n{calls}"
    );
    assert!(
        !lines.iter().any(|line| **line == public_name_inspect),
        "partial cleanup must not depend on mutable public-name identity; calls were:\n{calls}"
    );
    assert!(
        lines[container_create_index].contains(&format!(" --network {OWNED_NETWORK_ID} ")),
        "the failing container create must already be bound to admitted network identity; call was: {}",
        lines[container_create_index]
    );
    assert!(
        lines
            .iter()
            .any(|line| **line == format!("network rm {OWNED_NETWORK_ID}")),
        "partial-launch cleanup must target the exact admitted network ID without force; calls were:\n{calls}"
    );
    assert!(
        !lines
            .iter()
            .any(|line| line.starts_with("network rm --force ")),
        "partial-launch cleanup must never force-remove a generated network name; calls were:\n{calls}"
    );
    assert!(
        !lines.iter().any(|line| **line == format!("network rm {created_name}")),
        "public correlation must never become destructive cleanup authority; calls were:\n{calls}"
    );
}
