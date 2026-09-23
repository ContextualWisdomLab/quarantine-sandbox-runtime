//! RED: an event-selected network candidate is not destructive authority before P0 admission.
//!
//! Creation-event correlation can nominate a canonical backend ID for inspection, but contradiction
//! or inspection failure is evidence to reject that candidate, not evidence that the runtime owns it.
//! Pre-admission candidates must never be removed; durable orphan reconciliation belongs to the
//! recovery owner after exact-ID corroboration against private intent.

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

const STARTED_AT_EPOCH_SECONDS: u64 = 1_780_006_000;
const CANDIDATE_NETWORK_ID: &str = "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";

static NEXT_TEMP_PATH_ID: AtomicU64 = AtomicU64::new(0);

#[derive(Clone, Copy)]
enum CandidateContradiction {
    InspectFailure,
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
        "qsr-network-pre-admission-cleanup-red-{name}-{}-{nanos}-{unique_id}",
        std::process::id()
    ))
}

fn policy() -> IsolationPolicy {
    IsolationPolicy {
        policy_id: "network_pre_admission_cleanup_red_v1".to_owned(),
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
        request_id: "network_pre_admission_cleanup_red".to_owned(),
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
    contradiction: CandidateContradiction,
    log: &Path,
    created_network_name: &Path,
    container_create_marker: &Path,
) -> PathBuf {
    let program = temporary_path("fake-podman");
    let info = r#"{"host":{"security":{"rootless":true,"seccompEnabled":true,"seccompProfilePath":"/usr/share/containers/seccomp.json","apparmorEnabled":true,"selinuxEnabled":false}}}"#;
    let (inspect_exit, reported_name_assignment, internal, dns_enabled) = match contradiction {
        CandidateContradiction::InspectFailure => {
            ("exit 95", "reported_name=\"$network_name\"", "true", "false")
        }
        CandidateContradiction::WrongName => {
            (":", "reported_name='qsr-net-foreign'", "true", "false")
        }
        CandidateContradiction::ExternalNetwork => {
            (":", "reported_name=\"$network_name\"", "false", "false")
        }
        CandidateContradiction::DnsEnabled => {
            (":", "reported_name=\"$network_name\"", "true", "true")
        }
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
    printf '{{"ID":"%s","Network":"%s","Status":"create","Type":"network"}}\n' "$network_id" "$network_name"
    ;;
  network:inspect)
    network_name=$(cat "$network_name_file")
    selector=${{5:-}}
    [ "$selector" = "$network_id" ] || exit 96
    {inspect_exit}
    {reported_name_assignment}
    internal='{internal}'
    dns_enabled='{dns_enabled}'
    printf '[{{"name":"%s","id":"%s","internal":%s,"dns_enabled":%s,"containers":{{}}}}]\n' "$reported_name" "$network_id" "$internal" "$dns_enabled"
    ;;
  network:rm) : ;;
  create:--name)
    printf 'container-create-reached\n' > "$container_create_marker"
    exit 97
    ;;
  *) exit 91 ;;
esac
"#,
        log = log.display(),
        network_name_file = created_network_name.display(),
        container_create_marker = container_create_marker.display(),
        network_id = CANDIDATE_NETWORK_ID,
        info = info,
        inspect_exit = inspect_exit,
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

fn assert_unadmitted_candidate_is_not_removed(contradiction: CandidateContradiction) {
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

    assert!(result.is_err(), "untrusted P0 candidate must fail closed");
    assert!(
        calls
            .lines()
            .any(|line| line == format!("network inspect --format json {CANDIDATE_NETWORK_ID}")),
        "the candidate must be inspected by exact canonical ID before rejection; calls were:\n{calls}"
    );
    assert!(
        !container_create_reached && !calls.lines().any(|line| line.starts_with("create --name ")),
        "container creation must not begin before network ownership admission; calls were:\n{calls}"
    );
    assert!(
        !calls
            .lines()
            .any(|line| line == "network rm" || line.starts_with("network rm ")),
        "a pre-admission rejection must not authorize network removal through any selector; calls were:\n{calls}"
    );
}

#[test]
fn network_inspect_failure_does_not_authorize_candidate_cleanup() {
    assert_unadmitted_candidate_is_not_removed(CandidateContradiction::InspectFailure);
}

#[test]
fn wrong_network_name_does_not_authorize_candidate_cleanup() {
    assert_unadmitted_candidate_is_not_removed(CandidateContradiction::WrongName);
}

#[test]
fn external_network_state_does_not_authorize_candidate_cleanup() {
    assert_unadmitted_candidate_is_not_removed(CandidateContradiction::ExternalNetwork);
}

#[test]
fn dns_enabled_network_state_does_not_authorize_candidate_cleanup() {
    assert_unadmitted_candidate_is_not_removed(CandidateContradiction::DnsEnabled);
}
