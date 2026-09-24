//! RED: a published service lease must retain exact private network cleanup authority.
//!
//! The public lease may expose the generated `qsr-net-*` correlation identifier, but successful
//! launch must retain the admitted immutable Podman network ID in private lifecycle authority.
//! Explicit termination must remove only that exact ID and must not use network-level force.

#![cfg(target_os = "linux")]

use std::{
    fs,
    net::TcpListener,
    os::unix::fs::PermissionsExt,
    path::{Path, PathBuf},
    sync::atomic::{AtomicU64, Ordering},
    time::{SystemTime, UNIX_EPOCH},
};

use quarantine_sandbox_runtime::{
    ApplicationServiceRequest, IsolationPolicy, ResourceRequest, RootlessPodmanAdapter,
    ServiceProtocol,
};

const STARTED_AT_EPOCH_SECONDS: u64 = 1_780_007_200;
const OWNED_NETWORK_ID: &str = "abcdef0123456789abcdef0123456789abcdef0123456789abcdef0123456789";
const CONTAINER_ID: &str = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";

static NEXT_TEMP_PATH_ID: AtomicU64 = AtomicU64::new(0);

fn temporary_path(name: &str) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("test clock must be after the Unix epoch")
        .as_nanos();
    let unique_id = NEXT_TEMP_PATH_ID.fetch_add(1, Ordering::Relaxed);
    std::env::temp_dir().join(format!(
        "qsr-successful-lease-network-authority-red-{name}-{}-{nanos}-{unique_id}",
        std::process::id()
    ))
}

fn policy() -> IsolationPolicy {
    IsolationPolicy {
        policy_id: "successful_lease_network_authority_red_v1".to_owned(),
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
        request_id: "successful_lease_network_authority_red".to_owned(),
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

fn write_fake_podman(log: &Path, network_name_file: &Path, ready_port: u16) -> PathBuf {
    let program = temporary_path("fake-podman");
    let info = r#"{"host":{"security":{"rootless":true,"seccompEnabled":true,"seccompProfilePath":"/usr/share/containers/seccomp.json","apparmorEnabled":true,"selinuxEnabled":false}}}"#;
    let container = format!(
        r#"[{{"Id":"{container_id}","AppArmorProfile":"containers-default","ProcessLabel":"","EffectiveCaps":[],"BoundingCaps":[],"Config":{{"User":"65532:65532"}},"HostConfig":{{"ReadonlyRootfs":true,"Privileged":false,"SecurityOpt":["no-new-privileges"],"UsernsMode":"auto","PidMode":"private","IpcMode":"none","Memory":134217728,"NanoCpus":250000000,"PidsLimit":16}},"NetworkSettings":{{"Networks":{{"qsr":{{"NetworkID":"{network_id}"}}}}}}}}]"#,
        container_id = CONTAINER_ID,
        network_id = OWNED_NETWORK_ID,
    );
    let script = format!(
        r#"#!/bin/sh
set -eu
printf '%s\n' "$*" >> '{log}'
network_name_file='{network_name_file}'
network_id='{network_id}'
container_id='{container_id}'
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
    printf '{{"ID":"%s","Network":"%s","Status":"create","Type":"network"}}\n' "$network_id" "$network_name"
    ;;
  network:inspect)
    network_name=$(cat "$network_name_file")
    selector=${{5:-}}
    [ "$selector" = "$network_id" ] || exit 96
    printf '[{{"name":"%s","id":"%s","internal":true,"dns_enabled":false,"containers":{{}}}}]\n' "$network_name" "$network_id"
    ;;
  create:--name)
    printf '%s\n' "$container_id"
    ;;
  start:*) : ;;
  container:inspect) printf '%s\n' '{container}' ;;
  top:*) printf 'PID SECCOMP CAPEFF CAPBND CAPINH CAPPRM CAPAMB LABEL\n1 filter - - - - - containers-default (enforce)\n' ;;
  port:*) printf '127.0.0.1:{ready_port}\n' ;;
  stop:*) : ;;
  rm:*) : ;;
  network:rm) : ;;
  *) exit 91 ;;
esac
"#,
        log = log.display(),
        network_name_file = network_name_file.display(),
        network_id = OWNED_NETWORK_ID,
        container_id = CONTAINER_ID,
        info = info,
        container = container,
        ready_port = ready_port,
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
fn successful_lease_keeps_exact_network_id_private_for_non_force_termination() {
    let listener = TcpListener::bind(("127.0.0.1", 0)).expect("loopback listener must bind");
    let ready_port = listener
        .local_addr()
        .expect("listener address must be available")
        .port();
    let log = temporary_path("calls");
    let network_name_file = temporary_path("network-name");
    let program = write_fake_podman(&log, &network_name_file, ready_port);
    let adapter = RootlessPodmanAdapter::new(program.clone());

    let lease = adapter
        .launch_at(&request(), &policy(), STARTED_AT_EPOCH_SECONDS)
        .expect("service launch must reach published lease state");

    assert!(
        lease.network_id().starts_with("qsr-net-"),
        "public lease network identity remains correlation metadata"
    );
    assert_ne!(
        lease.network_id(),
        OWNED_NETWORK_ID,
        "public correlation and private destructive authority must stay distinct"
    );

    adapter
        .terminate_at(&lease, STARTED_AT_EPOCH_SECONDS + 10)
        .expect("termination of an unshared owned network must succeed");

    let calls = fs::read_to_string(&log).expect("fake Podman calls must be recorded");
    assert!(
        calls
            .lines()
            .any(|line| { line == format!("network inspect --format json {OWNED_NETWORK_ID}") }),
        "launch must admit and later verify the exact backend network ID; calls were:\n{calls}"
    );
    assert!(
        calls.lines().any(|line| {
            line.contains(&format!("--network {OWNED_NETWORK_ID}"))
                && line.starts_with("create --name ")
        }),
        "container creation must bind to the admitted backend network ID; calls were:\n{calls}"
    );

    let expected_network_removal = format!("network rm {OWNED_NETWORK_ID}");
    let network_removals = calls
        .lines()
        .filter(|line| line.starts_with("network rm"))
        .collect::<Vec<_>>();
    assert_eq!(
        network_removals.as_slice(),
        [expected_network_removal.as_str()],
        "explicit termination must perform exactly one network removal, by the admitted exact ID and without force or any alternate selector; calls were:\n{calls}"
    );

    let _ = fs::remove_file(program);
    let _ = fs::remove_file(log);
    let _ = fs::remove_file(network_name_file);
    drop(listener);
}
