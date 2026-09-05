//! Regression: sandbox-network cleanup must not delete foreign containers.
//!
//! Podman's `network rm --force` recursively removes containers using the
//! network. A sandbox runtime owns its own container and network, not every
//! same-principal container that may have attached to that network. Cleanup
//! must therefore fail closed on an in-use network rather than broaden its
//! destructive authority.

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
        "qsr-network-cleanup-ownership-red-{name}-{}-{nanos}-{unique_id}",
        std::process::id()
    ))
}

fn policy() -> IsolationPolicy {
    IsolationPolicy {
        policy_id: "network_cleanup_ownership_red_v1".to_owned(),
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
        request_id: "network_cleanup_ownership_red".to_owned(),
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

fn write_fake_podman(foreign_marker: &PathBuf) -> (PathBuf, PathBuf) {
    let program = temporary_path("fake-podman");
    let log = temporary_path("calls");
    let info = r#"{"host":{"security":{"rootless":true,"seccompEnabled":true,"seccompProfilePath":"/usr/share/containers/seccomp.json","apparmorEnabled":true,"selinuxEnabled":false}}}"#;
    let script = format!(
        "#!/bin/sh\nset -eu\nprintf '%s\\n' \"$*\" >> '{}'\nforeign='{}'\nif [ \"${{1:-}}\" = info ]; then\n  if [ \"${{3:-}}\" = json ]; then printf '%s\\n' '{}'; else printf 'true\\n'; fi\n  exit 0\nfi\ncase \"${{1:-}}:${{2:-}}\" in\n  network:create) printf 'qsr-test-network\\n'; exit 0 ;;\n  create:--name) exit 125 ;;\n  network:rm)\n    case \" $* \" in\n      *' --force '*) printf 'deleted-by-force\\n' > \"$foreign\"; exit 0 ;;\n      *) exit 2 ;;\n    esac\n    ;;\n  *) exit 91 ;;\nesac\n",
        log.display(),
        foreign_marker.display(),
        info,
    );
    fs::write(&program, script).expect("fake Podman must be writable");
    let mut permissions = fs::metadata(&program)
        .expect("fake Podman metadata must exist")
        .permissions();
    permissions.set_mode(0o700);
    fs::set_permissions(&program, permissions).expect("fake Podman must be executable");
    (program, log)
}

#[test]
fn failed_launch_never_force_removes_foreign_network_members() {
    let foreign_marker = temporary_path("foreign-container-marker");
    fs::write(&foreign_marker, "safe\n").expect("foreign marker must be writable");
    let (program, log) = write_fake_podman(&foreign_marker);
    let adapter = RootlessPodmanAdapter::new(program.clone());

    let result = adapter.launch_at(&request(), &policy(), 1_780_000_200);
    assert!(
        matches!(result, Err(ApplicationServiceError::CleanupFailed)),
        "an in-use sandbox network must surface cleanup uncertainty instead of broadening deletion authority; got {result:?}"
    );
    assert_eq!(
        fs::read_to_string(&foreign_marker).expect("foreign marker must remain readable"),
        "safe\n",
        "sandbox cleanup must never delete a foreign container through network-level force removal"
    );

    let calls = fs::read_to_string(&log).expect("fake Podman calls must be recorded");
    assert!(
        calls.lines().any(|line| line.starts_with("network rm ")),
        "the intended cleanup path must be exercised; calls were:\n{calls}"
    );
    assert!(
        !calls.lines().any(|line| line.starts_with("network rm --force ")),
        "network cleanup must not delegate foreign-container deletion to `podman network rm --force`; calls were:\n{calls}"
    );

    let _ = fs::remove_file(program);
    let _ = fs::remove_file(log);
    let _ = fs::remove_file(foreign_marker);
}
