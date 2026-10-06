//! Real-binary acceptance tests for the offline static analysis transport (#148).
//!
//! Each test launches the compiled `qsr-analyze` executable. Fixtures are
//! synthetic bytes; no artifact is executed and no network is used.
//!
//! ADR-0010 supports and tests the transport on Unix only.
#![cfg(unix)]

use std::{
    fs,
    path::{Path, PathBuf},
    process::{Command, Output},
};

use quarantine_sandbox_runtime::{
    CliExit, EvidenceBundle, IngestionPolicy, RuntimeDisposition, run_static_analysis_cli,
};

struct Fixture {
    root: PathBuf,
}

impl Fixture {
    fn new(name: &str) -> Self {
        let root = std::env::temp_dir().join(format!(
            "qsr-analyze-{name}-{}-{:?}",
            std::process::id(),
            std::thread::current().id()
        ));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).expect("fixture directory must be created");
        Self { root }
    }

    fn write(&self, name: &str, bytes: &[u8]) -> PathBuf {
        let path = self.root.join(name);
        fs::write(&path, bytes).expect("fixture file must be written");
        path
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}

fn request_json(profile: &str) -> String {
    format!(r#"{{"schema_version":"1.0.0","request_id":"cli_request_001","profile":"{profile}"}}"#)
}

fn run(args: &[&Path]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_qsr-analyze"))
        .args(args)
        .output()
        .expect("qsr-analyze binary must launch")
}

fn run_flags(request: &Path, artifact: &Path) -> Output {
    Command::new(env!("CARGO_BIN_EXE_qsr-analyze"))
        .arg("--request")
        .arg(request)
        .arg("--artifact")
        .arg(artifact)
        .output()
        .expect("qsr-analyze binary must launch")
}

#[test]
fn static_request_emits_validated_completed_bundle_on_stdout() {
    let fixture = Fixture::new("static");
    let request = fixture.write("request.json", request_json("static_only").as_bytes());
    let artifact = fixture.write("sample.bin", b"MZ\x90\x00");

    let output = run_flags(&request, &artifact);

    assert_eq!(output.status.code(), Some(0), "stderr: {:?}", output.stderr);
    assert!(output.stderr.is_empty());
    let bundle: EvidenceBundle =
        serde_json::from_slice(&output.stdout).expect("stdout must be an EvidenceBundle");
    assert_eq!(bundle.validate(), Ok(()));
    assert_eq!(bundle.disposition, RuntimeDisposition::Completed);
    assert!(bundle.consumer_verdict_required);
    assert!(!bundle.runtime.dynamic_execution_performed);
    assert!(!bundle.runtime.network_access_performed);
    assert_eq!(bundle.request_id, "cli_request_001");
}

#[test]
fn missing_arguments_are_a_usage_error_without_stdout() {
    let output = run(&[]);

    assert_eq!(output.status.code(), Some(64));
    assert!(output.stdout.is_empty());
}

fn assert_failure(output: &Output, exit: i32, code: &str, fixture: &Fixture) {
    assert_eq!(
        output.status.code(),
        Some(exit),
        "stderr: {:?}",
        output.stderr
    );
    assert!(output.stdout.is_empty(), "failure must not emit evidence");
    let stderr = String::from_utf8(output.stderr.clone()).expect("stderr must be UTF-8");
    assert_eq!(stderr, format!("qsr-analyze: error={code}\n"));
    assert!(!stderr.contains(fixture.root.to_string_lossy().as_ref()));
}

#[test]
fn malformed_argument_shapes_are_usage_errors() {
    let fixture = Fixture::new("usage");
    let request = fixture.write("request.json", request_json("static_only").as_bytes());
    let artifact = fixture.write("sample.bin", b"MZ");
    let unknown = Path::new("--verbose");
    let request_flag = Path::new("--request");
    let artifact_flag = Path::new("--artifact");

    for args in [
        vec![request_flag, &request],
        vec![request_flag, &request, artifact_flag],
        vec![
            request_flag,
            &request,
            request_flag,
            &request,
            artifact_flag,
            &artifact,
        ],
        vec![unknown, request_flag, &request, artifact_flag, &artifact],
    ] {
        assert_failure(&run(&args), 64, "usage", &fixture);
    }
}

#[test]
fn flags_are_accepted_in_either_order() {
    let fixture = Fixture::new("order");
    let request = fixture.write("request.json", request_json("static_only").as_bytes());
    let artifact = fixture.write("sample.bin", b"MZ");

    let output = run(&[
        Path::new("--artifact"),
        &artifact,
        Path::new("--request"),
        &request,
    ]);

    assert_eq!(output.status.code(), Some(0), "stderr: {:?}", output.stderr);
}

#[test]
fn invalid_requests_fail_closed_before_artifact_access() {
    let fixture = Fixture::new("request");
    let missing_artifact = fixture.root.join("absent.bin");
    let oversized = format!("{}{}", " ".repeat(64 * 1024), request_json("static_only"));
    let cases = [
        fixture.write("malformed.json", b"{"),
        fixture.write(
            "unknown.json",
            br#"{"schema_version":"1.0.0","request_id":"r","profile":"static_only","shell":"x"}"#,
        ),
        fixture.write(
            "version.json",
            br#"{"schema_version":"9.9.9","request_id":"r","profile":"static_only"}"#,
        ),
        fixture.write("oversized.json", oversized.as_bytes()),
        fixture.root.join("absent.json"),
    ];

    for request in cases {
        assert_failure(
            &run_flags(&request, &missing_artifact),
            65,
            "invalid_request",
            &fixture,
        );
    }
}

#[cfg(unix)]
#[test]
fn symlinked_request_is_rejected() {
    let fixture = Fixture::new("request-link");
    let target = fixture.write("target.json", request_json("static_only").as_bytes());
    let link = fixture.root.join("link.json");
    std::os::unix::fs::symlink(&target, &link).expect("symlink must be created");
    let artifact = fixture.write("sample.bin", b"MZ");

    assert_failure(
        &run_flags(&link, &artifact),
        65,
        "invalid_request",
        &fixture,
    );
}

#[test]
fn dynamic_profile_emits_inconclusive_bundle_without_execution() {
    let fixture = Fixture::new("dynamic");
    let request = fixture.write("request.json", request_json("linux_dynamic").as_bytes());
    let artifact = fixture.write("sample.bin", b"MZ\x90\x00");

    let output = run_flags(&request, &artifact);

    assert_eq!(output.status.code(), Some(0), "stderr: {:?}", output.stderr);
    let bundle: EvidenceBundle =
        serde_json::from_slice(&output.stdout).expect("stdout must be an EvidenceBundle");
    assert_eq!(bundle.disposition, RuntimeDisposition::Inconclusive);
    assert!(!bundle.runtime.dynamic_execution_performed);
    assert!(
        bundle
            .limitations
            .iter()
            .any(|limitation| limitation == "dynamic_analysis_not_configured")
    );
}

#[test]
fn unavailable_artifacts_fail_closed() {
    let fixture = Fixture::new("artifact-unavailable");
    let request = fixture.write("request.json", request_json("static_only").as_bytes());
    let directory = fixture.root.join("directory");
    fs::create_dir(&directory).expect("directory must be created");

    for artifact in [fixture.root.join("absent.bin"), directory] {
        assert_failure(
            &run_flags(&request, &artifact),
            66,
            "artifact_unavailable",
            &fixture,
        );
    }
}

#[cfg(unix)]
#[test]
fn symlinked_artifact_is_rejected() {
    let fixture = Fixture::new("artifact-link");
    let request = fixture.write("request.json", request_json("static_only").as_bytes());
    let target = fixture.write("target.bin", b"MZ");
    let link = fixture.root.join("link.bin");
    std::os::unix::fs::symlink(&target, &link).expect("symlink must be created");

    assert_failure(
        &run_flags(&request, &link),
        66,
        "artifact_unavailable",
        &fixture,
    );
}

#[cfg(unix)]
#[test]
fn character_device_artifact_is_rejected() {
    let fixture = Fixture::new("artifact-device");
    let request = fixture.write("request.json", request_json("static_only").as_bytes());

    assert_failure(
        &run_flags(&request, Path::new("/dev/null")),
        66,
        "artifact_unavailable",
        &fixture,
    );
}

#[cfg(unix)]
#[test]
fn fifo_artifact_is_rejected_without_blocking() {
    let fixture = Fixture::new("artifact-fifo");
    let request = fixture.write("request.json", request_json("static_only").as_bytes());
    let fifo = fixture.root.join("pipe.bin");
    let status = Command::new("mkfifo")
        .arg(&fifo)
        .status()
        .expect("mkfifo must launch");
    assert!(status.success());

    let mut child = Command::new(env!("CARGO_BIN_EXE_qsr-analyze"))
        .arg("--request")
        .arg(&request)
        .arg("--artifact")
        .arg(&fifo)
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .expect("qsr-analyze binary must launch");
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);
    while child
        .try_wait()
        .expect("child status must be readable")
        .is_none()
    {
        if std::time::Instant::now() > deadline {
            let _ = child.kill();
            let _ = child.wait();
            panic!("qsr-analyze blocked on a FIFO artifact");
        }
        std::thread::sleep(std::time::Duration::from_millis(20));
    }
    let output = child
        .wait_with_output()
        .expect("child output must be readable");
    assert_failure(&output, 66, "artifact_unavailable", &fixture);
}

