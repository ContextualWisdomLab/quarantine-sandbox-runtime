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
const GOOD_TOP: &str = "PID SECCOMP CAPEFF CAPBND CAPINH CAPPRM CAPAMB LABEL\n1 filter - - - - - containers-default (enforce)\n";

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

#[derive(Clone)]
struct Fixture {
    info: Value,
    container: Value,
    network: Value,
    process_top: String,
    create_identifier: String,
    info_output_override: Option<String>,
    container_output_override: Option<String>,
    network_output_override: Option<String>,
}

impl Default for Fixture {
    fn default() -> Self {
        Self {
            info: json!({
                "host": {
                    "security": {
                        "rootless": true,
                        "seccompEnabled": true,
                        "seccompProfilePath": "/usr/share/containers/seccomp.json",
                        "apparmorEnabled": true,
                        "selinuxEnabled": false
                    }
                }
            }),
            container: json!([{
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
            }]),
            network: json!([{"internal": true, "dns_enabled": false}]),
            process_top: GOOD_TOP.to_owned(),
            create_identifier: "fake-container-id\n".to_owned(),
            info_output_override: None,
            container_output_override: None,
            network_output_override: None,
        }
    }
}

fn write_fake_podman(fixture: &Fixture) -> (PathBuf, PathBuf) {
    let program = temporary_path("isolation-matrix-podman");
    let log = temporary_path("isolation-matrix-log");
    let info = fixture
        .info_output_override
        .clone()
        .unwrap_or_else(|| fixture.info.to_string());
    let container = fixture
        .container_output_override
        .clone()
        .unwrap_or_else(|| fixture.container.to_string());
    let network = fixture
        .network_output_override
        .clone()
        .unwrap_or_else(|| fixture.network.to_string());
    let script = format!(
        "#!/bin/sh\nset -eu\nprintf '%s\\n' \"$*\" >> '{}'\nif [ \"${{1:-}}\" = info ]; then\n  if [ \"${{3:-}}\" = json ]; then printf '%s\\n' '{}'; else printf 'true\\n'; fi\n  exit 0\nfi\ncase \"${{1:-}}:${{2:-}}\" in\n  network:create) : ;;\n  network:inspect) printf '%s\\n' '{}' ;;\n  network:rm) : ;;\n  container:inspect) printf '%s\\n' '{}' ;;\n  create:--name) printf '%s' '{}' ;;\n  start:*) : ;;\n  top:*) printf '%s' '{}' ;;\n  port:*) printf '127.0.0.1:9\\n' ;;\n  stop:*) : ;;\n  rm:*) : ;;\n  *) exit 91 ;;\nesac\n",
        log.display(),
        info,
        network,
        container,
        fixture.create_identifier,
        fixture.process_top,
    );
    fs::write(&program, script).expect("fake Podman should be writable");
    let mut permissions = fs::metadata(&program)
        .expect("fake Podman metadata should exist")
        .permissions();
    permissions.set_mode(0o700);
    fs::set_permissions(&program, permissions).expect("fake Podman should be executable");
    (program, log)
}

fn assert_fixture(fixture: Fixture, expected: ApplicationServiceError) {
    let (program, log) = write_fake_podman(&fixture);
    let adapter = RootlessPodmanAdapter::new(program.clone());
    assert_eq!(
        adapter.launch_at(&request(), &policy(), 1_780_000_000),
        Err(expected)
    );
    let _ = fs::remove_file(program);
    let _ = fs::remove_file(log);
}

#[test]
fn backend_info_validation_is_fail_closed() {
    let mut fixture = Fixture::default();
    fixture.info["host"]["security"]["rootless"] = json!(false);
    assert_fixture(fixture, ApplicationServiceError::BackendNotRootless);

    for mutate in [
        |value: &mut Value| value["host"]["security"]["seccompEnabled"] = json!(false),
        |value: &mut Value| value["host"]["security"]["seccompProfilePath"] = json!(""),
    ] {
        let mut fixture = Fixture::default();
        mutate(&mut fixture.info);
        assert_fixture(
            fixture,
            ApplicationServiceError::IsolationVerificationFailed {
                control_name: "seccomp",
            },
        );
    }

    let mut fixture = Fixture::default();
    fixture.info["host"]["security"]["apparmorEnabled"] = json!(false);
    fixture.info["host"]["security"]["selinuxEnabled"] = json!(false);
    assert_fixture(
        fixture,
        ApplicationServiceError::IsolationVerificationFailed {
            control_name: "lsm",
        },
    );

    let fixture = Fixture {
        info_output_override: Some("{".to_owned()),
        ..Fixture::default()
    };
    assert_fixture(
        fixture,
        ApplicationServiceError::MalformedIsolationInspection {
            operation: "backend_security_info",
        },
    );
}

