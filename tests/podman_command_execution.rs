//! Fake-Podman-process coverage for `RootlessPodmanAdapter::run_command_at`.
//!
//! Mirrors the established pattern in `tests/podman_launch_error_propagation.rs` and
//! `tests/podman_apparmor_mode_suffix.rs`: a `/bin/sh` script standing in for the real
//! `podman` binary, matched on its first two argv tokens. Real-Podman acceptance lives
//! separately in `tests/podman_command_execution_e2e.rs`.

#![cfg(target_os = "linux")]

use std::{
    fs,
    os::unix::fs::PermissionsExt,
    path::{Path, PathBuf},
    sync::Arc,
    time::Duration,
};
use tempfile::TempDir;

use quarantine_sandbox_runtime::{
    ApplicationServiceError, CommandExecutionBackend, CommandExecutionError,
    CommandExecutionRequest, IsolationPolicy, PrSourceArtifactInput, ResourceRequest,
    RootlessPodmanAdapter,
};
use sha2::{Digest, Sha256};

#[derive(Clone)]
struct FixturePath {
    _directory: Arc<TempDir>,
    path: PathBuf,
}

impl AsRef<Path> for FixturePath {
    fn as_ref(&self) -> &Path {
        &self.path
    }
}

impl std::ops::Deref for FixturePath {
    type Target = Path;

    fn deref(&self) -> &Self::Target {
        &self.path
    }
}

impl From<FixturePath> for PathBuf {
    fn from(value: FixturePath) -> Self {
        value.path.clone()
    }
}

fn temporary_path(name: &str) -> FixturePath {
    let directory = Arc::new(
        tempfile::Builder::new()
            .prefix("qsr-cmdexec-test-")
            .tempdir()
            .expect("isolated fixture directory"),
    );
    FixturePath {
        path: directory.path().join(name),
        _directory: directory,
    }
}

fn write_executable(name: &str, script: &str) -> FixturePath {
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
        policy_id: "cmdexec_fake_policy_v1".to_owned(),
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

fn request(lease_seconds: u32) -> CommandExecutionRequest {
    CommandExecutionRequest {
        schema_version: "1.0.0".to_owned(),
        request_id: "cmdexec-fake-request".to_owned(),
        image_reference: format!("localhost/cwl/tool@sha256:{}", "e".repeat(64)),
        command: vec!["pytest".to_owned(), "-q".to_owned()],
        source_artifact: None,
        resources: ResourceRequest {
            memory_bytes: 256 * 1024 * 1024,
            cpu_millicores: 1_000,
            maximum_processes: 16,
            lease_seconds,
            tmpfs_bytes: 16 * 1024 * 1024,
        },
    }
}

fn single_file_tree_digest(path: &str, bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update((path.len() as u64).to_be_bytes());
    hasher.update(path.as_bytes());
    hasher.update((bytes.len() as u64).to_be_bytes());
    hasher.update(bytes);
    format!("{:x}", hasher.finalize())
}

/// `UsernsMode` empty, `auto` only recorded as the Podman 6.1.0-style annotation.
fn security_info_json() -> &'static str {
    r#"{"host":{"security":{"rootless":true,"seccompEnabled":true,"seccompProfilePath":"/usr/share/containers/seccomp.json","apparmorEnabled":true,"selinuxEnabled":false}},"version":{"Version":"6.1.0"}}"#
}

fn container_inspect_json(id: &str) -> String {
    format!(
        "[{{\"Id\":\"{id}\",\"AppArmorProfile\":\"containers-default\",\"ProcessLabel\":\"\",\
         \"EffectiveCaps\":null,\"BoundingCaps\":null,\"Config\":{{\"User\":\"65532:65532\"}},\
         \"HostConfig\":{{\"ReadonlyRootfs\":true,\"Privileged\":false,\
         \"SecurityOpt\":[\"no-new-privileges\"],\"UsernsMode\":\"\",\
         \"Annotations\":{{\"io.podman.annotations.userns\":\"auto\"}},\
         \"PidMode\":\"private\",\"IpcMode\":\"none\",\"NetworkMode\":\"none\",\"Memory\":268435456,\
         \"NanoCpus\":1000000000,\"PidsLimit\":16}}}}]"
    )
}

