//! Public artifact-analysis receipt boundary with cross-field identity verification.

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use super::contracts::{
    ArtifactDescriptor, ContractError, EvidenceBundle as WireEvidenceBundle, EvidenceKind,
    EvidenceRecord, RuntimeDisposition, RuntimeManifest,
};

const ANALYSIS_JOB_PREFIX: &str = "analysis_job_";
const RECEIPT_IDENTITY_DIGEST_HEX_BYTES: usize = 64;
const ANALYZER_IDENTITY_SUFFIX_HEX_BYTES: usize = 32;
const ANALYSIS_JOB_IDENTITY_BOUNDARY: &str = "analysis_job_identity_binding";

/// Validated artifact-analysis receipt exposed to consumers.
///
/// The public shape remains the versioned `1.0.0` wire contract. Semantic
/// validation additionally binds the job identifier to receipt-visible request,
/// profile, artifact, policy, and runtime-source identity. The trailing
/// analyzer-sensitive digest remains correlation data until stable analyzer
/// provenance is supplied by the analyzer-provenance contract.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct EvidenceBundle {
    /// Contract version, currently `1.0.0`.
    pub schema_version: String,
    /// Deterministic analysis-job identifier with a receipt-visible identity digest.
    pub analysis_job_id: String,
    /// Original request identifier.
    pub request_id: String,
    /// Immutable artifact descriptor.
    pub artifact: ArtifactDescriptor,
    /// Runtime identity and boundary facts.
    pub runtime: RuntimeManifest,
    /// Execution completeness, not maliciousness.
    pub disposition: RuntimeDisposition,
    /// Always true because the consumer owns the verdict.
    pub consumer_verdict_required: bool,
    /// Ordered evidence records.
    pub evidence: Vec<EvidenceRecord>,
    /// Stable machine-readable limitations.
    pub limitations: Vec<String>,
}

impl EvidenceBundle {
    pub(super) fn from_generated(mut wire: WireEvidenceBundle) -> Result<Self, ContractError> {
        wire.validate()?;
        let analyzer_identity_suffix = wire
            .analysis_job_id
            .strip_prefix(ANALYSIS_JOB_PREFIX)
            .filter(|value| {
                value.len() == ANALYZER_IDENTITY_SUFFIX_HEX_BYTES && is_lowercase_hex(value)
            })
            .ok_or(identity_binding_error())?
            .to_owned();
        let receipt_identity_digest = receipt_identity_digest_from_wire(&wire)?;

        wire.analysis_job_id = format!(
            "{ANALYSIS_JOB_PREFIX}{receipt_identity_digest}_{analyzer_identity_suffix}"
        );
        for record in &mut wire.evidence {
            record.evidence_id = format!(
                "{}:evidence:{:04}",
                wire.analysis_job_id, record.sequence_number
            );
        }

        let bundle = Self::from_wire(wire);
        bundle.validate()?;
        Ok(bundle)
    }

    /// Validate wire structure and bind the job identifier to receipt-visible identity inputs.
    ///
    /// # Errors
    ///
    /// Returns [`ContractError`] when the underlying wire contract is invalid,
    /// the job identifier is malformed, the runtime policy identity is absent or ambiguous,
    /// or the embedded receipt-identity digest contradicts the receipt fields.
    pub fn validate(&self) -> Result<(), ContractError> {
        self.to_wire().validate()?;

        let identifier = self
            .analysis_job_id
            .strip_prefix(ANALYSIS_JOB_PREFIX)
            .ok_or(identity_binding_error())?;
        let (receipt_digest, analyzer_suffix) = identifier
            .split_once('_')
            .ok_or(identity_binding_error())?;
        if receipt_digest.len() != RECEIPT_IDENTITY_DIGEST_HEX_BYTES
            || !is_lowercase_hex(receipt_digest)
            || analyzer_suffix.len() != ANALYZER_IDENTITY_SUFFIX_HEX_BYTES
            || !is_lowercase_hex(analyzer_suffix)
        {
            return Err(identity_binding_error());
        }

        if receipt_digest != receipt_identity_digest(self)? {
            return Err(identity_binding_error());
        }

        Ok(())
    }

    fn from_wire(wire: WireEvidenceBundle) -> Self {
        Self {
            schema_version: wire.schema_version,
            analysis_job_id: wire.analysis_job_id,
            request_id: wire.request_id,
            artifact: wire.artifact,
            runtime: wire.runtime,
            disposition: wire.disposition,
            consumer_verdict_required: wire.consumer_verdict_required,
            evidence: wire.evidence,
            limitations: wire.limitations,
        }
    }

    fn to_wire(&self) -> WireEvidenceBundle {
        WireEvidenceBundle {
            schema_version: self.schema_version.clone(),
            analysis_job_id: self.analysis_job_id.clone(),
            request_id: self.request_id.clone(),
            artifact: self.artifact.clone(),
            runtime: self.runtime.clone(),
            disposition: self.disposition,
            consumer_verdict_required: self.consumer_verdict_required,
            evidence: self.evidence.clone(),
            limitations: self.limitations.clone(),
        }
    }
}

/// Serialize a validated bundle as stable human-readable JSON.
///
/// # Errors
///
/// Returns [`serde_json::Error`] if serialization fails.
pub fn to_pretty_json(bundle: &EvidenceBundle) -> Result<String, serde_json::Error> {
    super::runtime::to_pretty_json(&bundle.to_wire())
}

fn receipt_identity_digest(bundle: &EvidenceBundle) -> Result<String, ContractError> {
    let policy_id = runtime_policy_id(&bundle.evidence)?;
    Ok(receipt_identity_digest_components(
        &bundle.request_id,
        bundle.runtime.requested_profile.as_str(),
        &bundle.artifact.artifact_sha256,
        policy_id,
        &bundle.runtime.source_revision,
    ))
}

fn receipt_identity_digest_from_wire(
    bundle: &WireEvidenceBundle,
) -> Result<String, ContractError> {
    let policy_id = runtime_policy_id(&bundle.evidence)?;
    Ok(receipt_identity_digest_components(
        &bundle.request_id,
        bundle.runtime.requested_profile.as_str(),
        &bundle.artifact.artifact_sha256,
        policy_id,
        &bundle.runtime.source_revision,
    ))
}

fn receipt_identity_digest_components(
    request_id: &str,
    requested_profile: &str,
    artifact_sha256: &str,
    policy_id: &str,
    source_revision: &str,
) -> String {
    let mut hasher = Sha256::new();
    for component in [
        request_id,
        requested_profile,
        artifact_sha256,
        policy_id,
        source_revision,
    ] {
        hasher.update(component.as_bytes());
        hasher.update([0]);
    }
    format!("{:x}", hasher.finalize())
}

fn runtime_policy_id(evidence: &[EvidenceRecord]) -> Result<&str, ContractError> {
    let mut policy_boundaries = evidence
        .iter()
        .filter(|record| record.evidence_kind == EvidenceKind::PolicyBoundary);
    let policy_boundary = policy_boundaries.next().ok_or(identity_binding_error())?;
    if policy_boundaries.next().is_some() {
        return Err(identity_binding_error());
    }

    policy_boundary
        .attributes
        .get("policy_id")
        .map(String::as_str)
        .ok_or(identity_binding_error())
}

fn is_lowercase_hex(value: &str) -> bool {
    value
        .bytes()
        .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
}

const fn identity_binding_error() -> ContractError {
    ContractError::RuntimeBoundaryViolated {
        boundary_name: ANALYSIS_JOB_IDENTITY_BOUNDARY,
    }
}