#[test]
fn backend_and_inspection_identity_parsing_is_fail_closed() {
    for create_identifier in [
        String::new(),
        "fake container id\n".to_owned(),
        "fake\tid\n".to_owned(),
        format!("{}\n", "a".repeat(129)),
    ] {
        let fixture = Fixture {
            create_identifier,
            ..Fixture::default()
        };
        assert_fixture(
            fixture,
            ApplicationServiceError::MalformedIsolationInspection {
                operation: "container_create",
            },
        );
    }

    for container_output in ["[]", "[{},{}]", "["] {
        let fixture = Fixture {
            container_output_override: Some(container_output.to_owned()),
            ..Fixture::default()
        };
        assert_fixture(
            fixture,
            ApplicationServiceError::MalformedIsolationInspection {
                operation: "container_inspect",
            },
        );
    }

    let mut fixture = Fixture::default();
    fixture.container[0]["Id"] = json!("different-container-id");
    assert_fixture(
        fixture,
        ApplicationServiceError::MalformedIsolationInspection {
            operation: "container_inspect",
        },
    );

    for network_output in ["[]", "[{},{}]", "["] {
        let fixture = Fixture {
            network_output_override: Some(network_output.to_owned()),
            ..Fixture::default()
        };
        assert_fixture(
            fixture,
            ApplicationServiceError::MalformedIsolationInspection {
                operation: "network_inspect",
            },
        );
    }
}

#[test]
fn every_effective_container_isolation_control_fails_closed() {
    type Mutation = fn(&mut Fixture);
    let cases: [(&str, Mutation); 19] = [
        ("read_only_root_filesystem", |f| {
            f.container[0]["HostConfig"]["ReadonlyRootfs"] = json!(false)
        }),
        ("unprivileged_container", |f| {
            f.container[0]["HostConfig"]["Privileged"] = json!(true)
        }),
        ("all_capabilities_dropped", |f| {
            f.container[0]["EffectiveCaps"] = json!(["CAP_NET_RAW"])
        }),
        ("all_capabilities_dropped", |f| {
            f.container[0]["BoundingCaps"] = json!(["CAP_SYS_ADMIN"])
        }),
        ("all_capabilities_dropped", |f| {
            f.process_top = "PID SECCOMP CAPEFF CAPBND CAPINH CAPPRM CAPAMB LABEL\n1 filter 0x1 - - - - containers-default (enforce)\n".to_owned()
        }),
        ("no_new_privileges", |f| {
            f.container[0]["HostConfig"]["SecurityOpt"] = json!([])
        }),
        ("seccomp", |f| {
            f.container[0]["HostConfig"]["SecurityOpt"] =
                json!(["no-new-privileges", "seccomp=unconfined"])
        }),
        ("seccomp", |f| {
            f.process_top = "PID SECCOMP CAPEFF CAPBND CAPINH CAPPRM CAPAMB LABEL\n1 disabled - - - - - containers-default (enforce)\n".to_owned()
        }),
        ("isolated_user_namespace", |f| {
            f.container[0]["HostConfig"]["UsernsMode"] = json!("host")
        }),
        ("isolated_pid_namespace", |f| {
            f.container[0]["HostConfig"]["PidMode"] = json!("host")
        }),
        ("isolated_ipc_namespace", |f| {
            f.container[0]["HostConfig"]["IpcMode"] = json!("host")
        }),
        ("non_root_identity", |f| {
            f.container[0]["Config"]["User"] = json!("0:0")
        }),
        ("resource_limits", |f| {
            f.container[0]["HostConfig"]["Memory"] = json!(0)
        }),
        ("resource_limits", |f| {
            f.container[0]["HostConfig"]["Memory"] = json!(536_870_912_u64)
        }),
        ("resource_limits", |f| {
            f.container[0]["HostConfig"]["NanoCpus"] = json!(0)
        }),
        ("resource_limits", |f| {
            f.container[0]["HostConfig"]["NanoCpus"] = json!(2_000_000_000_u64)
        }),
        ("resource_limits", |f| {
            f.container[0]["HostConfig"]["PidsLimit"] = json!(0)
        }),
        ("resource_limits", |f| {
            f.container[0]["HostConfig"]["PidsLimit"] = json!(-1)
        }),
        ("resource_limits", |f| {
            f.container[0]["HostConfig"]["PidsLimit"] = json!(64)
        }),
    ];

    for (control_name, mutate) in cases {
        let mut fixture = Fixture::default();
        mutate(&mut fixture);
        assert_fixture(
            fixture,
            ApplicationServiceError::IsolationVerificationFailed { control_name },
        );
    }
}

