//! Deterministic process-boundary coverage for Podman inspection and isolation failures.
//!
//! These fixtures exercise backend parsing and fail-closed predicates without
//! claiming real confinement. Positive LSM/release evidence remains the dedicated
//! effective-isolation acceptance lane.

#![cfg(target_os = "linux")]

use std::{
    fs,
    os::unix::fs::PermissionsExt,
    path::PathBuf,
    sync::atomic::{AtomicU64, Ordering},
    time::{SystemTime, UNIX_EPOCH},
};

use quarantine_sandbox_runtime::{
    ApplicationServiceError, ApplicationServiceRequest, IsolationPolicy, ResourceRequest,
    RootlessPodmanAdapter, ServiceProtocol,
};
use serde_json::{Value, json};

static NEXT_TEMP_PATH_ID: AtomicU64 = AtomicU64::new(0);

fn digest_image() -> String {
    format!("localhost/cwl/tool@sha256:{}", "b".repeat(64))
}

fn policy() -> IsolationPolicy {
    IsolationPolicy {
        policy_id: "isolation_failure_matrix_v1".to_owned(),
        maximum_memory_bytes: 512 * 1024 * 1024,
        maximum_cpu_millicores: 2_000,
        maximum_processes: 128,
        maximum_lease_seconds: 900,
        maximum_tmpfs_bytes: 128 * 1024 * 1024,
        readiness_timeout_millis: 25,
        readiness_poll_interval_millis: 5,
        shutdown_grace_seconds: 2,
        run_as_user_id: 65_532,
        run_as_group_id: 65_532,
    }
}

