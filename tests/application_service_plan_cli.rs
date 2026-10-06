//! Acceptance tests for the offline application-service launch-plan transport (#149).
//!
//! The transport validates a request against an operator policy with the
//! runtime's UTF-8 byte bounds and prints the deterministic Podman plan. It
//! never executes a backend process, so a plan is never isolation evidence.

use std::{
    ffi::OsString,
    fs,
    io::{self, Write},
    path::{Path, PathBuf},
    process::Command,
    sync::atomic::{AtomicU64, Ordering},
};

use quarantine_sandbox_runtime::{
    ApplicationServiceError, ApplicationServiceRequest, IsolationPolicy, ResourceRequest,
    RootlessPodmanAdapter, ServicePlanExit, ServiceProtocol, run_application_service_plan_cli,
};
use serde_json::Value;

const STARTED_AT: u64 = 1_790_000_000;
const INPUT_LIMIT_BYTES: usize = 64 * 1024;

struct Fixture {
    root: PathBuf,
}

impl Fixture {
    fn new(name: &str) -> Self {
        static NEXT_FIXTURE: AtomicU64 = AtomicU64::new(0);
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_or(0, |elapsed| elapsed.as_nanos());
        let root = std::env::temp_dir().join(format!(
            "qsr-service-plan-{name}-{}-{nanos}-{}",
            std::process::id(),
            NEXT_FIXTURE.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&root).expect("fixture directory must be created exclusively");
        Self { root }
    }

    fn write(&self, name: &str, bytes: &[u8]) -> PathBuf {
        let path = self.root.join(name);
        fs::write(&path, bytes).expect("fixture file must be written");
        path
    }

    fn path(&self, name: &str) -> PathBuf {
        self.root.join(name)
    }

    fn valid_inputs(&self) -> (PathBuf, PathBuf) {
        (
            self.write("request.json", &json(&request())),
            self.write("policy.json", &json(&policy())),
        )
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}

fn policy() -> IsolationPolicy {
    IsolationPolicy {
        policy_id: "operator_policy_v1".to_owned(),
        maximum_memory_bytes: 512 * 1024 * 1024,
        maximum_cpu_millicores: 2_000,
        maximum_processes: 128,
        maximum_lease_seconds: 900,
        maximum_tmpfs_bytes: 128 * 1024 * 1024,
        readiness_timeout_millis: 10_000,
        readiness_poll_interval_millis: 50,
        shutdown_grace_seconds: 2,
        run_as_user_id: 65_532,
        run_as_group_id: 65_532,
    }
}

fn request() -> ApplicationServiceRequest {
    ApplicationServiceRequest {
        schema_version: "1.0.0".to_owned(),
        request_id: "preflight_request_001".to_owned(),
        image_reference: format!("localhost/cwl/tool@sha256:{}", "a".repeat(64)),
        container_port: 8_080,
        protocol: ServiceProtocol::Http,
        command: vec!["serve".to_owned(), "--port=8080".to_owned()],
        resources: ResourceRequest {
            memory_bytes: 256 * 1024 * 1024,
            cpu_millicores: 1_000,
            maximum_processes: 32,
            lease_seconds: 300,
            tmpfs_bytes: 32 * 1024 * 1024,
        },
    }
}

fn json<T: serde::Serialize>(value: &T) -> Vec<u8> {
    serde_json::to_vec(value).expect("fixture must serialize")
}

fn arguments(values: &[&str]) -> Vec<OsString> {
    values.iter().map(OsString::from).collect()
}

fn flags(request: &Path, policy: &Path, started_at: &str) -> Vec<OsString> {
    vec![
        "--request".into(),
        request.into(),
        "--policy".into(),
        policy.into(),
        "--started-at".into(),
        started_at.into(),
    ]
}

struct Outcome {
    exit: ServicePlanExit,
    stdout: Vec<u8>,
    stderr: String,
}

fn run(arguments: Vec<OsString>) -> Outcome {
    let mut stdout = Vec::new();
    let mut stderr = Vec::new();
    let exit = run_application_service_plan_cli(arguments, &mut stdout, &mut stderr);
    Outcome {
        exit,
        stdout,
        stderr: String::from_utf8(stderr).expect("diagnostics must be UTF-8"),
    }
}

fn assert_failure(outcome: &Outcome, exit: ServicePlanExit, status: u8, code: &str) {
    assert_eq!(outcome.exit, exit, "unexpected exit for {code}");
    assert_eq!(outcome.exit.as_u8(), status);
    assert!(outcome.stdout.is_empty(), "failure must not emit a plan");
    assert_eq!(outcome.stderr, format!("qsr-service-plan: error={code}\n"));
}

fn string_array(value: &Value) -> Vec<String> {
    value
        .as_array()
        .expect("argv must be a JSON array")
        .iter()
        .map(|item| {
            item.as_str()
                .expect("argv entries must be strings")
                .to_owned()
        })
        .collect()
}

#[test]
fn valid_inputs_emit_the_exact_library_plan_with_constant_non_evidence_markers() {
    let fixture = Fixture::new("success");
    let (request_path, policy_path) = fixture.valid_inputs();

    let outcome = run(flags(&request_path, &policy_path, &STARTED_AT.to_string()));

    assert_eq!(outcome.exit, ServicePlanExit::Success);
    assert_eq!(outcome.exit.as_u8(), 0);
    assert!(outcome.stderr.is_empty());
    assert_eq!(outcome.stdout.last(), Some(&b'\n'));
    let document: Value = serde_json::from_slice(&outcome.stdout).expect("plan must be JSON");
    let expected = RootlessPodmanAdapter::plan_at(&request(), &policy(), STARTED_AT)
        .expect("fixture must produce a plan");

    assert_eq!(document["schema_version"], "1.0.0");
    assert_eq!(document["kind"], "application_service_launch_plan");
    assert_eq!(document["execution"], "not_performed");
    assert_eq!(document["isolation_evidence"], "not_established");
    assert_eq!(document["sandbox_name"], expected.sandbox_name());
    assert_eq!(document["network_name"], expected.network_name());
    assert_eq!(
        document["expires_at_epoch_seconds"],
        expected.expires_at_epoch_seconds()
    );
    assert_eq!(
        string_array(&document["rootless_probe_args"]),
        expected.rootless_probe_args()
    );
    assert_eq!(
        string_array(&document["network_create_args"]),
        expected.network_create_args()
    );
    assert_eq!(
        string_array(&document["container_create_args"]),
        expected.container_create_args()
    );
    assert_eq!(document.as_object().map(serde_json::Map::len), Some(10));
}

#[test]
fn emitted_plan_matches_the_published_closed_schema() {
    let fixture = Fixture::new("schema");
    let (request_path, policy_path) = fixture.valid_inputs();
    let outcome = run(flags(&request_path, &policy_path, "0"));
    assert_eq!(outcome.exit, ServicePlanExit::Success);
    let document: Value = serde_json::from_slice(&outcome.stdout).expect("plan must be JSON");

    let schema_path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("schemas/application-service-launch-plan.schema.json");
    let schema: Value = serde_json::from_slice(
        &fs::read(schema_path).expect("launch-plan schema must be published"),
    )
    .expect("launch-plan schema must be JSON");

    assert_eq!(
        schema["$schema"],
        "https://json-schema.org/draft/2020-12/schema"
    );
    assert_eq!(
        schema["$id"],
        "https://contextualwisdomlab.org/schemas/quarantine/application-service-launch-plan-1.0.0.schema.json"
    );
    assert_eq!(schema["additionalProperties"], false);
    let properties = schema["properties"]
        .as_object()
        .expect("schema properties must be an object");
    let required: Vec<&str> = schema["required"]
        .as_array()
        .expect("schema required must be an array")
        .iter()
        .map(|name| name.as_str().expect("required names must be strings"))
        .collect();
    let emitted = document.as_object().expect("plan must be a JSON object");
    assert_eq!(required.len(), properties.len());
    assert_eq!(emitted.len(), properties.len());
    for name in required {
        assert!(properties.contains_key(name), "{name} must be declared");
        assert!(emitted.contains_key(name), "{name} must be emitted");
    }
    for constant in ["schema_version", "kind", "execution", "isolation_evidence"] {
        assert_eq!(properties[constant]["const"], document[constant]);
    }
    for (name, prefix) in [("sandbox_name", "qsr-app-"), ("network_name", "qsr-net-")] {
        assert_eq!(properties[name]["type"], "string");
        assert_eq!(
            properties[name]["pattern"],
            format!("^{prefix}[0-9a-f]{{16}}$")
        );
        let value = document[name].as_str().expect("names must be strings");
        let suffix = value
            .strip_prefix(prefix)
            .expect("name must use its prefix");
        assert_eq!(suffix.len(), 16, "{name} suffix length");
        assert!(
            suffix
                .bytes()
                .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte)),
            "{name} suffix must be lower-case hexadecimal"
        );
    }
    assert_eq!(properties["expires_at_epoch_seconds"]["type"], "integer");
    assert_eq!(properties["expires_at_epoch_seconds"]["minimum"], 0);
    assert_eq!(properties["expires_at_epoch_seconds"]["maximum"], u64::MAX);
    assert!(document["expires_at_epoch_seconds"].is_u64());
    for name in [
        "rootless_probe_args",
        "network_create_args",
        "container_create_args",
    ] {
        assert_eq!(properties[name]["type"], "array");
        assert_eq!(properties[name]["minItems"], 1);
        assert_eq!(properties[name]["items"]["type"], "string");
        let items = document[name].as_array().expect("argv must be an array");
        assert!(!items.is_empty(), "{name} must not be empty");
        assert!(items.iter().all(Value::is_string), "{name} items");
    }
}

