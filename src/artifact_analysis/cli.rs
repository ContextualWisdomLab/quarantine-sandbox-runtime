//! Offline static analysis command-line inbound adapter (ADR-0010).
//!
//! This adapter translates process arguments and local files into the
//! existing [`AnalysisEngine`] contract. It owns no domain vocabulary, never
//! executes artifact bytes, and never echoes request bodies, artifact bytes,
//! or host paths in diagnostics.

use std::{
    ffi::OsString,
    fs::File,
    io::{Read, Write},
    path::{Path, PathBuf},
};

use super::{AnalysisEngine, AnalysisError, AnalysisRequest, IngestionPolicy};

/// Maximum accepted analysis-request document size in bytes.
pub const MAXIMUM_REQUEST_BYTES: u64 = 64 * 1024;

/// Process outcome of the static analysis command-line transport.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CliExit {
    /// A validated evidence bundle was written to stdout.
    Success,
    /// Arguments were missing, duplicated, or unknown.
    Usage,
    /// The analysis request was unreadable, oversized, malformed, or invalid.
    InvalidRequest,
    /// The artifact was missing, not a regular file, or unreadable.
    ArtifactUnavailable,
    /// The artifact exceeded its byte bound or was rejected by ingestion.
    ArtifactRejected,
    /// The engine, output serialization, or output write failed.
    Internal,
}

impl CliExit {
    /// Return the stable process exit status.
    #[must_use]
    pub const fn as_u8(self) -> u8 {
        match self {
            Self::Success => 0,
            Self::Usage => 64,
            Self::InvalidRequest => 65,
            Self::ArtifactUnavailable => 66,
            Self::ArtifactRejected => 67,
            Self::Internal => 70,
        }
    }

    const fn code(self) -> &'static str {
        match self {
            Self::Success => "none",
            Self::Usage => "usage",
            Self::InvalidRequest => "invalid_request",
            Self::ArtifactUnavailable => "artifact_unavailable",
            Self::ArtifactRejected => "artifact_rejected",
            Self::Internal => "internal",
        }
    }
}

struct CliArguments {
    request_path: PathBuf,
    artifact_path: PathBuf,
}

/// Run the offline static analysis transport.
///
/// `arguments` excludes the program name. Evidence JSON is written to
/// `stdout`; one fixed diagnostic line is written to `stderr` on failure.
pub fn run_static_analysis_cli(
    arguments: impl IntoIterator<Item = OsString>,
    stdout: &mut impl Write,
    stderr: &mut impl Write,
) -> CliExit {
    let outcome = analyze(arguments.into_iter().collect(), stdout);
    if outcome != CliExit::Success {
        let _ = writeln!(stderr, "qsr-analyze: error={}", outcome.code());
    }
    outcome
}

fn analyze(arguments: Vec<OsString>, stdout: &mut dyn Write) -> CliExit {
    let Some(arguments) = parse_arguments(arguments) else {
        return CliExit::Usage;
    };
    let Ok(BoundedRead::Within(request_bytes)) =
        read_regular_file(&arguments.request_path, MAXIMUM_REQUEST_BYTES)
    else {
        return CliExit::InvalidRequest;
    };
    let Ok(request) = serde_json::from_slice::<AnalysisRequest>(&request_bytes) else {
        return CliExit::InvalidRequest;
    };
    if request.validate().is_err() {
        return CliExit::InvalidRequest;
    }
    let maximum_artifact_bytes =
        u64::try_from(IngestionPolicy::default().maximum_artifact_bytes).unwrap_or(u64::MAX);
    let artifact = match read_regular_file(&arguments.artifact_path, maximum_artifact_bytes) {
        Ok(BoundedRead::Within(bytes)) => bytes,
        Ok(BoundedRead::Exceeded) => return CliExit::ArtifactRejected,
        Err(()) => return CliExit::ArtifactUnavailable,
    };
    let bundle = match AnalysisEngine::default().analyze_bytes(&request, &artifact) {
        Ok(bundle) => bundle,
        Err(error) => return analysis_failure_exit(&error),
    };
    let written = serde_json::to_vec_pretty(&bundle).is_ok_and(|mut json| {
        json.push(b'\n');
        stdout
            .write_all(&json)
            .and_then(|()| stdout.flush())
            .is_ok()
    });
    if written {
        CliExit::Success
    } else {
        CliExit::Internal
    }
}

