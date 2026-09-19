//! RED: hostile application payload must remain held until effective isolation is attested.
//!
//! `podman start` may start a trusted runtime-owned hold process, but it must not make the
//! untrusted image command runnable before the exact acquired sandbox has passed effective
//! process/isolation verification. The fixture treats only the exact runtime-gate entrypoint plus
//! its runtime-owned read-only bind and interactive release channel as a hold. Any other
//! entrypoint still arms the hostile payload. A deliberately contradictory capability inspection
//! must then fail closed without that side effect.

#![cfg(target_os = "linux")]

use std::{
    fs,
    os::unix::fs::PermissionsExt,
    path::{Path, PathBuf},
    process::Command,
    sync::atomic::{AtomicU64, Ordering},
    time::{SystemTime, UNIX_EPOCH},
};

use quarantine_sandbox_runtime::{
    ApplicationServiceError, ApplicationServiceRequest, IsolationPolicy, ResourceRequest,
    RootlessPodmanAdapter, ServiceProtocol,
};

const STARTED_AT_EPOCH_SECONDS: u64 = 1_780_005_000;
const OWNED_NETWORK_ID: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
const OWNED_CONTAINER_ID: &str = "cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc";
const TRUSTED_GATE_CONTAINER_PATH: &str = "/qsr-runtime-gate";
static NEXT_TEMP_PATH_ID: AtomicU64 = AtomicU64::new(0);

fn temporary_path(name: &str) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("test clock must be after the Unix epoch")
        .as_nanos();
    let unique_id = NEXT_TEMP_PATH_ID.fetch_add(1, Ordering::Relaxed);
    std::env::temp_dir().join(format!(
        "qsr-application-pre-attestation-red-{name}-{}-{nanos}-{unique_id}",
        std::process::id()
    ))
}

fn policy() -> IsolationPolicy {
    IsolationPolicy {
        policy_id: "application_pre_attestation_execution_red_v1".to_owned(),
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
        request_id: "application_pre_attestation_execution_red".to_owned(),
        image_reference: format!("localhost/cwl/tool@sha256:{}", "f".repeat(64)),
        container_port: 8_080,
        protocol: ServiceProtocol::Tcp,
        command: vec!["serve".to_owned(), "--host=127.0.0.1".to_owned()],
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
    log: &Path,
    created_network_name: &Path,
    trusted_gate_path: &Path,
    direct_payload_armed: &Path,
    payload_side_effect: &Path,
) -> PathBuf {
    let program = temporary_path("fake-podman");
    let info = r#"{"host":{"security":{"rootless":true,"seccompEnabled":true,"seccompProfilePath":"/usr/share/containers/seccomp.json","apparmorEnabled":true,"selinuxEnabled":false}}}"#;
    let script = format!(
        r#"#!/bin/sh
set -eu
printf '%s\n' "$*" >> '{log}'
network_name_file='{network_name_file}'
trusted_gate_path='{trusted_gate_path}'
direct_payload_armed='{direct_payload_armed}'
payload_side_effect='{payload_side_effect}'
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
    [ "$selector" = "$network_name" ] || [ "$selector" = "$network_id" ] || exit 95
    printf '[{{"name":"%s","id":"%s","internal":true,"dns_enabled":false,"containers":{{}}}}]\n' "$network_name" "$network_id"
    ;;
  create:--name)
    cidfile=''
    network=''
    has_interactive=0
    has_trusted_entrypoint=0
    has_trusted_gate_mount=0
    previous=''
    for argument in "$@"; do
      case "$argument" in
        --cidfile=*) cidfile=${{argument#--cidfile=}} ;;
        --interactive) has_interactive=1 ;;
        --entrypoint={trusted_gate_container_path}) has_trusted_entrypoint=1 ;;
      esac
      if [ "$previous" = '--network' ]; then network="$argument"; fi
      if [ "$previous" = '--volume' ] && [ "$argument" = "$trusted_gate_path:{trusted_gate_container_path}:ro" ]; then
        has_trusted_gate_mount=1
      fi
      previous="$argument"
    done
    [ "$network" = "$network_id" ] || exit 92
    [ -n "$cidfile" ] || exit 93
    if [ "$has_interactive" -ne 1 ] || [ "$has_trusted_entrypoint" -ne 1 ] || [ "$has_trusted_gate_mount" -ne 1 ]; then
      : > "$direct_payload_armed"
    fi
    printf '%s\n' "$container_id" > "$cidfile"
    printf '%s\n' "$container_id"
    ;;
  start:*)
    if [ -f "$direct_payload_armed" ]; then
      printf 'consumer-ran-before-attestation\n' > "$payload_side_effect"
    fi
    ;;
  container:inspect)
    printf '[{{"Id":"%s","AppArmorProfile":"containers-default","ProcessLabel":"","EffectiveCaps":["CAP_NET_RAW"],"BoundingCaps":[],"Config":{{"User":"65532:65532"}},"HostConfig":{{"ReadonlyRootfs":true,"Privileged":false,"SecurityOpt":["no-new-privileges"],"UsernsMode":"auto","PidMode":"private","IpcMode":"none","Memory":134217728,"NanoCpus":250000000,"PidsLimit":16}}}}]\n' "$container_id"
    ;;
  top:*)
    printf 'PID SECCOMP CAPEFF CAPBND CAPINH CAPPRM CAPAMB LABEL\n1 filter - - - - - containers-default (enforce)\n'
    ;;
  stop:*) : ;;
  rm:--force)
    [ "${{3:-}}" = "$container_id" ] || exit 96
    ;;
  network:rm) : ;;
  port:*) exit 97 ;;
  *) exit 91 ;;
