//! Public artifact-analysis receipt boundary with cross-field identity verification.

use std::ops::{Deref, DerefMut};

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use super::contracts::{
    ContractError, EvidenceBundle as WireEvidenceBundle, EvidenceKind,
};

const ANALYSIS_JOB_PREFIX: &str = "analysis_job_";
const RECEIPT_IDENTITY_DIGEST_HEX_BYTES: usize = 64;
const ANALYZER_IDENTITY_SUFFIX_HEX_BYTES: usize = 32;
const ANALYSIS_JOB_IDENTITY_BOUNDARY: &str = "analysis_job_identity_binding";

/// Validated artifact-analysis receipt exposed to consumers.
///
/// The inner wire shape remains the versioned `1.0.0` contract. This boundary
/// adds semantic validation that JSON Schema cannot express: the public job
/// identifier contains a digest of receipt-visible request, profile, artifact,
/// policy, and runtime-source identity. The trailing analyzer-sensitive digest
/// remains correlation data until stable analyzer provenance is supplied by the
/// analyzer-provenance contract.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct EvidenceBundle(WireEvidenceBundle);

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
        let receipt_identity_digest = receipt_identity_digest(&wire)?;

        wire.analysis_job_id = format!(
            "{ANALYSIS_JOB_PREFIX}{receipt_identity_digest}_{analyzer_identity_suffix}"
        );
        for record in &mut wire.evidence {
            record.evidence_id = format!(
                "{}:evidence:{:04}",
                wire.analysis_job_id, record.sequence_number
            );
        }

        let bundle = Self(wire);
        bundle.validate()?;
        Ok(bundle)
    }

    /// Validate wire structure and bind the job identifier to receipt-visible identity inputs.
    ///
    /// # Errors
    ///
    /// Returns [`ContractError`] when the underlying wire contract is invalid,
    /// the job identifier is malformed, the runtime policy identity is absent,
    /// or the embedded receipt-identity digest contradicts the receipt fields.
    pub fn validate(&self) -> Result<(), ContractError> {
        self.0.validate()?;

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

        if receipt_digest != receipt_identity_digest(&self.0)? {
            return Err(identity_binding_error());
        }

        Ok(())
    }
}

impl Deref for EvidenceBundle {
    type Target = WireEvidenceBundle;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for EvidenceBundle {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

/// Serialize a validated bundle as stable human-readable JSON.
///
/// # Errors
///
/// Returns [`serde_json::Error`] if serialization fails.
pub fn to_pretty_json(bundle: &EvidenceBundle) -> Result<String, serde_json::Error> {
    super::runtime::to_pretty_json(&bundle.0)
}

fn receipt_identity_digest(bundle: &WireEvidenceBundle) -> Result<String, ContractError> {
    let policy_id = bundle
        .evidence
        .iter()
        .find(|record| record.evidence_kind == EvidenceKind::PolicyBoundary)
        .and_then(|record| record.attributes.get("policy_id"))
        .ok_or(identity_binding_error())?;

    let mut hasher = Sha256::new();
    for component in [
        bundle.request_id.as_str(),
        bundle.runtime.requested_profile.as_str(),
        bundle.artifact.artifact_sha256.as_str(),
        policy_id.as_str(),
        bundle.runtime.source_revision.as_str(),
    ] {
        hasher.update(component.as_bytes());
        hasher.update([0]);
    }
    Ok(format!("{:x}", hasher.finalize()))
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
