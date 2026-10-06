//! Offline launch-plan inbound adapter for the application-service context (ADR-0011).
//!
//! The adapter reads one operator policy and one consumer request from local
//! regular files, validates them through the existing domain contract with the
//! runtime's UTF-8 byte bounds, and prints the deterministic rootless-Podman
//! plan. It never starts a backend process, so its output is review material,
//! not isolation evidence. Diagnostics are fixed codes that never contain
//! request values, policy values, or host paths.

use std::{
    ffi::OsString,
    fs::File,
    io::{Read, Write},
    path::{Path, PathBuf},
};

use serde::de::DeserializeOwned;
use serde_json::{Map, Value};

use crate::{
    ApplicationServiceError, ApplicationServiceRequest, IsolationPolicy, RootlessPodmanAdapter,
};

const INPUT_LIMIT_BYTES: u64 = 64 * 1024;
const PLAN_SCHEMA_VERSION: &str = "1.0.0";
const PLAN_KIND: &str = "application_service_launch_plan";
const EXECUTION_NOT_PERFORMED: &str = "not_performed";
const ISOLATION_EVIDENCE_NOT_ESTABLISHED: &str = "not_established";

/// Process outcome of the offline application-service launch-plan transport.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ServicePlanExit {
    /// A validated launch plan was written to stdout.
    Success,
    /// Arguments were missing, duplicated, unknown, or malformed.
    Usage,
    /// A request or policy was malformed or failed domain validation.
    InvalidInput,
    /// A request or policy was missing, not a regular file, or unreadable.
    InputUnavailable,
    /// A request or policy exceeded the 64 KiB input bound.
    InputTooLarge,
    /// The plan could not be written to stdout.
    Internal,
}

impl ServicePlanExit {
    /// Return the stable process exit status.
    #[must_use]
    pub const fn as_u8(self) -> u8 {
        match self {
            Self::Success => 0,
            Self::Usage => 64,
            Self::InvalidInput => 65,
            Self::InputUnavailable => 66,
            Self::InputTooLarge => 67,
            Self::Internal => 70,
        }
    }
}

#[derive(Debug, PartialEq, Eq)]
struct Failure {
    exit: ServicePlanExit,
    code: &'static str,
}

impl Failure {
    const fn new(exit: ServicePlanExit, code: &'static str) -> Self {
        Self { exit, code }
    }
}

const USAGE: Failure = Failure::new(ServicePlanExit::Usage, "usage");

#[derive(Clone, Copy)]
struct InputRole {
    unavailable: &'static str,
    too_large: &'static str,
    malformed: &'static str,
}

const REQUEST_ROLE: InputRole = InputRole {
    unavailable: "request_unavailable",
    too_large: "request_too_large",
    malformed: "request_malformed",
};

const POLICY_ROLE: InputRole = InputRole {
    unavailable: "policy_unavailable",
    too_large: "policy_too_large",
    malformed: "policy_malformed",
};

struct PlanArguments {
    request_path: PathBuf,
    policy_path: PathBuf,
    started_at_epoch_seconds: u64,
}

/// Run the offline application-service launch-plan transport.
///
/// `arguments` excludes the program name. On success one JSON plan document
/// and a newline are written to `stdout` and `stderr` is untouched. On failure
/// `stdout` is untouched and exactly one line
/// `qsr-service-plan: error=<code>` is written to `stderr`.
pub fn run_application_service_plan_cli(
    arguments: impl IntoIterator<Item = OsString>,
    stdout: &mut impl Write,
    stderr: &mut impl Write,
) -> ServicePlanExit {
    match plan(arguments, stdout) {
        Ok(()) => ServicePlanExit::Success,
        Err(failure) => {
            let _ = writeln!(stderr, "qsr-service-plan: error={}", failure.code);
            failure.exit
        }
    }
}

