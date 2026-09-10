//! Real rootless-Podman acceptance for the bounded command-execution backend.
//!
//! The legacy probes below retain historical diagnostics for the pre-gate adapter. Release
//! acceptance is owned by the production [`RuntimeGatePodmanAdapter`] witness, which executes the
//! same immutable hold/attest/release boundary exposed through `CommandExecutionBackend`.
//!
//! The dedicated positive-LSM CI lane invokes the production-gated timeout witness explicitly
//! after pre-pulling the digest-pinned fixture image named by `QSR_PODMAN_E2E_IMAGE`. Legacy probes
//! remain available for targeted diagnosis on a developer machine with rootless Podman installed.
//!
//! ```sh
//! QSR_PODMAN_E2E_IMAGE="docker.io/library/python@sha256:<digest>" \
//!   cargo test --test podman_command_execution_e2e -- --ignored --nocapture --test-threads=1
//! ```
//!
//! `--test-threads=1` matters when running this whole file locally: each test's leak check scans
//! every container carrying this runtime's sandbox-identity label, so parallel real-runtime probes
//! can observe each other's in-flight containers as false leaks.

use std::{
    fs,
    process::{Command, Output},
    sync::atomic::{AtomicU64, Ordering},
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

use quarantine_sandbox_runtime::{
    CommandExecutionRequest, IsolationPolicy, ResourceRequest, RootlessPodmanAdapter,
    RuntimeGateArtifact,
};
use sha2::{Digest, Sha256};

// Sandbox names are derived from request_id+image+policy_id+started_at (seconds
// granularity), and these tests run concurrently by default: without a per-call
// nonce, two tests started in the same wall-clock second would race to create a
// container with the identical deterministic name.
static NEXT_REQUEST_NONCE: AtomicU64 = AtomicU64::new(0);

fn unique_request_id() -> String {
    format!(
        "podman_e2e_command_execution-{}",
        NEXT_REQUEST_NONCE.fetch_add(1, Ordering::Relaxed)
    )
}

fn fixture_image() -> String {
    let image = std::env::var("QSR_PODMAN_E2E_IMAGE")
        .expect("QSR_PODMAN_E2E_IMAGE must name the pre-pulled digest-pinned fixture");
    assert!(
        image.contains("@sha256:") && image.len() > "@sha256:".len() + 64,
        "fixture image must be an immutable repository@sha256 reference"
    );
    image
}

fn policy() -> IsolationPolicy {
    IsolationPolicy {
        policy_id: "podman_command_e2e_policy_v1".to_owned(),
        maximum_memory_bytes: 256 * 1024 * 1024,
        maximum_cpu_millicores: 500,
        maximum_processes: 32,
        maximum_lease_seconds: 90,
        maximum_tmpfs_bytes: 32 * 1024 * 1024,
        readiness_timeout_millis: 10_000,
        readiness_poll_interval_millis: 50,
        shutdown_grace_seconds: 2,
        run_as_user_id: 65_532,
        run_as_group_id: 65_532,
    }
}

fn request(command: Vec<String>, lease_seconds: u32) -> CommandExecutionRequest {
    let limits = policy();
    CommandExecutionRequest {
        schema_version: "1.0.0".to_owned(),
        request_id: unique_request_id(),
        image_reference: fixture_image(),
        command,
        source_artifact: None,
        resources: ResourceRequest {
            memory_bytes: limits.maximum_memory_bytes,
            cpu_millicores: limits.maximum_cpu_millicores,
            maximum_processes: limits.maximum_processes,
            lease_seconds,
            tmpfs_bytes: limits.maximum_tmpfs_bytes,
        },
    }
}

fn started_at() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system time must be after the Unix epoch")
        .as_secs()
}

fn podman(args: &[&str]) -> Output {
    Command::new("podman")
        .args(args)
        .output()
        .expect("Podman should be invokable in the dedicated E2E job")
}

