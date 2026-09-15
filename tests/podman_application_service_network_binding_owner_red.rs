//! RED: canonical application-service owner must prove the effective network attachment set.
//!
//! This fixture runs on the random-identity/exact-container-ID owner lineage. It avoids
//! predicting generated correlation names and instead makes the fake backend record the
//! network selector actually supplied to container creation. A separately inspectable
//! internal network is not proof that the running container is attached exclusively to it.

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

const STARTED_AT_EPOCH_SECONDS: u64 = 1_780_002_000;
const OWNED_CONTAINER_ID: &str = "cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc";

static NEXT_TEMP_PATH_ID: AtomicU64 = AtomicU64::new(0);

#[derive(Clone, Copy)]
enum AttachmentCase {
    DifferentNetwork,
    MissingAttachment,
    AdditionalNetwork,
}

impl AttachmentCase {
    const fn script_name(self) -> &'static str {
        match self {
            Self::DifferentNetwork => "different",
            Self::MissingAttachment => "missing",
            Self::AdditionalNetwork => "additional",
        }
    }
}

fn temporary_path(name: &str) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("test clock must be after the Unix epoch")
        .as_nanos();
    let unique_id = NEXT_TEMP_PATH_ID.fetch_add(1, Ordering::Relaxed);
    std::env::temp_dir().join(format!(
        "qsr-network-owner-red-{name}-{}-{nanos}-{unique_id}",
        std::process::id()
    ))
}

fn policy() -> IsolationPolicy {
    IsolationPolicy {
        policy_id: "network_owner_binding_red_v1".to_owned(),
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
        request_id: "network_owner_binding_red".to_owned(),
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

fn write_fake_podman(
    case: AttachmentCase,
    ready_port: u16,
    log: &Path,
    network_selector: &Path,
) -> PathBuf {
    let program = temporary_path("fake-podman");
    let info = r#"{"host":{"security":{"rootless":true,"seccompEnabled":true,"seccompProfilePath":"/usr/share/containers/seccomp.json","apparmorEnabled":true,"selinuxEnabled":false}}}"#;
    let script = format!(
        r#"#!/bin/sh
set -eu
printf '%s\n' "$*" >> '{log}'
selector_file='{selector_file}'
case_name='{case_name}'
container_id='{container_id}'
if [ "${{1:-}}" = info ]; then
  if [ "${{3:-}}" = json ]; then printf '%s\n' '{info}'; else printf 'true\n'; fi
  exit 0
fi
case "${{1:-}}:${{2:-}}" in
  network:create)
    :
    ;;
  network:inspect)
    printf '%s\n' '[{{"internal":true,"dns_enabled":false}}]'
    ;;
  create:--name)
    network=''
    cidfile=''
    previous=''
    for argument in "$@"; do
      case "$argument" in
        --cidfile=*) cidfile=${{argument#--cidfile=}} ;;
      esac
      if [ "$previous" = '--network' ]; then network="$argument"; fi
      previous="$argument"
    done
    [ -n "$network" ] || exit 92
    [ -n "$cidfile" ] || exit 93
    printf '%s\n' "$network" > "$selector_file"
    printf '%s\n' "$container_id" > "$cidfile"
    printf '%s\n' "$container_id"
    ;;
  start:*) : ;;
  container:inspect)
    network=$(cat "$selector_file")
    case "$case_name" in
      different)
        network_mode='bridge'
        networks='{{"podman":{{}}}}'
        ;;
      missing)
        network_mode="$network"
        networks='{{}}'
        ;;
      additional)
        network_mode="$network"
        networks=$(printf '{{"%s":{{}},"podman":{{}}}}' "$network")
        ;;
      *) exit 94 ;;
    esac
    printf '[{{"Id":"%s","AppArmorProfile":"containers-default","ProcessLabel":"","EffectiveCaps":[],"BoundingCaps":[],"Config":{{"User":"65532:65532"}},"HostConfig":{{"ReadonlyRootfs":true,"Privileged":false,"SecurityOpt":["no-new-privileges"],"UsernsMode":"auto","PidMode":"private","IpcMode":"none","Memory":134217728,"NanoCpus":250000000,"PidsLimit":16,"NetworkMode":"%s"}},"NetworkSettings":{{"Networks":%s}}}}]\n' "$container_id" "$network_mode" "$networks"
    ;;
  top:*)
    printf 'PID SECCOMP CAPEFF CAPBND CAPINH CAPPRM CAPAMB LABEL\n1 filter - - - - - containers-default (enforce)\n'
    ;;
  port:*) printf '127.0.0.1:{ready_port}\n' ;;
  stop:*) : ;;
  rm:*) : ;;
  network:rm) : ;;
  *) exit 91 ;;
esac
"#,
        log = log.display(),
        selector_file = network_selector.display(),
        case_name = case.script_name(),
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

fn assert_network_binding_rejected(case: AttachmentCase) {
    let listener = TcpListener::bind(("127.0.0.1", 0)).expect("loopback listener must bind");
    let ready_port = listener
        .local_addr()
        .expect("listener must expose an address")
        .port();
    let log = temporary_path("calls");
    let network_selector = temporary_path("network-selector");
    let program = write_fake_podman(case, ready_port, &log, &network_selector);
    let adapter = RootlessPodmanAdapter::new(program.clone());

    assert_eq!(
        adapter.launch_at(&request(), &policy(), STARTED_AT_EPOCH_SECONDS),
        Err(ApplicationServiceError::IsolationVerificationFailed {
            control_name: "sandbox_network_binding",
        }),
        "network-object configuration must not substitute for exact container-attachment proof"
    );

    let calls = fs::read_to_string(&log).expect("fake Podman calls must be recorded");
    assert!(
        calls.lines().any(|line| line.starts_with("network create --internal --disable-dns qsr-net-")),
        "the runtime-owned network must be created; calls were:\n{calls}"
    );
    assert!(
        calls.lines().any(|line| line == format!("container inspect --format json {OWNED_CONTAINER_ID}")),
        "effective isolation must remain bound to the exact acquired container ID; calls were:\n{calls}"
    );
    assert!(
        !calls.lines().any(|line| line.starts_with("port ")),
        "readiness must not be queried after an effective attachment mismatch; calls were:\n{calls}"
    );

    let _ = fs::remove_file(program);
    let _ = fs::remove_file(log);
    let _ = fs::remove_file(network_selector);
    drop(listener);
}

#[test]
fn container_on_different_network_fails_closed_before_readiness() {
    assert_network_binding_rejected(AttachmentCase::DifferentNetwork);
}

#[test]
fn container_with_expected_mode_but_missing_attachment_fails_closed_before_readiness() {
    assert_network_binding_rejected(AttachmentCase::MissingAttachment);
}

#[test]
fn container_with_additional_egress_network_fails_closed_before_readiness() {
    assert_network_binding_rejected(AttachmentCase::AdditionalNetwork);
}
