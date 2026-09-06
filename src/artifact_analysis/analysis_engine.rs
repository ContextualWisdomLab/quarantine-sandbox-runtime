//! Public artifact-analysis engine boundary that emits semantically verified receipts.

use super::{
    contracts::AnalysisRequest,
    evidence_bundle::EvidenceBundle,
    ingestion::IngestionPolicy,
    runtime::{AnalysisEngine as RuntimeAnalysisEngine, AnalysisError, StaticAnalyzer},
};

/// Artifact-analysis engine that preserves runtime isolation while publishing validated receipts.
pub struct AnalysisEngine {
    inner: RuntimeAnalysisEngine,
}

impl AnalysisEngine {
    /// Construct an engine with an explicit policy, revision, and analyzer order.
    ///
    /// Externally supplied analyzers remain fail closed until the runtime-owned
    /// isolated-worker execution port is available.
    ///
    /// # Errors
    ///
    /// Returns [`AnalysisError`] for invalid policy, identifiers, or analyzer configuration.
    pub fn new(
        ingestion_policy: IngestionPolicy,
        policy_id: &str,
        source_revision: &str,
        analyzers: Vec<Box<dyn StaticAnalyzer>>,
    ) -> Result<Self, AnalysisError> {
        RuntimeAnalysisEngine::new(ingestion_policy, policy_id, source_revision, analyzers)
            .map(|inner| Self { inner })
    }

    /// Construct the runtime-owned foundation engine with bundled non-executing analyzers.
    ///
    /// # Errors
    ///
    /// Returns [`AnalysisError`] when the ingestion policy or engine identifiers are invalid.
    pub fn with_bundled_static_analyzers(
        ingestion_policy: IngestionPolicy,
        policy_id: &str,
        source_revision: &str,
    ) -> Result<Self, AnalysisError> {
        RuntimeAnalysisEngine::with_bundled_static_analyzers(
            ingestion_policy,
            policy_id,
            source_revision,
        )
        .map(|inner| Self { inner })
    }

    /// Analyze bounded bytes and return a receipt whose job identity is self-consistent.
    ///
    /// # Errors
    ///
    /// Returns [`AnalysisError`] for request, ingestion, isolation, analyzer, or
    /// receipt-integrity failures.
    pub fn analyze_bytes(
        &self,
        request: &AnalysisRequest,
        bytes: &[u8],
    ) -> Result<EvidenceBundle, AnalysisError> {
        let wire = self.inner.analyze_bytes(request, bytes)?;
        EvidenceBundle::from_generated(wire).map_err(AnalysisError::from)
    }
}

impl Default for AnalysisEngine {
    fn default() -> Self {
        Self {
            inner: RuntimeAnalysisEngine::default(),
        }
    }
}