#[test]
fn plan_carries_p0_isolation_flags_and_no_host_authority() {
    let fixture = Fixture::new("flags");
    let (request_path, policy_path) = fixture.valid_inputs();
    let outcome = run(flags(&request_path, &policy_path, "1"));
    let document: Value = serde_json::from_slice(&outcome.stdout).expect("plan must be JSON");
    let create = string_array(&document["container_create_args"]);

    for required in [
        "--pull=never",
        "--read-only",
        "--cap-drop=all",
        "--security-opt=no-new-privileges",
        "--userns=auto",
        "--ipc=none",
        "--pid=private",
        "--restart=no",
    ] {
        assert!(
            create.iter().any(|argument| argument == required),
            "{required}"
        );
    }
    assert!(
        create
            .iter()
            .any(|argument| argument == "127.0.0.1::8080/tcp")
    );
    for forbidden in ["--privileged", "--network=host", "--pid=host", "--ipc=host"] {
        assert!(
            !create.iter().any(|argument| argument == forbidden),
            "{forbidden}"
        );
    }
    assert!(!create.iter().any(|argument| argument.contains(".sock")));
    assert!(
        string_array(&document["network_create_args"])
            .iter()
            .any(|argument| argument == "--internal")
    );
}

#[test]
fn arguments_are_accepted_in_any_order() {
    let fixture = Fixture::new("order");
    let (request_path, policy_path) = fixture.valid_inputs();
    let reordered = vec![
        OsString::from("--started-at"),
        OsString::from("7"),
        OsString::from("--policy"),
        policy_path.into(),
        OsString::from("--request"),
        request_path.into(),
    ];
    assert_eq!(run(reordered).exit, ServicePlanExit::Success);
}