fn parse_arguments(arguments: Vec<OsString>) -> Option<CliArguments> {
    let mut request_path = None;
    let mut artifact_path = None;
    let mut arguments = arguments.into_iter();
    while let Some(flag) = arguments.next() {
        let slot = match flag.to_str() {
            Some("--request") => &mut request_path,
            Some("--artifact") => &mut artifact_path,
            _ => return None,
        };
        if slot.is_some() {
            return None;
        }
        *slot = Some(PathBuf::from(arguments.next()?));
    }
    Some(CliArguments {
        request_path: request_path?,
        artifact_path: artifact_path?,
    })
}

enum BoundedRead {
    Within(Vec<u8>),
    Exceeded,
}

/// Read one regular file without following a final symbolic link, without
/// blocking on FIFOs, without acquiring a controlling terminal, and without
/// reading more than `maximum_bytes + 1` bytes.
fn read_regular_file(path: &Path, maximum_bytes: u64) -> Result<BoundedRead, ()> {
    let file = open_no_follow(path).map_err(drop)?;
    let mut bytes = Vec::new();
    let readable = file.metadata().is_ok_and(|metadata| metadata.is_file())
        && (&file)
            .take(maximum_bytes.saturating_add(1))
            .read_to_end(&mut bytes)
            .is_ok();
    if !readable {
        return Err(());
    }
    if u64::try_from(bytes.len()).unwrap_or(u64::MAX) > maximum_bytes {
        Ok(BoundedRead::Exceeded)
    } else {
        Ok(BoundedRead::Within(bytes))
    }
}

#[cfg(unix)]
fn open_no_follow(path: &Path) -> std::io::Result<File> {
    use std::os::unix::fs::OpenOptionsExt;

    std::fs::OpenOptions::new()
        .read(true)
        .custom_flags(libc::O_NOFOLLOW | libc::O_NONBLOCK | libc::O_NOCTTY)
        .open(path)
}

#[cfg(not(unix))]
fn open_no_follow(_path: &Path) -> std::io::Result<File> {
    Err(std::io::Error::from(std::io::ErrorKind::Unsupported))
}

/// Map an engine failure after request validation to a process outcome.
///
/// Ingestion failures describe the submitted artifact. Every other failure is
/// an engine or evidence defect, because the request was already validated.
const fn analysis_failure_exit(error: &AnalysisError) -> CliExit {
    match error {
        AnalysisError::Ingestion(_) => CliExit::ArtifactRejected,
        AnalysisError::Contract(_)
        | AnalysisError::InvalidEngineConfiguration { .. }
        | AnalysisError::InvalidAnalyzerIdentifier { .. }
        | AnalysisError::DuplicateAnalyzerIdentifier { .. }
        | AnalysisError::NoAnalyzersConfigured => CliExit::Internal,
    }
}

#[cfg(test)]
mod tests {
    use super::{AnalysisError, CliExit, analysis_failure_exit};
    use crate::{ContractError, IngestionError};

    #[test]
    fn diagnostic_codes_are_stable() {
        assert_eq!(
            [
                CliExit::Success,
                CliExit::Usage,
                CliExit::InvalidRequest,
                CliExit::ArtifactUnavailable,
                CliExit::ArtifactRejected,
                CliExit::Internal,
            ]
            .map(CliExit::code),
            [
                "none",
                "usage",
                "invalid_request",
                "artifact_unavailable",
                "artifact_rejected",
                "internal",
            ]
        );
    }

    #[test]
    fn ingestion_failures_reject_the_artifact() {
        assert_eq!(
            analysis_failure_exit(&AnalysisError::Ingestion(IngestionError::EmptyArtifact)),
            CliExit::ArtifactRejected
        );
    }

    #[test]
    fn post_validation_engine_failures_are_internal() {
        assert_eq!(
            analysis_failure_exit(&AnalysisError::Contract(
                ContractError::EmptyBoundedSourceContext
            )),
            CliExit::Internal
        );
        for error in [
            AnalysisError::InvalidEngineConfiguration {
                field_name: "policy_version",
            },
            AnalysisError::InvalidAnalyzerIdentifier {
                analyzer_id: String::new(),
            },
            AnalysisError::DuplicateAnalyzerIdentifier {
                analyzer_id: "format".to_owned(),
            },
            AnalysisError::NoAnalyzersConfigured,
        ] {
            assert_eq!(analysis_failure_exit(&error), CliExit::Internal);
        }
    }
}