fn container_inspect_json_with_source(id: &str) -> String {
    format!(
        "[{{\"Id\":\"{id}\",\"AppArmorProfile\":\"containers-default\",\"ProcessLabel\":\"\",\
         \"EffectiveCaps\":null,\"BoundingCaps\":null,\"Config\":{{\"User\":\"65532:65532\"}},\
         \"HostConfig\":{{\"ReadonlyRootfs\":true,\"Privileged\":false,\
         \"SecurityOpt\":[\"no-new-privileges\"],\"UsernsMode\":\"\",\
         \"Annotations\":{{\"io.podman.annotations.userns\":\"auto\"}},\
         \"PidMode\":\"private\",\"IpcMode\":\"none\",\"NetworkMode\":\"none\",\"Memory\":268435456,\
         \"NanoCpus\":1000000000,\"PidsLimit\":16}},\
         \"Mounts\":[{{\"Source\":\"SOURCE_PATH\",\"Destination\":\"/workspace\",\"Type\":\"bind\",\
         \"Options\":[\"noexec\",\"nosuid\",\"nodev\"],\"RW\":false}}]}}]"
    )
}

const TOP_HEADER: &str = "PID SECCOMP CAPEFF CAPBND CAPINH CAPPRM CAPAMB LABEL";

fn top_line_apparmor_confined() -> String {
    format!("{TOP_HEADER}\n1 filter - - - - - containers-default (enforce)\n")
}

#[test]
fn run_command_at_returns_a_successful_result_against_a_well_behaved_fake_backend() {
    let script = format!(
        "#!/bin/sh\nset -eu\ncase \"${{1:-}}:${{2:-}}\" in\n  \
         info:--format) printf '%s\\n' '{}' ;;\n  \
         create:--name) printf 'fake-command-container-id\\n' ;;\n  \
         start:*) : ;;\n  \
         container:inspect) printf '%s\\n' '{}' ;;\n  \
         top:*) printf '%s' '{}' ;;\n  \
         wait:*) printf '0\\n' ;;\n  \
         logs:*) printf 'sandbox stdout\\n'; printf 'sandbox stderr\\n' >&2 ;;\n  \
         rm:--force) : ;;\n  \
         *) exit 91 ;;\nesac\n",
        security_info_json(),
        container_inspect_json("fake-command-container-id"),
        top_line_apparmor_confined(),
    );
    let program = write_executable("success", &script);
    let adapter = RootlessPodmanAdapter::new(program.clone());

    let result = adapter
        .run_command_at(&request(20), &policy(), 1_780_000_000)
        .expect("well-behaved fake backend should run to completion");

    assert_eq!(result.exit_code(), 0);
    assert!(!result.timed_out());
    assert_eq!(result.stdout(), "sandbox stdout\n");
    assert!(!result.stdout_truncated());
    assert_eq!(result.stderr(), "sandbox stderr\n");
    assert!(!result.stderr_truncated());
    assert_eq!(result.backend_id(), "rootless_podman");
    assert_eq!(result.backend_version(), "6.1.0");
    assert!(
        result.sandbox_id().starts_with("qsr-cmd-"),
        "command-execution sandbox names must carry the qsr-cmd- prefix, got {}",
        result.sandbox_id()
    );

    let _ = fs::remove_file(program);
}

#[test]
fn run_command_at_rejects_an_invalid_policy_before_invoking_podman() {
    let invocation_log = temporary_path("invalid-policy-invocations");
    let script = format!(
        "#!/bin/sh\nset -eu\nprintf 'invoked\\n' >> '{}'\nexit 91\n",
        invocation_log.display()
    );
    let program = write_executable("invalid-policy", &script);
    let adapter = RootlessPodmanAdapter::new(program.clone());
    let mut invalid_policy = policy();
    invalid_policy.run_as_user_id = 0;

    let result = adapter.run_command_at(&request(20), &invalid_policy, 1_780_000_000);

    assert_eq!(
        result,
        Err(CommandExecutionError::Backend(
            ApplicationServiceError::InvalidPolicy {
                field_name: "run_as_user_id",
            },
        ))
    );
    assert!(
        !invocation_log.exists(),
        "invalid direct-adapter requests must not invoke Podman"
    );

    let _ = fs::remove_file(program);
}

