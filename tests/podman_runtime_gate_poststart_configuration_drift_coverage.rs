//! Runtime-gate release must fail closed when applied configuration drifts after start.
//!
//! The command lifecycle verifies applied configuration before `podman start` and again before
//! live process attestation/release. This witness returns a valid first inspection and a root-user
//! contradiction on the second inspection. The consumer must remain held, `podman top`/attach must
//! never run, and cleanup authority must stay bound to the exact runtime-owned container ID.

#![cfg(target_os = "linux")]

#[path = "support/runtime_gate_fixture.rs"]
mod runtime_gate_fixture;

use std::{fs, os::unix::fs::symlink, path::Path};

use quarantine_sandbox_runtime::{
    ApplicationServiceError, CommandExecutionError, CommandExecutionRequest, IsolationPolicy,
    ResourceRequest, RootlessPodmanAdapter, RuntimeGateArtifact,
};
use runtime_gate_fixture::write_self_contained_gate;
use sha2::{Digest, Sha256};
use tempfile::tempdir;

const OWNED_CONTAINER_ID: &str = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";

fn policy() -> IsolationPolicy {
    IsolationPolicy {
        policy_id: "runtime_gate_poststart_drift_policy_v1".to_owned(),
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
        request_id: "runtime-gate-poststart-configuration-drift".to_owned(),
        image_reference: format!("localhost/cwl/tool@sha256:{}", "a".repeat(64)),
        command: vec!["payload-sentinel".to_owned()],
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

fn write_fake_podman(
    directory: &Path,
    gate_path: &str,
) -> (std::path::PathBuf, std::path::PathBuf) {
    let immutable_fixture =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/fake_podman.sh");
    let program = directory.join("podman");
    let calls = directory.join("calls");
    let scenario = directory.join("runtime_gate_poststart_drift_scenario.sh");
    let inspect_counter = directory.join("inspect-count");
    let config = directory.join("podman.config");
    let valid_inspect = format!(
        "[{{\"Id\":\"{OWNED_CONTAINER_ID}\",\"AppArmorProfile\":\"containers-default\",\"ProcessLabel\":\"\",\"EffectiveCaps\":[],\"BoundingCaps\":[],\"Config\":{{\"User\":\"65532:65532\",\"Timeout\":20}},\"HostConfig\":{{\"ReadonlyRootfs\":true,\"Privileged\":false,\"SecurityOpt\":[\"no-new-privileges\"],\"UsernsMode\":\"auto\",\"PidMode\":\"private\",\"IpcMode\":\"none\",\"NetworkMode\":\"none\",\"UTSMode\":\"private\",\"CgroupMode\":\"private\",\"Memory\":268435456,\"NanoCpus\":1000000000,\"PidsLimit\":16,\"Tmpfs\":{{\"/tmp\":\"rw,noexec,nosuid,nodev,size=16777216\"}}}},\"Mounts\":[{{\"Source\":\"{gate_path}\",\"Destination\":\"/qsr-runtime-gate\",\"Type\":\"bind\",\"Options\":[\"ro\"],\"RW\":false}}]}}]"
    );
    let drifted_inspect = valid_inspect.replace("65532:65532", "0:0");
    let script = format!(
        r#"case "${{1:-}}:${{2:-}}" in
  info:--format)
    printf '%s\n' '{{"host":{{"security":{{"rootless":true,"seccompEnabled":true,"seccompProfilePath":"/usr/share/containers/seccomp.json","apparmorEnabled":true,"selinuxEnabled":false}}}},"version":{{"Version":"6.1.0"}}}}'
    ;;
  create:--name)
    previous=''
    for argument in "$@"; do
      if [ "$previous" = '--cidfile' ]; then printf '%s\n' '{owned_id}' > "$argument"; fi
      case "$argument" in --cidfile=*) printf '%s\n' '{owned_id}' > "${{argument#--cidfile=}}" ;; esac
      previous="$argument"
    done
    printf '%s\n' '{owned_id}'
    ;;
  init:*) : ;;
  container:inspect)
    inspect_count=0
    if [ -f '{inspect_counter}' ]; then inspect_count=$(cat '{inspect_counter}'); fi
    inspect_count=$((inspect_count + 1))
    printf '%s\n' "$inspect_count" > '{inspect_counter}'
    if [ "$inspect_count" -eq 1 ]; then
      printf '%s\n' '{valid_inspect}'
    else
      printf '%s\n' '{drifted_inspect}'
    fi
    ;;
  start:*) : ;;
  top:*|attach:--sig-proxy=false) exit 95 ;;
  rm:--force)
    [ "${{3:-}}" = '--ignore' ] && [ "${{4:-}}" = '{owned_id}' ] || exit 96
    ;;
  *) exit 91 ;;
esac
"#,
        owned_id = OWNED_CONTAINER_ID,
        inspect_counter = inspect_counter.display(),
        valid_inspect = valid_inspect,
        drifted_inspect = drifted_inspect,
    );
    fs::write(&scenario, script).expect("fake Podman scenario data should be writable");
    fs::write(
        &config,
        format!(
            "MODE=source_script\nLOG='{}'\nSCRIPT='{}'\n",
            calls.display(),
            scenario.display()
        ),
    )
    .expect("fake Podman config should be writable");
    symlink(&immutable_fixture, &program)
        .expect("fake Podman should symlink to the checked-in immutable executable");
    (program, calls)
}

#[test]
fn poststart_configuration_drift_fails_before_live_attestation_and_release() {
    let fixture = tempdir().expect("isolated post-start drift fixture should exist");
    let (gate_source, gate_bytes) = write_self_contained_gate(fixture.path());
    let gate_sha256 = format!("{:x}", Sha256::digest(&gate_bytes));
    let gate = RuntimeGateArtifact::stage(&gate_source, &gate_sha256, std::env::consts::ARCH)
        .expect("verified self-contained gate should stage");
    let gate_path = gate.path().display().to_string();
    let (program, calls_path) = write_fake_podman(fixture.path(), &gate_path);
    let adapter = RootlessPodmanAdapter::new(&program).with_runtime_gate_artifact(gate);

    let result = adapter.run_command_at(&request(), &policy(), 1_780_000_401);
    let calls = fs::read_to_string(calls_path).expect("backend calls should be recorded");

    assert_eq!(
        result,
        Err(CommandExecutionError::Backend(
            ApplicationServiceError::IsolationVerificationFailed {
                control_name: "non_root_identity",
            },
        )),
    );
    assert_eq!(
        calls
            .lines()
            .filter(|line| line.starts_with("container inspect "))
            .count(),
        2,
        "configuration must be checked before start and again before release: {calls}"
    );
    assert!(
        calls.lines().any(|line| line.starts_with("start ")),
        "the second configuration check must occur after the held gate starts: {calls}"
    );
    assert!(
        !calls
            .lines()
            .any(|line| line.starts_with("top ") || line.starts_with("attach ")),
        "configuration drift must stop the lifecycle before live attestation or token release: {calls}"
    );
    let expected_cleanup = format!("rm --force --ignore {OWNED_CONTAINER_ID}");
    let cleanup_calls = calls
        .lines()
        .filter(|line| line.starts_with("rm --force --ignore "))
        .collect::<Vec<_>>();
    assert_eq!(
        cleanup_calls,
        vec![expected_cleanup.as_str()],
        "post-start contradiction must clean only the exact acquired container ID: {calls}"
    );
}