#[test]
fn malformed_arguments_are_usage_errors() {
    let fixture = Fixture::new("usage");
    let (request_path, policy_path) = fixture.valid_inputs();
    let request_text = request_path.to_str().expect("fixture path must be UTF-8");
    let policy_text = policy_path.to_str().expect("fixture path must be UTF-8");

    let cases: Vec<Vec<OsString>> = vec![
        arguments(&[]),
        arguments(&["--request", request_text, "--policy", policy_text]),
        arguments(&["--request", request_text, "--started-at", "1"]),
        arguments(&["--policy", policy_text, "--started-at", "1"]),
        arguments(&["--request"]),
        arguments(&[
            "--request",
            request_text,
            "--request",
            request_text,
            "--policy",
            policy_text,
            "--started-at",
            "1",
        ]),
        arguments(&[
            "--request",
            request_text,
            "--policy",
            policy_text,
            "--started-at",
            "1",
            "--extra",
        ]),
        arguments(&[
            "--request",
            request_text,
            "--policy",
            policy_text,
            "--started-at",
            "",
        ]),
        arguments(&[
            "--request",
            request_text,
            "--policy",
            policy_text,
            "--started-at",
            "+5",
        ]),
        arguments(&[
            "--request",
            request_text,
            "--policy",
            policy_text,
            "--started-at",
            "-1",
        ]),
        arguments(&[
            "--request",
            request_text,
            "--policy",
            policy_text,
            "--started-at",
            " 1",
        ]),
        arguments(&[
            "--request",
            request_text,
            "--policy",
            policy_text,
            "--started-at",
            "1e3",
        ]),
        arguments(&[
            "--request",
            request_text,
            "--policy",
            policy_text,
            "--started-at",
            "18446744073709551616",
        ]),
    ];
    for case in cases {
        assert_failure(&run(case), ServicePlanExit::Usage, 64, "usage");
    }
}