#[test]
fn run_command_at_mounts_only_staged_exact_revision_source_and_returns_its_receipt() {
    let source = temporary_path("pr-source");
    fs::create_dir_all(&source).expect("source directory");
    let source_bytes = b"print('isolated')\n";
    fs::write(source.join("check.py"), source_bytes).expect("source file");
    fs::set_permissions(source.join("check.py"), fs::Permissions::from_mode(0o755))
        .expect("executable source fixture");
    let create_log = temporary_path("pr-source-create-args");
    let mount_source_log = temporary_path("pr-source-mount-source");
    let script = format!(
        "#!/bin/sh\nset -eu\ncase \"${{1:-}}:${{2:-}}\" in\n  \
         info:--format) printf '%s\\n' '{}' ;;\n  \
         create:--name) printf '%s\\n' \"$@\" > '{}'; for arg in \"$@\"; do case \"$arg\" in *:/workspace:*) printf '%s' \"${{arg%%:/workspace:*}}\" > '{}' ;; esac; done; printf 'fake-command-container-id\\n' ;;\n  \
         start:*) : ;;\n  \
         container:inspect) source_path=$(cat '{}'); printf '%s\\n' '{}' | sed \"s|SOURCE_PATH|$source_path|\" ;;\n  \
         top:*) printf '%s' '{}' ;;\n  \
         wait:*) printf '0\\n' ;;\n  \
         logs:*) : ;;\n  \
         rm:--force) : ;;\n  \
         *) exit 91 ;;\nesac\n",
        security_info_json(),
        create_log.display(),
        mount_source_log.display(),
        mount_source_log.display(),
        container_inspect_json_with_source("fake-command-container-id"),
        top_line_apparmor_confined(),
    );
    let program = write_executable("pr-source", &script);
    let adapter = RootlessPodmanAdapter::new(program.clone());
    let mut command_request = request(20);
    command_request.source_artifact = Some(PrSourceArtifactInput {
        host_path: source.to_path_buf(),
        revision_sha: "d".repeat(40),
        expected_tree_sha256: single_file_tree_digest("check.py", source_bytes),
    });

    let result = adapter
        .run_command_at(&command_request, &policy(), 1_780_000_000)
        .expect("verified staged source should execute");

    let create_args = fs::read_to_string(&create_log).expect("create args should be recorded");
    assert!(create_args.contains(":/workspace:ro,noexec,nosuid,nodev,Z"));
    assert!(create_args.contains("--workdir\n/workspace"));
    assert!(!create_args.contains(&source.display().to_string()));
    let receipt = result.source_artifact_receipt().expect("source receipt");
    assert_eq!(receipt.revision_sha(), "d".repeat(40));
    assert_eq!(receipt.executable_files_stripped(), 1);
    assert!(receipt.mounted_read_only());
    assert!(receipt.mounted_noexec());

    let wrong_source_inspection = container_inspect_json_with_source("wrong-source-container")
        .replace("SOURCE_PATH", "/tmp/not-this-invocation");
    let wrong_source_script = format!(
        "#!/bin/sh\nset -eu\ncase \"${{1:-}}:${{2:-}}\" in\n  \
         info:--format) printf '%s\\n' '{}' ;;\n  \
         create:--name) printf 'wrong-source-container\\n' ;;\n  \
         start:*) : ;;\n  \
         container:inspect) printf '%s\\n' '{}' ;;\n  \
         stop:*|rm:--force) : ;;\n  \
         *) exit 91 ;;\nesac\n",
        security_info_json(),
        wrong_source_inspection,
    );
    let wrong_source_program = write_executable("pr-source-wrong-bind", &wrong_source_script);
    let wrong_source_adapter = RootlessPodmanAdapter::new(wrong_source_program.clone());
    assert_eq!(
        wrong_source_adapter.run_command_at(&command_request, &policy(), 1_780_000_001),
        Err(CommandExecutionError::Backend(
            ApplicationServiceError::IsolationVerificationFailed {
                control_name: "source_artifact_bind_source",
            },
        ))
    );

    let _ = fs::remove_file(program);
    let _ = fs::remove_file(wrong_source_program);
    let _ = fs::remove_file(create_log);
    let _ = fs::remove_file(mount_source_log);
    let _ = fs::remove_dir_all(source);
}