esac
"#,
        log = log.display(),
        network_name_file = created_network_name.display(),
        trusted_gate_path = trusted_gate_path.display(),
        trusted_gate_container_path = TRUSTED_GATE_CONTAINER_PATH,
        direct_payload_armed = direct_payload_armed.display(),
        payload_side_effect = payload_side_effect.display(),
        network_id = OWNED_NETWORK_ID,
        container_id = OWNED_CONTAINER_ID,
        info = info,
    );

    fs::write(&program, script).expect("fake Podman must be writable");
    let mut permissions = fs::metadata(&program)
        .expect("fake Podman metadata must exist")
        .permissions();
    permissions.set_mode(0o700);
    fs::set_permissions(&program, permissions).expect("fake Podman must be executable");
    program
}

fn run_fake_podman(program: &Path, arguments: &[String]) {
    let status = Command::new(program)
        .args(arguments)
        .status()
        .expect("fake Podman invocation must start");
    assert!(status.success(), "fake Podman invocation must succeed");
}

fn exercise_fake_create_with_entrypoint(entrypoint: &str) -> bool {
    let log = temporary_path("fixture-calls");
    let created_network_name = temporary_path("fixture-network-name");
    let trusted_gate_path = temporary_path("fixture-trusted-gate");
    let direct_payload_armed = temporary_path("fixture-direct-payload-armed");
    let payload_side_effect = temporary_path("fixture-payload-side-effect");
    let cidfile = temporary_path("fixture-cidfile");
    fs::write(&trusted_gate_path, b"trusted-runtime-gate").expect("gate fixture must be writable");
    let program = write_fake_podman(
        &log,
        &created_network_name,
        &trusted_gate_path,
        &direct_payload_armed,
        &payload_side_effect,
    );

    run_fake_podman(
        &program,
        &[
            "network".to_owned(),
            "create".to_owned(),
            "--internal".to_owned(),
            "--disable-dns".to_owned(),
            "qsr-net-fixture".to_owned(),
        ],
    );
    run_fake_podman(
        &program,
        &[
            "create".to_owned(),
            "--name".to_owned(),
            "qsr-app-fixture".to_owned(),
            format!("--cidfile={}", cidfile.display()),
            "--interactive".to_owned(),
            "--volume".to_owned(),
            format!(
                "{}:{TRUSTED_GATE_CONTAINER_PATH}:ro",
                trusted_gate_path.display()
            ),
            format!("--entrypoint={entrypoint}"),
            "--network".to_owned(),
            OWNED_NETWORK_ID.to_owned(),
            "localhost/cwl/tool@sha256:ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff"
                .to_owned(),
        ],
    );
    run_fake_podman(
        &program,
        &["start".to_owned(), OWNED_CONTAINER_ID.to_owned()],
    );

    let consumer_ran = payload_side_effect.exists();
    let _ = fs::remove_file(program);
    let _ = fs::remove_file(log);
    let _ = fs::remove_file(created_network_name);
    let _ = fs::remove_file(trusted_gate_path);
    let _ = fs::remove_file(direct_payload_armed);
    let _ = fs::remove_file(payload_side_effect);
    let _ = fs::remove_file(cidfile);
    consumer_ran
}

