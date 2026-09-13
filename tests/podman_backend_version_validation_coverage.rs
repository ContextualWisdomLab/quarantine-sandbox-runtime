//! Backend evidence validation coverage for the rootless Podman adapter.

#![cfg(target_os = "linux")]

use std::{
    fs,
    path::{Path, PathBuf},
    sync::Arc,
};

use quarantine_sandbox_runtime::{
    ApplicationServiceError, CommandExecutionError, CommandExecutionRequest, IsolationPolicy,
    ResourceRequest, RootlessPodmanAdapter,
};
use tempfile::TempDir;

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

impl From<FixturePath> for PathBuf {
    fn from(value: FixturePath) -> Self {
        value.path.clone()
    }
}

fn temporary_path(name: &str) -> FixturePath {
    let directory = Arc::new(
        tempfile::Builder::new()
            .prefix("qsr-backend-evidence-coverage-")
            .tempdir()
            .expect("isolated fixture directory"),
    );
    FixturePath {
        path: directory.path().join(name),
        _directory: directory,
    }
}

fn immutable_fixture_executable() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/fake_podman.sh")
}

fn fixture_sidecar(program: &Path, suffix: &str) -> PathBuf {
    PathBuf::from(format!("{}.{suffix}", program.display()))
}

fn write_executable(name: &str, script: &str) -> FixturePath {
    let program = temporary_path(name);
    std::os::unix::fs::symlink(immutable_fixture_executable(), &program)
        .expect("fake Podman immutable symlink should be creatable");
    let script_path = fixture_sidecar(program.as_ref(), "script");
    let init_capable_script = format!("if [ \"${{1:-}}\" = init ]; then exit 0; fi\n{script}");
    fs::write(&script_path, init_capable_script)
        .expect("fake Podman scenario data should be writable");
    fs::write(
        fixture_sidecar(program.as_ref(), "config"),
        format!(
            "MODE='source_script'\nLOG='/dev/null'\nSCRIPT='{}'\n",
            script_path.display()
        ),
    )
    .expect("fake Podman dispatcher config should be writable");
    program
}