#[cfg(unix)]
#[test]
fn non_utf8_flags_and_timestamps_are_usage_errors() {
    use std::os::unix::ffi::OsStringExt;

    let fixture = Fixture::new("non-utf8");
    let (request_path, policy_path) = fixture.valid_inputs();
    let invalid = OsString::from_vec(vec![0xff, 0xfe]);

    let mut bad_flag = flags(&request_path, &policy_path, "1");
    bad_flag[0] = invalid.clone();
    assert_failure(&run(bad_flag), ServicePlanExit::Usage, 64, "usage");

    let mut bad_timestamp = flags(&request_path, &policy_path, "1");
    bad_timestamp[5] = invalid;
    assert_failure(&run(bad_timestamp), ServicePlanExit::Usage, 64, "usage");
}

#[test]
fn missing_inputs_are_unavailable_by_role() {
    let fixture = Fixture::new("missing");
    let (request_path, policy_path) = fixture.valid_inputs();
    let missing = fixture.path("missing.json");

    assert_failure(
        &run(flags(&missing, &policy_path, "1")),
        ServicePlanExit::InputUnavailable,
        66,
        "request_unavailable",
    );
    assert_failure(
        &run(flags(&request_path, &missing, "1")),
        ServicePlanExit::InputUnavailable,
        66,
        "policy_unavailable",
    );
}

#[cfg(unix)]
#[test]
fn symlink_directory_and_fifo_inputs_are_rejected_without_blocking() {
    let fixture = Fixture::new("special");
    let (request_path, policy_path) = fixture.valid_inputs();
    let link = fixture.path("request-link.json");
    std::os::unix::fs::symlink(&request_path, &link).expect("symlink fixture");
    let directory = fixture.path("directory.json");
    fs::create_dir(&directory).expect("directory fixture");
    let fifo = fixture.path("fifo.json");
    let status = Command::new("mkfifo")
        .arg(&fifo)
        .status()
        .expect("mkfifo must run");
    assert!(status.success());

    for special in [&link, &directory, &fifo] {
        let (request_path, policy_path, special) =
            (request_path.clone(), policy_path.clone(), special.clone());
        let (sender, receiver) = std::sync::mpsc::channel();
        std::thread::spawn(move || {
            let outcomes = (
                run(flags(&special, &policy_path, "1")),
                run(flags(&request_path, &special, "1")),
            );
            let _ = sender.send(outcomes);
        });
        let (request_outcome, policy_outcome) = receiver
            .recv_timeout(std::time::Duration::from_secs(5))
            .expect("special-file inputs must be rejected without blocking");
        assert_failure(
            &request_outcome,
            ServicePlanExit::InputUnavailable,
            66,
            "request_unavailable",
        );
        assert_failure(
            &policy_outcome,
            ServicePlanExit::InputUnavailable,
            66,
            "policy_unavailable",
        );
    }
}