fn plan(
    arguments: impl IntoIterator<Item = OsString>,
    stdout: &mut impl Write,
) -> Result<(), Failure> {
    let arguments = parse_arguments(arguments).ok_or(USAGE)?;
    let policy: IsolationPolicy = read_document(&arguments.policy_path, POLICY_ROLE)?;
    policy
        .validate()
        .map_err(|error| invalid_input(&ApplicationServiceError::from(error)))?;
    let request: ApplicationServiceRequest = read_document(&arguments.request_path, REQUEST_ROLE)?;
    let plan =
        RootlessPodmanAdapter::plan_at(&request, &policy, arguments.started_at_epoch_seconds)
            .map_err(|error| invalid_input(&error))?;

    let mut document = Map::new();
    document.insert("schema_version".to_owned(), PLAN_SCHEMA_VERSION.into());
    document.insert("kind".to_owned(), PLAN_KIND.into());
    document.insert("execution".to_owned(), EXECUTION_NOT_PERFORMED.into());
    document.insert(
        "isolation_evidence".to_owned(),
        ISOLATION_EVIDENCE_NOT_ESTABLISHED.into(),
    );
    document.insert("sandbox_name".to_owned(), plan.sandbox_name().into());
    document.insert("network_name".to_owned(), plan.network_name().into());
    document.insert(
        "expires_at_epoch_seconds".to_owned(),
        plan.expires_at_epoch_seconds().into(),
    );
    document.insert(
        "rootless_probe_args".to_owned(),
        string_array(plan.rootless_probe_args()),
    );
    document.insert(
        "network_create_args".to_owned(),
        string_array(plan.network_create_args()),
    );
    document.insert(
        "container_create_args".to_owned(),
        string_array(plan.container_create_args()),
    );

    let mut output = Value::Object(document).to_string().into_bytes();
    output.push(b'\n');
    stdout
        .write_all(&output)
        .and_then(|()| stdout.flush())
        .map_err(|_| Failure::new(ServicePlanExit::Internal, "output_write_failed"))
}

fn string_array(values: &[String]) -> Value {
    Value::Array(values.iter().cloned().map(Value::String).collect())
}

const fn invalid_input(error: &ApplicationServiceError) -> Failure {
    Failure::new(ServicePlanExit::InvalidInput, error.code())
}

fn parse_arguments(arguments: impl IntoIterator<Item = OsString>) -> Option<PlanArguments> {
    let mut request_path = None;
    let mut policy_path = None;
    let mut started_at = None;
    let mut arguments = arguments.into_iter();
    while let Some(flag) = arguments.next() {
        let slot = match flag.to_str() {
            Some("--request") => &mut request_path,
            Some("--policy") => &mut policy_path,
            Some("--started-at") => &mut started_at,
            _ => return None,
        };
        if slot.is_some() {
            return None;
        }
        *slot = Some(arguments.next()?);
    }
    Some(PlanArguments {
        request_path: PathBuf::from(request_path?),
        policy_path: PathBuf::from(policy_path?),
        started_at_epoch_seconds: parse_epoch_seconds(&started_at?)?,
    })
}

fn parse_epoch_seconds(value: &OsString) -> Option<u64> {
    let text = value.to_str()?;
    if text.is_empty() || !text.bytes().all(|byte| byte.is_ascii_digit()) {
        return None;
    }
    text.parse().ok()
}

fn read_document<T: DeserializeOwned>(path: &Path, role: InputRole) -> Result<T, Failure> {
    let mut file = open_regular_file(path).ok_or(Failure::new(
        ServicePlanExit::InputUnavailable,
        role.unavailable,
    ))?;
    let bytes = read_bounded(&mut file, role)?;
    serde_json::from_slice(&bytes)
        .map_err(|_| Failure::new(ServicePlanExit::InvalidInput, role.malformed))
}

#[cfg(unix)]
fn open_regular_file(path: &Path) -> Option<File> {
    use std::{fs::OpenOptions, os::unix::fs::OpenOptionsExt};

    let file = OpenOptions::new()
        .read(true)
        .custom_flags(libc::O_NOFOLLOW | libc::O_NONBLOCK | libc::O_NOCTTY)
        .open(path)
        .ok()?;
    file.metadata()
        .is_ok_and(|metadata| metadata.is_file())
        .then_some(file)
}

#[cfg(not(unix))]
fn open_regular_file(_path: &Path) -> Option<File> {
    None
}

fn read_bounded(reader: &mut dyn Read, role: InputRole) -> Result<Vec<u8>, Failure> {
    let mut bytes = Vec::new();
    match reader.take(INPUT_LIMIT_BYTES + 1).read_to_end(&mut bytes) {
        Ok(_) if bytes.len() as u64 <= INPUT_LIMIT_BYTES => Ok(bytes),
        Ok(_) => Err(Failure::new(ServicePlanExit::InputTooLarge, role.too_large)),
        Err(_) => Err(Failure::new(
            ServicePlanExit::InputUnavailable,
            role.unavailable,
        )),
    }
}

#[cfg(test)]
mod tests {
    use std::io;

    use super::{Failure, REQUEST_ROLE, ServicePlanExit, read_bounded};

    struct FailingReader;

    impl io::Read for FailingReader {
        fn read(&mut self, _buffer: &mut [u8]) -> io::Result<usize> {
            Err(io::Error::other("unreadable"))
        }
    }

    #[test]
    fn read_errors_are_reported_as_unavailable_input() {
        assert_eq!(
            read_bounded(&mut FailingReader, REQUEST_ROLE),
            Err(Failure::new(
                ServicePlanExit::InputUnavailable,
                "request_unavailable"
            ))
        );
    }
}
