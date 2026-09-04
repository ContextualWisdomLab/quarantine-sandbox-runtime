//! RED regression for one-shot command sandbox resource identity.
//!
//! `CommandExecutionRequest::request_id` is consumer correlation metadata, not
//! an idempotency/resource key. Two legitimate command invocations may reuse it
//! in the same supplied start second. Each invocation must still receive an
//! independent runtime-owned sandbox identity so cleanup can never target a
//! sibling invocation's container.
//!
//! Runtime uniqueness also needs enough retained collision resistance. A
//! 128-bit execution nonce that is hashed and then truncated to a 64-bit
//! resource name does not preserve the security margin the runtime generated.

#![cfg(target_os = "linux")]

use std::{
    collections::BTreeSet,
    fs,
    os::unix::fs::PermissionsExt,
    path::PathBuf,
    sync::atomic::{AtomicU64, Ordering},
    thread,
    time::{SystemTime, UNIX_EPOCH},
};

use quarantine_sandbox_runtime::{
    CommandExecutionRequest, IsolationPolicy, ResourceRequest, RootlessPodmanAdapter,
};

static NEXT_PATH_ID: AtomicU64 = AtomicU64::new(0);

fn temporary_path(name: &str) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock should be after the Unix epoch")
        .as_nanos();
    let unique_id = NEXT_PATH_ID.fetch_add(1, Ordering::Relaxed);
    std::env::temp_dir().join(format!(
        "quarantine-command-identity-race-red-{name}-{}-{nanos}-{unique_id}",
        std::process::id()
    ))
}

fn write_executable(name: &str, script: &str) -> PathBuf {
    let program = temporary_path(name);
    fs::write(&program, script).expect("fake Podman should be writable");
    let mut permissions = fs::metadata(&program)
        .expect("fake Podman metadata should exist")
        .permissions();
    permissions.set_mode(0o700);
    fs::set_permissions(&program, permissions).expect("fake Podman should be executable");
    program
}

fn policy() -> IsolationPolicy {
    IsolationPolicy {
        policy_id: "command_identity_race_policy_v1".to_owned(),
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
        request_id: "same-consumer-correlation-id".to_owned(),
        image_reference: format!("localhost/cwl/tool@sha256:{}", "c".repeat(64)),
        command: vec!["pytest".to_owned(), "-q".to_owned()],
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

fn assert_runtime_identity_retains_128_bits(sandbox_id: &str) {
    let suffix = sandbox_id
        .strip_prefix("qsr-cmd-")
        .expect("command sandbox identity must use the qsr-cmd prefix");
    assert!(
        suffix.len() >= 32,
        "command sandbox identity must retain at least 128 bits as 32 hexadecimal characters"
    );
    assert!(
        suffix
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte)),
        "command sandbox identity suffix must remain lowercase hexadecimal"
    );
}

#[test]
fn repeated_consumer_correlation_same_start_second_uses_distinct_runtime_resources() {
    let call_log = temporary_path("calls");
    let script = format!(
        "#!/bin/sh\nset -eu\nprintf '%s\\n' \"$*\" >> '{}'\ncase \"${{1:-}}:${{2:-}}\" in\n  info:--format) printf '%s\\n' '{{\"host\":{{\"security\":{{\"rootless\":true,\"seccompEnabled\":true,\"seccompProfilePath\":\"/usr/share/containers/seccomp.json\",\"apparmorEnabled\":true,\"selinuxEnabled\":false}}}},\"version\":{{\"Version\":\"6.1.0\"}}}}' ;;\n  create:--name) printf 'fake-command-container-id\\n' ;;\n  start:*) : ;;\n  container:inspect) printf '%s\\n' '[{{\"Id\":\"fake-command-container-id\",\"AppArmorProfile\":\"containers-default\",\"ProcessLabel\":\"\",\"EffectiveCaps\":[],\"BoundingCaps\":[],\"Config\":{{\"User\":\"65532:65532\"}},\"HostConfig\":{{\"ReadonlyRootfs\":true,\"Privileged\":false,\"SecurityOpt\":[\"no-new-privileges\"],\"UsernsMode\":\"auto\",\"PidMode\":\"private\",\"IpcMode\":\"none\",\"NetworkMode\":\"none\",\"Memory\":268435456,\"NanoCpus\":1000000000,\"PidsLimit\":16}}}}]' ;;\n  top:*) printf 'PID SECCOMP CAPEFF CAPBND CAPINH CAPPRM CAPAMB LABEL\\n1 filter - - - - - containers-default (enforce)\\n' ;;\n  wait:*) printf '0\\n' ;;\n  logs:*) printf 'ok\\n' ;;\n  rm:--force) : ;;\n  *) exit 91 ;;\nesac\n",
        call_log.display(),
    );
    let program = write_executable("fake-podman", &script);
    let adapter = RootlessPodmanAdapter::new(program.clone());
    let same_start_second = 1_780_000_000;

    let first_adapter = adapter.clone();
    let first_handle = thread::spawn(move || {
        first_adapter
            .run_command_at(&request(), &policy(), same_start_second)
            .expect("first command invocation should complete")
    });
    let second_handle = thread::spawn(move || {
        adapter
            .run_command_at(&request(), &policy(), same_start_second)
            .expect("second command invocation should complete independently")
    });
    let first = first_handle
        .join()
        .expect("first command invocation thread should not panic");
    let second = second_handle
        .join()
        .expect("second command invocation thread should not panic");

    assert_eq!(first.request_id(), "same-consumer-correlation-id");
    assert_eq!(second.request_id(), "same-consumer-correlation-id");
    assert_ne!(
        first.sandbox_id(),
        second.sandbox_id(),
        "parallel one-shot invocations must not share a runtime resource identity"
    );
    assert_runtime_identity_retains_128_bits(first.sandbox_id());
    assert_runtime_identity_retains_128_bits(second.sandbox_id());

    let calls = fs::read_to_string(&call_log).expect("fake Podman calls should be recorded");
    let create_names = calls
        .lines()
        .filter_map(|line| {
            let mut fields = line.split_whitespace();
            (fields.next() == Some("create") && fields.next() == Some("--name"))
                .then(|| fields.next().map(str::to_owned))
                .flatten()
        })
        .collect::<Vec<_>>();
    let remove_names = calls
        .lines()
        .filter_map(|line| {
            let fields = line.split_whitespace().collect::<Vec<_>>();
            (fields.first() == Some(&"rm")
                && fields.get(1) == Some(&"--force")
                && fields.get(2) == Some(&"--ignore"))
            .then(|| fields.get(3).map(|value| (*value).to_owned()))
            .flatten()
        })
        .collect::<Vec<_>>();

    assert_eq!(create_names.len(), 2);
    assert_eq!(remove_names.len(), 2);
    assert_eq!(
        create_names.iter().cloned().collect::<BTreeSet<_>>().len(),
        2
    );
    assert_eq!(
        remove_names.iter().cloned().collect::<BTreeSet<_>>().len(),
        2
    );
    assert_eq!(
        create_names.iter().cloned().collect::<BTreeSet<_>>(),
        remove_names.iter().cloned().collect::<BTreeSet<_>>(),
        "cleanup must remain scoped to the exact runtime-owned resource identity"
    );

    let _ = fs::remove_file(program);
    let _ = fs::remove_file(call_log);
}