#[test]
fn empty_and_oversized_artifacts_are_rejected() {
    let fixture = Fixture::new("artifact-rejected");
    let request = fixture.write("request.json", request_json("static_only").as_bytes());
    let empty = fixture.write("empty.bin", b"");
    let oversized = fixture.root.join("oversized.bin");
    let file = fs::File::create(&oversized).expect("oversized fixture must be created");
    let limit = u64::try_from(IngestionPolicy::default().maximum_artifact_bytes)
        .expect("policy bound must fit in u64");
    file.set_len(limit + 1)
        .expect("oversized fixture must be sized");

    for artifact in [empty, oversized] {
        assert_failure(
            &run_flags(&request, &artifact),
            67,
            "artifact_rejected",
            &fixture,
        );
    }
}

struct FailingWriter;

impl std::io::Write for FailingWriter {
    fn write(&mut self, _bytes: &[u8]) -> std::io::Result<usize> {
        Err(std::io::Error::other("closed"))
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Err(std::io::Error::other("closed"))
    }
}

#[test]
fn output_write_failure_is_internal_error() {
    let fixture = Fixture::new("write-failure");
    let request = fixture.write("request.json", request_json("static_only").as_bytes());
    let artifact = fixture.write("sample.bin", b"MZ");
    let mut stderr = Vec::new();

    let exit = run_static_analysis_cli(
        [
            "--request".into(),
            request.into_os_string(),
            "--artifact".into(),
            artifact.into_os_string(),
        ],
        &mut FailingWriter,
        &mut stderr,
    );

    assert_eq!(exit, CliExit::Internal);
    assert_eq!(exit.as_u8(), 70);
    assert_eq!(stderr, b"qsr-analyze: error=internal\n");
}

#[test]
fn exit_statuses_are_stable() {
    assert_eq!(
        [
            CliExit::Success,
            CliExit::Usage,
            CliExit::InvalidRequest,
            CliExit::ArtifactUnavailable,
            CliExit::ArtifactRejected,
            CliExit::Internal,
        ]
        .map(CliExit::as_u8),
        [0, 64, 65, 66, 67, 70]
    );
}
