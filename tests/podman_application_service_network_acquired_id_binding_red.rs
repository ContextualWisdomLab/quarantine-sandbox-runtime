//! RED: acquired network identity must not erase attachment-mismatch causality.
//!
//! The runtime must acquire the Podman network ID before container creation, bind
//! the container to that exact ID, and still fail closed when effective container
//! attachment does not match the owned deny-by-default network. Cleanup must use
//! the acquired authority without network-level force removal.

#![cfg(target_os = "linux")]

use std::{
    fs,
    net::TcpListener,
    os::unix::fs::PermissionsExt,
    path::PathBuf,
    sync::atomic::{AtomicU64, Ordering},
    time::{SystemTime, UNIX_EPOCH},
};

use quarantine_sandbox_runtime::{
    ApplicationServiceError, ApplicationServiceRequest, IsolationPolicy, ResourceRequest,
    RootlessPodmanAdapter, ServiceProtocol,
};

const STARTED_AT_EPOCH_SECONDS: u64 = 1_780_001_000;
const OWNED_NETWORK_ID: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
const OWNED_CONTAINER_ID: &str = "cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc";

static NEXT_TEMP_PATH_ID: AtomicU64 = AtomicU64::new(0);

fn temporary_path(name: &str) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("test clock must be after the Unix epoch")
        .as_nanos();
    let unique_id = NEXT_TEMP_PATH_ID.fetch_add(1, Ordering::Relaxed);
    std::env::temp_dir().join(format!(
        "qsr-network-acquired-id-binding-red-{name}-{}-{nanos}-{unique_id}",
        std::process::id()
    ))
}

fn policy() -> IsolationPolicy {
    IsolationPolicy {
        policy_id: "network_acquired_id_binding_red_v1".to_owned(),
        maximum_memory_bytes: 256 * 1024 * 1024,
        maximum_cpu_millicores: 500,
        maximum_processes: 32,
        maximum_lease_seconds: 60,
        maximum_tmpfs_bytes: 32 * 1024 * 1024,
        readiness_timeout_millis: 500,
        readiness_poll_interval_millis: 10,
        shutdown_grace_seconds: 1,
        run_as_user_id: 65_532,
        run_as_group_id: 65_532,
    }
}

fn request() -> ApplicationServiceRequest {
    ApplicationServiceRequest {
        schema_version: "1.0.0".to_owned(),
        request_id: "network_acquired_id_binding_red".to_owned(),
        image_reference: format!("localhost/cwl/tool@sha256:{}", "d".repeat(64)),
        container_port: 8_080,
        protocol: ServiceProtocol::Http,
        command: vec!["serve".to_owned()],
        resources: ResourceRequest {
            memory_bytes: 128 * 1024 * 1024,
            cpu_millicores: 250,
            maximum_processes: 16,
            lease_seconds: 30,
            tmpfs_bytes: 16 * 1024 * 1024,
        },
    }
}