#[test]
fn run_command_at_reports_a_nonzero_exit_status_as_a_successful_call() {
    let script = format!(
        "#!/bin/sh\nset -eu\ncase \"${{1:-}}:${{2:-}}\" in\n  \
         info:--format) printf '%s\\n' '{}' ;;\n  \
         create:--name) printf 'fake-command-container-id\\n' ;;\n  \
         start:*) : ;;\n  \
         container:inspect) printf '%s\\n' '{}' ;;\n  \
         top:*) printf '%s' '{}' ;;\n  \
         wait:*) printf '17\\n' ;;\n  \
         logs:*) : ;;\n  \
         rm:--force) : ;;\n  \
         *) exit 91 ;;\nesac\n",
        security_info_json(),
        container_inspect_json("fake-command-container-id"),
        top_line_apparmor_confined(),
    );
    let program = write_executable("nonzero-exit", &script);
    let adapter = RootlessPodmanAdapter::new(program.clone());

    let result = adapter
        .run_command_at(&request(20), &policy(), 1_780_000_000)
        .expect("a nonzero workload exit status must not be a CommandExecutionError");

    assert_eq!(result.exit_code(), 17);
    assert!(!result.timed_out());

    let _ = fs::remove_file(program);
}

#[test]
fn run_command_at_rejects_a_backend_that_is_not_rootless() {
    let script = "#!/bin/sh\nset -eu\ncase \"${1:-}:${2:-}\" in\n  \
         info:--format) printf '%s\\n' '{\"host\":{\"security\":{\"rootless\":false,\"seccompEnabled\":true,\"seccompProfilePath\":\"/x\",\"apparmorEnabled\":true,\"selinuxEnabled\":false}},\"version\":{\"Version\":\"6.1.0\"}}' ;;\n  \
         *) exit 91 ;;\nesac\n"
        .to_owned();
    let program = write_executable("not-rootless", &script);
    let adapter = RootlessPodmanAdapter::new(program.clone());

    let error = adapter
        .run_command_at(&request(20), &policy(), 1_780_000_000)
        .unwrap_err();

    assert_eq!(
        error,
        CommandExecutionError::Backend(ApplicationServiceError::BackendNotRootless)
    );
    let _ = fs::remove_file(program);
}

#[test]
fn run_command_at_rejects_a_command_container_attached_to_a_network() {
    let inspect = container_inspect_json("fake-command-container-id")
        .replace("\"NetworkMode\":\"none\"", "\"NetworkMode\":\"bridge\"");
    let script = format!(
        "#!/bin/sh\nset -eu\ncase \"${{1:-}}:${{2:-}}\" in\n  \
         info:--format) printf '%s\\n' '{}' ;;\n  \
         create:--name) printf 'fake-command-container-id\\n' ;;\n  \
         start:*) : ;;\n  \
         container:inspect) printf '%s\\n' '{}' ;;\n  \
         rm:--force) : ;;\n  \
         *) exit 91 ;;\nesac\n",
        security_info_json(),
        inspect,
    );
    let program = write_executable("network-attached", &script);
    let adapter = RootlessPodmanAdapter::new(program.clone());

    let result = adapter.run_command_at(&request(20), &policy(), 1_780_000_000);

    assert_eq!(
        result,
        Err(CommandExecutionError::Backend(
            ApplicationServiceError::IsolationVerificationFailed {
                control_name: "external_egress_denied",
            },
        ))
    );
    let _ = fs::remove_file(program);
}