#[test]
fn inputs_are_bounded_to_sixty_four_kibibytes_inclusive() {
    let fixture = Fixture::new("bounds");
    let (request_path, policy_path) = fixture.valid_inputs();

    let mut exact_request = json(&request());
    exact_request.resize(INPUT_LIMIT_BYTES, b' ');
    let exact_request_path = fixture.write("exact-request.json", &exact_request);
    let mut exact_policy = json(&policy());
    exact_policy.resize(INPUT_LIMIT_BYTES, b' ');
    let exact_policy_path = fixture.write("exact-policy.json", &exact_policy);
    assert_eq!(
        run(flags(&exact_request_path, &exact_policy_path, "1")).exit,
        ServicePlanExit::Success
    );

    exact_request.push(b' ');
    let over_request = fixture.write("over-request.json", &exact_request);
    assert_failure(
        &run(flags(&over_request, &policy_path, "1")),
        ServicePlanExit::InputTooLarge,
        67,
        "request_too_large",
    );
    exact_policy.push(b' ');
    let over_policy = fixture.write("over-policy.json", &exact_policy);
    assert_failure(
        &run(flags(&request_path, &over_policy, "1")),
        ServicePlanExit::InputTooLarge,
        67,
        "policy_too_large",
    );
}

#[test]
fn malformed_and_unknown_field_inputs_are_rejected_by_role() {
    let fixture = Fixture::new("malformed");
    let (request_path, policy_path) = fixture.valid_inputs();

    let mut unknown_request: Value = serde_json::from_slice(&json(&request())).expect("JSON");
    unknown_request["environment"] = Value::from("SECRET=1");
    let mut unknown_policy: Value = serde_json::from_slice(&json(&policy())).expect("JSON");
    unknown_policy["privileged"] = Value::from(true);

    for (name, bytes) in [
        ("not-json.json", b"{".to_vec()),
        ("invalid-utf8.json", vec![0xff, 0xfe]),
        ("unknown-request.json", json(&unknown_request)),
    ] {
        let path = fixture.write(name, &bytes);
        assert_failure(
            &run(flags(&path, &policy_path, "1")),
            ServicePlanExit::InvalidInput,
            65,
            "request_malformed",
        );
    }
    for (name, bytes) in [
        ("policy-not-json.json", b"{".to_vec()),
        ("policy-invalid-utf8.json", vec![0xff, 0xfe]),
        ("policy-wrong-json-type.json", b"[]".to_vec()),
        ("unknown-policy.json", json(&unknown_policy)),
    ] {
        let path = fixture.write(name, &bytes);
        assert_failure(
            &run(flags(&request_path, &path, "1")),
            ServicePlanExit::InvalidInput,
            65,
            "policy_malformed",
        );
    }
}

#[test]
fn validation_failures_map_to_fixed_codes_without_echoing_values() {
    let fixture = Fixture::new("validation");
    let policy_path = fixture.write("policy.json", &json(&policy()));
    let secret = "do-not-echo-this-value";

    let mut cases: Vec<(ApplicationServiceRequest, &str)> = Vec::new();
    let mut candidate = request();
    candidate.schema_version = secret.to_owned();
    cases.push((candidate, "unsupported_schema_version"));
    let mut candidate = request();
    candidate.request_id = format!("{secret}\u{7}");
    cases.push((candidate, "invalid_request_id"));
    let mut candidate = request();
    candidate.image_reference = format!("localhost/{secret}:latest");
    cases.push((candidate, "image_reference_not_digest_pinned"));
    let mut candidate = request();
    candidate.container_port = 0;
    cases.push((candidate, "invalid_container_port"));
    let mut candidate = request();
    candidate.command = vec!["x".to_owned(); 65];
    cases.push((candidate, "too_many_command_arguments"));
    let mut candidate = request();
    candidate.command = vec![String::new()];
    cases.push((candidate, "invalid_command_argument"));
    let mut candidate = request();
    candidate.resources.memory_bytes = policy().maximum_memory_bytes + 1;
    cases.push((candidate, "resource_limit_exceeded"));

    for (index, (candidate, code)) in cases.into_iter().enumerate() {
        let path = fixture.write(&format!("request-{index}.json"), &json(&candidate));
        let outcome = run(flags(&path, &policy_path, "1"));
        assert_failure(&outcome, ServicePlanExit::InvalidInput, 65, code);
        assert!(!outcome.stderr.contains(secret));
        assert!(
            !outcome
                .stderr
                .contains(fixture.root.to_str().unwrap_or("/"))
        );
    }

    let mut invalid_policy = policy();
    invalid_policy.run_as_user_id = 0;
    let request_path = fixture.write("request.json", &json(&request()));
    let invalid_policy_path = fixture.write("invalid-policy.json", &json(&invalid_policy));
    assert_failure(
        &run(flags(&request_path, &invalid_policy_path, "1")),
        ServicePlanExit::InvalidInput,
        65,
        "invalid_policy",
    );
}

