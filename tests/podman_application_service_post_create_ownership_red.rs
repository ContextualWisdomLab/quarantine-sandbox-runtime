//! Regression: application-service container lifecycle must stay bound to the acquired Podman ID.
//!
//! The generated `qsr-app-*` name is correlation metadata. Once `podman create`
//! returns a long container ID, later start/inspect/top/port/stop/remove operations
//! must not re-resolve that mutable name and risk acting on a foreign container.

#![cfg(target_os = "linux")]

use std::{
    fs,
    net::TcpListener,
    os::unix::fs::PermissionsExt,
    path::PathBuf,
    sync::atomic::{AtomicU64, Ordering},
    time::{SystemTime, UNIX_EPOCH},
};

use quarantine_sandbox_runtime::{
    ApplicationServiceRequest, IsolationPolicy, ResourceRequest, RootlessPodmanAdapter,
    ServiceProtocol,
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
        "qsr-application-service-post-create-ownership-red-{name}-{}-{nanos}-{unique_id}",
        std::process::id()
    ))
}

fn policy() -> IsolationPolicy {
    IsolationPolicy {
        policy_id: "application_service_post_create_ownership_red_v1".to_owned(),
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
        request_id: "application_service_post_create_ownership_red".to_owned(),
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

fn write_fake_podman(ready_port: u16, foreign_marker: &PathBuf) -> (PathBuf, PathBuf) {
    let program = temporary_path("fake-podman");
    let log = temporary_path("calls");
    let info = r#"{"host":{"security":{"rootless":true,"seccompEnabled":true,"seccompProfilePath":"/usr/share/containers/seccomp.json","apparmorEnabled":true,"selinuxEnabled":false}}}"#;
    let container = format!(
        r#"[{{"Id":"{OWNED_CONTAINER_ID}","AppArmorProfile":"containers-default","ProcessLabel":"","EffectiveCaps":[],"BoundingCaps":[],"Config":{{"User":"65532:65532"}},"HostConfig":{{"ReadonlyRootfs":true,"Privileged":false,"SecurityOpt":["no-new-privileges"],"UsernsMode":"auto","PidMode":"private","IpcMode":"none","Memory":134217728,"NanoCpus":250000000,"PidsLimit":16}}}}]"#
    );
    let network = r#"[{"internal":true,"dns_enabled":false}]"#;
    let script = format!(
        "#!/bin/sh\nset -eu\nprintf '%s\\n' \"$*\" >> '{}'\nforeign='{}'\nowned='{}'\ntouch_foreign() {{ printf 'touched\\n' > \"$foreign\"; }}\nif [ \"${{1:-}}\" = info ]; then\n  if [ \"${{3:-}}\" = json ]; then printf '%s\\n' '{}'; else printf 'true\\n'; fi\n  exit 0\nfi\ncase \"${{1:-}}:${{2:-}}\" in\n  network:create) : ;;\n  network:inspect) printf '%s\\n' '{}' ;;\n  network:rm) : ;;\n  create:--name) printf '%s\\n' \"$owned\" ;;\n  start:*) if [ \"${{2:-}}\" = \"$owned\" ]; then :; else touch_foreign; exit 93; fi ;;\n  container:inspect) if [ \"${{5:-}}\" = \"$owned\" ]; then printf '%s\\n' '{}'; else touch_foreign; exit 94; fi ;;\n  top:*) if [ \"${{2:-}}\" = \"$owned\" ]; then printf 'PID SECCOMP CAPEFF CAPBND CAPINH CAPPRM CAPAMB LABEL\\n1 filter - - - - - containers-default (enforce)\\n'; else touch_foreign; exit 95; fi ;;\n  port:*) if [ \"${{2:-}}\" = \"$owned\" ]; then printf '127.0.0.1:{ready_port}\\n'; else touch_foreign; exit 96; fi ;;\n  stop:*) if [ \"${{4:-}}\" = \"$owned\" ]; then :; else touch_foreign; exit 97; fi ;;\n  rm:*) if [ \"${{3:-}}\" = \"$owned\" ]; then :; else touch_foreign; exit 98; fi ;;\n  *) exit 91 ;;\nesac\n",
        log.display(),
        foreign_marker.display(),
        OWNED_CONTAINER_ID,
        info,
        network,
        container,
    );
    fs::write(&program, script).expect("fake Podman must be writable");
    let mut permissions = fs::metadata(&program)
        .expect("fake Podman metadata must exist")
        .permissions();
    permissions.set_mode(0o700);
    fs::set_permissions(&program, permissions).expect("fake Podman must be executable");
    (program, log)
}

fn lifecycle_targets(calls: &str) -> Vec<&str> {
    calls
        .lines()
        .filter_map(|line| {
            let fields = line.split_whitespace().collect::<Vec<_>>();
            match fields.as_slice() {
                ["start", target] => Some(*target),
                ["container", "inspect", "--format", "json", target] => Some(*target),
                ["top", target, ..] => Some(*target),
                ["port", target, ..] => Some(*target),
                ["stop", "--time", _, target] => Some(*target),
                ["rm", "--force", target] => Some(*target),
                _ => None,
            }
        })
        .collect()
}

#[test]
fn service_container_lifecycle_uses_acquired_id_after_create() {
    let listener = TcpListener::bind(("127.0.0.1", 0)).expect("loopback listener must bind");
    let ready_port = listener
        .local_addr()
        .expect("listener must expose an address")
        .port();
    let foreign_marker = temporary_path("foreign-container-marker");
    fs::write(&foreign_marker, "safe\n").expect("foreign marker must be writable");
    let (program, log) = write_fake_podman(ready_port, &foreign_marker);

    let adapter = RootlessPodmanAdapter::new(program.clone());
    let started_at_epoch_seconds = 1_780_000_100;
    let lease = adapter
        .launch_at(&request(), &policy(), started_at_epoch_seconds)
        .expect("launch must stay on the invocation-owned container ID");

    assert!(
        lease.sandbox_id().starts_with("qsr-app-"),
        "generated application sandbox name must remain correlation metadata"
    );
    assert_ne!(
        lease.sandbox_id(),
        OWNED_CONTAINER_ID,
        "public sandbox name semantics must not be silently rewritten by this security repair"
    );

    adapter
        .terminate_at(&lease, started_at_epoch_seconds + 1)
        .expect("termination must keep using the invocation-owned container ID");

    assert_eq!(
        fs::read_to_string(&foreign_marker).expect("foreign marker must remain readable"),
        "safe\n",
        "name rebinding must never let this invocation touch a foreign container"
    );

    let calls = fs::read_to_string(&log).expect("fake Podman calls must be recorded");
    let targets = lifecycle_targets(&calls);
    assert!(
        !targets.is_empty(),
        "the test must observe post-create container lifecycle operations"
    );
    assert!(
        targets.iter().all(|target| *target == OWNED_CONTAINER_ID),
        "every post-create container lifecycle operation must target the exact acquired long ID; calls were:\n{calls}"
    );

    let _ = fs::remove_file(program);
    let _ = fs::remove_file(log);
    let _ = fs::remove_file(foreign_marker);
    drop(listener);
}