#[test]
fn run_command_at_cleans_up_when_container_create_returns_a_malformed_identifier() {
    let log = temporary_path("malformed-create-log");
    let script = format!(
        "#!/bin/sh\nset -eu\nprintf '%s\\n' \"$*\" >> '{}'\ncase \"${{1:-}}:${{2:-}}\" in\n  \
         info:--format) printf '%s\\n' '{}' ;;\n  \
         create:--name) printf '\\n' ;;\n  \
         rm:--force) : ;;\n  \
         *) exit 91 ;;\nesac\n",
        log.display(),
        security_info_json(),
    );
    let program = write_executable("malformed-create", &script);
    let adapter = RootlessPodmanAdapter::new(program.clone());

    let error = adapter
        .run_command_at(&request(20), &policy(), 1_780_000_000)
        .unwrap_err();

    assert_eq!(
        error,
        CommandExecutionError::Backend(ApplicationServiceError::MalformedIsolationInspection {
            operation: "container_create",
        })
    );
    let calls = fs::read_to_string(&log).expect("cleanup calls should be recorded");
    assert!(calls.contains("rm --force"));

    let _ = fs::remove_file(program);
    let _ = fs::remove_file(log);
}

#[test]
fn run_command_at_cleans_up_when_container_start_fails() {
    let log = temporary_path("start-fails-log");
    let script = format!(
        "#!/bin/sh\nset -eu\nprintf '%s\\n' \"$*\" >> '{}'\ncase \"${{1:-}}:${{2:-}}\" in\n  \
         info:--format) printf '%s\\n' '{}' ;;\n  \
         create:--name) printf 'fake-command-container-id\\n' ;;\n  \
         start:*) exit 1 ;;\n  \
         rm:--force) : ;;\n  \
         *) exit 91 ;;\nesac\n",
        log.display(),
        security_info_json(),
    );
    let program = write_executable("start-fails", &script);
    let adapter = RootlessPodmanAdapter::new(program.clone());

    let error = adapter
        .run_command_at(&request(20), &policy(), 1_780_000_000)
        .unwrap_err();

    assert_eq!(
        error,
        CommandExecutionError::Backend(ApplicationServiceError::BackendCommandFailed {
            operation: "container_start",
        })
    );
    let calls = fs::read_to_string(&log).expect("cleanup calls should be recorded");
    assert!(calls.contains("rm --force"));

    let _ = fs::remove_file(program);
    let _ = fs::remove_file(log);
}

#[test]
fn run_command_at_cleans_up_when_isolation_verification_fails() {
    let log = temporary_path("verify-fails-log");
    // ReadonlyRootfs:false must fail the very first isolation control.
    let malformed_inspect = container_inspect_json("fake-command-container-id")
        .replace("\"ReadonlyRootfs\":true", "\"ReadonlyRootfs\":false");
    let script = format!(
        "#!/bin/sh\nset -eu\nprintf '%s\\n' \"$*\" >> '{}'\ncase \"${{1:-}}:${{2:-}}\" in\n  \
         info:--format) printf '%s\\n' '{}' ;;\n  \
         create:--name) printf 'fake-command-container-id\\n' ;;\n  \
         start:*) : ;;\n  \
         container:inspect) printf '%s\\n' '{}' ;;\n  \
         rm:--force) : ;;\n  \
         *) exit 91 ;;\nesac\n",
        log.display(),
        security_info_json(),
        malformed_inspect,
    );
    let program = write_executable("verify-fails", &script);
    let adapter = RootlessPodmanAdapter::new(program.clone());

    let error = adapter
        .run_command_at(&request(20), &policy(), 1_780_000_000)
        .unwrap_err();

    assert_eq!(
        error,
        CommandExecutionError::Backend(ApplicationServiceError::IsolationVerificationFailed {
            control_name: "read_only_root_filesystem",
        })
    );
    let calls = fs::read_to_string(&log).expect("cleanup calls should be recorded");
    assert!(calls.contains("rm --force"));

    let _ = fs::remove_file(program);
    let _ = fs::remove_file(log);
}