fn podman_stdout(args: &[&str]) -> String {
    let output = podman(args);
    assert!(
        output.status.success(),
        "Podman {:?} failed: {}",
        args,
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8(output.stdout)
        .expect("Podman acceptance output must be UTF-8")
        .trim()
        .to_owned()
}

fn assert_no_runtime_leaks() {
    let leaked_containers = podman_stdout(&[
        "ps",
        "-a",
        "--filter",
        "label=org.contextualwisdomlab.sandbox.identity",
        "--format",
        "{{.ID}}",
    ]);
    assert!(
        leaked_containers.is_empty(),
        "command execution must not leak runtime-owned containers: {leaked_containers}"
    );
}

fn production_runtime_gate() -> RuntimeGateArtifact {
    assert_eq!(
        std::env::consts::ARCH,
        "x86_64",
        "the dedicated positive-LSM runner contract currently installs the x86_64 musl target"
    );

    let build_directory = tempfile::tempdir().expect("runtime-gate build directory must exist");
    let gate_path = build_directory.path().join("qsr-runtime-gate");
    let output = Command::new("rustc")
        .args([
            "--edition=2021",
            "--target",
            "x86_64-unknown-linux-musl",
            "-O",
            "src/bin/qsr_runtime_gate.rs",
            "-o",
        ])
        .arg(&gate_path)
        .output()
        .expect("rustc must be available in the dedicated positive-LSM job");
    assert!(
        output.status.success(),
        "runtime gate must compile as a self-contained musl executable: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let gate_bytes = fs::read(&gate_path).expect("compiled runtime gate must be readable");
    let gate_sha256 = format!("{:x}", Sha256::digest(&gate_bytes));
    RuntimeGateArtifact::stage(&gate_path, &gate_sha256, std::env::consts::ARCH)
        .expect("compiled runtime gate must satisfy immutable loading admission")
}

#[test]
#[ignore = "requires a real rootless-Podman installation on PATH"]
fn command_execution_reports_exit_status_and_bounded_output() {
    assert_eq!(
        podman_stdout(&["info", "--format", "{{.Host.Security.Rootless}}"]),
        "true",
        "E2E backend must actually be rootless"
    );

    let adapter = RootlessPodmanAdapter::default();
    let request = request(
        vec![
            "python".to_owned(),
            "-B".to_owned(),
            "-c".to_owned(),
            "import sys; print('hello from sandbox'); print('warn', file=sys.stderr); \
             sys.exit(3)"
                .to_owned(),
        ],
        30,
    );

    let result = adapter
        .run_legacy_command_at_for_test(&request, &policy(), started_at())
        .expect("digest-pinned fixture must run to completion under the P0 isolation policy");

    assert_eq!(
        result.exit_code(),
        3,
        "exit status must be observed, not fabricated"
    );
    assert!(!result.timed_out());
    assert!(result.stdout().contains("hello from sandbox"));
    assert!(result.stderr().contains("warn"));
    assert_eq!(result.backend_id(), "rootless_podman");
    assert!(!result.backend_version().is_empty());
    assert_no_runtime_leaks();
}

#[test]
#[ignore = "requires a real rootless-Podman installation on PATH"]
fn command_execution_cannot_see_host_filesystem_or_reach_the_network() {
    let adapter = RootlessPodmanAdapter::default();
    // The sandbox's root filesystem is the pulled image, not the host: this
    // repository's own Cargo.toml is invisible from inside the container even
    // though the host process running this test has it on disk right next to it.
    let host_visibility_probe = request(
        vec![
            "python".to_owned(),
            "-B".to_owned(),
            "-c".to_owned(),
            "import os; sys_exit = 0 if not os.path.exists('/Cargo.toml') else 1; \
             raise SystemExit(sys_exit)"
                .to_owned(),
        ],
        30,
    );
    let host_visibility_result = adapter
        .run_legacy_command_at_for_test(&host_visibility_probe, &policy(), started_at())
        .expect("host-visibility probe must run to completion");
    assert_eq!(
        host_visibility_result.exit_code(),
        0,
        "sandbox must not see the host's own repository checkout at /Cargo.toml"
    );

    // No network namespace is attached at all: an outbound connection attempt
    // must fail, not hang or succeed.
    let egress_probe = request(
        vec![
            "python".to_owned(),
            "-B".to_owned(),
            "-c".to_owned(),
            "import socket\
try:\
    socket.create_connection(('1.1.1.1', 80), 2)\
\
             except OSError:\
    raise SystemExit(0)\
raise SystemExit(1)"
                .to_owned(),
        ],
        30,
    );
    let egress_result = adapter
        .run_legacy_command_at_for_test(&egress_probe, &policy(), started_at())
        .expect("egress probe must run to completion");
    assert_eq!(
        egress_result.exit_code(),
        0,
        "sandbox with no network namespace must not reach external hosts"
    );

    assert_no_runtime_leaks();
}

#[test]
#[ignore = "requires a real rootless-Podman installation on PATH"]
fn command_execution_kills_and_reports_a_command_that_exceeds_its_timeout() {
    let adapter = RootlessPodmanAdapter::default();
    let request = request(
        vec![
            "python".to_owned(),
            "-B".to_owned(),
            "-c".to_owned(),
            "import time; time.sleep(120)".to_owned(),
        ],
        // Below this crate's IsolationPolicy minimum-viable bound but a real,
        // small positive lease: the sandbox must be killed well before the
        // sleeping workload would exit on its own.
        3,
    );

    let started = Instant::now();
    let result = adapter
        .run_legacy_command_at_for_test(&request, &policy(), started_at())
        .expect("a killed command is still a completed, successfully-observed run");
    let elapsed = started.elapsed();

    assert!(
        result.timed_out(),
        "a command exceeding its lease must be reported as timed out, not left running"
    );
    assert!(
        elapsed < Duration::from_secs(30),
        "the sandbox must be killed near its bounded lease, not run to its own completion: \
         took {elapsed:?}"
    );
    assert_no_runtime_leaks();
}

#[test]
#[ignore = "requires the dedicated rootless-Podman positive-LSM acceptance environment"]
fn production_gated_command_execution_kills_and_reports_a_command_that_exceeds_its_timeout() {
    assert_eq!(
        podman_stdout(&["info", "--format", "{{.Host.Security.Rootless}}"]),
        "true",
        "production command E2E must execute on a rootless Podman runtime"
    );

    let adapter =
        RootlessPodmanAdapter::default().with_runtime_gate_artifact(production_runtime_gate());
    let request = request(
        vec![
            "python".to_owned(),
            "-B".to_owned(),
            "-c".to_owned(),
            "import time; time.sleep(120)".to_owned(),
        ],
        3,
    );

    let started = Instant::now();
    let result = adapter
        .run_command_at(&request, &policy(), started_at())
        .expect("production gated command runtime must kill and observe an over-lease workload");
    let elapsed = started.elapsed();

    assert!(
        result.timed_out(),
        "production gated runtime must report a workload that exceeded its lease as timed out"
    );
    assert!(
        elapsed < Duration::from_secs(30),
        "runtime-owned termination evidence must arrive well before natural workload completion: \
         took {elapsed:?}"
    );
    assert_eq!(result.backend_id(), "rootless_podman");
    assert!(!result.backend_version().is_empty());
    assert_no_runtime_leaks();
}
