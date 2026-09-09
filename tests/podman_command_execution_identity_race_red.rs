//! Regression for one-shot command sandbox resource identity.
//!
//! `CommandExecutionRequest::request_id` is consumer correlation metadata, not
//! an idempotency/resource key. Two legitimate command invocations may reuse it
//! in the same supplied start second. Each invocation must still receive an
//! independent runtime-owned sandbox identity.
//!
//! Post-create lifecycle authority is the exact container ID returned by the
//! runtime, not the generated correlation name. The fixture therefore records
//! create names, acquired IDs, and removal IDs independently, using one marker
//! file per fact so concurrent fake-runtime processes cannot lose trace lines
//! through a shared append log.

#![cfg(target_os = "linux")]

use std::{
    collections::BTreeSet,
    fs,
    os::unix::fs::PermissionsExt,
    path::{Path, PathBuf},
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

fn marker_values(directory: &Path, prefix: &str) -> BTreeSet<String> {
    fs::read_dir(directory)
        .expect("marker directory should be readable")
        .filter_map(|entry| {
            let file_name = entry.ok()?.file_name().into_string().ok()?;
            file_name.strip_prefix(prefix).map(str::to_owned)
        })
        .collect()
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
    let marker_directory = temporary_path("markers");
    fs::create_dir(&marker_directory).expect("marker directory should be creatable");
    let script = format!(
        "#!/bin/sh\nset -eu\nMARKERS='{}'\ncase \"${{1:-}}:${{2:-}}\" in\n  info:--format) printf '%s\\n' '{{\"host\":{{\"security\":{{\"rootless\":true,\"seccompEnabled\":true,\"seccompProfilePath\":\"/usr/share/containers/seccomp.json\",\"apparmorEnabled\":true,\"selinuxEnabled\":false}}}},\"version\":{{\"Version\":\"6.1.0\"}}}}' ;;\n  create:--name)\n    sandbox_name=\"${{3:?missing sandbox name}}\"\n    sandbox_suffix=\"${{sandbox_name#qsr-cmd-}}\"\n    owned_id=\"owned-${{sandbox_suffix}}\"\n    : > \"$MARKERS/create-$sandbox_name\"\n    : > \"$MARKERS/acquired-$owned_id\"\n    printf '%s\\n' \"$owned_id\"\n    ;;\n  init:*) : ;;\n  start:*) : ;;\n  container:inspect)\n    container_id=\"${{5:?missing container id}}\"\n    printf '[{{\"Id\":\"%s\",\"AppArmorProfile\":\"containers-default\",\"ProcessLabel\":\"\",\"EffectiveCaps\":[],\"BoundingCaps\":[],\"Config\":{{\"User\":\"65532:65532\"}},\"HostConfig\":{{\"ReadonlyRootfs\":true,\"Privileged\":false,\"SecurityOpt\":[\"no-new-privileges\"],\"UsernsMode\":\"auto\",\"PidMode\":\"private\",\"IpcMode\":\"none\",\"NetworkMode\":\"none\",\"Memory\":268435456,\"NanoCpus\":1000000000,\"PidsLimit\":16}}}}]\\n' \"$container_id\"\n    ;;\n  top:*) printf 'PID SECCOMP CAPEFF CAPBND CAPINH CAPPRM CAPAMB LABEL\\n1 filter - - - - - containers-default (enforce)\\n' ;;\n  wait:*) printf '0\\n' ;;\n  logs:*) printf 'ok\\n' ;;\n  rm:--force)\n    container_id=\"${{4:?missing cleanup container id}}\"\n    : > \"$MARKERS/remove-$container_id\"\n    ;;\n  *) exit 91 ;;\nesac\n",
        marker_directory.display(),
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

    let create_names = marker_values(&marker_directory, "create-");
    let acquired_ids = marker_values(&marker_directory, "acquired-");
    let remove_ids = marker_values(&marker_directory, "remove-");
    let result_names = BTreeSet::from([
        first.sandbox_id().to_owned(),
        second.sandbox_id().to_owned(),
    ]);
    let expected_acquired_ids = create_names
        .iter()
        .map(|name| {
            format!(
                "owned-{}",
                name.strip_prefix("qsr-cmd-")
                    .expect("create marker must retain the command sandbox prefix")
            )
        })
        .collect::<BTreeSet<_>>();

    assert_eq!(create_names.len(), 2);
    assert_eq!(acquired_ids.len(), 2);
    assert_eq!(remove_ids.len(), 2);
    assert_eq!(create_names, result_names);
    assert_eq!(acquired_ids, expected_acquired_ids);
    assert_eq!(remove_ids, acquired_ids);
    assert!(
        create_names.is_disjoint(&remove_ids),
        "post-create cleanup must use acquired container IDs, not correlation names"
    );

    let _ = fs::remove_file(program);
    let _ = fs::remove_dir_all(marker_directory);
}