#[test]
fn arbitrary_entrypoint_does_not_count_as_runtime_owned_hold() {
    assert!(
        exercise_fake_create_with_entrypoint("/bin/false"),
        "an arbitrary explicit entrypoint must still arm the hostile payload in the witness"
    );
}

#[test]
fn exact_runtime_gate_binding_holds_payload_in_fixture() {
    assert!(
        !exercise_fake_create_with_entrypoint(TRUSTED_GATE_CONTAINER_PATH),
        "the fixture's exact runtime-owned gate binding must hold the payload at start"
    );
}

#[test]
fn hostile_service_payload_is_not_released_before_effective_attestation() {
    let log = temporary_path("calls");
    let created_network_name = temporary_path("network-name");
    let trusted_gate_path = temporary_path("trusted-gate");
    let direct_payload_armed = temporary_path("direct-payload-armed");
    let payload_side_effect = temporary_path("payload-side-effect");
    fs::write(&trusted_gate_path, b"trusted-runtime-gate").expect("gate fixture must be writable");
    let program = write_fake_podman(
        &log,
        &created_network_name,
        &trusted_gate_path,
        &direct_payload_armed,
        &payload_side_effect,
    );
    let adapter = RootlessPodmanAdapter::new(program.clone());

    let result = adapter.launch_at(&request(), &policy(), STARTED_AT_EPOCH_SECONDS);
    let calls = fs::read_to_string(&log).expect("fake Podman calls must be recorded");
    let consumer_ran = payload_side_effect.exists();

    let _ = fs::remove_file(program);
    let _ = fs::remove_file(log);
    let _ = fs::remove_file(created_network_name);
    let _ = fs::remove_file(trusted_gate_path);
    let _ = fs::remove_file(direct_payload_armed);
    let _ = fs::remove_file(payload_side_effect);

    assert_eq!(
        result,
        Err(ApplicationServiceError::IsolationVerificationFailed {
            control_name: "all_capabilities_dropped",
        }),
        "the witness must reach live effective isolation rejection rather than fail at an unrelated prerequisite"
    );
    assert!(
        calls
            .lines()
            .any(|line| line == format!("start {OWNED_CONTAINER_ID}")),
        "the witness must exercise the current start boundary; calls were:\n{calls}"
    );
    assert!(
        calls
            .lines()
            .any(|line| line == format!("container inspect --format json {OWNED_CONTAINER_ID}")),
        "effective isolation evidence must be queried for the exact acquired container; calls were:\n{calls}"
    );
    assert!(
        !calls.lines().any(|line| line.starts_with("port ")),
        "readiness must not run after the deliberate isolation contradiction; calls were:\n{calls}"
    );
    assert!(
        !consumer_ran,
        "the hostile application payload became runnable before effective isolation attestation; calls were:\n{calls}"
    );
}
