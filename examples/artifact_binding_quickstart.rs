//! Synthetic public-API quickstart: byte binding is not execution or authority.

use std::io::Write;

#[cfg(not(test))]
use std::io;

use quarantine_sandbox_runtime::{
    AnalysisEngine, AnalysisProfile, AnalysisRequest, ArtifactBindingError, ArtifactBindingReport,
    ContractError, EvidenceBundle, verify_artifact_binding,
};
use serde::Serialize;

/// Harmless synthetic bytes; these bytes are inspected, never executed.
pub const SYNTHETIC_BYTES: &[u8] = b"MZ\x90\x00synthetic quickstart fixture";

#[derive(Serialize)]
struct QuickstartOutput<'a> {
    synthetic_fixture: bool,
    evidence_scope: &'static str,
    release_ready: bool,
    operator_authenticated: bool,
    bundle: &'a EvidenceBundle,
    matched_report: ArtifactBindingReport,
    modified_bytes_report: ArtifactBindingReport,
    empty_input_rejected: bool,
    invalid_bundle_rejected: bool,
}

/// Exercise the public API and emit a complete, explicitly synthetic envelope.
///
/// # Errors
///
/// Returns analysis, binding, serialization, or output-write errors.
pub fn run(output: &mut impl Write) -> Result<(), Box<dyn std::error::Error>> {
    let request = AnalysisRequest {
        schema_version: "1.0.0".to_owned(),
        request_id: "synthetic_binding_quickstart".to_owned(),
        profile: AnalysisProfile::LinuxDynamic,
        bounded_source_context: None,
    };
    let bundle = AnalysisEngine::default().analyze_bytes(&request, SYNTHETIC_BYTES)?;
    let matched_report = verify_artifact_binding(&bundle, SYNTHETIC_BYTES)?;
    let modified_bytes_report =
        verify_artifact_binding(&bundle, b"MZ\x90\x00different synthetic fixture")?;
    let empty_input_rejected = matches!(
        verify_artifact_binding(&bundle, b""),
        Err(ArtifactBindingError::EmptyArtifact)
    );
    let mut invalid_bundle = bundle.clone();
    invalid_bundle.consumer_verdict_required = false;
    let invalid_bundle_rejected = matches!(
        verify_artifact_binding(&invalid_bundle, SYNTHETIC_BYTES),
        Err(ArtifactBindingError::Contract(
            ContractError::ConsumerVerdictMustBeRequired
        ))
    );
    let result = QuickstartOutput {
        synthetic_fixture: true,
        evidence_scope: "offline_artifact_consistency_only",
        release_ready: false,
        operator_authenticated: false,
        bundle: &bundle,
        matched_report,
        modified_bytes_report,
        empty_input_rejected,
        invalid_bundle_rejected,
    };
    let mut json = serde_json::to_vec_pretty(&result)?;
    json.push(b'\n');
    output.write_all(&json)?;
    output.flush()?;
    Ok(())
}

#[cfg(not(test))]
fn main() -> Result<(), Box<dyn std::error::Error>> {
    run(&mut io::stdout().lock())
}