#[test]
fn acquired_network_id_preserves_attachment_mismatch_as_the_causal_failure() {
    let request = request();
    let policy = policy();
    let plan = RootlessPodmanAdapter::plan_at(&request, &policy, STARTED_AT_EPOCH_SECONDS)
        .expect("valid request and policy must yield a launch plan");
    let expected_network_name = plan.network_name().to_owned();
    let expected_sandbox = plan.sandbox_name().to_owned();

    let listener = TcpListener::bind(("127.0.0.1", 0)).expect("loopback listener must bind");
    let ready_port = listener
        .local_addr()
        .expect("listener must expose an address")
        .port();

    let program = temporary_path("fake-podman");
    let log = temporary_path("calls");
    let info = r#"{"host":{"security":{"rootless":true,"seccompEnabled":true,"seccompProfilePath":"/usr/share/containers/seccomp.json","apparmorEnabled":true,"selinuxEnabled":false}}}"#;
    let network = format!(
        r#"[{{"name":"{expected_network_name}","id":"{OWNED_NETWORK_ID}","internal":true,"dns_enabled":false,"containers":{{}}}}]"#
    );
    let container = format!(
        r#"[{{"Id":"{OWNED_CONTAINER_ID}","AppArmorProfile":"containers-default","ProcessLabel":"","EffectiveCaps":[],"BoundingCaps":[],"Config":{{"User":"65532:65532"}},"HostConfig":{{"ReadonlyRootfs":true,"Privileged":false,"SecurityOpt":["no-new-privileges"],"UsernsMode":"auto","PidMode":"private","IpcMode":"none","Memory":134217728,"NanoCpus":250000000,"PidsLimit":16,"NetworkMode":"bridge"}},"NetworkSettings":{{"Networks":{{"podman":{{}}}}}}}}]"#
    );
    let script = format!(
        "#!/bin/sh\nset -eu\nprintf '%s\\n' \"$*\" >> '{}'\nif [ \"${{1:-}}\" = info ]; then\n  if [ \"${{3:-}}\" = json ]; then printf '%s\\n' '{}'; else printf 'true\\n'; fi\n  exit 0\nfi\ncase \"${{1:-}}:${{2:-}}\" in\n  network:create) printf '%s\\n' '{}' ;;\n  network:inspect) printf '%s\\n' '{}' ;;\n  create:--name)\n    case \" $* \" in\n      *' --network {} '*) printf '%s\\n' '{}' ;;\n      *) exit 92 ;;\n    esac\n    ;;\n  start:*) : ;;\n  container:inspect) printf '%s\\n' '{}' ;;\n  top:*) printf 'PID SECCOMP CAPEFF CAPBND CAPINH CAPPRM CAPAMB LABEL\\n1 filter - - - - - containers-default (enforce)\\n' ;;\n  port:*) printf '127.0.0.1:{}\\n' ;;\n  stop:*) : ;;\n  rm:*) : ;;\n  network:rm)\n    if [ \"$*\" = \"network rm {}\" ]; then exit 0; fi\n    exit 93\n    ;;\n  *) exit 91 ;;\nesac\n",
        log.display(),
        info,
        expected_network_name,
        network,
        OWNED_NETWORK_ID,
        OWNED_CONTAINER_ID,
        container,
        ready_port,
        OWNED_NETWORK_ID,
    );
    fs::write(&program, script).expect("fake Podman must be writable");
    let mut permissions = fs::metadata(&program)
        .expect("fake Podman metadata must exist")
        .permissions();
    permissions.set_mode(0o700);
    fs::set_permissions(&program, permissions).expect("fake Podman must be executable");

    let adapter = RootlessPodmanAdapter::new(program.clone());
    assert_eq!(
        adapter.launch_at(&request, &policy, STARTED_AT_EPOCH_SECONDS),
        Err(ApplicationServiceError::IsolationVerificationFailed {
            control_name: "sandbox_network_binding",
        }),
        "after acquired-ID binding, the hostile attachment mismatch must remain the causal failure"
    );

    let calls = fs::read_to_string(&log).expect("fake Podman calls must be recorded");
    let lines: Vec<&str> = calls.lines().collect();
    let create_index = lines
        .iter()
        .position(|line| line.starts_with("create --name "))
        .expect("container create must be exercised");
    let identity_inspect_index = lines
        .iter()
        .position(|line| line.starts_with("network inspect --format json "))
        .expect("created network identity must be inspected");
    assert!(
        identity_inspect_index < create_index,
        "network identity must be acquired before container creation; calls were:\n{calls}"
    );
    assert!(
        lines[create_index].contains(&format!(" --network {OWNED_NETWORK_ID} ")),
        "container creation must bind to acquired network authority; call was: {}",
        lines[create_index]
    );
    assert!(
        calls
            .lines()
            .any(|line| line == format!("network rm {OWNED_NETWORK_ID}")),
        "cleanup must target the exact acquired network authority without force; calls were:\n{calls}"
    );
    assert!(
        !calls.lines().any(|line| line.starts_with("network rm --force ")),
        "network-level force cleanup must never be used; calls were:\n{calls}"
    );
    assert!(
        !calls.lines().any(|line| line.starts_with("port ")),
        "readiness must not be queried after effective attachment mismatch; calls were:\n{calls}"
    );
    assert!(
        calls
            .lines()
            .any(|line| line == format!("stop --time 1 {expected_sandbox}")),
        "started container must be stopped before bounded cleanup; calls were:\n{calls}"
    );

    let _ = fs::remove_file(program);
    let _ = fs::remove_file(log);
    drop(listener);
}
