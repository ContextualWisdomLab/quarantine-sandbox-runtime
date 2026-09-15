//! RED: explicit application-service termination must not delete foreign network members.
//!
//! This fixture is compatible with both the current correlation-name binding and the intended
//! acquired-network-ID binding so the failure cause stays on destructive cleanup authority.

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

const STARTED_AT_EPOCH_SECONDS: u64 = 1_780_004_000;
const TERMINATED_AT_EPOCH_SECONDS: u64 = 1_780_004_010;
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
        "qsr-network-termination-owner-red-{name}-{}-{nanos}-{unique_id}",
        std::process::id()
    ))
}

fn policy() -> IsolationPolicy {
    IsolationPolicy {
        policy_id: "network_termination_owner_red_v1".to_owned(),
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
        request_id: "network_termination_owner_red".to_owned(),
        image_reference: format!("localhost/cwl/tool@sha256:{}", "d".repeat(64)),
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

fn write_fake_podman(
    ready_port: u16,
    log: &Path,
    foreign_marker: &Path,
    created_network_name: &Path,
    selected_network: &Path,
) -> PathBuf {
    let program = temporary_path("fake-podman");
    let info = r#"{"host":{"security":{"rootless":true,"seccompEnabled":true,"seccompProfilePath":"/usr/share/containers/seccomp.json","apparmorEnabled":true,"selinuxEnabled":false}}}"#;
    let script = format!(
        r#"#!/bin/sh
set -eu
printf '%s\n' "$*" >> '{log}'
foreign='{foreign_marker}'
network_name_file='{network_name_file}'
selected_network_file='{selected_network_file}'
network_id='{network_id}'
container_id='{container_id}'
if [ "${{1:-}}" = info ]; then
  if [ "${{3:-}}" = json ]; then printf '%s\n' '{info}'; else printf 'true\n'; fi
  exit 0
fi
case "${{1:-}}:${{2:-}}" in
  network:create)
    network_name=''
    for argument in "$@"; do network_name="$argument"; done
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
    network_name=$(cat "$network_name_file")
    if [ "$network" != "$network_name" ] && [ "$network" != "$network_id" ]; then exit 92; fi
    [ -n "$cidfile" ] || exit 93
    printf '%s\n' "$network" > "$selected_network_file"
    printf '%s\n' "$container_id" > "$cidfile"
    printf '%s\n' "$container_id"
    ;;
  start:*) : ;;
  container:inspect)
    network=$(cat "$selected_network_file")
    printf '[{{"Id":"%s","AppArmorProfile":"containers-default","ProcessLabel":"","EffectiveCaps":[],"BoundingCaps":[],"Config":{{"User":"65532:65532"}},"HostConfig":{{"ReadonlyRootfs":true,"Privileged":false,"SecurityOpt":["no-new-privileges"],"UsernsMode":"auto","PidMode":"private","IpcMode":"none","Memory":134217728,"NanoCpus":250000000,"PidsLimit":16,"NetworkMode":"%s"}},"NetworkSettings":{{"Networks":{{"%s":{{"NetworkID":"%s"}}}}}}}}]\n' "$container_id" "$network" "$network" "$network_id"
    ;;
  top:*)
    printf 'PID SECCOMP CAPEFF CAPBND CAPINH CAPPRM CAPAMB LABEL\n1 filter - - - - - containers-default (enforce)\n'
    ;;
  port:*) printf '127.0.0.1:{ready_port}\n' ;;
  stop:*) : ;;
  rm:--force)
    [ "${{3:-}}" = "$container_id" ] || exit 96
    ;;
  network:rm)
    if [ -e "$foreign" ]; then
      case " $* " in
        *' --force '*) printf 'deleted-by-force\n' > "$foreign"; exit 0 ;;
        *) exit 2 ;;
      esac
    fi
    :
    ;;
  *) exit 91 ;;
esac
"#,
        log = log.display(),
        foreign_marker = foreign_marker.display(),
        network_name_file = created_network_name.display(),
        selected_network_file = selected_network.display(),
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
fn explicit_termination_preserves_foreign_network_member() {
    let listener = TcpListener::bind(("127.0.0.1", 0)).expect("loopback listener must bind");
    let ready_port = listener
        .local_addr()
        .expect("listener address must be available")
        .port();
    let log = temporary_path("calls");
    let foreign_marker = temporary_path("foreign-member");
    let created_network_name = temporary_path("network-name");
    let selected_network = temporary_path("selected-network");
    let program = write_fake_podman(
        ready_port,
        &log,
        &foreign_marker,
        &created_network_name,
        &selected_network,
    );
    let adapter = RootlessPodmanAdapter::new(program.clone());

    let lease = adapter
        .launch_at(&request(), &policy(), STARTED_AT_EPOCH_SECONDS)
        .expect("service must launch before the foreign member appears");
    fs::write(&foreign_marker, "safe\n").expect("foreign marker must be writable");

    let result = adapter.terminate_at(&lease, TERMINATED_AT_EPOCH_SECONDS);
    let calls = fs::read_to_string(&log).expect("fake Podman calls must be recorded");
    let foreign_state = fs::read_to_string(&foreign_marker).expect("foreign marker must remain");

    let _ = fs::remove_file(program);
    let _ = fs::remove_file(log);
    let _ = fs::remove_file(foreign_marker);
    let _ = fs::remove_file(created_network_name);
    let _ = fs::remove_file(selected_network);
    drop(listener);

    assert_eq!(
        result,
        Err(ApplicationServiceError::CleanupFailed),
        "an in-use owned network must fail closed instead of deleting a foreign member"
    );
    assert_eq!(
        foreign_state, "safe\n",
        "explicit termination must preserve the foreign network member"
    );
    assert!(
        calls
            .lines()
            .any(|line| line == format!("stop --time 1 {OWNED_CONTAINER_ID}")),
        "termination must stay bound to the exact acquired container ID; calls were:\n{calls}"
    );
    assert!(
        calls
            .lines()
            .any(|line| line == format!("rm --force {OWNED_CONTAINER_ID}")),
        "termination must remove only the exact acquired container ID; calls were:\n{calls}"
    );
    assert!(
        calls
            .lines()
            .any(|line| line == format!("network rm {OWNED_NETWORK_ID}")),
        "network cleanup must target exact acquired network authority without force; calls were:\n{calls}"
    );
    assert!(
        !calls
            .lines()
            .any(|line| line.starts_with("network rm --force ")),
        "network-level force cleanup must never be used; calls were:\n{calls}"
    );
}