#[test]
fn run_command_at_kills_and_reports_a_command_that_exceeds_its_lease() {
    let marker = temporary_path("timeout-marker");
    let script = format!(
        "#!/bin/sh\nset -eu\ncase \"${{1:-}}:${{2:-}}\" in\n  \
         info:--format) printf '%s\\n' '{}' ;;\n  \
         create:--name) printf 'fake-command-container-id\\n' ;;\n  \
         start:*) : ;;\n  \
         container:inspect) printf '%s\\n' '{}' ;;\n  \
         top:*) printf '%s' '{}' ;;\n  \
         wait:*) if [ -f '{}' ]; then printf '137\\n'; else touch '{}'; sleep 5; fi ;;\n  \
         kill:*) : ;;\n  \
         logs:*) : ;;\n  \
         rm:--force) : ;;\n  \
         *) exit 91 ;;\nesac\n",
        security_info_json(),
        container_inspect_json("fake-command-container-id"),
        top_line_apparmor_confined(),
        marker.display(),
        marker.display(),
    );
    let program = write_executable("timeout", &script);
    let adapter =
        RootlessPodmanAdapter::new(program.clone()).with_command_timeout(Duration::from_secs(2));

    let result = adapter
        .run_command_at(&request(1), &policy(), 1_780_000_000)
        .expect("a killed workload is still a completed, successfully-observed run");

    assert!(result.timed_out());
    assert_eq!(result.exit_code(), 137);

    let _ = fs::remove_file(program);
    let _ = fs::remove_file(marker);
}

#[test]
fn run_command_at_fails_closed_when_log_retrieval_itself_hangs() {
    let script = format!(
        "#!/bin/sh\nset -eu\ncase \"${{1:-}}:${{2:-}}\" in\n  \
         info:--format) printf '%s\\n' '{}' ;;\n  \
         create:--name) printf 'fake-command-container-id\\n' ;;\n  \
         start:*) : ;;\n  \
         container:inspect) printf '%s\\n' '{}' ;;\n  \
         top:*) printf '%s' '{}' ;;\n  \
         wait:*) printf '0\\n' ;;\n  \
         logs:*) sleep 5 ;;\n  \
         rm:--force) : ;;\n  \
         *) exit 91 ;;\nesac\n",
        security_info_json(),
        container_inspect_json("fake-command-container-id"),
        top_line_apparmor_confined(),
    );
    let program = write_executable("logs-hang", &script);
    let adapter = RootlessPodmanAdapter::new(program.clone())
        .with_command_timeout(Duration::from_millis(200));

    let error = adapter
        .run_command_at(&request(20), &policy(), 1_780_000_000)
        .unwrap_err();

    assert_eq!(
        error,
        CommandExecutionError::Backend(ApplicationServiceError::BackendCommandTimedOut {
            operation: "container_logs",
        })
    );

    let _ = fs::remove_file(program);
}

#[test]
fn run_command_at_reports_cleanup_failure_after_an_otherwise_successful_run() {
    let script = format!(
        "#!/bin/sh\nset -eu\ncase \"${{1:-}}:${{2:-}}\" in\n  \
         info:--format) printf '%s\\n' '{}' ;;\n  \
         create:--name) printf 'fake-command-container-id\\n' ;;\n  \
         start:*) : ;;\n  \
         container:inspect) printf '%s\\n' '{}' ;;\n  \
         top:*) printf '%s' '{}' ;;\n  \
         wait:*) printf '0\\n' ;;\n  \
         logs:*) : ;;\n  \
         rm:--force) exit 1 ;;\n  \
         *) exit 91 ;;\nesac\n",
        security_info_json(),
        container_inspect_json("fake-command-container-id"),
        top_line_apparmor_confined(),
    );
    let program = write_executable("cleanup-fails", &script);
    let adapter = RootlessPodmanAdapter::new(program.clone());

    let error = adapter
        .run_command_at(&request(20), &policy(), 1_780_000_000)
        .unwrap_err();

    assert_eq!(
        error,
        CommandExecutionError::Backend(ApplicationServiceError::CleanupFailed)
    );

    let _ = fs::remove_file(program);
}

