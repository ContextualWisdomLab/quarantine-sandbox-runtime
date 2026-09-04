//! Regression: configured resource intent is not effective resource-bound evidence.
//!
//! The P0 profile requires bounded tmpfs and wall time. Podman inspection must
//! first bind backend-applied configuration to the exact request, and a lease
//! must still fail closed when no live runtime-enforcement proof exists. Neither
//! launch argv nor inspect configuration alone may authorize publication.

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
        "qsr-effective-resource-red-{name}-{}-{nanos}-{unique_id}",
        std::process::id()
    ))
}

fn policy() -> IsolationPolicy {
    IsolationPolicy {
        policy_id: "effective_resource_red_v1".to_owned(),
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
        request_id: "effective_resource_red".to_owned(),
        image_reference: format!("localhost/cwl/tool@sha256:{}", "c".repeat(64)),
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

fn write_fake_podman(container_json: &str, ready_port: u16) -> (PathBuf, PathBuf) {
    let program = temporary_path("fake-podman");
    let log = temporary_path("calls");
    let info = r#"{"host":{"security":{"rootless":true,"seccompEnabled":true,"seccompProfilePath":"/usr/share/containers/seccomp.json","apparmorEnabled":true,"selinuxEnabled":false}}}"#;
    let network = r#"[{"internal":true,"dns_enabled":false}]"#;
    let script = format!(
        "#!/bin/sh\nset -eu\nprintf '%s\\n' \"$*\" >> '{}'\nif [ \"${{1:-}}\" = info ]; then\n  if [ \"${{3:-}}\" = json ]; then printf '%s\\n' '{}'; else printf 'true\\n'; fi\n  exit 0\nfi\ncase \"${{1:-}}:${{2:-}}\" in\n  network:create) : ;;\n  network:inspect) printf '%s\\n' '{}' ;;\n  network:rm) : ;;\n  create:--name) printf 'fake-container-id\\n' ;;\n  container:inspect) printf '%s\\n' '{}' ;;\n  start:*) : ;;\n  top:*) printf 'PID SECCOMP CAPEFF CAPBND CAPINH CAPPRM CAPAMB LABEL\\n1 filter - - - - - containers-default (enforce)\\n' ;;\n  port:*) printf '127.0.0.1:{ready_port}\\n' ;;\n  stop:*) : ;;\n  rm:*) : ;;\n  *) exit 91 ;;\nesac\n",
        log.display(),
        info,
        network,
        container_json,
    );
    fs::write(&program, script).expect("fake Podman must be writable");
    let mut permissions = fs::metadata(&program)
        .expect("fake Podman metadata must exist")
        .permissions();
    permissions.set_mode(0o700);
    fs::set_permissions(&program, permissions).expect("fake Podman must be executable");
    (program, log)
}

fn run_effective_resource_case(
    container_json: &str,
) -> (Result<(), ApplicationServiceError>, String) {
    let listener = TcpListener::bind(("127.0.0.1", 0)).expect("loopback listener must bind");
    let ready_port = listener
        .local_addr()
        .expect("listener must expose an address")
        .port();
    let (program, log) = write_fake_podman(container_json, ready_port);
    let adapter = RootlessPodmanAdapter::new(program.clone());

    let result = adapter
        .launch_at(&request(), &policy(), 1_780_000_000)
        .map(|_| ());
    let calls = fs::read_to_string(&log).expect("fake Podman calls must be recorded");

    let _ = fs::remove_file(program);
    let _ = fs::remove_file(log);
    drop(listener);
    (result, calls)
}

fn assert_resource_attestation_failure(container_json: &str, evidence_name: &str) {
    let (result, calls) = run_effective_resource_case(container_json);

    assert_eq!(
        result,
        Err(ApplicationServiceError::IsolationVerificationFailed {
            control_name: "resource_limits",
        }),
        "{evidence_name} must fail the resource attestation"
    );
    assert!(
        calls.contains("stop --time 1"),
        "attestation failure must stop the sandbox for {evidence_name}: {calls}"
    );
    assert!(
        calls.contains("rm --force"),
        "attestation failure must remove the sandbox for {evidence_name}: {calls}"
    );
    assert!(
        calls.contains("network rm --force"),
        "attestation failure must remove the network for {evidence_name}: {calls}"
    );
    assert!(
        !calls.contains("port "),
        "publication must not be trusted before {evidence_name} is verified: {calls}"
    );
}

#[test]
fn missing_effective_tmpfs_limit_fails_before_publication_and_cleans_up() {
    let container_without_tmpfs = r#"[{"Id":"fake-container-id","AppArmorProfile":"containers-default","ProcessLabel":"","EffectiveCaps":[],"BoundingCaps":[],"Config":{"User":"65532:65532","Timeout":30},"HostConfig":{"ReadonlyRootfs":true,"Privileged":false,"SecurityOpt":["no-new-privileges"],"UsernsMode":"auto","PidMode":"private","IpcMode":"none","Memory":134217728,"NanoCpus":250000000,"PidsLimit":16}}]"#;

    assert_resource_attestation_failure(container_without_tmpfs, "applied tmpfs configuration");
}

#[test]
fn mismatched_effective_tmpfs_size_fails_before_publication_and_cleans_up() {
    let container_with_wrong_tmpfs_size = r#"[{"Id":"fake-container-id","AppArmorProfile":"containers-default","ProcessLabel":"","EffectiveCaps":[],"BoundingCaps":[],"Config":{"User":"65532:65532","Timeout":30},"HostConfig":{"ReadonlyRootfs":true,"Privileged":false,"SecurityOpt":["no-new-privileges"],"UsernsMode":"auto","PidMode":"private","IpcMode":"none","Memory":134217728,"NanoCpus":250000000,"PidsLimit":16,"Tmpfs":{"/tmp":"rw,noexec,nosuid,nodev,size=33554432"}}}]"#;

    assert_resource_attestation_failure(
        container_with_wrong_tmpfs_size,
        "request-bound tmpfs size",
    );
}

#[test]
fn missing_tmpfs_hardening_option_fails_before_publication_and_cleans_up() {
    let container_with_executable_tmpfs = r#"[{"Id":"fake-container-id","AppArmorProfile":"containers-default","ProcessLabel":"","EffectiveCaps":[],"BoundingCaps":[],"Config":{"User":"65532:65532","Timeout":30},"HostConfig":{"ReadonlyRootfs":true,"Privileged":false,"SecurityOpt":["no-new-privileges"],"UsernsMode":"auto","PidMode":"private","IpcMode":"none","Memory":134217728,"NanoCpus":250000000,"PidsLimit":16,"Tmpfs":{"/tmp":"rw,nosuid,nodev,size=16777216"}}}]"#;

    assert_resource_attestation_failure(
        container_with_executable_tmpfs,
        "non-executable hardened tmpfs options",
    );
}

#[test]
fn unbounded_effective_wall_time_fails_before_publication_and_cleans_up() {
    let container_with_unbounded_timeout = r#"[{"Id":"fake-container-id","AppArmorProfile":"containers-default","ProcessLabel":"","EffectiveCaps":[],"BoundingCaps":[],"Config":{"User":"65532:65532","Timeout":0},"HostConfig":{"ReadonlyRootfs":true,"Privileged":false,"SecurityOpt":["no-new-privileges"],"UsernsMode":"auto","PidMode":"private","IpcMode":"none","Memory":134217728,"NanoCpus":250000000,"PidsLimit":16,"Tmpfs":{"/tmp":"rw,noexec,nosuid,nodev,size=16777216"}}}]"#;

    assert_resource_attestation_failure(
        container_with_unbounded_timeout,
        "bounded applied wall-time configuration",
    );
}

#[test]
fn mismatched_positive_wall_time_fails_before_publication_and_cleans_up() {
    let container_with_wrong_timeout = r#"[{"Id":"fake-container-id","AppArmorProfile":"containers-default","ProcessLabel":"","EffectiveCaps":[],"BoundingCaps":[],"Config":{"User":"65532:65532","Timeout":31},"HostConfig":{"ReadonlyRootfs":true,"Privileged":false,"SecurityOpt":["no-new-privileges"],"UsernsMode":"auto","PidMode":"private","IpcMode":"none","Memory":134217728,"NanoCpus":250000000,"PidsLimit":16,"Tmpfs":{"/tmp":"rw,noexec,nosuid,nodev,size=16777216"}}}]"#;

    assert_resource_attestation_failure(
        container_with_wrong_timeout,
        "request-bound applied wall-time configuration",
    );
}

#[test]
fn exact_inspect_configuration_without_live_runtime_proof_fails_closed() {
    let container_with_exact_inspect_configuration = r#"[{"Id":"fake-container-id","AppArmorProfile":"containers-default","ProcessLabel":"","EffectiveCaps":[],"BoundingCaps":[],"Config":{"User":"65532:65532","Timeout":30},"HostConfig":{"ReadonlyRootfs":true,"Privileged":false,"SecurityOpt":["no-new-privileges"],"UsernsMode":"auto","PidMode":"private","IpcMode":"none","Memory":134217728,"NanoCpus":250000000,"PidsLimit":16,"Tmpfs":{"/tmp":"size=16777216,nodev,rw,nosuid,noexec"}}}]"#;

    assert_resource_attestation_failure(
        container_with_exact_inspect_configuration,
        "live kernel/runtime resource enforcement proof",
    );
}
