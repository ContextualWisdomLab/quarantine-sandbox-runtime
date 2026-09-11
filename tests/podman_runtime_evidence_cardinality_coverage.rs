//! Reject ambiguous or incomplete Podman runtime evidence before consumer release.

#![cfg(target_os = "linux")]

use std::{
    fs,
    os::unix::fs::symlink,
    path::{Path, PathBuf},
};

use quarantine_sandbox_runtime::{
    ApplicationServiceError, CommandExecutionError, CommandExecutionRequest, IsolationPolicy,
    ResourceRequest, RootlessPodmanAdapter,
};

const BACKEND_INFO: &str = "{\"host\":{\"security\":{\"rootless\":true,\"seccompEnabled\":true,\"seccompProfilePath\":\"/usr/share/containers/seccomp.json\",\"apparmorEnabled\":true,\"selinuxEnabled\":false}},\"version\":{\"Version\":\"6.1.0\"}}";
const CONTAINER_INSPECTION: &str = "[{\"Id\":\"fake-command-container-id\",\"AppArmorProfile\":\"containers-default\",\"ProcessLabel\":\"\",\"EffectiveCaps\":null,\"BoundingCaps\":null,\"Config\":{\"User\":\"65532:65532\",\"Timeout\":20},\"HostConfig\":{\"ReadonlyRootfs\":true,\"Privileged\":false,\"SecurityOpt\":[\"no-new-privileges\"],\"UsernsMode\":\"auto\",\"Annotations\":{},\"PidMode\":\"private\",\"IpcMode\":\"none\",\"NetworkMode\":\"none\",\"UTSMode\":\"private\",\"CgroupMode\":\"private\",\"Memory\":268435456,\"NanoCpus\":1000000000,\"PidsLimit\":16,\"Tmpfs\":{\"/tmp\":\"rw,noexec,nosuid,nodev,size=16777216\"}},\"Mounts\":[]}]";

fn policy() -> IsolationPolicy {
    IsolationPolicy {
        policy_id: "runtime_evidence_cardinality_v1".to_owned(),
        maximum_memory_bytes: 512 * 1024 * 1024,
        maximum_cpu_millicores: 2_000,
        maximum_processes: 64,
        maximum_lease_seconds: 300,
        maximum_tmpfs_bytes: 64 * 1024 * 1024,
        readiness_timeout_millis: 1_000,
        readiness_poll_interval_millis: 10,
        shutdown_grace_seconds: 2,
        run_as_user_id: 65_532,
        run_as_group_id: 65_532,
    }
}

fn request() -> CommandExecutionRequest {
    CommandExecutionRequest {
        schema_version: "1.0.0".to_owned(),
        request_id: "runtime-evidence-cardinality".to_owned(),
        image_reference: format!("localhost/cwl/tool@sha256:{}", "d".repeat(64)),
        command: vec!["true".to_owned()],
        source_artifact: None,
        resources: ResourceRequest {
            memory_bytes: 256 * 1024 * 1024,
            cpu_millicores: 1_000,
            maximum_processes: 16,
            lease_seconds: 20,
            tmpfs_bytes: 16 * 1024 * 1024,
        },
    }
}

fn fixture_executable() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/fake_podman.sh")
}

fn config_path(program: &Path) -> PathBuf {
    PathBuf::from(format!("{}.config", program.display()))
}

fn execute_with_evidence(
    name: &str,
    container_inspection: &str,
    process_top: &str,
) -> Result<quarantine_sandbox_runtime::CommandExecutionResult, CommandExecutionError> {
    let directory = tempfile::Builder::new()
        .prefix("qsr-runtime-evidence-cardinality-")
        .tempdir()
        .expect("isolated fake-Podman directory");
    let program = directory.path().join(name);
    let scenario = directory.path().join("scenario.sh");
    symlink(fixture_executable(), &program).expect("fake Podman symlink must be creatable");

    fs::write(
        &scenario,
        format!(
            r#"#!/bin/sh
set -eu
case "${{1:-}}:${{2:-}}" in
  info:--format) printf '%s\n' '{BACKEND_INFO}' ;;
  create:*) printf '%s\n' 'fake-command-container-id' ;;
  init:*) : ;;
  container:inspect) printf '%s\n' '{container_inspection}' ;;
  start:*) : ;;
  top:*) printf '%s\n' '{process_top}' ;;
  rm:--force) : ;;
  *) exit 91 ;;
