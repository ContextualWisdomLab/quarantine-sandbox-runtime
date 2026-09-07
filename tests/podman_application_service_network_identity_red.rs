//! RED: an application-service network name is correlation metadata, not lifecycle authority.
//!
//! Podman network creation reports the network name while network inspection exposes a
//! distinct full network ID. The runtime must acquire that identity before container
//! creation and use it for network selection so a same-name replacement cannot capture
//! the sandbox after the runtime created its own network.

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
    ApplicationServiceRequest, IsolationPolicy, ResourceRequest, RootlessPodmanAdapter,
    ServiceProtocol,
};

const STARTED_AT_EPOCH_SECONDS: u64 = 1_780_000_600;
const OWNED_NETWORK_ID: &str =
    "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
const FOREIGN_NETWORK_ID: &str =
    "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";
const OWNED_CONTAINER_ID: &str =
    "cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc";

static NEXT_TEMP_PATH_ID: AtomicU64 = AtomicU64::new(0);

fn temporary_path(name: &str) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("test clock must be after the Unix epoch")
        .as_nanos();
    let unique_id = NEXT_TEMP_PATH_ID.fetch_add(1, Ordering::Relaxed);
    std::env::temp_dir().join(format!(
        "qsr-network-identity-red-{name}-{}-{nanos}-{unique_id}",
        std::process::id()
    ))
}

fn policy() -> IsolationPolicy {
    IsolationPolicy {
        policy_id: "network_identity_red_v1".to_owned(),
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
        request_id: "network_identity_red".to_owned(),
        image_reference: format!("localhost/cwl/tool@sha256:{}", "f".repeat(64)),
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
fn acquired_network_id_is_bound_before_container_creation() {
    let request = request();
    let policy = policy();
    let plan = RootlessPodmanAdapter::plan_at(&request, &policy, STARTED_AT_EPOCH_SECONDS)
        .expect("valid request and policy must yield a launch plan");
    let expected_network = plan.network_name().to_owned();

    let listener = TcpListener::bind(("127.0.0.1", 0)).expect("loopback listener must bind");
    let ready_port = listener
        .local_addr()
        .expect("listener must expose an address")
        .port();

    let program = temporary_path("fake-podman");
    let log = temporary_path("calls");
    let foreign_marker = temporary_path("foreign-network-marker");
    fs::write(&foreign_marker, "safe\n").expect("foreign marker must be writable");

    let info = r#"{"host":{"security":{"rootless":true,"seccompEnabled":true,"seccompProfilePath":"/usr/share/containers/seccomp.json","apparmorEnabled":true,"selinuxEnabled":false}}}"#;
    let owned_network = format!(
        r#"[{{"name":"{expected_network}","id":"{OWNED_NETWORK_ID}","internal":true,"dns_enabled":false,"containers":{{}}}}]"#
    );
    let foreign_network = format!(
        r#"[{{"name":"{expected_network}","id":"{FOREIGN_NETWORK_ID}","internal":true,"dns_enabled":false,"containers":{{}}}}]"#
    );
    let container = format!(
        r#"[{{"Id":"{OWNED_CONTAINER_ID}","ImageDigest":"sha256:{}","AppArmorProfile":"containers-default","ProcessLabel":"","EffectiveCaps":[],"BoundingCaps":[],"Config":{{"User":"65532:65532","Timeout":30}},"HostConfig":{{"ReadonlyRootfs":true,"Privileged":false,"SecurityOpt":["no-new-privileges"],"UsernsMode":"auto","PidMode":"private","IpcMode":"none","UTSMode":"private","CgroupMode":"private","Memory":134217728,"NanoCpus":250000000,"PidsLimit":16,"Tmpfs":{{"/tmp":"rw,noexec,nosuid,nodev,size=16777216"}},"NetworkMode":"{expected_network}"}},"NetworkSettings":{{"Networks":{{"{expected_network}":{{}}}}}}}}]"#,
        "f".repeat(64)
    );
    let script = format!(
        "#!/bin/sh\nset -eu\nprintf '%s\\n' \"$*\" >> '{}'\nforeign='{}'\nif [ \"${{1:-}}\" = info ]; then\n  if [ \"${{3:-}}\" = json ]; then printf '%s\\n' '{}'; else printf 'true\\n'; fi\n  exit 0\nfi\ncase \"${{1:-}}:${{2:-}}\" in\n  network:create) printf '%s\\n' '{}' ;;\n  network:inspect)\n    if [ \"$(cat \"$foreign\")\" = foreign-selected ]; then printf '%s\\n' '{}'; else printf '%s\\n' '{}'; fi\n    ;;\n  create:--name)\n    case \" $* \" in\n      *' --network {} '*) : ;;\n      *' --network {} '*) printf 'foreign-selected\\n' > \"$foreign\" ;;\n      *) exit 92 ;;\n    esac\n    printf '%s\\n' '{}'\n    ;;\n  container:inspect) printf '%s\\n' '{}' ;;\n  start:*) : ;;\n  top:*) printf 'PID SECCOMP CAPEFF CAPBND CAPINH CAPPRM CAPAMB LABEL\\n1 filter - - - - - containers-default (enforce)\\n' ;;\n  port:*) printf '127.0.0.1:{}\\n' ;;\n  stop:*) : ;;\n  rm:*) : ;;\n  network:rm) : ;;\n  *) exit 91 ;;\nesac\n",
        log.display(),
        foreign_marker.display(),
        info,
        expected_network,
        foreign_network,
        owned_network,
        OWNED_NETWORK_ID,
        expected_network,
        OWNED_CONTAINER_ID,
        container,
        ready_port,
    );
    fs::write(&program, script).expect("fake Podman must be writable");
    let mut permissions = fs::metadata(&program)
        .expect("fake Podman metadata must exist")
        .permissions();
    permissions.set_mode(0o700);
    fs::set_permissions(&program, permissions).expect("fake Podman must be executable");

    let adapter = RootlessPodmanAdapter::new(program.clone());
    let lease = adapter
        .launch_at(&request, &policy, STARTED_AT_EPOCH_SECONDS)
        .expect("otherwise-positive launch demonstrates whether network ownership can false-GREEN");
    assert_eq!(
        lease.network_id(),
        expected_network,
        "public network identity remains correlation metadata until a versioned contract changes it"
    );

    let calls = fs::read_to_string(&log).expect("fake Podman calls must be recorded");
    let lines: Vec<&str> = calls.lines().collect();
    let create_index = lines
        .iter()
        .position(|line| line.starts_with("create --name "))
        .expect("container create must be exercised");
    let identity_inspect_index = lines
        .iter()
        .position(|line| line.starts_with(&format!(
            "network inspect --format json {expected_network}"
        )))
        .expect("created network identity must be inspected");
    assert!(
        identity_inspect_index < create_index,
        "network ID must be acquired immediately after network creation and before container selection; calls were:\n{calls}"
    );

    let create_call = lines[create_index];
    assert!(
        create_call.contains(&format!(" --network {OWNED_NETWORK_ID} ")),
        "container creation must bind to the acquired network ID rather than re-resolve the generated name; call was: {create_call}"
    );
    assert_eq!(
        fs::read_to_string(&foreign_marker).expect("foreign marker must remain readable"),
        "safe\n",
        "a same-name replacement network must never become the sandbox network through name re-resolution"
    );

    let _ = fs::remove_file(program);
    let _ = fs::remove_file(log);
    let _ = fs::remove_file(foreign_marker);
    drop(listener);
}