#[test]
fn process_security_and_lsm_evidence_is_fail_closed() {
    for process_top in [
        "PID SECCOMP CAPEFF CAPBND CAPINH CAPPRM CAPAMB LABEL\n2 filter - - - - - containers-default (enforce)\n",
        "PID SECCOMP CAPEFF\n1 filter -\n",
        "PID SECCOMP CAPEFF CAPBND CAPINH CAPPRM CAPAMB LABEL\n1 filter - - - - - containers-default (enforce)\n1 filter - - - - - containers-default (enforce)\n",
    ] {
        let fixture = Fixture {
            process_top: process_top.to_owned(),
            ..Fixture::default()
        };
        assert_fixture(
            fixture,
            ApplicationServiceError::MalformedIsolationInspection {
                operation: "process_security_top",
            },
        );
    }

    for process_top in [
        "PID SECCOMP CAPEFF CAPBND CAPINH CAPPRM CAPAMB LABEL\n1 filter - - - - - unconfined\n",
        "PID SECCOMP CAPEFF CAPBND CAPINH CAPPRM CAPAMB LABEL\n1 filter - - - - - containers-default\n",
        "PID SECCOMP CAPEFF CAPBND CAPINH CAPPRM CAPAMB LABEL\n1 filter - - - - - containers-default (enforce\n",
        "PID SECCOMP CAPEFF CAPBND CAPINH CAPPRM CAPAMB LABEL\n1 filter - - - - - (enforce)\n",
        "PID SECCOMP CAPEFF CAPBND CAPINH CAPPRM CAPAMB LABEL\n1 filter - - - - - containers-default (complain)\n",
    ] {
        let fixture = Fixture {
            process_top: process_top.to_owned(),
            ..Fixture::default()
        };
        assert_fixture(
            fixture,
            ApplicationServiceError::IsolationVerificationFailed {
                control_name: "lsm",
            },
        );
    }

    for profile in ["", "unconfined", "other-profile"] {
        let mut fixture = Fixture::default();
        fixture.container[0]["AppArmorProfile"] = json!(profile);
        assert_fixture(
            fixture,
            ApplicationServiceError::IsolationVerificationFailed {
                control_name: "lsm",
            },
        );
    }

    for process_label in ["", "unconfined", "system_u:system_r:container_t:s0:c3,c4"] {
        let mut fixture = Fixture::default();
        fixture.info["host"]["security"]["apparmorEnabled"] = json!(false);
        fixture.info["host"]["security"]["selinuxEnabled"] = json!(true);
        fixture.container[0]["ProcessLabel"] = json!(process_label);
        fixture.process_top = "PID SECCOMP CAPEFF CAPBND CAPINH CAPPRM CAPAMB LABEL\n1 filter - - - - - system_u:system_r:container_t:s0:c1,c2\n".to_owned();
        assert_fixture(
            fixture,
            ApplicationServiceError::IsolationVerificationFailed {
                control_name: "lsm",
            },
        );
    }
}

#[test]
fn accepted_security_encodings_reach_external_egress_gate() {
    let mut fixtures = Vec::new();

    let mut fixture = Fixture::default();
    fixture.container[0]["HostConfig"]["SecurityOpt"] = json!(["no-new-privileges=true"]);
    fixtures.push(fixture);

    let fixture = Fixture {
        process_top: "PID SECCOMP CAPEFF CAPBND CAPINH CAPPRM CAPAMB LABEL\n1 strict - - - - - containers-default (enforce)\n".to_owned(),
        ..Fixture::default()
    };
    fixtures.push(fixture);

    let mut fixture = Fixture::default();
    fixture.info["host"]["security"]["apparmorEnabled"] = json!(false);
    fixture.info["host"]["security"]["selinuxEnabled"] = json!(true);
    fixture.container[0]["ProcessLabel"] = json!("system_u:system_r:container_t:s0:c1,c2");
    fixture.process_top = "PID SECCOMP CAPEFF CAPBND CAPINH CAPPRM CAPAMB LABEL\n1 filter - - - - - system_u:system_r:container_t:s0:c1,c2\n".to_owned();
    fixtures.push(fixture);

    for capability in ["none", "0", "0x0", "0000", "0x0000"] {
        let fixture = Fixture {
            process_top: format!(
                "PID SECCOMP CAPEFF CAPBND CAPINH CAPPRM CAPAMB LABEL\n1 filter {0} {0} {0} {0} {0} containers-default (enforce)\n",
                capability
            ),
            ..Fixture::default()
        };
        fixtures.push(fixture);
    }

    for mut fixture in fixtures {
        fixture.network[0]["internal"] = json!(false);
        assert_fixture(
            fixture,
            ApplicationServiceError::IsolationVerificationFailed {
                control_name: "external_egress_denied",
            },
        );
    }

    let mut fixture = Fixture::default();
    fixture.network[0]["dns_enabled"] = json!(true);
    assert_fixture(
        fixture,
        ApplicationServiceError::IsolationVerificationFailed {
            control_name: "external_egress_denied",
        },
    );
}
