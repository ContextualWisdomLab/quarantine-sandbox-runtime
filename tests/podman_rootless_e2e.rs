//! Real rootless-Podman acceptance for effective application-service isolation.
//!
//! The ordinary unit suite intentionally does not require a host container runtime.
//! CI invokes this ignored test explicitly after pre-pulling the digest-pinned
//! fixture image named by `QSR_PODMAN_E2E_IMAGE`.

use std::{
    io::{Read, Write},
    net::TcpStream,
    panic::{catch_unwind, resume_unwind},
    process::{Command, Output},
    time::{SystemTime, UNIX_EPOCH},
};

use quarantine_sandbox_runtime::{
    ApplicationServiceRequest, IsolationPolicy, ResourceRequest, RootlessPodmanAdapter,
    ServiceProtocol,
};

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
        policy_id: "podman_e2e_policy_v1".to_owned(),
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

fn request() -> ApplicationServiceRequest {
    let limits = policy();
    ApplicationServiceRequest {
        schema_version: "1.0.0".to_owned(),
        request_id: "podman_e2e_application_service".to_owned(),
        image_reference: fixture_image(),
        container_port: 8_080,
        protocol: ServiceProtocol::Http,
        command: vec![
            "python".to_owned(),
            "-B".to_owned(),
            "-m".to_owned(),
            "http.server".to_owned(),
            "8080".to_owned(),
            "--bind".to_owned(),
            "0.0.0.0".to_owned(),
        ],
        resources: ResourceRequest {
            memory_bytes: limits.maximum_memory_bytes,
            cpu_millicores: limits.maximum_cpu_millicores,
            maximum_processes: limits.maximum_processes,
            lease_seconds: limits.maximum_lease_seconds,
            tmpfs_bytes: limits.maximum_tmpfs_bytes,
        },
    }
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

fn assert_podman_fails(args: &[&str], reason: &str) {
    let output = podman(args);
    assert!(
        !output.status.success(),
        "{reason}; Podman {:?} unexpectedly succeeded: {}",
        args,
        String::from_utf8_lossy(&output.stdout)
    );
}

fn assert_http_ready(port: u16) {
    let mut stream = TcpStream::connect(("127.0.0.1", port))
        .expect("leased loopback endpoint must remain reachable");
    stream
        .write_all(b"GET / HTTP/1.0\r\nHost: localhost\r\n\r\n")
        .expect("HTTP probe request should be writable");
    let mut response = String::new();
    stream
        .read_to_string(&mut response)
        .expect("HTTP probe response should be readable UTF-8");
    assert!(
        response.starts_with("HTTP/1.0 200") || response.starts_with("HTTP/1.1 200"),
        "isolated fixture did not return HTTP 200: {response:?}"
    );
}

#[test]
#[ignore = "requires the dedicated rootless-Podman CI acceptance environment"]
fn rootless_podman_effective_isolation_and_cleanup() {
    let rootless = podman_stdout(&["info", "--format", "{{.Host.Security.Rootless}}"]);
    assert_eq!(rootless, "true", "E2E backend must actually be rootless");

    let adapter = RootlessPodmanAdapter::default();
    let policy = policy();
    let request = request();
    let started_at = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system time must be after the Unix epoch")
        .as_secs();
    let lease = adapter
        .launch_at(&request, &policy, started_at)
        .expect("digest-pinned fixture must launch under the P0 isolation policy");

    assert_eq!(lease.endpoint().host(), "127.0.0.1");
    assert_http_ready(lease.endpoint().port());
    let sandbox = lease.sandbox_id().to_owned();
    let network = lease.network_id().to_owned();

    let verification = catch_unwind(|| {
        assert_podman_fails(
            &["exec", &sandbox, "sh", "-c", "touch /qsr-root-write-test"],
            "read-only root filesystem must reject writes",
        );
        let tmp_write = podman(&[
            "exec",
            &sandbox,
            "sh",
            "-c",
            "touch /tmp/qsr-tmp-write-test",
        ]);
        assert!(
            tmp_write.status.success(),
            "bounded tmpfs must remain writable: {}",
            String::from_utf8_lossy(&tmp_write.stderr)
        );

        let cap_eff = podman_stdout(&[
            "exec",
            &sandbox,
            "sh",
            "-c",
            "awk '/^CapEff:/ {print $2}' /proc/1/status",
        ]);
        assert!(
            cap_eff.bytes().all(|byte| byte == b'0'),
            "all effective Linux capabilities must be dropped, got {cap_eff}"
        );

        assert_podman_fails(
            &[
                "exec",
                &sandbox,
                "sh",
                "-c",
                "env | grep -q '^QSR_HOST_SECRET_CANARY='",
            ],
            "host secret canary must not become ambient container environment",
        );
        assert_podman_fails(
            &[
                "exec",
                &sandbox,
                "python",
                "-B",
                "-c",
                "import socket; socket.create_connection(('1.1.1.1', 80), 0.5)",
            ],
            "standard profile must not have externally routed egress",
        );

        let memory_max = podman_stdout(&["exec", &sandbox, "cat", "/sys/fs/cgroup/memory.max"]);
        assert_eq!(memory_max, request.resources.memory_bytes.to_string());
        let pids_max = podman_stdout(&["exec", &sandbox, "cat", "/sys/fs/cgroup/pids.max"]);
        assert_eq!(pids_max, request.resources.maximum_processes.to_string());
        let cpu_max = podman_stdout(&["exec", &sandbox, "cat", "/sys/fs/cgroup/cpu.max"]);
        let mut cpu_parts = cpu_max.split_ascii_whitespace();
        let quota = cpu_parts
            .next()
            .expect("cpu.max must contain quota")
            .parse::<u64>()
            .expect("CPU quota must be numeric");
        let period = cpu_parts
            .next()
            .expect("cpu.max must contain period")
            .parse::<u64>()
            .expect("CPU period must be numeric");
        assert!(
            cpu_parts.next().is_none(),
            "cpu.max must have exactly two values"
        );
        assert_eq!(
            quota * 1_000,
            period * u64::from(request.resources.cpu_millicores),
            "effective cgroup CPU quota must match requested millicores"
        );
    });

    let receipt = adapter
        .terminate_at(&lease, started_at + 1)
        .expect("termination must prove container and network cleanup");
    assert!(receipt.container_removed());
    assert!(receipt.network_removed());
    assert_podman_fails(
        &["container", "exists", &sandbox],
        "terminated sandbox container must not remain present",
    );
    assert_podman_fails(
        &["network", "exists", &network],
        "terminated per-sandbox network must not remain present",
    );

    if let Err(payload) = verification {
        resume_unwind(payload);
    }
}
