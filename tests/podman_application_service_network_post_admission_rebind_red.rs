//! RED: admitted network identity must remain the selector for later P0 verification.
//!
//! After the adapter has admitted one canonical backend network ID and bound container creation
//! to it, the public `qsr-net-*` correlation must not be resolved again for security evidence.
//! A same-name replacement can otherwise satisfy `internal=true` / DNS-disabled checks while
//! the exact acquired container remains attached to a different backend network object.

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

const STARTED_AT_EPOCH_SECONDS: u64 = 1_780_006_000;
const OWNED_NETWORK_ID: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
const REBOUND_NETWORK_ID: &str = "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";
const OWNED_CONTAINER_ID: &str = "cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc";

static NEXT_TEMP_PATH_ID: AtomicU64 = AtomicU64::new(0);

fn temporary_path(name: &str) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("test clock must be after the Unix epoch")
        .as_nanos();
    let unique_id = NEXT_TEMP_PATH_ID.fetch_add(1, Ordering::Relaxed);
    std::env::temp_dir().join(format!(
        "qsr-network-post-admission-rebind-red-{name}-{}-{nanos}-{unique_id}",
        std::process::id()
    ))
}

fn policy() -> IsolationPolicy {
    IsolationPolicy {
        policy_id: "network_post_admission_rebind_red_v1".to_owned(),
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
        request_id: "network_post_admission_rebind_red".to_owned(),
        image_reference: format!("localhost/cwl/tool@sha256:{}", "9".repeat(64)),
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
    created_network_name: &Path,
    name_inspection_count: &Path,
) -> PathBuf {
    let program = temporary_path("fake-podman");
    let info = r#"{"host":{"security":{"rootless":true,"seccompEnabled":true,"seccompProfilePath":"/usr/share/containers/seccomp.json","apparmorEnabled":true,"selinuxEnabled":false}}}"#;
    let script = format!(
        r#"#!/bin/sh
set -eu
printf '%s\n' "$*" >> '{log}'
network_name_file='{network_name_file}'
name_inspection_count_file='{name_inspection_count_file}'
owned_network_id='{owned_network_id}'
rebound_network_id='{rebound_network_id}'
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
    printf '0\n' > "$name_inspection_count_file"
    printf '%s\n' "$network_name"
    ;;
  network:inspect)
    network_name=$(cat "$network_name_file")
    selector=${{5:-}}
    if [ "$selector" = "$network_name" ]; then
      count=$(cat "$name_inspection_count_file")
      if [ "$count" -eq 0 ]; then
        printf '1\n' > "$name_inspection_count_file"
        printf '[{{"name":"%s","id":"%s","internal":true,"dns_enabled":false,"containers":{{}}}}]\n' "$network_name" "$owned_network_id"
      else
        printf '[{{"name":"%s","id":"%s","internal":true,"dns_enabled":false,"containers":{{}}}}]\n' "$network_name" "$rebound_network_id"
      fi
      exit 0
    fi
    if [ "$selector" = "$owned_network_id" ]; then
      printf '[{{"name":"%s","id":"%s","internal":true,"dns_enabled":false,"containers":{{}}}}]\n' "$network_name" "$owned_network_id"
      exit 0
    fi
    exit 95
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
    [ "$network" = "$owned_network_id" ] || exit 92
    [ -n "$cidfile" ] || exit 93
    printf '%s\n' "$container_id" > "$cidfile"
    printf '%s\n' "$container_id"
    ;;
  start:*)
    [ "${{2:-}}" = "$container_id" ] || exit 96
    ;;
  container:inspect)
    [ "${{5:-}}" = "$container_id" ] || exit 97
    printf '[{{"Id":"%s","AppArmorProfile":"containers-default","ProcessLabel":"","EffectiveCaps":[],"BoundingCaps":[],"Config":{{"User":"65532:65532"}},"HostConfig":{{"ReadonlyRootfs":true,"Privileged":false,"SecurityOpt":["no-new-privileges"],"UsernsMode":"auto","PidMode":"private","IpcMode":"none","Memory":134217728,"NanoCpus":250000000,"PidsLimit":16}}}}]\n' "$container_id"
    ;;
  top:*)
    printf 'PID SECCOMP CAPEFF CAPBND CAPINH CAPPRM CAPAMB LABEL\n1 filter - - - - - containers-default (enforce)\n'
    ;;
  port:*) printf '127.0.0.1:{ready_port}\n' ;;
  stop:*) : ;;
  rm:--force) : ;;
  network:rm) : ;;
  *) exit 91 ;;
esac
"#,
        log = log.display(),
        network_name_file = created_network_name.display(),
        name_inspection_count_file = name_inspection_count.display(),
        owned_network_id = OWNED_NETWORK_ID,
        rebound_network_id = REBOUND_NETWORK_ID,
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
fn post_admission_network_state_uses_exact_acquired_id_not_public_name() {
    let listener = TcpListener::bind(("127.0.0.1", 0)).expect("loopback listener must bind");
    let ready_port = listener
        .local_addr()
        .expect("listener address must be available")
        .port();
    let log = temporary_path("calls");
    let created_network_name = temporary_path("network-name");
    let name_inspection_count = temporary_path("name-inspection-count");
    let program = write_fake_podman(
        ready_port,
        &log,
        &created_network_name,
        &name_inspection_count,
    );
    let adapter = RootlessPodmanAdapter::new(program.clone());

    let _result = adapter.launch_at(&request(), &policy(), STARTED_AT_EPOCH_SECONDS);
    let calls = fs::read_to_string(&log).expect("fake Podman calls must be recorded");
    let public_network_name = fs::read_to_string(&created_network_name)
        .expect("created network correlation must be recorded")
        .trim()
        .to_owned();

    let _ = fs::remove_file(program);
    let _ = fs::remove_file(log);
    let _ = fs::remove_file(created_network_name);
    let _ = fs::remove_file(name_inspection_count);
    drop(listener);

    let lines: Vec<&str> = calls.lines().collect();
    let start_index = lines
        .iter()
        .position(|line| *line == format!("start {OWNED_CONTAINER_ID}"))
        .expect("the witness must reach the post-admission started-container verification phase");
    let post_start_network_inspections: Vec<&str> = lines[start_index + 1..]
        .iter()
        .copied()
        .filter(|line| line.starts_with("network inspect --format json "))
        .collect();

    assert!(
        !post_start_network_inspections.is_empty(),
        "effective P0 network state must be verified after the exact acquired container starts; calls were:\n{calls}"
    );
    assert!(
        post_start_network_inspections
            .iter()
            .all(|line| *line == format!("network inspect --format json {OWNED_NETWORK_ID}")),
        "after immutable network identity is admitted, later P0 verification must use that exact ID rather than re-resolve public correlation {public_network_name}; calls were:\n{calls}"
    );
    assert!(
        !post_start_network_inspections
            .iter()
            .any(|line| line.ends_with(&public_network_name)),
        "a same-name replacement must not supply post-admission isolation evidence; calls were:\n{calls}"
    );
}
