//! RED: malformed successful service create without a trustworthy receipt must fail closed.
//!
//! The generated `qsr-app-*` name is correlation metadata, not destructive authority. If create
//! succeeds but neither stdout nor a runtime-owned receipt yields an admitted exact container ID,
//! the runtime must report the unreconciled receipt condition without attempting name-based rm.

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
        "#!/bin/sh\nset -eu\nprintf '%s\\n' \"$*\" >> '{}'\ndestructive_marker='{}'\nif [ \"${{1:-}}\" = info ]; then\n  if [ \"${{3:-}}\" = json ]; then printf '%s\\n' '{}'; else printf 'true\\n'; fi\n  exit 0\nfi\ncase \"${{1:-}}:${{2:-}}\" in\n  network:create) : ;;\n  network:rm) : ;;\n  create:--name) printf 'bad identifier with spaces\\n' ;;\n  rm:--force) printf 'unexpected destructive container action\\n' > \"$destructive_marker\"; exit 93 ;;\n  start:*|container:inspect|top:*|port:*) exit 94 ;;\n  *) exit 91 ;;\nesac\n",
        log.display(),
        destructive_marker.display(),
        info,
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