#[test]
fn policy_is_validated_before_the_request_is_read() {
    let fixture = Fixture::new("policy-first");
    let mut invalid_policy = policy();
    invalid_policy.run_as_user_id = 0;
    let invalid_policy_path = fixture.write("invalid-policy.json", &json(&invalid_policy));
    let mut invalid_request = request();
    invalid_request.request_id = String::new();
    let invalid_request_path = fixture.write("invalid-request.json", &json(&invalid_request));
    let malformed_request_path = fixture.write("malformed-request.json", b"{");
    let oversized_request_path =
        fixture.write("oversized-request.json", &vec![b' '; INPUT_LIMIT_BYTES + 1]);

    for request_path in [
        fixture.path("absent-request.json"),
        malformed_request_path,
        oversized_request_path,
        invalid_request_path,
    ] {
        assert_failure(
            &run(flags(&request_path, &invalid_policy_path, "1")),
            ServicePlanExit::InvalidInput,
            65,
            "invalid_policy",
        );
    }
}

#[test]
fn lease_expiry_overflow_is_an_invalid_input_not_a_panic() {
    let fixture = Fixture::new("overflow");
    let (request_path, policy_path) = fixture.valid_inputs();
    let lease = u64::from(request().resources.lease_seconds);

    let last_representable = (u64::MAX - lease).to_string();
    assert_eq!(
        run(flags(&request_path, &policy_path, &last_representable)).exit,
        ServicePlanExit::Success
    );
    for started_at in [u64::MAX - lease + 1, u64::MAX] {
        assert_failure(
            &run(flags(&request_path, &policy_path, &started_at.to_string())),
            ServicePlanExit::InvalidInput,
            65,
            "lease_expiry_overflow",
        );
    }
}

#[test]
fn multibyte_request_identifier_and_argv_use_inclusive_utf8_byte_bounds() {
    let fixture = Fixture::new("utf8");
    let policy_path = fixture.write("policy.json", &json(&policy()));

    let mut exact = request();
    exact.request_id = format!("{}aa", "한".repeat(42));
    exact.command = vec!["😀".repeat(256)];
    assert_eq!(exact.request_id.len(), 128);
    assert_eq!(exact.command[0].len(), 1_024);
    let exact_path = fixture.write("exact.json", &json(&exact));
    assert_eq!(
        run(flags(&exact_path, &policy_path, "1")).exit,
        ServicePlanExit::Success
    );

    let mut over_identifier = request();
    over_identifier.request_id = "é".repeat(65);
    assert_eq!(over_identifier.request_id.chars().count(), 65);
    let path = fixture.write("over-identifier.json", &json(&over_identifier));
    assert_failure(
        &run(flags(&path, &policy_path, "1")),
        ServicePlanExit::InvalidInput,
        65,
        "invalid_request_id",
    );

    let mut over_argument = request();
    over_argument.command = vec!["serve".to_owned(), "é".repeat(513)];
    let path = fixture.write("over-argument.json", &json(&over_argument));
    assert_failure(
        &run(flags(&path, &policy_path, "1")),
        ServicePlanExit::InvalidInput,
        65,
        "invalid_command_argument",
    );
}

