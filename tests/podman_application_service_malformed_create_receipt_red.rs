//! RED: successful service create with malformed stdout must clean by an exact runtime receipt.
//!
//! A successful `podman create` may already have materialized a container even when stdout cannot
//! be admitted as lifecycle authority. The application-service adapter must therefore provision a
//! runtime-owned create receipt and use only the exact acquired container ID for destructive
//! cleanup; the generated `qsr-app-*` correlation name is never a cleanup fallback.

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
const OWNED_CONTAINER_ID: &str =
    "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";

fn temporary_path(name: &str) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("test clock must be after the Unix epoch")
        .as_nanos();
    let unique_id = NEXT_TEMP_PATH_ID.fetch_add(1, Ordering::Relaxed);
    std::env::temp_dir().join(format!(
        "qsr-application-service-malformed-create-receipt-red-{name}-{}-{nanos}-{unique_id}",
        std::process::id()
    ))
}

fn policy() -> IsolationPolicy {
    IsolationPolicy {
        policy_id: "application_service_malformed_create_receipt_red_v1".to_owned(),
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
        request_id: "application_service_malformed_create_receipt_red".to_owned(),
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

fn write_fake_podman() -> (PathBuf, PathBuf, PathBuf) {
    let program = temporary_path("fake-podman");
    let log = temporary_path("calls");
    let foreign_cleanup_marker = temporary_path("foreign-cleanup");
    let info = r#"{"host":{"security":{"rootless":true,"seccompEnabled":true,"seccompProfilePath":"/usr/share/containers/seccomp.json","apparmorEnabled":true,"selinuxEnabled":false}}}"#;
    let script = format!(
        "#!/bin/sh\nset -eu\nprintf '%s\\n' \"$*\" >> '{}'\nforeign_marker='{}'\nowned_id='{}'\nif [ \"${{1:-}}\" = info ]; then\n  if [ \"${{3:-}}\" = json ]; then printf '%s\\n' '{}'; else printf 'true\\n'; fi\n  exit 0\nfi\ncase \"${{1:-}}:${{2:-}}\" in\n  network:create) : ;;\n  network:rm) : ;;\n  create:--name)\n    cidfile=''\n    for argument in \"$@\"; do\n      case \"$argument\" in --cidfile=*) cidfile=${{argument#--cidfile=}} ;; esac\n    done\n    if [ -n \"$cidfile\" ]; then printf '%s\\n' \"$owned_id\" > \"$cidfile\"; fi\n    printf 'bad identifier with spaces\\n'\n    ;;\n  rm:--force)\n    if [ \"${{3:-}}\" = \"$owned_id\" ]; then exit 0; fi\n    printf 'generated-name cleanup attempted\\n' > \"$foreign_marker\"\n    exit 93\n    ;;\n  start:*|container:inspect|top:*|port:*) exit 94 ;;\n  *) exit 91 ;;\nesac\n",
        log.display(),
        foreign_cleanup_marker.display(),
        OWNED_CONTAINER_ID,
        info,
    );
    fs::write(&program, script).expect("fake Podman must be writable");
    let mut permissions = fs::metadata(&program)
        .expect("fake Podman metadata must exist")
        .permissions();
    permissions.set_mode(0o700);
    fs::set_permissions(&program, permissions).expect("fake Podman must be executable");
    (program, log, foreign_cleanup_marker)
}

#[test]
fn malformed_successful_service_create_uses_runtime_receipt_for_exact_id_cleanup() {
    let (program, log, foreign_cleanup_marker) = write_fake_podman();
    let adapter = RootlessPodmanAdapter::new(program.clone());

    let result = adapter.launch_at(&request(), &policy(), 1_780_001_100);
    assert_eq!(
        result,
        Err(ApplicationServiceError::MalformedIsolationInspection {
            operation: "container_create",
        }),
        "successful exact-ID cleanup must preserve the original malformed-create error"
    );

    let calls = fs::read_to_string(&log).expect("fake Podman calls must be recorded");
    let create_call = calls
        .lines()
        .find(|line| line.starts_with("create --name "))
        .expect("service create must execute");
    assert!(
        create_call
            .split_whitespace()
            .any(|argument| argument.starts_with("--cidfile=")),
        "service create must provision a runtime-owned exact identity receipt; calls were:\n{calls}"
    );
    assert!(
        calls
            .lines()
            .any(|line| line == format!("rm --force {OWNED_CONTAINER_ID}")),
        "malformed stdout must clean only the receipt-acquired exact container ID; calls were:\n{calls}"
    );
    assert!(
        !foreign_cleanup_marker.exists(),
        "generated correlation name must never become destructive cleanup authority"
    );
    assert!(
        !calls.lines().any(|line| {
            line.starts_with("start ")
                || line.starts_with("container inspect ")
                || line.starts_with("top ")
                || line.starts_with("port ")
        }),
        "malformed create identity must fail before post-create service lifecycle; calls were:\n{calls}"
    );

    let _ = fs::remove_file(program);
    let _ = fs::remove_file(log);
    let _ = fs::remove_file(foreign_cleanup_marker);
}
