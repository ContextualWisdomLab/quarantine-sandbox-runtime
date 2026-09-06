//! Public artifact-analysis receipt boundary with cross-field identity verification.

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use super::contracts::{
    ArtifactDescriptor, CONTRACT_SCHEMA_VERSION, ContractError,
    EvidenceBundle as WireEvidenceBundle, EvidenceKind, EvidenceRecord, RuntimeDisposition,
    RuntimeManifest,
};

/// Current public artifact-analysis evidence-bundle schema version.
pub const EVIDENCE_BUNDLE_SCHEMA_VERSION: &str = "1.1.0";

const ANALYSIS_JOB_IDENTITY_BOUNDARY: &str = "analysis_job_identity_binding";
const SHA256_HEX_BYTES: usize = 64;

/// Validated artifact-analysis receipt exposed to consumers.
///
/// Schema `1.1.0` preserves the opaque `analysis_job_id` semantics from `1.0.0`
/// and adds `analysis_job_identity_sha256` as required security evidence. The
/// digest binds the published job identifier to receipt-visible request,
/// profile, artifact, policy, and runtime-source identity without pretending to
/// authenticate the producer. Stable analyzer provenance remains a separate
/// contract.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EvidenceBundle {
    /// Evidence contract version, currently `1.1.0`.
    pub schema_version: String,
    /// Deterministic analysis-job correlation identifier.
    pub analysis_job_id: String,
    /// SHA-256 binding of the job identifier and receipt-visible identity inputs.
    pub analysis_job_identity_sha256: String,
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
    pub(super) fn from_generated(wire: WireEvidenceBundle) -> Result<Self, ContractError> {
        wire.validate()?;
        let analysis_job_identity_sha256 = receipt_identity_digest_from_wire(&wire)?;
        let bundle = Self::from_wire(wire, analysis_job_identity_sha256);
        bundle.validate()?;
        Ok(bundle)
    }

    /// Validate the versioned wire structure and bind job identity to receipt-visible inputs.
    ///
    /// # Errors
    ///
    /// Returns [`ContractError`] when the evidence schema version is unsupported,
    /// the inherited structural contract is invalid, the job-identity digest is
    /// malformed, the runtime policy identity is absent or ambiguous, or the
    /// digest contradicts the receipt fields.
    pub fn validate(&self) -> Result<(), ContractError> {
        if self.schema_version != EVIDENCE_BUNDLE_SCHEMA_VERSION {
            return Err(ContractError::UnsupportedSchemaVersion {
                actual_version: self.schema_version.clone(),
            });
        }
        if self.analysis_job_identity_sha256.len() != SHA256_HEX_BYTES
            || !is_lowercase_hex(&self.analysis_job_identity_sha256)
        {
            return Err(identity_binding_error());
        }

        self.to_wire().validate()?;
        if self.analysis_job_identity_sha256 != receipt_identity_digest(self)? {
            return Err(identity_binding_error());
        }

        Ok(())
    }

    fn from_wire(wire: WireEvidenceBundle, analysis_job_identity_sha256: String) -> Self {
        Self {
            schema_version: EVIDENCE_BUNDLE_SCHEMA_VERSION.to_owned(),
            analysis_job_id: wire.analysis_job_id,
            analysis_job_identity_sha256,
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
            schema_version: CONTRACT_SCHEMA_VERSION.to_owned(),
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

/// Serialize a validated evidence bundle as stable human-readable JSON.
///
/// # Errors
///
/// Returns [`serde_json::Error`] if serialization fails.
pub fn to_pretty_json(bundle: &EvidenceBundle) -> Result<String, serde_json::Error> {
    serde_json::to_string_pretty(bundle)
}

fn receipt_identity_digest(bundle: &EvidenceBundle) -> Result<String, ContractError> {
    let policy_id = runtime_policy_id(&bundle.evidence)?;
    Ok(receipt_identity_digest_components(
        &bundle.analysis_job_id,
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
        &bundle.analysis_job_id,
        &bundle.request_id,
        bundle.runtime.requested_profile.as_str(),
        &bundle.artifact.artifact_sha256,
        policy_id,
        &bundle.runtime.source_revision,
    ))
}

fn receipt_identity_digest_components(
    analysis_job_id: &str,
    request_id: &str,
    requested_profile: &str,
    artifact_sha256: &str,
    policy_id: &str,
    source_revision: &str,
) -> String {
    let mut hasher = Sha256::new();
    for component in [
        analysis_job_id,
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
    let mut policy_identity_sources = evidence.iter().filter(|record| {
        record.evidence_kind == EvidenceKind::PolicyBoundary
            && record.attributes.contains_key("policy_id")
    });
    let policy_boundary = policy_identity_sources
        .next()
        .ok_or(identity_binding_error())?;
    if policy_identity_sources.next().is_some() {
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
