//! Regression: `podman create` output used as lifecycle authority must be a full long ID.
//!
//! Podman names and short IDs are valid human-facing selectors, but they are mutable or
//! ambiguous compared with the full container ID returned by a successful create. The
//! application-service adapter must reject name-like, short, and non-hex create output
//! before any post-create lifecycle operation can treat it as destructive authority.

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

fn temporary_path(name: &str) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("test clock must be after the Unix epoch")
        .as_nanos();
    let unique_id = NEXT_TEMP_PATH_ID.fetch_add(1, Ordering::Relaxed);
    std::env::temp_dir().join(format!(
        "qsr-application-service-create-identifier-red-{name}-{}-{nanos}-{unique_id}",
        std::process::id()
    ))
}

fn policy() -> IsolationPolicy {
    IsolationPolicy {
        policy_id: "application_service_create_identifier_red_v1".to_owned(),
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

fn request(case_name: &str) -> ApplicationServiceRequest {
    ApplicationServiceRequest {
        schema_version: "1.0.0".to_owned(),
        request_id: format!("application_service_create_identifier_red_{case_name}"),
        image_reference: format!("localhost/cwl/tool@sha256:{}", "e".repeat(64)),
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

fn write_fake_podman(
    create_identifier: &str,
    lifecycle_marker: &PathBuf,
) -> (PathBuf, PathBuf) {
    let program = temporary_path("fake-podman");
    let log = temporary_path("calls");
    let info = r#"{"host":{"security":{"rootless":true,"seccompEnabled":true,"seccompProfilePath":"/usr/share/containers/seccomp.json","apparmorEnabled":true,"selinuxEnabled":false}}}"#;
    let script = format!(
        "#!/bin/sh\nset -eu\nprintf '%s\\n' \"$*\" >> '{}'\nmarker='{}'\nif [ \"${{1:-}}\" = info ]; then\n  if [ \"${{3:-}}\" = json ]; then printf '%s\\n' '{}'; else printf 'true\\n'; fi\n  exit 0\nfi\ncase \"${{1:-}}:${{2:-}}\" in\n  network:create) : ;;\n  network:rm) : ;;\n  create:--name) printf '%s\\n' '{}' ;;\n  start:*) printf 'post-create lifecycle reached\\n' > \"$marker\"; exit 93 ;;\n  container:inspect|top:*|port:*) printf 'post-create lifecycle reached\\n' > \"$marker\"; exit 94 ;;\n  rm:*) : ;;\n  *) exit 91 ;;\nesac\n",
        log.display(),
        lifecycle_marker.display(),
        info,
        create_identifier,
    );
    fs::write(&program, script).expect("fake Podman must be writable");
    let mut permissions = fs::metadata(&program)
        .expect("fake Podman metadata must exist")
        .permissions();
    permissions.set_mode(0o700);
    fs::set_permissions(&program, permissions).expect("fake Podman must be executable");
    (program, log)
}

fn assert_create_identifier_rejected(case_name: &str, create_identifier: &str) {
    let lifecycle_marker = temporary_path("lifecycle-marker");
    fs::write(&lifecycle_marker, "safe\n").expect("lifecycle marker must be writable");
    let (program, log) = write_fake_podman(create_identifier, &lifecycle_marker);
    let adapter = RootlessPodmanAdapter::new(program.clone());

    let result = adapter.launch_at(&request(case_name), &policy(), 1_780_001_000);
    assert!(
        matches!(
            result,
            Err(ApplicationServiceError::MalformedIsolationInspection {
                operation: "container_create"
            })
        ),
        "invalid create output must fail at the create-identifier boundary, got {result:?}"
    );

    assert_eq!(
        fs::read_to_string(&lifecycle_marker).expect("lifecycle marker must remain readable"),
        "safe\n",
        "invalid create output must be rejected before start/inspect/top/port can run"
    );

    let calls = fs::read_to_string(&log).expect("fake Podman calls must be recorded");
    assert!(
        !calls.lines().any(|line| line.starts_with("start ")
            || line.starts_with("container inspect ")
            || line.starts_with("top ")
            || line.starts_with("port ")),
        "invalid create output must not select a post-create lifecycle target; calls were:\n{calls}"
    );

    let _ = fs::remove_file(program);
    let _ = fs::remove_file(log);
    let _ = fs::remove_file(lifecycle_marker);
}

#[test]
fn service_create_rejects_name_like_identifier_before_lifecycle_use() {
    assert_create_identifier_rejected("name_like", "foreign-container");
}

#[test]
fn service_create_rejects_short_container_id_before_lifecycle_use() {
    assert_create_identifier_rejected("short_id", "0123456789ab");
}

#[test]
fn service_create_rejects_non_hex_long_identifier_before_lifecycle_use() {
    assert_create_identifier_rejected("non_hex_long", &"g".repeat(64));
}
