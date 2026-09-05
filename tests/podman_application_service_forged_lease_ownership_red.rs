//! Regression: a serialized application-service lease is evidence, not cleanup authority.
//!
//! `ApplicationServiceLease` is a public wire type and can currently be
//! deserialized from caller-controlled JSON. Destructive Podman lifecycle
//! operations must never trust resource identifiers obtained only from that
//! serialized evidence object.

#![cfg(target_os = "linux")]

use std::{
    fs,
    os::unix::fs::PermissionsExt,
    path::PathBuf,
    sync::atomic::{AtomicU64, Ordering},
    time::{SystemTime, UNIX_EPOCH},
};

use quarantine_sandbox_runtime::{ApplicationServiceLease, RootlessPodmanAdapter};

static NEXT_TEMP_PATH_ID: AtomicU64 = AtomicU64::new(0);

fn temporary_path(name: &str) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("test clock must be after the Unix epoch")
        .as_nanos();
    let unique_id = NEXT_TEMP_PATH_ID.fetch_add(1, Ordering::Relaxed);
    std::env::temp_dir().join(format!(
        "qsr-application-service-forged-lease-red-{name}-{}-{nanos}-{unique_id}",
        std::process::id()
    ))
}

fn forged_lease() -> ApplicationServiceLease {
    let json = format!(
        r#"{{
            "schema_version":"1.1.0",
            "request_id":"forged_cleanup_authority",
            "image_reference":"localhost/cwl/tool@sha256:{}",
            "backend_id":"rootless_podman",
            "sandbox_id":"foreign-container",
            "network_id":"foreign-network",
            "policy_id":"forged_policy",
            "policy_sha256":"{}",
            "endpoint":{{"host":"127.0.0.1","port":8080,"protocol":"http"}},
            "started_at_epoch_seconds":1780000300,
            "expires_at_epoch_seconds":1780000330,
            "shutdown_grace_seconds":1,
            "isolation_attestation":{{
                "rootless":true,
                "read_only_root_filesystem":true,
                "all_capabilities_dropped":true,
                "no_new_privileges":true,
                "isolated_user_namespace":true,
                "external_egress_denied":true,
                "loopback_only_publication":true,
                "credentials_available":false
            }}
        }}"#,
        "a".repeat(64),
        "b".repeat(64),
    );
    serde_json::from_str(&json).expect("public lease wire type currently accepts deserialization")
}

fn write_fake_podman(foreign_marker: &PathBuf) -> (PathBuf, PathBuf) {
    let program = temporary_path("fake-podman");
    let log = temporary_path("calls");
    let script = format!(
        "#!/bin/sh\nset -eu\nprintf '%s\\n' \"$*\" >> '{}'\nforeign='{}'\ncase \"${{1:-}}:${{2:-}}\" in\n  stop:--time)\n    [ \"${{4:-}}\" = foreign-container ] || exit 91\n    printf 'foreign-container-stopped\\n' > \"$foreign\"\n    exit 0\n    ;;\n  rm:--force)\n    [ \"${{3:-}}\" = foreign-container ] || exit 92\n    printf 'foreign-container-removed\\n' > \"$foreign\"\n    exit 0\n    ;;\n  network:rm)\n    [ \"${{4:-}}\" = foreign-network ] || exit 93\n    printf 'foreign-network-removed\\n' > \"$foreign\"\n    exit 0\n    ;;\n  *) exit 94 ;;\nesac\n",
        log.display(),
        foreign_marker.display(),
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
fn deserialized_lease_cannot_select_destructive_backend_resources() {
    let foreign_marker = temporary_path("foreign-resource-marker");
    fs::write(&foreign_marker, "safe\n").expect("foreign marker must be writable");
    let (program, log) = write_fake_podman(&foreign_marker);
    let adapter = RootlessPodmanAdapter::new(program.clone());
    let lease = forged_lease();

    let result = adapter.terminate_at(&lease, 1_780_000_301);
    assert!(
        result.is_err(),
        "a caller-deserialized lease must not be accepted as destructive backend authority; got {result:?}"
    );
    assert_eq!(
        fs::read_to_string(&foreign_marker).expect("foreign marker must remain readable"),
        "safe\n",
        "forged lease identifiers must never stop, remove, or network-remove foreign resources"
    );

    let calls = fs::read_to_string(&log).unwrap_or_default();
    assert!(
        calls.trim().is_empty(),
        "termination must reject unowned serialized authority before invoking destructive Podman operations; calls were:\n{calls}"
    );

    let _ = fs::remove_file(program);
    let _ = fs::remove_file(log);
    let _ = fs::remove_file(foreign_marker);
}
