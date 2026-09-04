//! Process-boundary security regressions for effective application-service isolation.

#![cfg(target_os = "linux")]

use std::{
    fs,
    net::TcpListener,
    os::unix::fs::PermissionsExt,
    path::PathBuf,
    time::{SystemTime, UNIX_EPOCH},
};

use quarantine_sandbox_runtime::{
    ApplicationServiceError, ApplicationServiceRequest, IsolationPolicy, ResourceRequest,
    RootlessPodmanAdapter, ServiceProtocol,
};

fn digest_image() -> String {
    format!("localhost/cwl/tool@sha256:{}", "d".repeat(64))
}

fn policy() -> IsolationPolicy {
    IsolationPolicy {
        policy_id: "effective_isolation_policy_v1".to_owned(),
        maximum_memory_bytes: 512 * 1024 * 1024,
        maximum_cpu_millicores: 2_000,
        maximum_processes: 128,
        maximum_lease_seconds: 900,
        maximum_tmpfs_bytes: 128 * 1024 * 1024,
        readiness_timeout_millis: 500,
        readiness_poll_interval_millis: 10,
        shutdown_grace_seconds: 2,
        run_as_user_id: 65_532,
        run_as_group_id: 65_532,
    }
}

fn request() -> ApplicationServiceRequest {
    ApplicationServiceRequest {
        schema_version: "1.0.0".to_owned(),
        request_id: "effective_isolation_request".to_owned(),
        image_reference: digest_image(),
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

fn write_fake_podman(mode: &str, ready_port: u16) -> (PathBuf, PathBuf) {
    let program = temporary_path("effective-isolation-podman");
    let log = temporary_path("effective-isolation-log");
    let host_security = if mode == "seccomp_disabled" {
        r#"{"host":{"security":{"rootless":true,"seccompEnabled":false,"seccompProfilePath":"","apparmorEnabled":true,"selinuxEnabled":false}},"version":{"Version":"5.6.2"}}"#
    } else if mode == "lsm_disabled" {
        r#"{"host":{"security":{"rootless":true,"seccompEnabled":true,"seccompProfilePath":"/usr/share/containers/seccomp.json","apparmorEnabled":false,"selinuxEnabled":false}},"version":{"Version":"5.6.2"}}"#
    } else {
        r#"{"host":{"security":{"rootless":true,"seccompEnabled":true,"seccompProfilePath":"/usr/share/containers/seccomp.json","apparmorEnabled":true,"selinuxEnabled":false}},"version":{"Version":"5.6.2"}}"#
    };
    let readonly_rootfs = if mode == "readwrite_root" {
        "false"
    } else {
        "true"
    };
    let internal_network = if mode == "external_network" {
        "false"
    } else {
        "true"
    };
    let bounding_caps = if mode == "bounding_caps_present" {
        r#"["CAP_SYS_ADMIN"]"#
    } else {
        "[]"
    };
    let apparmor_profile = if mode == "lsm_unconfined" {
        "unconfined"
    } else {
        "containers-default"
    };
    let top_label = if mode == "lsm_unconfined" {
        "unconfined"
    } else if mode == "lsm_mismatch" {
        "unexpected-profile"
    } else {
        "containers-default"
    };
    let script = format!(
        "#!/bin/sh\nset -eu\nprintf '%s\\n' \"$*\" >> '{}'\ncase \"${{1:-}}:${{2:-}}\" in\n  info:--format) printf '%s\\n' '{}' ;;\n  network:create) : ;;\n  create:--name) printf 'fake-container-id\\n' ;;\n  start:*) : ;;\n  top:*) printf 'PID SECCOMP CAPEFF CAPBND CAPINH CAPPRM CAPAMB LABEL\\n1 filter - - - - - {top_label}\\n' ;;\n  port:*) printf '127.0.0.1:{ready_port}\\n' ;;\n  container:inspect) printf '%s\\n' '[{{\"Id\":\"fake-container-id\",\"AppArmorProfile\":\"{apparmor_profile}\",\"ProcessLabel\":\"\",\"EffectiveCaps\":[],\"BoundingCaps\":{bounding_caps},\"Config\":{{\"User\":\"65532:65532\"}},\"HostConfig\":{{\"ReadonlyRootfs\":{readonly_rootfs},\"Privileged\":false,\"SecurityOpt\":[\"no-new-privileges\"],\"UsernsMode\":\"auto\",\"PidMode\":\"private\",\"IpcMode\":\"none\",\"Memory\":268435456,\"NanoCpus\":1000000000,\"PidsLimit\":32}}}}]' ;;\n  network:inspect) printf '%s\\n' '[{{\"internal\":{internal_network},\"dns_enabled\":false}}]' ;;\n  stop:*) : ;;\n  rm:*) : ;;\n  network:rm) : ;;\n  *) exit 91 ;;\nesac\n",
        log.display(),
        host_security,
    );
    fs::write(&program, script).expect("fake Podman should be writable");
    let mut permissions = fs::metadata(&program)
        .expect("fake Podman metadata should exist")
        .permissions();
    permissions.set_mode(0o700);
    fs::set_permissions(&program, permissions).expect("fake Podman should be executable");
    (program, log)
}

fn remove_fixture(program: PathBuf, log: PathBuf) {
    let _ = fs::remove_file(program);
    let _ = fs::remove_file(log);
}

#[test]
fn lease_attests_only_effective_controls_verified_after_start() {
    let listener = TcpListener::bind(("127.0.0.1", 0)).expect("loopback listener should bind");
    let ready_port = listener
        .local_addr()
        .expect("listener address should resolve")
        .port();
    let (program, log) = write_fake_podman("success", ready_port);
    let adapter = RootlessPodmanAdapter::new(program.clone());

    let lease = adapter
        .launch_at(&request(), &policy(), 1_780_000_000)
        .expect("verified effective isolation should produce a lease");
    assert_eq!(lease.schema_version(), "1.2.0");
    let attestation = lease.isolation_attestation();
    assert!(attestation.rootless());
    assert!(attestation.read_only_root_filesystem());
    assert!(attestation.all_capabilities_dropped());
    assert!(attestation.no_new_privileges());
    assert!(attestation.isolated_user_namespace());
    assert!(attestation.external_egress_denied());
    assert!(attestation.loopback_only_publication());
    assert!(attestation.seccomp_enforced());
    assert!(attestation.lsm_enforced());
    assert!(attestation.resource_limits_verified());
    assert!(!attestation.credentials_available());

    let calls = fs::read_to_string(&log).expect("fake Podman calls should be recorded");
    assert!(calls.contains("info --format json"));
    assert!(calls.contains("container inspect --format json"));
    assert!(calls.contains("top "));
    assert!(calls.contains("network inspect --format json"));

    remove_fixture(program, log);
    drop(listener);
}

#[test]
fn missing_host_seccomp_or_lsm_fails_before_resources_are_created() {
    for (mode, control_name) in [("seccomp_disabled", "seccomp"), ("lsm_disabled", "lsm")] {
        let (program, log) = write_fake_podman(mode, 49_152);
        let adapter = RootlessPodmanAdapter::new(program.clone());
        assert_eq!(
            adapter.launch_at(&request(), &policy(), 1_780_000_000),
            Err(ApplicationServiceError::IsolationVerificationFailed { control_name })
        );
        let calls = fs::read_to_string(&log).expect("probe call should be recorded");
        assert!(!calls.contains("network create"));
        remove_fixture(program, log);
    }
}

#[test]
fn unconfined_lsm_profile_fails_closed_and_cleanup() {
    let listener = TcpListener::bind(("127.0.0.1", 0)).expect("loopback listener should bind");
    let ready_port = listener
        .local_addr()
        .expect("listener address should resolve")
        .port();
    let (program, log) = write_fake_podman("lsm_unconfined", ready_port);
    let adapter = RootlessPodmanAdapter::new(program.clone());

    assert_eq!(
        adapter.launch_at(&request(), &policy(), 1_780_000_000),
        Err(ApplicationServiceError::IsolationVerificationFailed {
            control_name: "lsm"
        })
    );
    let calls = fs::read_to_string(&log).expect("cleanup calls should be recorded");
    assert!(calls.contains("stop --time 2"));
    assert!(calls.contains("rm --force"));
    assert!(calls.contains("network rm --force"));

    remove_fixture(program, log);
    drop(listener);
}

#[test]
fn contradictory_lsm_evidence_fails_closed_and_cleanup() {
    let listener = TcpListener::bind(("127.0.0.1", 0)).expect("loopback listener should bind");
    let ready_port = listener
        .local_addr()
        .expect("listener address should resolve")
        .port();
    let (program, log) = write_fake_podman("lsm_mismatch", ready_port);
    let adapter = RootlessPodmanAdapter::new(program.clone());

    assert_eq!(
        adapter.launch_at(&request(), &policy(), 1_780_000_000),
        Err(ApplicationServiceError::IsolationVerificationFailed {
            control_name: "lsm"
        })
    );
    let calls = fs::read_to_string(&log).expect("cleanup calls should be recorded");
    assert!(calls.contains("stop --time 2"));
    assert!(calls.contains("rm --force"));
    assert!(calls.contains("network rm --force"));

    remove_fixture(program, log);
    drop(listener);
}

#[test]
fn bounding_capabilities_fail_closed_and_cleanup() {
    let listener = TcpListener::bind(("127.0.0.1", 0)).expect("loopback listener should bind");
    let ready_port = listener
        .local_addr()
        .expect("listener address should resolve")
        .port();
    let (program, log) = write_fake_podman("bounding_caps_present", ready_port);
    let adapter = RootlessPodmanAdapter::new(program.clone());

    assert_eq!(
        adapter.launch_at(&request(), &policy(), 1_780_000_000),
        Err(ApplicationServiceError::IsolationVerificationFailed {
            control_name: "all_capabilities_dropped"
        })
    );
    let calls = fs::read_to_string(&log).expect("cleanup calls should be recorded");
    assert!(calls.contains("stop --time 2"));
    assert!(calls.contains("rm --force"));
    assert!(calls.contains("network rm --force"));

    remove_fixture(program, log);
    drop(listener);
}

#[test]
fn weaker_effective_container_or_network_state_fails_and_cleans_up() {
    let listener = TcpListener::bind(("127.0.0.1", 0)).expect("loopback listener should bind");
    let ready_port = listener
        .local_addr()
        .expect("listener address should resolve")
        .port();

    for (mode, control_name) in [
        ("readwrite_root", "read_only_root_filesystem"),
        ("external_network", "external_egress_denied"),
    ] {
        let (program, log) = write_fake_podman(mode, ready_port);
        let adapter = RootlessPodmanAdapter::new(program.clone());
        assert_eq!(
            adapter.launch_at(&request(), &policy(), 1_780_000_000),
            Err(ApplicationServiceError::IsolationVerificationFailed { control_name })
        );
        let calls = fs::read_to_string(&log).expect("cleanup calls should be recorded");
        assert!(calls.contains("stop --time 2"));
        assert!(calls.contains("rm --force"));
        assert!(calls.contains("network rm --force"));
        remove_fixture(program, log);
    }

    drop(listener);
}
