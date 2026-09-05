//! Real rootless-Podman acceptance for the bounded command-execution backend.
//!
//! Mirrors `tests/podman_rootless_e2e.rs`'s acceptance shape for the service-lease
//! backend, but for `RootlessPodmanAdapter::run_command_at`: run one command to
//! completion inside an isolated sandbox and prove the isolation properties the
//! contract exists to guarantee -- no host filesystem visibility beyond the
//! container's own root, no external network egress, and a forced kill (not a
//! hang) when the command exceeds its bounded wall-clock budget.
//!
//! CI invokes these ignored tests explicitly after pre-pulling the digest-pinned
//! fixture image named by `QSR_PODMAN_E2E_IMAGE`. They can also be run directly on
//! any developer machine with rootless Podman installed and reachable on `PATH`:
//!
//! ```sh
//! QSR_PODMAN_E2E_IMAGE="docker.io/library/python@sha256:<digest>" \
//!   cargo test --test podman_command_execution_e2e -- --ignored --nocapture --test-threads=1
//! ```
//!
//! `--test-threads=1` matters only for running this whole file locally: each
//! test's own leak check scans every container carrying this runtime's
//! sandbox-identity label, so two of these tests racing in parallel can see
//! each other's still-in-flight container as a false "leak". CI does not hit
//! this -- it invokes one `--exact` test per job step, matching
//! `tests/podman_rootless_e2e.rs`'s existing pattern.

use std::{
    process::{Command, Output},
    sync::atomic::{AtomicU64, Ordering},
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

use quarantine_sandbox_runtime::{
    CommandExecutionRequest, IsolationPolicy, ResourceRequest, RootlessPodmanAdapter,
};

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
        .run_command_at(&request, &policy(), started_at())
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
        .run_command_at(&host_visibility_probe, &policy(), started_at())
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
            "import socket\ntry:\n    socket.create_connection(('1.1.1.1', 80), 2)\n\
             except OSError:\n    raise SystemExit(0)\nraise SystemExit(1)"
                .to_owned(),
        ],
        30,
    );
    let egress_result = adapter
        .run_command_at(&egress_probe, &policy(), started_at())
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
        .run_command_at(&request, &policy(), started_at())
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
