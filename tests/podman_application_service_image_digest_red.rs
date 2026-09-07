//! RED: application-service launch must bind the applied image digest to the request.
//!
//! The request contract already requires an immutable lower-case SHA-256 image
//! reference and the Podman plan uses `--pull=never`. Current service isolation
//! verification ignores `container inspect .ImageDigest`, so an otherwise-positive
//! container created from a different image can still reach loopback publication
//! and become lease evidence. Production remains unchanged until this RED executes
//! for that exact image-identity cause.

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

const STARTED_AT_EPOCH_SECONDS: u64 = 1_780_000_300;
const CONTAINER_ID: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
const EXPECTED_DIGEST: &str =
    "eeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeee";
const DIFFERENT_DIGEST: &str =
    "dddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddd";
static NEXT_TEMP_PATH_ID: AtomicU64 = AtomicU64::new(0);

fn temporary_path(name: &str) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("test clock must be after the Unix epoch")
        .as_nanos();
    let unique_id = NEXT_TEMP_PATH_ID.fetch_add(1, Ordering::Relaxed);
    std::env::temp_dir().join(format!(
        "qsr-app-image-digest-red-{name}-{}-{nanos}-{unique_id}",
        std::process::id()
    ))
}

fn policy() -> IsolationPolicy {
    IsolationPolicy {
        policy_id: "application_image_digest_red_v1".to_owned(),
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
        request_id: "application_image_digest_red".to_owned(),
        image_reference: format!("localhost/cwl/tool@sha256:{EXPECTED_DIGEST}"),
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
    let calls = temporary_path("calls");
    let info = r#"{"host":{"security":{"rootless":true,"seccompEnabled":true,"seccompProfilePath":"/usr/share/containers/seccomp.json","apparmorEnabled":true,"selinuxEnabled":false}}}"#;
    let network = r#"[{"internal":true,"dns_enabled":false}]"#;
    let top = "PID SECCOMP CAPEFF CAPBND CAPINH CAPPRM CAPAMB LABEL\n1 filter - - - - - containers-default (enforce)\n";
    let script = format!(
        "#!/bin/sh\nset -eu\nprintf '%s\\n' \"$*\" >> '{}'\nif [ \"${{1:-}}\" = info ]; then\n  if [ \"${{3:-}}\" = json ]; then printf '%s\\n' '{}'; else printf 'true\\n'; fi\n  exit 0\nfi\ncase \"${{1:-}}:${{2:-}}\" in\n  network:create) : ;;\n  network:inspect) printf '%s\\n' '{}' ;;\n  network:rm) : ;;\n  create:--name) printf '%s\\n' '{}' ;;\n  container:inspect) printf '%s\\n' '{}' ;;\n  start:*) : ;;\n  top:*) printf '%s' '{}' ;;\n  port:*) printf '127.0.0.1:{ready_port}\\n' ;;\n  stop:*) : ;;\n  rm:*) : ;;\n  *) exit 91 ;;\nesac\n",
        calls.display(),
        info,
        network,
        CONTAINER_ID,
        container_json,
        top,
    );
    fs::write(&program, script).expect("fake Podman must be writable");
    let mut permissions = fs::metadata(&program)
        .expect("fake Podman metadata must exist")
        .permissions();
    permissions.set_mode(0o700);
    fs::set_permissions(&program, permissions).expect("fake Podman must be executable");
    (program, calls)
}

#[test]
fn mismatched_applied_image_digest_fails_before_publication_and_cleans_up() {
    let request = request();
    let policy = policy();
    let plan = RootlessPodmanAdapter::plan_at(&request, &policy, STARTED_AT_EPOCH_SECONDS)
        .expect("valid request and policy must yield a launch plan");
    let expected_sandbox = plan.sandbox_name().to_owned();
    let expected_network = plan.network_name().to_owned();

    let listener = TcpListener::bind(("127.0.0.1", 0)).expect("loopback listener must bind");
    let ready_port = listener
        .local_addr()
        .expect("listener must expose an address")
        .port();
    let container_json = format!(
        r#"[{{"Id":"{CONTAINER_ID}","ImageDigest":"sha256:{DIFFERENT_DIGEST}","ImageName":"localhost/cwl/tool@sha256:{DIFFERENT_DIGEST}","AppArmorProfile":"containers-default","ProcessLabel":"","EffectiveCaps":[],"BoundingCaps":[],"Config":{{"User":"65532:65532"}},"HostConfig":{{"ReadonlyRootfs":true,"Privileged":false,"SecurityOpt":["no-new-privileges"],"UsernsMode":"auto","PidMode":"private","IpcMode":"none","NetworkMode":"{expected_network}","UTSMode":"private","CgroupMode":"private","Memory":134217728,"NanoCpus":250000000,"PidsLimit":16}},"Mounts":[]}}]"#
    );
    let (program, calls_path) = write_fake_podman(&container_json, ready_port);
    let adapter = RootlessPodmanAdapter::new(program.clone());

    let result = adapter
        .launch_at(&request, &policy, STARTED_AT_EPOCH_SECONDS)
        .map(|_| ());
    let calls = fs::read_to_string(&calls_path).expect("fake Podman calls must be recorded");

    let _ = fs::remove_file(program);
    let _ = fs::remove_file(calls_path);
    drop(listener);

    assert_eq!(
        result,
        Err(ApplicationServiceError::IsolationVerificationFailed {
            control_name: "immutable_image_identity",
        }),
        "a service container whose applied image digest differs from the immutable request must never become lease evidence"
    );
    for expected in [
        format!("network create --internal --disable-dns {expected_network}"),
        format!("container inspect --format json {expected_sandbox}"),
        format!("stop --time 1 {expected_sandbox}"),
        format!("rm --force {expected_sandbox}"),
        format!("network rm --force {expected_network}"),
    ] {
        assert!(
            calls.lines().any(|line| line == expected),
            "missing exact runtime-owned operation {expected}: {calls}"
        );
    }
    assert!(
        !calls.lines().any(|line| line.starts_with("port ")),
        "port publication must not be queried before applied image identity is verified: {calls}"
    );
}