fn policy() -> IsolationPolicy {
    IsolationPolicy {
        policy_id: "backend_evidence_coverage".to_owned(),
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
        request_id: "backend-evidence-coverage".to_owned(),
        image_reference: format!("localhost/cwl/tool@sha256:{}", "e".repeat(64)),
        command: vec!["payload-sentinel".to_owned()],
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

fn security_info_json(version_json: &str) -> String {
    format!(
        "{{\"host\":{{\"security\":{{\"rootless\":true,\"seccompEnabled\":true,\"seccompProfilePath\":\"/usr/share/containers/seccomp.json\",\"apparmorEnabled\":true,\"selinuxEnabled\":false}}}},\"version\":{{\"Version\":{version_json}}}}}"
    )
}

fn security_info_lsm_json(apparmor_enabled: bool, selinux_enabled: bool) -> String {
    format!(
        "{{\"host\":{{\"security\":{{\"rootless\":true,\"seccompEnabled\":true,\"seccompProfilePath\":\"/usr/share/containers/seccomp.json\",\"apparmorEnabled\":{apparmor_enabled},\"selinuxEnabled\":{selinux_enabled}}}}},\"version\":{{\"Version\":\"6.1.0\"}}}}"
    )
}

fn container_inspect_json(id: &str) -> String {
    container_inspect_lsm_json(id, "containers-default", "")
}

fn container_inspect_lsm_json(id: &str, apparmor_profile: &str, process_label: &str) -> String {
    format!(
        "[{{\"Id\":\"{id}\",\"AppArmorProfile\":\"{apparmor_profile}\",\"ProcessLabel\":\"{process_label}\",\
         \"EffectiveCaps\":null,\"BoundingCaps\":null,\"Config\":{{\"User\":\"65532:65532\",\"Timeout\":20}},\
         \"HostConfig\":{{\"ReadonlyRootfs\":true,\"Privileged\":false,\
         \"SecurityOpt\":[\"no-new-privileges\"],\"UsernsMode\":\"\",\
         \"Annotations\":{{\"io.podman.annotations.userns\":\"auto\"}},\
         \"PidMode\":\"private\",\"IpcMode\":\"none\",\"NetworkMode\":\"none\",\"UTSMode\":\"private\",\"CgroupMode\":\"private\",\"Memory\":268435456,\
         \"NanoCpus\":1000000000,\"PidsLimit\":16,\"Tmpfs\":{{\"/tmp\":\"rw,noexec,nosuid,nodev,size=16777216\"}}}}}}]"
    )
}

fn assert_version_is_rejected(name: &str, version_json: &str) {
    let backend_info = security_info_json(version_json);
    let script = format!(
        "#!/bin/sh\nset -eu\ncase \"${{1:-}}:${{2:-}}\" in\n  info:--format) printf '%s\\n' '{backend_info}' ;;\n  *) exit 91 ;;\nesac\n"
    );
    let program = write_executable(name, &script);
    let adapter = RootlessPodmanAdapter::new(program.clone());

    assert_eq!(
        adapter.run_legacy_command_at_for_test(&request(), &policy(), 1_780_000_000),
        Err(CommandExecutionError::Backend(
            ApplicationServiceError::MalformedIsolationInspection {
                operation: "backend_security_info",
            },
        ))
    );

    let _ = fs::remove_file(program);
}

fn assert_create_identifier_is_rejected(name: &str, create_response: &str) {
    let backend_info = security_info_json("\"6.1.0\"");
    let script = format!(
        "#!/bin/sh\nset -eu\ncase \"${{1:-}}:${{2:-}}\" in\n  info:--format) printf '%s\\n' '{backend_info}' ;;\n  create:*) {create_response} ;;\n  *) exit 91 ;;\nesac\n"
    );
    let program = write_executable(name, &script);
    let adapter = RootlessPodmanAdapter::new(program.clone());

    assert_eq!(
        adapter.run_legacy_command_at_for_test(&request(), &policy(), 1_780_000_000),
        Err(CommandExecutionError::Backend(
            ApplicationServiceError::MalformedIsolationInspection {
                operation: "container_create",
            },
        ))
    );

    let _ = fs::remove_file(program);
}

fn assert_process_security_top_is_rejected(name: &str, top_payload: &str) {
    let backend_info = security_info_json("\"6.1.0\"");
    let inspect = container_inspect_json("fake-command-container-id");
    let script = format!(
        "#!/bin/sh\nset -eu\ncase \"${{1:-}}:${{2:-}}\" in\n  info:--format) printf '%s\\n' '{backend_info}' ;;\n  create:--name) printf 'fake-command-container-id\\n' ;;\n  start:*) : ;;\n  container:inspect) printf '%s\\n' '{inspect}' ;;\n  top:*) printf '%s' '{top_payload}' ;;\n  rm:--force) : ;;\n  *) exit 91 ;;\nesac\n"
    );
    let program = write_executable(name, &script);
    let adapter = RootlessPodmanAdapter::new(program.clone());

    assert_eq!(
        adapter.run_legacy_command_at_for_test(&request(), &policy(), 1_780_000_000),
        Err(CommandExecutionError::Backend(
            ApplicationServiceError::MalformedIsolationInspection {
                operation: "process_security_top",
            },
        ))
    );

    let _ = fs::remove_file(program);
}

fn assert_lsm_evidence_is_rejected(
    name: &str,
    apparmor_enabled: bool,
    selinux_enabled: bool,
    apparmor_profile: &str,
    process_label: &str,
    runtime_label: &str,
) {
    let backend_info = security_info_lsm_json(apparmor_enabled, selinux_enabled);
    let inspect =
        container_inspect_lsm_json("fake-command-container-id", apparmor_profile, process_label);
    let top_payload = format!(
        "PID SECCOMP CAPEFF CAPBND CAPINH CAPPRM CAPAMB LABEL\n1 filter - - - - - {runtime_label}\n"
    );
    let script = format!(
        "#!/bin/sh\nset -eu\ncase \"${{1:-}}:${{2:-}}\" in\n  info:--format) printf '%s\\n' '{backend_info}' ;;\n  create:--name) printf 'fake-command-container-id\\n' ;;\n  start:*) : ;;\n  container:inspect) printf '%s\\n' '{inspect}' ;;\n  top:*) printf '%s' '{top_payload}' ;;\n  rm:--force) : ;;\n  *) exit 91 ;;\nesac\n"
    );
    let program = write_executable(name, &script);
    let adapter = RootlessPodmanAdapter::new(program.clone());

    assert_eq!(
        adapter.run_legacy_command_at_for_test(&request(), &policy(), 1_780_000_000),
        Err(CommandExecutionError::Backend(
            ApplicationServiceError::IsolationVerificationFailed {
                control_name: "lsm",
            },
        ))
    );

    let _ = fs::remove_file(program);
}

fn assert_process_capability_evidence(name: &str, capability_value: &str, accepted: bool) {
    let backend_info = security_info_json("\"6.1.0\"");
    let inspect = container_inspect_json("fake-command-container-id");
    let top_payload = format!(
        "PID SECCOMP CAPEFF CAPBND CAPINH CAPPRM CAPAMB LABEL\n1 filter {capability_value} {capability_value} {capability_value} {capability_value} {capability_value} containers-default (enforce)\n"
    );
    let script = format!(
        "#!/bin/sh\nset -eu\ncase \"${{1:-}}:${{2:-}}\" in\n  info:--format) printf '%s\\n' '{backend_info}' ;;\n  create:--name) printf 'fake-command-container-id\\n' ;;\n  start:*) : ;;\n  container:inspect) printf '%s\\n' '{inspect}' ;;\n  top:*) printf '%s' '{top_payload}' ;;\n  wait:*) printf '0\\n' ;;\n  logs:*) : ;;\n  rm:--force) : ;;\n  *) exit 91 ;;\nesac\n"
    );
    let program = write_executable(name, &script);
    let adapter = RootlessPodmanAdapter::new(program.clone());
    let result = adapter.run_legacy_command_at_for_test(&request(), &policy(), 1_780_000_000);

    if accepted {
        assert!(result.is_ok(), "{name} should be accepted: {result:?}");
    } else {
        assert_eq!(
            result,
            Err(CommandExecutionError::Backend(
                ApplicationServiceError::IsolationVerificationFailed {
                    control_name: "all_capabilities_dropped",
                },
            ))
        );
    }

    let _ = fs::remove_file(program);
}

#[test]
fn backend_security_version_rejects_each_malformed_text_class() {
    assert_version_is_rejected("empty-version", "\"\"");
    assert_version_is_rejected("oversized-version", &format!("\"{}\"", "x".repeat(129)));
    assert_version_is_rejected("control-version", "\"6.1.0\\n\"");
}

#[test]
fn container_create_rejects_empty_and_whitespace_identifiers() {
    assert_create_identifier_is_rejected("empty-create-id", "printf '\\n'");
    assert_create_identifier_is_rejected("whitespace-create-id", "printf 'bad id\\n'");
}

#[test]
fn process_security_top_rejects_missing_short_and_duplicate_pid_one_evidence() {
    const HEADER: &str = "PID SECCOMP CAPEFF CAPBND CAPINH CAPPRM CAPAMB LABEL\n";
    assert_process_security_top_is_rejected(
        "missing-pid-one",
        &format!("{HEADER}2 filter - - - - - containers-default (enforce)\n"),
    );
    assert_process_security_top_is_rejected("short-pid-one", &format!("{HEADER}1 filter -\n"));
    assert_process_security_top_is_rejected(
        "duplicate-pid-one",
        &format!(
            "{HEADER}1 filter - - - - - containers-default (enforce)\n1 filter - - - - - containers-default (enforce)\n"
        ),
    );
}

#[test]
fn effective_lsm_rejects_untrusted_selinux_and_apparmor_evidence_shapes() {
    assert_lsm_evidence_is_rejected(
        "selinux-empty-inspect-label",
        false,
        true,
        "",
        "",
        "system_u:system_r:container_t:s0:c1,c2",
    );
    assert_lsm_evidence_is_rejected(
        "selinux-unconfined-inspect-label",
        false,
        true,
        "",
        "unconfined",
        "system_u:system_r:container_t:s0:c1,c2",
    );
    assert_lsm_evidence_is_rejected(
        "selinux-mismatched-runtime-label",
        false,
        true,
        "",
        "system_u:system_r:container_t:s0:c1,c2",
        "system_u:system_r:container_t:s0:c3,c4",
    );
    assert_lsm_evidence_is_rejected(
        "apparmor-missing-mode",
        true,
        false,
        "containers-default",
        "",
        "containers-default",
    );
    assert_lsm_evidence_is_rejected(
        "apparmor-unterminated-mode",
        true,
        false,
        "containers-default",
        "",
        "containers-default (enforce",
    );
    assert_lsm_evidence_is_rejected(
        "apparmor-unconfined-profile",
        true,
        false,
        "unconfined",
        "",
        "containers-default (enforce)",
    );
    assert_lsm_evidence_is_rejected(
        "no-runtime-lsm-enabled",
        false,
        false,
        "",
        "",
        "containers-default (enforce)",
    );
}

#[test]
fn process_capability_evidence_accepts_empty_masks_and_rejects_nonzero_masks() {
    for (name, capability_value) in [
        ("dash-capabilities", "-"),
        ("none-capabilities", "none"),
        ("decimal-zero-capabilities", "0"),
        ("hex-zero-capabilities", "0x0"),
        ("long-hex-zero-capabilities", "0x0000000000000000"),
        ("bare-zero-mask-capabilities", "0000000000000000"),
    ] {
        assert_process_capability_evidence(name, capability_value, true);
    }
    assert_process_capability_evidence("nonzero-capabilities", "0x1", false);
}
