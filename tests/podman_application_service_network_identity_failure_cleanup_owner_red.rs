//! RED: failed network-ID admission must not authorize correlation-name destruction.
//!
//! Once network creation succeeds, a malformed creation receipt means the runtime still has no
//! immutable backend authority. The generated `qsr-net-*` value remains correlation data and must
//! not be promoted to inspection, attachment, or destructive cleanup authority. This witness keeps
//! the failure taxonomy open while requiring fail-closed launch with no private action after the
//! malformed receipt.

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
static NEXT_TEMP_PATH_ID: AtomicU64 = AtomicU64::new(0);

fn temporary_path(name: &str) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("test clock must be after the Unix epoch")
        .as_nanos();
    let unique_id = NEXT_TEMP_PATH_ID.fetch_add(1, Ordering::Relaxed);
    std::env::temp_dir().join(format!(
        "qsr-network-id-failure-cleanup-red-{name}-{}-{nanos}-{unique_id}",
        std::process::id()
    ))
}

fn policy() -> IsolationPolicy {
    IsolationPolicy {
        policy_id: "network_id_failure_cleanup_owner_red_v1".to_owned(),
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
        request_id: "network_id_failure_cleanup_owner_red".to_owned(),
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

fn write_fake_podman(log: &Path, created_network_name: &Path) -> PathBuf {
    let program = temporary_path("fake-podman");
    let info = r#"{"host":{"security":{"rootless":true,"seccompEnabled":true,"seccompProfilePath":"/usr/share/containers/seccomp.json","apparmorEnabled":true,"selinuxEnabled":false}}}"#;
    let script = format!(
        r#"#!/bin/sh
set -eu
printf '%s\n' "$*" >> '{log}'
network_name_file='{network_name_file}'
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
    printf '{{"ID":"not-a-canonical-runtime-id","Network":"%s","Status":"create","Type":"network"}}\n' "$network_name"
    ;;
  network:inspect|network:rm|create:--name)
    exit 97
    ;;
  *)
    exit 91
    ;;
esac
"#,
        log = log.display(),
        network_name_file = created_network_name.display(),
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
fn malformed_network_identity_never_authorizes_correlation_name_cleanup() {
    let log = temporary_path("calls");
    let created_network_name = temporary_path("network-name");
    let program = write_fake_podman(&log, &created_network_name);
    let adapter = RootlessPodmanAdapter::new(program.clone());

    let result = adapter.launch_at(&request(), &policy(), STARTED_AT_EPOCH_SECONDS);
    let calls = fs::read_to_string(&log).expect("fake Podman calls must be recorded");

    let _ = fs::remove_file(program);
    let _ = fs::remove_file(log);
    let _ = fs::remove_file(created_network_name);

    assert!(
        result.is_err(),
        "malformed creation-bound identity must fail closed rather than publish a lease"
    );
    assert!(
        calls
            .lines()
            .any(|line| line.starts_with("network create --internal --disable-dns qsr-net-")),
        "the witness must reach network creation; calls were:\n{calls}"
    );
    assert!(
        calls
            .lines()
            .any(|line| line.starts_with("events --stream=false ")),
        "the witness must reach creation-receipt admission; calls were:\n{calls}"
    );
    assert!(
        !calls
            .lines()
            .any(|line| line.starts_with("network inspect ")),
        "malformed receipt identity must stop before any network inspection; calls were:\n{calls}"
    );
    assert!(
        !calls.lines().any(|line| line.starts_with("create --name ")),
        "container creation must not begin without admitted network identity; calls were:\n{calls}"
    );
    assert!(
        !calls.lines().any(|line| line.starts_with("network rm ")),
        "public correlation must not become destructive authority when no canonical ID is admitted; calls were:\n{calls}"
    );
}
