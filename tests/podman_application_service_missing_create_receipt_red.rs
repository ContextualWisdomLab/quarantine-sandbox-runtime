//! RED: malformed successful service create without a trustworthy receipt must fail closed.
//!
//! The generated `qsr-app-*` name is correlation metadata, not destructive authority. If create
//! succeeds but neither stdout nor a runtime-owned receipt yields an admitted exact container ID,
//! the runtime must report the unreconciled receipt condition without attempting name-based rm.
//! Network prerequisites use creation-bound exact-ID authority.

#![cfg(target_os = "linux")]

use std::{
    fs,
    os::unix::fs::PermissionsExt,
    path::PathBuf,
    sync::atomic::{AtomicU64, Ordering},
    time::{SystemTime, UNIX_EPOCH},
};

use quarantine_sandbox_runtime::{
    ApplicationServiceError, ApplicationServiceRequest, IsolationPolicy, ResourceRequest,
    RootlessPodmanAdapter, ServiceProtocol,
};

static NEXT_TEMP_PATH_ID: AtomicU64 = AtomicU64::new(0);
const OWNED_NETWORK_ID: &str = "abcdef0123456789abcdef0123456789abcdef0123456789abcdef0123456789";

fn temporary_path(name: &str) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("test clock must be after the Unix epoch")
        .as_nanos();
    let unique_id = NEXT_TEMP_PATH_ID.fetch_add(1, Ordering::Relaxed);
    std::env::temp_dir().join(format!(
        "qsr-application-service-missing-create-receipt-red-{name}-{}-{nanos}-{unique_id}",
        std::process::id()
    ))
}

fn policy() -> IsolationPolicy {
    IsolationPolicy {
        policy_id: "application_service_missing_create_receipt_red_v1".to_owned(),
        maximum_memory_bytes: 256 * 1024 * 1024,
        maximum_cpu_millicores: 500,
        maximum_processes: 32,
        maximum_lease_seconds: 60,
        maximum_tmpfs_bytes: 32 * 1024 * 1024,
        readiness_timeout_millis: 250,
        readiness_poll_interval_millis: 10,
        shutdown_grace_seconds: 1,
        run_as_user_id: 65_532,
        run_as_group_id: 65_532,
    }
}

fn request() -> ApplicationServiceRequest {
    ApplicationServiceRequest {
        schema_version: "1.0.0".to_owned(),
        request_id: "application_service_missing_create_receipt_red".to_owned(),
        image_reference: format!("localhost/cwl/tool@sha256:{}", "f".repeat(64)),
        container_port: 8_080,
        protocol: ServiceProtocol::Http,
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

fn write_fake_podman() -> (PathBuf, PathBuf, PathBuf) {
    let program = temporary_path("fake-podman");
    let log = temporary_path("calls");
    let destructive_marker = temporary_path("destructive-container-action");
    let info = r#"{"host":{"security":{"rootless":true,"seccompEnabled":true,"seccompProfilePath":"/usr/share/containers/seccomp.json","apparmorEnabled":true,"selinuxEnabled":false}}}"#;
    let script = format!(
        r#"#!/bin/sh
set -eu
printf '%s\n' "$*" >> '{log}'
destructive_marker='{destructive_marker}'
owned_network_id='{network_id}'
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
    printf '{{"ID":"%s","Network":"%s","Status":"create","Type":"network"}}\n' "$owned_network_id" "$created_name"
    ;;
  network:inspect)
    [ "${{5:-}}" = "$owned_network_id" ] || exit 95
    created_name=$(awk '$1 == "network" && $2 == "create" {{ name=$NF }} END {{ print name }}' '{log}')
    printf '[{{"name":"%s","id":"%s","internal":true,"dns_enabled":false,"containers":{{}}}}]\n' "$created_name" "$owned_network_id"
    ;;
  network:rm) : ;;
  create:--name) printf 'bad identifier with spaces\n' ;;
  rm:--force) printf 'unexpected destructive container action\n' > "$destructive_marker"; exit 93 ;;
  start:*|container:inspect|top:*|port:*) exit 94 ;;
  *) exit 91 ;;
esac
"#,
        log = log.display(),
        destructive_marker = destructive_marker.display(),
        network_id = OWNED_NETWORK_ID,
        info = info,
    );
    fs::write(&program, script).expect("fake Podman must be writable");
    let mut permissions = fs::metadata(&program)
        .expect("fake Podman metadata must exist")
        .permissions();
    permissions.set_mode(0o700);
    fs::set_permissions(&program, permissions).expect("fake Podman must be executable");
    (program, log, destructive_marker)
}

#[test]
fn malformed_successful_create_without_receipt_never_uses_generated_name_for_cleanup() {
    let (program, log, destructive_marker) = write_fake_podman();
    let adapter = RootlessPodmanAdapter::new(program.clone());

    let result = adapter.launch_at(&request(), &policy(), 1_780_001_200);
    assert_eq!(
        result,
        Err(ApplicationServiceError::MalformedIsolationInspection {
            operation: "container_create_receipt",
        }),
        "missing trustworthy create receipt must remain an explicit unreconciled-identity error"
    );

    let calls = fs::read_to_string(&log).expect("fake Podman calls must be recorded");
    assert!(
        calls
            .lines()
            .any(|line| line.starts_with("events --stream=false ")),
        "the missing container receipt witness must cross creation-bound network admission; calls were:\n{calls}"
    );
    assert!(
        calls
            .lines()
            .any(|line| line == format!("network inspect --format json {OWNED_NETWORK_ID}")),
        "network P0 evidence must use exact admitted identity before container creation; calls were:\n{calls}"
    );
    let create_call = calls
        .lines()
        .find(|line| line.starts_with("create --name "))
        .expect("service create must execute");
    assert!(
        create_call
            .split_whitespace()
            .any(|argument| argument.starts_with("--cidfile=")),
        "service create must provision a private runtime-owned cidfile; calls were:\n{calls}"
    );
    assert!(
        !destructive_marker.exists(),
        "missing receipt must never authorize generated-name or guessed container cleanup"
    );
    assert!(
        !calls.lines().any(|line| {
            line.starts_with("start ")
                || line.starts_with("container inspect ")
                || line.starts_with("top ")
                || line.starts_with("port ")
        }),
        "unreconciled create identity must fail before post-create lifecycle; calls were:\n{calls}"
    );

    let _ = fs::remove_file(program);
    let _ = fs::remove_file(log);
    let _ = fs::remove_file(destructive_marker);
}