#[test]
fn every_application_service_error_has_one_stable_wire_code() {
    let cases = [
        (
            ApplicationServiceError::UnsupportedSchemaVersion {
                actual_version: "do-not-echo".to_owned(),
            },
            "unsupported_schema_version",
        ),
        (
            ApplicationServiceError::InvalidRequestId,
            "invalid_request_id",
        ),
        (
            ApplicationServiceError::ImageReferenceNotDigestPinned,
            "image_reference_not_digest_pinned",
        ),
        (
            ApplicationServiceError::InvalidContainerPort,
            "invalid_container_port",
        ),
        (
            ApplicationServiceError::TooManyCommandArguments {
                maximum_arguments: 64,
            },
            "too_many_command_arguments",
        ),
        (
            ApplicationServiceError::InvalidCommandArgument { argument_index: 3 },
            "invalid_command_argument",
        ),
        (
            ApplicationServiceError::InvalidPolicy {
                field_name: "policy_id",
            },
            "invalid_policy",
        ),
        (
            ApplicationServiceError::ResourceLimitExceeded {
                resource_name: "memory_bytes",
            },
            "resource_limit_exceeded",
        ),
        (
            ApplicationServiceError::LeaseExpiryOverflow,
            "lease_expiry_overflow",
        ),
        (
            ApplicationServiceError::BackendInvocationFailed { operation: "x" },
            "backend_invocation_failed",
        ),
        (
            ApplicationServiceError::BackendCommandTimedOut { operation: "x" },
            "backend_command_timed_out",
        ),
        (
            ApplicationServiceError::BackendOutputLimitExceeded { operation: "x" },
            "backend_output_limit_exceeded",
        ),
        (
            ApplicationServiceError::BackendCommandFailed { operation: "x" },
            "backend_command_failed",
        ),
        (
            ApplicationServiceError::BackendNotRootless,
            "backend_not_rootless",
        ),
        (
            ApplicationServiceError::IsolationVerificationFailed {
                control_name: "lsm",
            },
            "isolation_verification_failed",
        ),
        (
            ApplicationServiceError::MalformedIsolationInspection { operation: "x" },
            "malformed_isolation_inspection",
        ),
        (
            ApplicationServiceError::InvalidPortMapping,
            "invalid_port_mapping",
        ),
        (
            ApplicationServiceError::ReadinessTimeout,
            "readiness_timeout",
        ),
        (ApplicationServiceError::CleanupFailed, "cleanup_failed"),
    ];
    let mut seen = std::collections::BTreeSet::new();
    for (error, code) in cases {
        assert_eq!(error.code(), code);
        assert!(seen.insert(code), "duplicate wire code {code}");
    }
}

struct FailingWriter;

impl Write for FailingWriter {
    fn write(&mut self, _buffer: &[u8]) -> io::Result<usize> {
        Err(io::Error::other("closed"))
    }

    fn flush(&mut self) -> io::Result<()> {
        Err(io::Error::other("closed"))
    }
}

#[test]
fn output_write_failure_is_internal() {
    let fixture = Fixture::new("write-failure");
    let (request_path, policy_path) = fixture.valid_inputs();
    let mut stderr = Vec::new();

    let exit = run_application_service_plan_cli(
        flags(&request_path, &policy_path, "1"),
        &mut FailingWriter,
        &mut stderr,
    );

    assert_eq!(exit, ServicePlanExit::Internal);
    assert_eq!(exit.as_u8(), 70);
    assert_eq!(stderr, b"qsr-service-plan: error=output_write_failed\n");
}

#[test]
fn real_binary_emits_plan_and_fixed_failures() {
    let fixture = Fixture::new("binary");
    let (request_path, policy_path) = fixture.valid_inputs();
    let binary = env!("CARGO_BIN_EXE_qsr-service-plan");

    let success = Command::new(binary)
        .args(flags(&request_path, &policy_path, "5"))
        .output()
        .expect("qsr-service-plan must launch");
    assert_eq!(success.status.code(), Some(0));
    assert!(success.stderr.is_empty());
    let document: Value = serde_json::from_slice(&success.stdout).expect("plan must be JSON");
    assert_eq!(document["execution"], "not_performed");

    let usage = Command::new(binary)
        .arg("--help")
        .output()
        .expect("qsr-service-plan must launch");
    assert_eq!(usage.status.code(), Some(64));
    assert!(usage.stdout.is_empty());
    assert_eq!(usage.stderr, b"qsr-service-plan: error=usage\n");

    let missing = Command::new(binary)
        .args(flags(&fixture.path("absent.json"), &policy_path, "5"))
        .output()
        .expect("qsr-service-plan must launch");
    assert_eq!(missing.status.code(), Some(66));
    assert!(missing.stdout.is_empty());
    assert_eq!(
        missing.stderr,
        b"qsr-service-plan: error=request_unavailable\n"
    );
}
