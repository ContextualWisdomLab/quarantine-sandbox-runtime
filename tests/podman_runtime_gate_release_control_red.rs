//! Causal RED for the controller-owned runtime-gate release channel.
//!
//! Issue #25 already proves that the runtime-owned gate can hold hostile consumer argv while
//! effective isolation is observed. This regression requires the controller to release that gate
//! through one bounded Podman attach operation addressed only by the acquired container ID. The
//! fake attach acknowledges receipt and deliberately stays alive, so a correct implementation must
//! stop supervising the attach client after the acknowledgement instead of waiting unboundedly for
//! the consumer process to terminate.

#![cfg(target_os = "linux")]

use std::{
    fs,
    os::unix::fs::symlink,
    path::{Path, PathBuf},
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use quarantine_sandbox_runtime::{
    CommandExecutionRequest, IsolationPolicy, ResourceRequest, RootlessPodmanAdapter,
    RuntimeGateArtifact,
};
use sha2::{Digest, Sha256};

const OWNED_CONTAINER_ID: &str =
    "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";

fn policy() -> IsolationPolicy {
    IsolationPolicy {
        policy_id: "runtime_gate_release_control_policy_v1".to_owned(),
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
        request_id: "runtime-gate-release-control-request".to_owned(),
        image_reference: format!("localhost/cwl/tool@sha256:{}", "a".repeat(64)),
        command: vec![
            "payload-sentinel".to_owned(),
            "argument with spaces".to_owned(),
        ],
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

fn temporary_path(name: &str) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock should be after the Unix epoch")
        .as_nanos();
    std::env::temp_dir().join(format!(
        "quarantine-sandbox-runtime-{name}-{}-{nanos}",
        std::process::id()
    ))
}

fn immutable_fixture_executable() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/fake_podman.sh")
}

fn sidecar(program: &Path, suffix: &str) -> PathBuf {
    PathBuf::from(format!("{}.{suffix}", program.display()))
}

fn write_release_backend() -> (PathBuf, PathBuf, PathBuf) {
    let program = temporary_path("gate-release-podman");
    let log = temporary_path("gate-release-log");
    let token_capture = temporary_path("gate-release-token");
    let script_path = sidecar(&program, "script");
    let config_path = sidecar(&program, "config");
    let script = r#"
case "${1:-}:${2:-}" in
  attach:--sig-proxy=false)
    [ "${3:-}" = "$OWNED_CONTAINER_ID" ] || exit 97
    IFS= read -r release_token || exit 98
    printf '%s\n' "$release_token" > "$TOKEN_CAPTURE"
    printf 'QSR_GATE_RELEASED\n'
    while :; do :; done
    ;;
  *) exit 91 ;;
esac
"#;

    symlink(immutable_fixture_executable(), &program)
        .expect("fake Podman immutable symlink should be creatable");
    fs::write(&script_path, script).expect("release scenario data should be writable");
    fs::write(
        &config_path,
        format!(
            "MODE='source_script'\nLOG='{}'\nSCRIPT='{}'\nOWNED_CONTAINER_ID='{OWNED_CONTAINER_ID}'\nTOKEN_CAPTURE='{}'\n",
            log.display(),
            script_path.display(),
            token_capture.display()
        ),
    )
    .expect("release dispatcher config should be writable");
    (program, log, token_capture)
}

fn remove_release_backend(program: PathBuf, log: PathBuf, token_capture: PathBuf) {
    let _ = fs::remove_file(sidecar(&program, "script"));
    let _ = fs::remove_file(sidecar(&program, "config"));
    let _ = fs::remove_file(program);
    let _ = fs::remove_file(log);
    let _ = fs::remove_file(token_capture);
}

#[test]
fn controller_releases_only_the_exact_acquired_container_with_one_bounded_token() {
    let (program, log, token_capture) = write_release_backend();
    let gate_source = std::env::current_exe().expect("current test executable should exist");
    let gate_bytes = fs::read(&gate_source).expect("current test executable should be readable");
    let gate_sha256 = format!("{:x}", Sha256::digest(gate_bytes));
    let gate = RuntimeGateArtifact::stage(&gate_source, &gate_sha256, std::env::consts::ARCH)
        .expect("matching runtime gate artifact should stage");
    let adapter = RootlessPodmanAdapter::new(program.clone())
        .with_command_timeout(Duration::from_millis(250))
        .with_runtime_gate_artifact(gate);
    let request = request();
    let plan = adapter
        .plan_command_binding(&request, &policy())
        .expect("valid request should produce a gate binding plan");
    let args = plan.container_create_binding_args();
    let image_index = args
        .iter()
        .position(|argument| argument == &request.image_reference)
        .expect("gate binding must retain the immutable image operand");
    let expected_release_token = args
        .get(image_index + 1)
        .expect("gate binding must carry the one-time release token")
        .clone();

    adapter
        .release_command_gate(OWNED_CONTAINER_ID, plan)
        .expect("acknowledged exact-ID gate release should complete within the bounded deadline");

    let captured = fs::read_to_string(&token_capture)
        .expect("fake Podman must record the delivered release token");
    assert_eq!(captured.trim_end(), expected_release_token);
    assert_eq!(
        fs::read_to_string(&log).expect("release invocation should be recorded"),
        format!("attach --sig-proxy=false {OWNED_CONTAINER_ID}\n")
    );

    remove_release_backend(program, log, token_capture);
}
