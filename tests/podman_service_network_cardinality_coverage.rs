//! Reject ambiguous service-network inspection before readiness evidence can be published.

#![cfg(target_os = "linux")]

use std::{fs, os::unix::fs::symlink, path::PathBuf};

use quarantine_sandbox_runtime::{
    ApplicationServiceError, ApplicationServiceRequest, IsolationPolicy, ResourceRequest,
    RootlessPodmanAdapter, ServiceProtocol,
};
use tempfile::TempDir;

const CONTAINER_ID: &str = "fake-container-id";
const BACKEND_INFO: &str = "{\"host\":{\"security\":{\"rootless\":true,\"seccompEnabled\":true,\"seccompProfilePath\":\"/usr/share/containers/seccomp.json\",\"apparmorEnabled\":true,\"selinuxEnabled\":false}},\"version\":{\"Version\":\"6.1.0\"}}";
const CONTAINER_INSPECTION: &str = "[{\"Id\":\"fake-container-id\",\"AppArmorProfile\":\"containers-default\",\"ProcessLabel\":\"\",\"EffectiveCaps\":[],\"BoundingCaps\":[],\"Config\":{\"User\":\"65532:65532\"},\"HostConfig\":{\"ReadonlyRootfs\":true,\"Privileged\":false,\"SecurityOpt\":[\"no-new-privileges\"],\"UsernsMode\":\"auto\",\"Annotations\":{},\"PidMode\":\"private\",\"IpcMode\":\"none\",\"Memory\":268435456,\"NanoCpus\":1000000000,\"PidsLimit\":32},\"Mounts\":[]}]";
const PROCESS_TOP: &str = "PID SECCOMP CAPEFF CAPBND CAPINH CAPPRM CAPAMB LABEL\\n1 filter - - - - - containers-default (enforce)";

fn policy() -> IsolationPolicy {
    IsolationPolicy {
        policy_id: "service_network_cardinality_v1".to_owned(),
        maximum_memory_bytes: 512 * 1024 * 1024,
        maximum_cpu_millicores: 2_000,
        maximum_processes: 128,
        maximum_lease_seconds: 900,
        maximum_tmpfs_bytes: 128 * 1024 * 1024,
        readiness_timeout_millis: 100,
        readiness_poll_interval_millis: 10,
        shutdown_grace_seconds: 2,
        run_as_user_id: 65_532,
        run_as_group_id: 65_532,
    }
}

fn request() -> ApplicationServiceRequest {
    ApplicationServiceRequest {
        schema_version: "1.0.0".to_owned(),
        request_id: "service-network-cardinality".to_owned(),
        image_reference: format!("localhost/cwl/tool@sha256:{}", "b".repeat(64)),
        container_port: 8_080,
        protocol: ServiceProtocol::Http,
        command: vec!["serve".to_owned()],
        resources: ResourceRequest {
            memory_bytes: 256 * 1024 * 1024,
            cpu_millicores: 1_000,
            maximum_processes: 32,
            lease_seconds: 300,
            tmpfs_bytes: 32 * 1024 * 1024,
        },
    }
}

fn fake_podman() -> (TempDir, PathBuf, PathBuf) {
    let directory = tempfile::Builder::new()
        .prefix("qsr-service-network-cardinality-")
        .tempdir()
        .expect("isolated fake-Podman directory");
    let program = directory.path().join("podman");
    let calls = directory.path().join("calls");
    let scenario = directory.path().join("scenario.sh");
    let config = PathBuf::from(format!("{}.config", program.display()));
    let dispatcher =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/fake_podman.sh");

    fs::write(
        &scenario,
        format!(
            r#"
case "${{1:-}}:${{2:-}}" in
  info:--format) printf '%s\n' '{BACKEND_INFO}' ;;
  network:create) : ;;
  create:--name) printf '%s\n' '{CONTAINER_ID}' ;;
  start:*) : ;;
  container:inspect) printf '%s\n' '{CONTAINER_INSPECTION}' ;;
  top:*) printf '%b\n' '{PROCESS_TOP}' ;;
  network:inspect) printf '%s\n' '[]' ;;
  stop:--time) : ;;
  rm:--force) : ;;
  network:rm) : ;;
  *) exit 91 ;;
esac
"#
        ),
    )
    .expect("fake Podman scenario should be writable");
    fs::write(
        &config,
        format!(
            "MODE='source_script'\nLOG='{}'\nSCRIPT='{}'\n",
            calls.display(),
            scenario.display(),
        ),
    )
    .expect("fake Podman dispatcher config should be writable");
    symlink(dispatcher, &program).expect("immutable fake Podman dispatcher should be linkable");
    (directory, program, calls)
}

#[test]
fn empty_network_inspection_fails_closed_and_cleans_started_resources() {
    let (_directory, program, calls_path) = fake_podman();
    let adapter = RootlessPodmanAdapter::new(program);

    assert_eq!(
        adapter.launch_at(&request(), &policy(), 1_780_000_000),
        Err(ApplicationServiceError::MalformedIsolationInspection {
            operation: "network_inspect",
        }),
    );

    let calls = fs::read_to_string(calls_path).expect("fake Podman calls should be recorded");
    assert!(
        calls
            .lines()
            .any(|line| line.starts_with("network inspect "))
    );
    assert!(calls.lines().any(|line| line.starts_with("stop --time 2 ")));
    assert!(
        calls
            .lines()
            .any(|line| line.starts_with("rm --force qsr-app-"))
    );
    assert!(
        calls
            .lines()
            .any(|line| line.starts_with("network rm --force qsr-net-"))
    );
    assert!(
        !calls.lines().any(|line| line.starts_with("port ")),
        "ambiguous network evidence must fail before port/readiness evidence: {calls}"
    );
}
