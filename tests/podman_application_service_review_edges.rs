//! Review-driven application-service isolation evidence edges on the canonical owner path.
//!
//! These fake-backend witnesses validate evidence cardinality and independent live capability
//! columns. They are contradiction/ACL tests only and do not substitute for positive confinement
//! evidence on a real rootless Podman host.

#![cfg(target_os = "linux")]

use std::{fs, os::unix::fs::symlink, path::PathBuf};

use quarantine_sandbox_runtime::{
    ApplicationServiceError, ApplicationServiceRequest, IsolationPolicy, ResourceRequest,
    RootlessPodmanAdapter, ServiceProtocol,
};

const CONTAINER_ID: &str = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";
const BACKEND_INFO: &str = "{\"host\":{\"security\":{\"rootless\":true,\"seccompEnabled\":true,\"seccompProfilePath\":\"/usr/share/containers/seccomp.json\",\"apparmorEnabled\":true,\"selinuxEnabled\":false}}}";
const CONTAINER_INSPECTION: &str = "[{\"Id\":\"0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef\",\"AppArmorProfile\":\"containers-default\",\"ProcessLabel\":\"\",\"EffectiveCaps\":[],\"BoundingCaps\":[],\"Config\":{\"User\":\"65532:65532\"},\"HostConfig\":{\"ReadonlyRootfs\":true,\"Privileged\":false,\"SecurityOpt\":[\"no-new-privileges\"],\"UsernsMode\":\"auto\",\"PidMode\":\"private\",\"IpcMode\":\"none\",\"Memory\":268435456,\"NanoCpus\":1000000000,\"PidsLimit\":32}}]";
const SECURE_NETWORK_INSPECTION: &str = "[{\"internal\":true,\"dns_enabled\":false}]";
const SECURE_PROCESS_TOP: &str = "PID SECCOMP CAPEFF CAPBND CAPINH CAPPRM CAPAMB LABEL\\n1 filter - - - - - containers-default (enforce)";

fn policy() -> IsolationPolicy {
    IsolationPolicy {
        policy_id: "application_service_review_edges_v1".to_owned(),
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
        request_id: "application-service-review-edges".to_owned(),
        image_reference: format!("localhost/cwl/tool@sha256:{}", "b".repeat(64)),
        container_port: 8_080,
        protocol: ServiceProtocol::Tcp,
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

fn fixture_executable() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/fake_podman.sh")
}

fn execute_with_evidence(
    name: &str,
    process_top: &str,
    network_inspection: &str,
) -> (
    Result<quarantine_sandbox_runtime::ApplicationServiceLease, ApplicationServiceError>,
    String,
) {
    let directory = tempfile::Builder::new()
        .prefix("qsr-application-service-review-")
        .tempdir()
        .expect("isolated fake-Podman directory");
    let program = directory.path().join(name);
    let scenario = directory.path().join("scenario.sh");
    let calls = directory.path().join("calls.log");
    let config = PathBuf::from(format!("{}.config", program.display()));

    symlink(fixture_executable(), &program).expect("immutable fake Podman should be linkable");
    fs::write(
        &scenario,
        format!(
            r#"#!/bin/sh
set -eu
if [ "${{1:-}}" = info ]; then
  if [ "${{3:-}}" = json ]; then printf '%s\n' '{BACKEND_INFO}'; else printf 'true\n'; fi
  exit 0
fi
case "${{1:-}}:${{2:-}}" in
  network:create) : ;;
  create:--name) printf '%s\n' '{CONTAINER_ID}' ;;
  start:*) : ;;
  container:inspect) printf '%s\n' '{CONTAINER_INSPECTION}' ;;
  top:*) printf '%b\n' '{process_top}' ;;
  network:inspect) printf '%s\n' '{network_inspection}' ;;
  port:*) printf '127.0.0.1:9\n' ;;
  stop:--time) : ;;
  rm:--force) : ;;
  network:rm) : ;;
  *) exit 91 ;;
esac
"#,
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
    .expect("fake Podman config should be writable");

    let result =
        RootlessPodmanAdapter::new(program).launch_at(&request(), &policy(), 1_780_001_000);
    let calls = fs::read_to_string(calls).expect("fake Podman calls should be recorded");
    (result, calls)
}

#[test]
fn each_live_capability_column_fails_closed_independently() {
    let header = "PID SECCOMP CAPEFF CAPBND CAPINH CAPPRM CAPAMB LABEL";
    for (name, values) in [
        ("capeff", "0x1 - - - -"),
        ("capbnd", "- 0x1 - - -"),
        ("capinh", "- - 0x1 - -"),
        ("capprm", "- - - 0x1 -"),
        ("capamb", "- - - - 0x1"),
    ] {
        let process_top = format!("{header}\\n1 filter {values} containers-default (enforce)");
        let (result, calls) = execute_with_evidence(name, &process_top, SECURE_NETWORK_INSPECTION);
        assert_eq!(
            result,
            Err(ApplicationServiceError::IsolationVerificationFailed {
                control_name: "all_capabilities_dropped",
            }),
            "a non-empty {name} column must fail closed"
        );
        assert!(calls.contains(&format!("rm --force {CONTAINER_ID}")));
        assert!(
            !calls.lines().any(|line| line.starts_with("port ")),
            "capability contradiction must fail before port/readiness evidence: {calls}"
        );
    }
}

#[test]
fn duplicate_network_inspection_fails_closed_and_cleans_exact_container() {
    let duplicate_network =
        "[{\"internal\":true,\"dns_enabled\":false},{\"internal\":false,\"dns_enabled\":true}]";
    let (result, calls) =
        execute_with_evidence("duplicate-network", SECURE_PROCESS_TOP, duplicate_network);

    assert_eq!(
        result,
        Err(ApplicationServiceError::MalformedIsolationInspection {
            operation: "network_inspect",
        })
    );
    assert!(calls.contains(&format!("rm --force {CONTAINER_ID}")));
    assert!(
        !calls
            .lines()
            .any(|line| line.starts_with("rm --force qsr-app-")),
        "generated application-service names must not become destructive authority: {calls}"
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
