//! Regression: independent application-service launches must not share runtime resource identity.
//!
//! `request_id` is consumer correlation/idempotency metadata, not a globally unique
//! Podman resource identifier. Two independent runtime instances can legitimately
//! receive the same immutable request in the same second. Their sandbox/network
//! identities must remain distinct so cleanup from one invocation cannot target
//! resources owned by the other invocation.

#![cfg(target_os = "linux")]

use std::{
    fs,
    net::TcpListener,
    os::unix::fs::PermissionsExt,
    path::PathBuf,
    sync::atomic::{AtomicU64, Ordering},
    thread,
    time::{SystemTime, UNIX_EPOCH},
};

use quarantine_sandbox_runtime::{
    ApplicationServiceRequest, IsolationPolicy, ResourceRequest, RootlessPodmanAdapter,
    ServiceProtocol,
};

static NEXT_TEMP_PATH_ID: AtomicU64 = AtomicU64::new(0);

fn temporary_path(name: &str) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("test clock must be after the Unix epoch")
        .as_nanos();
    let unique_id = NEXT_TEMP_PATH_ID.fetch_add(1, Ordering::Relaxed);
    std::env::temp_dir().join(format!(
        "qsr-application-service-identity-red-{name}-{}-{nanos}-{unique_id}",
        std::process::id()
    ))
}

fn policy() -> IsolationPolicy {
    IsolationPolicy {
        policy_id: "application_service_identity_red_v1".to_owned(),
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
        request_id: "shared_consumer_correlation".to_owned(),
        image_reference: format!("localhost/cwl/tool@sha256:{}", "d".repeat(64)),
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

fn write_fake_podman(ready_port: u16) -> (PathBuf, PathBuf) {
    let program = temporary_path("fake-podman");
    let log = temporary_path("calls");
    let info = r#"{"host":{"security":{"rootless":true,"seccompEnabled":true,"seccompProfilePath":"/usr/share/containers/seccomp.json","apparmorEnabled":true,"selinuxEnabled":false}}}"#;
    let container = r#"[{"Id":"fake-container-id","AppArmorProfile":"containers-default","ProcessLabel":"","EffectiveCaps":[],"BoundingCaps":[],"Config":{"User":"65532:65532"},"HostConfig":{"ReadonlyRootfs":true,"Privileged":false,"SecurityOpt":["no-new-privileges"],"UsernsMode":"auto","PidMode":"private","IpcMode":"none","Memory":134217728,"NanoCpus":250000000,"PidsLimit":16}}]"#;
    let network = r#"[{"internal":true,"dns_enabled":false}]"#;
    let script = format!(
        "#!/bin/sh\nset -eu\nprintf '%s\\n' \"$*\" >> '{}'\nif [ \"${{1:-}}\" = info ]; then\n  if [ \"${{3:-}}\" = json ]; then printf '%s\\n' '{}'; else printf 'true\\n'; fi\n  exit 0\nfi\ncase \"${{1:-}}:${{2:-}}\" in\n  network:create) : ;;\n  network:inspect) printf '%s\\n' '{}' ;;\n  network:rm) : ;;\n  create:--name) printf 'fake-container-id\\n' ;;\n  container:inspect) printf '%s\\n' '{}' ;;\n  start:*) : ;;\n  top:*) printf 'PID SECCOMP CAPEFF CAPBND CAPINH CAPPRM CAPAMB LABEL\\n1 filter - - - - - containers-default (enforce)\\n' ;;\n  port:*) printf '127.0.0.1:{ready_port}\\n' ;;\n  stop:*) : ;;\n  rm:*) : ;;\n  *) exit 91 ;;\nesac\n",
        log.display(),
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

#[test]
fn independent_same_request_launches_use_distinct_runtime_owned_resource_identities() {
    let listener = TcpListener::bind(("127.0.0.1", 0)).expect("loopback listener must bind");
    let ready_port = listener
        .local_addr()
        .expect("listener must expose an address")
        .port();
    let (program, log) = write_fake_podman(ready_port);

    let first_adapter = RootlessPodmanAdapter::new(program.clone());
    let second_adapter = RootlessPodmanAdapter::new(program.clone());
    let first_request = request();
    let second_request = request();
    let first_policy = policy();
    let second_policy = policy();
    let started_at_epoch_seconds = 1_780_000_000;

    let (first_result, second_result) = thread::scope(|scope| {
        let first = scope.spawn(move || {
            first_adapter.launch_at(
                &first_request,
                &first_policy,
                started_at_epoch_seconds,
            )
        });
        let second = scope.spawn(move || {
            second_adapter.launch_at(
                &second_request,
                &second_policy,
                started_at_epoch_seconds,
            )
        });
        (
            first.join().expect("first launch thread must not panic"),
            second.join().expect("second launch thread must not panic"),
        )
    });

    let first_lease = first_result.expect("first isolated application service must launch");
    let second_lease = second_result.expect("second isolated application service must launch");

    assert_eq!(
        first_lease.request_id(),
        second_lease.request_id(),
        "consumer correlation identity must remain unchanged"
    );
    assert_ne!(
        first_lease.sandbox_id(),
        second_lease.sandbox_id(),
        "independent launches must never share a cleanup-owning sandbox identity"
    );
    assert_ne!(
        first_lease.network_id(),
        second_lease.network_id(),
        "independent launches must never share a cleanup-owning network identity"
    );

    let calls = fs::read_to_string(&log).expect("fake Podman calls must be recorded");
    assert_eq!(
        calls.lines().filter(|line| line.starts_with("create --name ")).count(),
        2,
        "both independent launches must reach distinct container creation attempts"
    );

    let _ = fs::remove_file(program);
    let _ = fs::remove_file(log);
    drop(listener);
}