fn request() -> ApplicationServiceRequest {
    ApplicationServiceRequest {
        schema_version: "1.0.0".to_owned(),
        request_id: "isolation_failure_matrix_request".to_owned(),
        image_reference: digest_image(),
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

fn temporary_path(name: &str) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock should be after the Unix epoch")
        .as_nanos();
    let unique_id = NEXT_TEMP_PATH_ID.fetch_add(1, Ordering::Relaxed);
    std::env::temp_dir().join(format!(
        "quarantine-sandbox-runtime-{name}-{}-{nanos}-{unique_id}",
        std::process::id()
    ))
}

fn base_info() -> Value {
    json!({
        "host": {
            "security": {
                "rootless": true,
                "seccompEnabled": true,
                "seccompProfilePath": "/usr/share/containers/seccomp.json",
                "apparmorEnabled": true,
                "selinuxEnabled": false
            }
        }
    })
}

fn base_container() -> Value {
    json!([{
        "Id": "fake-container-id",
        "AppArmorProfile": "containers-default",
        "ProcessLabel": "",
        "EffectiveCaps": [],
        "BoundingCaps": [],
        "Config": {"User": "65532:65532"},
        "HostConfig": {
            "ReadonlyRootfs": true,
            "Privileged": false,
            "SecurityOpt": ["no-new-privileges"],
            "UsernsMode": "auto",
            "PidMode": "private",
            "IpcMode": "none",
            "Memory": 268435456,
            "NanoCpus": 1000000000_u64,
            "PidsLimit": 32
        }
    }])
}

fn base_network() -> Value {
    json!([{"internal": true, "dns_enabled": false}])
}

fn fixtures(mode: &str) -> (String, String, String, String, String) {
    let mut info = base_info();
    let mut container = base_container();
    let mut network = base_network();
    let mut process =
        "PID SECCOMP CAPEFF CAPBND CAPINH CAPPRM CAPAMB LABEL\n1 filter - - - - - containers-default (enforce)\n"
            .to_owned();
    let mut create_identifier = "fake-container-id\n".to_owned();

    match mode {
        "info_rootless_false" => info["host"]["security"]["rootless"] = json!(false),
        "info_seccomp_disabled" => info["host"]["security"]["seccompEnabled"] = json!(false),
        "info_seccomp_profile_empty" => {
            info["host"]["security"]["seccompProfilePath"] = json!("")
        }
        "info_lsm_disabled" => {
            info["host"]["security"]["apparmorEnabled"] = json!(false);
            info["host"]["security"]["selinuxEnabled"] = json!(false);
        }
        "container_id_mismatch" => container[0]["Id"] = json!("different-container-id"),
        "read_only_false" => container[0]["HostConfig"]["ReadonlyRootfs"] = json!(false),
        "privileged_true" => container[0]["HostConfig"]["Privileged"] = json!(true),
        "effective_caps_present" => container[0]["EffectiveCaps"] = json!(["CAP_NET_RAW"]),
        "bounding_caps_present" => container[0]["BoundingCaps"] = json!(["CAP_SYS_ADMIN"]),
        "process_caps_present" => {
            process = "PID SECCOMP CAPEFF CAPBND CAPINH CAPPRM CAPAMB LABEL\n1 filter 0x1 - - - - containers-default (enforce)\n".to_owned();
        }
        "no_new_privileges_missing" => container[0]["HostConfig"]["SecurityOpt"] = json!([]),
        "no_new_privileges_true_variant" => {
            container[0]["HostConfig"]["SecurityOpt"] = json!(["no-new-privileges=true"]);
            network[0]["internal"] = json!(false);
        }
        "seccomp_unconfined_option" => {
            container[0]["HostConfig"]["SecurityOpt"] =
                json!(["no-new-privileges", "seccomp=unconfined"]);
        }
        "seccomp_process_invalid" => {
            process = "PID SECCOMP CAPEFF CAPBND CAPINH CAPPRM CAPAMB LABEL\n1 disabled - - - - - containers-default (enforce)\n".to_owned();
        }
        "seccomp_strict" => {
            process = "PID SECCOMP CAPEFF CAPBND CAPINH CAPPRM CAPAMB LABEL\n1 strict - - - - - containers-default (enforce)\n".to_owned();
            network[0]["internal"] = json!(false);
        }
        "userns_wrong" => container[0]["HostConfig"]["UsernsMode"] = json!("host"),
        "pid_wrong" => container[0]["HostConfig"]["PidMode"] = json!("host"),
        "ipc_wrong" => container[0]["HostConfig"]["IpcMode"] = json!("host"),
        "user_wrong" => container[0]["Config"]["User"] = json!("0:0"),
        "memory_zero" => container[0]["HostConfig"]["Memory"] = json!(0),
        "memory_high" => container[0]["HostConfig"]["Memory"] = json!(536_870_912_u64),
        "cpu_zero" => container[0]["HostConfig"]["NanoCpus"] = json!(0),
        "cpu_high" => container[0]["HostConfig"]["NanoCpus"] = json!(2_000_000_000_u64),
        "pids_zero" => container[0]["HostConfig"]["PidsLimit"] = json!(0),
        "pids_negative" => container[0]["HostConfig"]["PidsLimit"] = json!(-1),
        "pids_high" => container[0]["HostConfig"]["PidsLimit"] = json!(64),
        "lsm_runtime_empty" => {
            process = "PID SECCOMP CAPEFF CAPBND CAPINH CAPPRM CAPAMB LABEL\n1 filter - - - - - \n".to_owned();
        }
        "lsm_runtime_unconfined" => {
            process = "PID SECCOMP CAPEFF CAPBND CAPINH CAPPRM CAPAMB LABEL\n1 filter - - - - - unconfined\n".to_owned();
        }
        "apparmor_inspect_empty" => container[0]["AppArmorProfile"] = json!(""),
        "apparmor_inspect_unconfined" => container[0]["AppArmorProfile"] = json!("unconfined"),
        "apparmor_runtime_no_mode" => {
            process = "PID SECCOMP CAPEFF CAPBND CAPINH CAPPRM CAPAMB LABEL\n1 filter - - - - - containers-default\n".to_owned();
        }
        "apparmor_runtime_missing_paren" => {
            process = "PID SECCOMP CAPEFF CAPBND CAPINH CAPPRM CAPAMB LABEL\n1 filter - - - - - containers-default (enforce\n".to_owned();
        }
        "apparmor_runtime_empty_profile" => {
            process = "PID SECCOMP CAPEFF CAPBND CAPINH CAPPRM CAPAMB LABEL\n1 filter - - - - -  (enforce)\n".to_owned();
        }
        "apparmor_runtime_complain" => {
            process = "PID SECCOMP CAPEFF CAPBND CAPINH CAPPRM CAPAMB LABEL\n1 filter - - - - - containers-default (complain)\n".to_owned();
        }
        "apparmor_profile_mismatch" => container[0]["AppArmorProfile"] = json!("other-profile"),
        "selinux_match" => {
            info["host"]["security"]["apparmorEnabled"] = json!(false);
            info["host"]["security"]["selinuxEnabled"] = json!(true);
            container[0]["ProcessLabel"] = json!("system_u:system_r:container_t:s0:c1,c2");
            process = "PID SECCOMP CAPEFF CAPBND CAPINH CAPPRM CAPAMB LABEL\n1 filter - - - - - system_u:system_r:container_t:s0:c1,c2\n".to_owned();
            network[0]["internal"] = json!(false);
        }
        "selinux_inspect_empty" | "selinux_inspect_unconfined" | "selinux_mismatch" => {
            info["host"]["security"]["apparmorEnabled"] = json!(false);
            info["host"]["security"]["selinuxEnabled"] = json!(true);
            container[0]["ProcessLabel"] = match mode {
                "selinux_inspect_empty" => json!(""),
                "selinux_inspect_unconfined" => json!("unconfined"),
                _ => json!("system_u:system_r:container_t:s0:c3,c4"),
            };
            process = "PID SECCOMP CAPEFF CAPBND CAPINH CAPPRM CAPAMB LABEL\n1 filter - - - - - system_u:system_r:container_t:s0:c1,c2\n".to_owned();
        }
        "network_not_internal" => network[0]["internal"] = json!(false),
        "network_dns_enabled" => network[0]["dns_enabled"] = json!(true),
        "create_identifier_empty" => create_identifier.clear(),
        "create_identifier_whitespace" => create_identifier = "fake container id\n".to_owned(),
        "create_identifier_control" => create_identifier = "fake\tid\n".to_owned(),
        "create_identifier_long" => create_identifier = format!("{}\n", "a".repeat(129)),
        "container_inspect_empty" => container = json!([]),
        "container_inspect_many" => {
            let duplicate = container[0].clone();
            container = json!([duplicate.clone(), duplicate]);
        }
        "network_inspect_empty" => network = json!([]),
        "network_inspect_many" => {
            let duplicate = network[0].clone();
            network = json!([duplicate.clone(), duplicate]);
        }
        "process_no_pid_one" => {
            process = "PID SECCOMP CAPEFF CAPBND CAPINH CAPPRM CAPAMB LABEL\n2 filter - - - - - containers-default (enforce)\n".to_owned();
        }
        "process_short" => {
            process = "PID SECCOMP CAPEFF\n1 filter -\n".to_owned();
        }
        "process_duplicate_pid_one" => {
            process = "PID SECCOMP CAPEFF CAPBND CAPINH CAPPRM CAPAMB LABEL\n1 filter - - - - - containers-default (enforce)\n1 filter - - - - - containers-default (enforce)\n".to_owned();
        }
        "caps_none" => {
            process = "PID SECCOMP CAPEFF CAPBND CAPINH CAPPRM CAPAMB LABEL\n1 filter none none none none none containers-default (enforce)\n".to_owned();
            network[0]["internal"] = json!(false);
        }
        "caps_zero" => {
            process = "PID SECCOMP CAPEFF CAPBND CAPINH CAPPRM CAPAMB LABEL\n1 filter 0 0 0 0 0 containers-default (enforce)\n".to_owned();
            network[0]["internal"] = json!(false);
        }
        "caps_hex_zero" => {
            process = "PID SECCOMP CAPEFF CAPBND CAPINH CAPPRM CAPAMB LABEL\n1 filter 0x0 0x0 0x0 0x0 0x0 containers-default (enforce)\n".to_owned();
            network[0]["internal"] = json!(false);
        }
        "caps_zero_digits" => {
            process = "PID SECCOMP CAPEFF CAPBND CAPINH CAPPRM CAPAMB LABEL\n1 filter 0000 0000 0000 0000 0000 containers-default (enforce)\n".to_owned();
            network[0]["internal"] = json!(false);
        }
        "caps_hex_zero_digits" => {
            process = "PID SECCOMP CAPEFF CAPBND CAPINH CAPPRM CAPAMB LABEL\n1 filter 0x0000 0x0000 0x0000 0x0000 0x0000 containers-default (enforce)\n".to_owned();
            network[0]["internal"] = json!(false);
        }
        _ => {}
    }

    (
        info.to_string(),
        container.to_string(),
        network.to_string(),
        process,
        create_identifier,
    )
}

fn write_fake_podman(mode: &str) -> (PathBuf, PathBuf) {
    let program = temporary_path("isolation-matrix-podman");
    let log = temporary_path("isolation-matrix-log");
    let (info, container, network, process, create_identifier) = fixtures(mode);
    let script = format!(
        "#!/bin/sh\nset -eu\nMODE='{mode}'\nprintf '%s\\n' \"$*\" >> '{}'\nif [ \"${{1:-}}\" = info ]; then\n  if [ \"${{3:-}}\" = json ]; then\n    if [ \"$MODE\" = info_malformed_json ]; then printf '{{'; else printf '%s\\n' '{}'; fi\n  else\n    printf 'true\\n'\n  fi\n  exit 0\nfi\ncase \"${{1:-}}:${{2:-}}\" in\n  network:create) : ;;\n  network:inspect)\n    if [ \"$MODE\" = network_inspect_malformed ]; then printf '['; else printf '%s\\n' '{}'; fi ;;\n  network:rm) : ;;\n  container:inspect)\n    if [ \"$MODE\" = container_inspect_malformed ]; then printf '['; else printf '%s\\n' '{}'; fi ;;\n  create:--name) printf '%s' '{}' ;;\n  start:*) : ;;\n  top:*)\n    if [ \"$MODE\" = process_non_utf8 ]; then printf '\\377\\n'; else printf '%s' '{}'; fi ;;\n  port:*) printf '127.0.0.1:9\\n' ;;\n  stop:*) : ;;\n  rm:*) : ;;\n  *) exit 91 ;;\nesac\n",
        log.display(),
        info,
        network,
        container,
        create_identifier,
        process,
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

fn assert_mode(mode: &str, expected: ApplicationServiceError) {
    let (program, log) = write_fake_podman(mode);
    let adapter = RootlessPodmanAdapter::new(program.clone());
    assert_eq!(
        adapter.launch_at(&request(), &policy(), 1_780_000_000),
        Err(expected),
        "mode {mode} must fail with its causal contract"
    );
    remove_fixture(program, log);
}

#[test]
fn backend_info_validation_is_fail_closed() {
    for (mode, expected) in [
        ("info_rootless_false", ApplicationServiceError::BackendNotRootless),
        (
            "info_seccomp_disabled",
            ApplicationServiceError::IsolationVerificationFailed {
                control_name: "seccomp",
            },
        ),
        (
            "info_seccomp_profile_empty",
            ApplicationServiceError::IsolationVerificationFailed {
                control_name: "seccomp",
            },
        ),
        (
            "info_lsm_disabled",
            ApplicationServiceError::IsolationVerificationFailed {
                control_name: "lsm",
            },
        ),
        (
            "info_malformed_json",
            ApplicationServiceError::MalformedIsolationInspection {
                operation: "backend_security_info",
            },
        ),
    ] {
        assert_mode(mode, expected);
    }
}

#[test]
fn backend_and_inspection_identity_parsing_is_fail_closed() {
    for mode in [
        "create_identifier_empty",
        "create_identifier_whitespace",
        "create_identifier_control",
        "create_identifier_long",
    ] {
        assert_mode(
            mode,
            ApplicationServiceError::MalformedIsolationInspection {
                operation: "container_create",
            },
        );
    }

    for mode in [
        "container_inspect_empty",
        "container_inspect_many",
        "container_inspect_malformed",
        "container_id_mismatch",
    ] {
        assert_mode(
            mode,
            ApplicationServiceError::MalformedIsolationInspection {
                operation: "container_inspect",
            },
        );
    }

    for mode in [
        "network_inspect_empty",
        "network_inspect_many",
        "network_inspect_malformed",
    ] {
        assert_mode(
            mode,
            ApplicationServiceError::MalformedIsolationInspection {
                operation: "network_inspect",
            },
        );
    }
}

#[test]
fn every_effective_container_isolation_control_fails_closed() {
    for (mode, control_name) in [
        ("read_only_false", "read_only_root_filesystem"),
        ("privileged_true", "unprivileged_container"),
        ("effective_caps_present", "all_capabilities_dropped"),
        ("bounding_caps_present", "all_capabilities_dropped"),
        ("process_caps_present", "all_capabilities_dropped"),
        ("no_new_privileges_missing", "no_new_privileges"),
        ("seccomp_unconfined_option", "seccomp"),
        ("seccomp_process_invalid", "seccomp"),
        ("userns_wrong", "isolated_user_namespace"),
        ("pid_wrong", "isolated_pid_namespace"),
        ("ipc_wrong", "isolated_ipc_namespace"),
        ("user_wrong", "non_root_identity"),
        ("memory_zero", "resource_limits"),
        ("memory_high", "resource_limits"),
        ("cpu_zero", "resource_limits"),
        ("cpu_high", "resource_limits"),
        ("pids_zero", "resource_limits"),
        ("pids_negative", "resource_limits"),
        ("pids_high", "resource_limits"),
    ] {
        assert_mode(
            mode,
            ApplicationServiceError::IsolationVerificationFailed { control_name },
        );
    }
}

#[test]
fn process_security_and_lsm_evidence_is_fail_closed() {
    for mode in [
        "process_non_utf8",
        "process_no_pid_one",
        "process_short",
        "process_duplicate_pid_one",
    ] {
        assert_mode(
            mode,
            ApplicationServiceError::MalformedIsolationInspection {
                operation: "process_security_top",
            },
        );
    }

    for mode in [
        "lsm_runtime_empty",
        "lsm_runtime_unconfined",
        "apparmor_inspect_empty",
        "apparmor_inspect_unconfined",
        "apparmor_runtime_no_mode",
        "apparmor_runtime_missing_paren",
        "apparmor_runtime_empty_profile",
        "apparmor_runtime_complain",
        "apparmor_profile_mismatch",
        "selinux_inspect_empty",
        "selinux_inspect_unconfined",
        "selinux_mismatch",
    ] {
        assert_mode(
            mode,
            ApplicationServiceError::IsolationVerificationFailed {
                control_name: "lsm",
            },
        );
    }
}

#[test]
fn accepted_capability_and_lsm_encodings_still_reach_network_fail_closed_gate() {
    for mode in [
        "no_new_privileges_true_variant",
        "seccomp_strict",
        "selinux_match",
        "caps_none",
        "caps_zero",
        "caps_hex_zero",
        "caps_zero_digits",
        "caps_hex_zero_digits",
        "network_not_internal",
        "network_dns_enabled",
    ] {
        assert_mode(
            mode,
            ApplicationServiceError::IsolationVerificationFailed {
                control_name: "external_egress_denied",
            },
        );
    }
}