#[test]
fn command_execution_backend_trait_forwards_to_run_command_at() {
    let script = format!(
        "#!/bin/sh\nset -eu\ncase \"${{1:-}}:${{2:-}}\" in\n  \
         info:--format) printf '%s\\n' '{}' ;;\n  \
         create:--name) printf 'fake-command-container-id\\n' ;;\n  \
         start:*) : ;;\n  \
         container:inspect) printf '%s\\n' '{}' ;;\n  \
         top:*) printf '%s' '{}' ;;\n  \
         wait:*) printf '0\\n' ;;\n  \
         logs:*) : ;;\n  \
         rm:--force) : ;;\n  \
         *) exit 91 ;;\nesac\n",
        security_info_json(),
        container_inspect_json("fake-command-container-id"),
        top_line_apparmor_confined(),
    );
    let program = write_executable("trait-forward", &script);
    let adapter = RootlessPodmanAdapter::new(program.clone());
    let backend: &dyn CommandExecutionBackend = &adapter;

    let result = backend
        .run_to_completion_at(&request(20), &policy(), 1_780_000_000)
        .expect("the trait object must forward to the inherent run_command_at");

    assert_eq!(result.exit_code(), 0);

    let _ = fs::remove_file(program);
}

#[test]
fn run_command_at_fails_closed_when_backend_loss_prevents_cleanup_during_wait() {
    // The script deletes itself once isolation verification's `top` call
    // completes. Both the following `podman wait` and the mandatory cleanup
    // spawn therefore fail. Cleanup provenance has higher safety priority than
    // the earlier backend error because the sandbox may still be present.
    let script = format!(
        "#!/bin/sh\nset -eu\ncase \"${{1:-}}:${{2:-}}\" in\n  \
         info:--format) printf '%s\\n' '{}' ;;\n  \
         create:--name) printf 'fake-command-container-id\\n' ;;\n  \
         start:*) : ;;\n  \
         container:inspect) printf '%s\\n' '{}' ;;\n  \
         top:*) printf '%s' '{}'; rm -f \"$0\" ;;\n  \
         *) exit 91 ;;\nesac\n",
        security_info_json(),
        container_inspect_json("fake-command-container-id"),
        top_line_apparmor_confined(),
    );
    let program = write_executable("wait-spawn-fails", &script);
    let adapter = RootlessPodmanAdapter::new(program.clone());

    let error = adapter
        .run_command_at(&request(20), &policy(), 1_780_000_000)
        .unwrap_err();

    assert_eq!(
        error,
        CommandExecutionError::Backend(ApplicationServiceError::CleanupFailed)
    );

    let _ = fs::remove_file(program);
}

#[test]
fn run_command_at_fails_closed_when_backend_loss_prevents_cleanup_during_log_retrieval() {
    // Same technique as the `command_wait` variant above, one call later:
    // the script deletes itself once `wait` succeeds, so `podman logs` and
    // the mandatory cleanup can no longer be spawned. The unverified cleanup
    // state must dominate the earlier log-retrieval failure.
    let script = format!(
        "#!/bin/sh\nset -eu\ncase \"${{1:-}}:${{2:-}}\" in\n  \
         info:--format) printf '%s\\n' '{}' ;;\n  \
         create:--name) printf 'fake-command-container-id\\n' ;;\n  \
         start:*) : ;;\n  \
         container:inspect) printf '%s\\n' '{}' ;;\n  \
         top:*) printf '%s' '{}' ;;\n  \
         wait:*) printf '0\\n'; rm -f \"$0\" ;;\n  \
         *) exit 91 ;;\nesac\n",
        security_info_json(),
        container_inspect_json("fake-command-container-id"),
        top_line_apparmor_confined(),
    );
    let program = write_executable("logs-spawn-fails", &script);
    let adapter = RootlessPodmanAdapter::new(program.clone());

    let error = adapter
        .run_command_at(&request(20), &policy(), 1_780_000_000)
        .unwrap_err();

    assert_eq!(
        error,
        CommandExecutionError::Backend(ApplicationServiceError::CleanupFailed)
    );

    let _ = fs::remove_file(program);
}