esac
"#,
        ),
    )
    .expect("fake Podman scenario must be writable");
    fs::write(
        config_path(&program),
        format!(
            "MODE='source_script'\nLOG='/dev/null'\nSCRIPT='{}'\n",
            scenario.display()
        ),
    )
    .expect("fake Podman config must be writable");

    RootlessPodmanAdapter::new(program).run_legacy_command_at_for_test(
        &request(),
        &policy(),
        1_780_000_500,
    )
}

fn malformed(operation: &'static str) -> CommandExecutionError {
    CommandExecutionError::Backend(ApplicationServiceError::MalformedIsolationInspection {
        operation,
    })
}

fn isolation_failure(control_name: &'static str) -> CommandExecutionError {
    CommandExecutionError::Backend(ApplicationServiceError::IsolationVerificationFailed {
        control_name,
    })
}

#[test]
fn container_inspection_requires_exactly_one_runtime_record() {
    assert_eq!(
        execute_with_evidence("empty-inspection", "[]", "unused"),
        Err(malformed("container_inspect"))
    );
}

#[test]
fn container_inspection_must_match_the_acquired_runtime_identity() {
    let foreign_identity =
        CONTAINER_INSPECTION.replace("fake-command-container-id", "foreign-command-container-id");
    assert_eq!(
        execute_with_evidence("foreign-inspection-id", &foreign_identity, "unused"),
        Err(malformed("container_inspect")),
        "inspection evidence for a foreign container must not attest the acquired runtime identity"
    );
}

#[test]
fn configured_effective_capabilities_fail_closed() {
    let nonempty_effective =
        CONTAINER_INSPECTION.replace("\"EffectiveCaps\":null", "\"EffectiveCaps\":[\"CAP_NET_ADMIN\"]");
    let process_top =
        "PID SECCOMP CAPEFF CAPBND CAPINH CAPPRM CAPAMB LABEL\n1 filter - - - - - containers-default (enforce)";
    assert_eq!(
        execute_with_evidence("configured-effective-cap", &nonempty_effective, process_top),
        Err(isolation_failure("all_capabilities_dropped")),
        "configured effective capabilities must not be treated as fully dropped"
    );
}

#[test]
fn configured_bounding_capabilities_fail_closed() {
    let nonempty_bounding =
        CONTAINER_INSPECTION.replace("\"BoundingCaps\":null", "\"BoundingCaps\":[\"CAP_NET_ADMIN\"]");
    let process_top =
        "PID SECCOMP CAPEFF CAPBND CAPINH CAPPRM CAPAMB LABEL\n1 filter - - - - - containers-default (enforce)";
    assert_eq!(
        execute_with_evidence("configured-bounding-cap", &nonempty_bounding, process_top),
        Err(isolation_failure("all_capabilities_dropped")),
        "configured bounding capabilities must not be treated as fully dropped"
    );
}

#[test]
fn process_security_top_requires_one_complete_pid_one_record() {
    let header = "PID SECCOMP CAPEFF CAPBND CAPINH CAPPRM CAPAMB LABEL";
    let non_pid_one = format!("{header}\n2 filter - - - - - containers-default (enforce)");
    assert_eq!(
        execute_with_evidence("missing-pid-one", CONTAINER_INSPECTION, &non_pid_one),
        Err(malformed("process_security_top"))
    );

    let short_pid_one = format!("{header}\n1 filter -");
    assert_eq!(
        execute_with_evidence("short-pid-one", CONTAINER_INSPECTION, &short_pid_one),
        Err(malformed("process_security_top"))
    );

    let duplicate_pid_one = format!(
        "{header}\n1 filter - - - - - containers-default (enforce)\n1 filter - - - - - containers-default (enforce)"
    );
    assert_eq!(
        execute_with_evidence(
            "duplicate-pid-one",
            CONTAINER_INSPECTION,
            &duplicate_pid_one,
        ),
        Err(malformed("process_security_top"))
    );
}
