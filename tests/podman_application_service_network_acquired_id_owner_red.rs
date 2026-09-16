//! RED: canonical application-service owner must acquire exact network authority before create.
//!
//! The per-invocation `qsr-net-*` value is correlation metadata, not destructive or
//! attachment authority. The runtime must inspect the network created by this invocation,
//! acquire its Podman `.ID` before container creation, bind the container to that ID, and
//! preserve an effective-attachment mismatch as `sandbox_network_binding` before readiness.

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
    ApplicationServiceError, ApplicationServiceRequest, IsolationPolicy, ResourceRequest,
    RootlessPodmanAdapter, ServiceProtocol,
};

const STARTED_AT_EPOCH_SECONDS: u64 = 1_780_003_000;
const OWNED_NETWORK_ID: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
const OWNED_CONTAINER_ID: &str = "cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc";

static NEXT_TEMP_PATH_ID: AtomicU64 = AtomicU64::new(0);

fn temporary_path(name: &str) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("test clock must be after the Unix epoch")
        .as_nanos();
    let unique_id = NEXT_TEMP_PATH_ID.fetch_add(1, Ordering::Relaxed);
    std::env::temp_dir().join(format!(
        "qsr-network-owner-id-red-{name}-{}-{nanos}-{unique_id}",
        std::process::id()
    ))
}

fn policy() -> IsolationPolicy {
    IsolationPolicy {
        policy_id: "network_owner_acquired_id_red_v1".to_owned(),
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
        request_id: "network_owner_acquired_id_red".to_owned(),
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

fn write_fake_podman(ready_port: u16, log: &Path, created_network_name: &Path) -> PathBuf {
    let program = temporary_path("fake-podman");
    let info = r#"{"host":{"security":{"rootless":true,"seccompEnabled":true,"seccompProfilePath":"/usr/share/containers/seccomp.json","apparmorEnabled":true,"selinuxEnabled":false}}}"#;
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
  network:inspect)
    network_name=$(cat "$network_name_file")
    selector=${{5:-}}
    if [ "$selector" != "$network_name" ] && [ "$selector" != "$network_id" ]; then exit 95; fi
    printf '[{{"name":"%s","id":"%s","internal":true,"dns_enabled":false,"containers":{{}}}}]\n' "$network_name" "$network_id"
    ;;
  create:--name)
    cidfile=''
    network=''
    previous=''
    for argument in "$@"; do
      case "$argument" in
        --cidfile=*) cidfile=${{argument#--cidfile=}} ;;
      esac
      if [ "$previous" = '--network' ]; then network="$argument"; fi
      previous="$argument"
    done
    [ "$network" = "$network_id" ] || exit 92
    [ -n "$cidfile" ] || exit 93
    printf '%s\n' "$container_id" > "$cidfile"
    printf '%s\n' "$container_id"
    ;;
  start:*) : ;;
  container:inspect)
    printf '[{{"Id":"%s","AppArmorProfile":"containers-default","ProcessLabel":"","EffectiveCaps":[],"BoundingCaps":[],"Config":{{"User":"65532:65532"}},"HostConfig":{{"ReadonlyRootfs":true,"Privileged":false,"SecurityOpt":["no-new-privileges"],"UsernsMode":"auto","PidMode":"private","IpcMode":"none","Memory":134217728,"NanoCpus":250000000,"PidsLimit":16,"NetworkMode":"bridge"}},"NetworkSettings":{{"Networks":{{"podman":{{"NetworkID":"foreign"}}}}}}}}]\n' "$container_id"
    ;;
  top:*)
    printf 'PID SECCOMP CAPEFF CAPBND CAPINH CAPPRM CAPAMB LABEL\n1 filter - - - - - containers-default (enforce)\n'
    ;;
  port:*) printf '127.0.0.1:{ready_port}\n' ;;
  stop:*) : ;;
  rm:--force)
    [ "${{3:-}}" = "$container_id" ] || exit 96
    ;;
  network:rm) : ;;
  *) exit 91 ;;
esac
"#,
        log = log.display(),
        network_name_file = created_network_name.display(),
        network_id = OWNED_NETWORK_ID,
        container_id = OWNED_CONTAINER_ID,
        info = info,
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
fn acquired_network_identity_precedes_create_and_preserves_attachment_red() {
    let listener = TcpListener::bind(("127.0.0.1", 0)).expect("loopback listener must bind");
    let ready_port = listener
        .local_addr()
        .expect("listener must expose an address")
        .port();
    let log = temporary_path("calls");
    let created_network_name = temporary_path("network-name");
    let program = write_fake_podman(ready_port, &log, &created_network_name);
    let adapter = RootlessPodmanAdapter::new(program.clone());

    let result = adapter.launch_at(&request(), &policy(), STARTED_AT_EPOCH_SECONDS);
    let calls = fs::read_to_string(&log).expect("fake Podman calls must be recorded");

    let _ = fs::remove_file(program);
    let _ = fs::remove_file(log);
    let _ = fs::remove_file(created_network_name);
    drop(listener);

    let lines: Vec<&str> = calls.lines().collect();
    let network_create_index = lines
        .iter()
        .position(|line| line.starts_with("network create --internal --disable-dns qsr-net-"))
        .expect("runtime-owned network must be created");
    let created_network_name = lines[network_create_index]
        .split_whitespace()
        .last()
        .expect("network create must include the correlation name");
    let identity_inspect_index = lines
        .iter()
        .position(|line| *line == format!("network inspect --format json {created_network_name}"))
        .expect("created network identity must be inspected by its exact correlation name");
    let container_create_index = lines
        .iter()
        .position(|line| line.starts_with("create --name "))
        .expect("container create must be exercised");

    assert!(
        network_create_index < identity_inspect_index,
        "network identity must be acquired from the network created by this invocation; calls were:\n{calls}"
    );
    assert!(
        identity_inspect_index < container_create_index,
        "network identity must be acquired before container creation; calls were:\n{calls}"
    );
    assert!(
        lines[container_create_index].contains(&format!(" --network {OWNED_NETWORK_ID} ")),
        "container creation must bind to exact acquired network authority; call was: {}",
        lines[container_create_index]
    );
    assert_eq!(
        result,
        Err(ApplicationServiceError::IsolationVerificationFailed {
            control_name: "sandbox_network_binding",
        }),
        "acquired-ID binding must not erase the hostile effective-attachment mismatch"
    );
    assert!(
        calls
            .lines()
            .any(|line| line == format!("stop --time 1 {OWNED_CONTAINER_ID}")),
        "started-container cleanup must stay bound to the exact acquired container ID; calls were:\n{calls}"
    );
    assert!(
        calls
            .lines()
            .any(|line| line == format!("rm --force {OWNED_CONTAINER_ID}")),
        "container cleanup must remove only the exact acquired container ID; calls were:\n{calls}"
    );
    assert!(
        calls
            .lines()
            .any(|line| line == format!("network rm {OWNED_NETWORK_ID}")),
        "network cleanup must use exact acquired authority without force; calls were:\n{calls}"
    );
    assert!(
        !calls
            .lines()
            .any(|line| line.starts_with("network rm --force ")),
        "network-level force cleanup must never be used; calls were:\n{calls}"
    );
    assert!(
        !calls.lines().any(|line| line.starts_with("port ")),
        "readiness must not be queried after the effective attachment mismatch; calls were:\n{calls}"
    );
}
