//! Regression coverage for bounded Podman diagnostic output.

#![cfg(target_os = "linux")]

use std::{
    fs,
    os::unix::fs::PermissionsExt,
    path::PathBuf,
    thread,
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use quarantine_sandbox_runtime::{
    ApplicationServiceError, ApplicationServiceRequest, IsolationPolicy, ResourceRequest,
    RootlessPodmanAdapter, ServiceProtocol,
};

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

fn write_noisy_podman() -> (PathBuf, PathBuf) {
    let program = temporary_path("noisy-podman");
    let pid_file = temporary_path("noisy-podman-pid");
    let script = format!(
        "#!/bin/sh\nset -eu\nif [ \"${{1:-}}\" = info ]; then\n  printf '%s\\n' \"$$\" > '{}'\n  i=0\n  while [ \"$i\" -lt 4096 ]; do\n    printf x\n    i=$((i + 1))\n  done\n  exec sleep 60\nfi\nexit 91\n",
        pid_file.display()
    );
    fs::write(&program, script).expect("fake Podman should be writable");
    let mut permissions = fs::metadata(&program)
        .expect("fake Podman metadata should exist")
        .permissions();
    permissions.set_mode(0o700);
    fs::set_permissions(&program, permissions).expect("fake Podman should be executable");
    (program, pid_file)
}

fn policy() -> IsolationPolicy {
    IsolationPolicy {
        policy_id: "bounded_podman_output_policy_v1".to_owned(),
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
        request_id: "bounded_podman_output_request".to_owned(),
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

#[test]
fn oversized_backend_output_is_bounded_killed_reaped_and_reported() {
    let (program, pid_file) = write_noisy_podman();
    let adapter = RootlessPodmanAdapter::new(program.clone())
        .with_command_timeout(Duration::from_secs(2))
        .with_command_output_limit_bytes(16);

    assert_eq!(
        adapter.launch_at(&request(), &policy(), 1_780_000_000),
        Err(ApplicationServiceError::BackendOutputLimitExceeded {
            operation: "backend_security_info",
        })
    );

    let pid = fs::read_to_string(&pid_file)
        .expect("fake Podman should record its process id")
        .trim()
        .to_owned();
    let proc_path = PathBuf::from(format!("/proc/{pid}"));
    for _ in 0..50 {
        if !proc_path.exists() {
            break;
        }
        thread::sleep(Duration::from_millis(10));
    }
    assert!(
        !proc_path.exists(),
        "output-limited Podman process must be killed and reaped"
    );

    let _ = fs::remove_file(program);
    let _ = fs::remove_file(pid_file);
}
