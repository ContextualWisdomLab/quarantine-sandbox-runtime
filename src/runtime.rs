//! Static-analyzer orchestration and deterministic evidence assembly.

use std::collections::{BTreeMap, BTreeSet};

use serde_json::Error as SerializationError;
use sha2::{Digest, Sha256};
use thiserror::Error;

use crate::{
    AnalysisProfile, AnalysisRequest, ContractError, EvidenceBundle, EvidenceKind, EvidenceRecord,
    IngestedArtifact, IngestionError, IngestionPolicy, RuntimeDisposition, RuntimeManifest,
    ingest_bytes,
};

const MAX_ENGINE_IDENTIFIER_BYTES: usize = 128;

/// Analyzer-neutral finding before deterministic evidence identifiers are assigned.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AnalyzerFinding {
    /// Evidence category produced by the analyzer.
    pub evidence_kind: EvidenceKind,
    /// Bounded human-readable explanation.
    pub summary: String,
    /// Deterministically ordered machine-readable attributes.
    pub attributes: BTreeMap<String, String>,
}

/// Attributable static-analyzer failure.
#[derive(Clone, Debug, Error, PartialEq, Eq)]
#[error("analyzer {analyzer_id} failed with code {failure_code}")]
pub struct AnalyzerFailure {
    analyzer_id: String,
    failure_code: String,
}

impl AnalyzerFailure {
    /// Create a failure with a stable analyzer identifier and machine code.
    #[must_use]
    pub fn new(analyzer_id: impl Into<String>, failure_code: impl Into<String>) -> Self {
        Self {
            analyzer_id: analyzer_id.into(),
            failure_code: failure_code.into(),
        }
    }

    fn analyzer_id(&self) -> &str {
        &self.analyzer_id
    }

    fn failure_code(&self) -> &str {
        &self.failure_code
    }
}

/// Pluggable, non-executing static analyzer.
pub trait StaticAnalyzer: Send + Sync {
    /// Return a stable producer identifier.
    fn analyzer_id(&self) -> &'static str;

    /// Inspect an immutable artifact and return typed findings.
    ///
    /// # Errors
    ///
    /// Returns [`AnalyzerFailure`] when analysis cannot complete. The engine
    /// preserves the failure as evidence and marks the bundle inconclusive.
    fn analyze(&self, artifact: &IngestedArtifact)
    -> Result<Vec<AnalyzerFinding>, AnalyzerFailure>;
}

/// Foundation analyzer that records the ingestion format classification.
#[derive(Clone, Copy, Debug, Default)]
pub struct FormatAnalyzer;

impl StaticAnalyzer for FormatAnalyzer {
    fn analyzer_id(&self) -> &'static str {
        "format_analyzer"
    }

    fn analyze(
        &self,
        artifact: &IngestedArtifact,
    ) -> Result<Vec<AnalyzerFinding>, AnalyzerFailure> {
        Ok(vec![AnalyzerFinding {
            evidence_kind: EvidenceKind::FileFormat,
            summary: format!(
                "Detected artifact format: {}.",
                artifact.descriptor().artifact_kind.as_str()
            ),
            attributes: BTreeMap::from([(
                "artifact_kind".to_owned(),
                artifact.descriptor().artifact_kind.as_str().to_owned(),
            )]),
        }])
    }
}

/// Runtime orchestration or input failure.
#[derive(Clone, Debug, Error, PartialEq, Eq)]
pub enum AnalysisError {
    /// Request or generated evidence violated the public contract.
    #[error(transparent)]
    Contract(#[from] ContractError),
    /// Artifact ingestion failed before analyzer invocation.
    #[error(transparent)]
    Ingestion(#[from] IngestionError),
    /// An engine identity field is empty, oversized, or contains control text.
    #[error("invalid engine configuration: {field_name}")]
    InvalidEngineConfiguration {
        /// Invalid field.
        field_name: &'static str,
    },
    /// An analyzer identifier violates the engine identifier contract.
    #[error("invalid analyzer identifier: {analyzer_id}")]
    InvalidAnalyzerIdentifier {
        /// Invalid analyzer identifier.
        analyzer_id: String,
    },
    /// Two analyzers claim the same producer identifier.
    #[error("duplicate analyzer identifier: {analyzer_id}")]
    DuplicateAnalyzerIdentifier {
        /// Repeated analyzer identifier.
        analyzer_id: String,
    },
    /// At least one analyzer is required.
    #[error("no analyzers configured")]
    NoAnalyzersConfigured,
}

/// Ordered static-analysis engine.
pub struct AnalysisEngine {
    ingestion_policy: IngestionPolicy,
    policy_id: String,
    source_revision: String,
    analyzers: Vec<Box<dyn StaticAnalyzer>>,
}

impl AnalysisEngine {
    /// Construct an engine with an explicit policy, revision, and analyzer order.
    ///
    /// # Errors
    ///
    /// Returns [`AnalysisError`] for invalid policy, identifiers, or analyzer
    /// configuration.
    pub fn new(
        ingestion_policy: IngestionPolicy,
        policy_id: impl Into<String>,
        source_revision: impl Into<String>,
        analyzers: Vec<Box<dyn StaticAnalyzer>>,
    ) -> Result<Self, AnalysisError> {
        ingestion_policy.validate()?;
        let policy_id = policy_id.into();
        let source_revision = source_revision.into();

        if !is_valid_engine_identifier(&policy_id) {
            return Err(AnalysisError::InvalidEngineConfiguration {
                field_name: "policy_id",
            });
        }
        if !is_valid_engine_identifier(&source_revision) {
            return Err(AnalysisError::InvalidEngineConfiguration {
                field_name: "source_revision",
            });
        }
        if analyzers.is_empty() {
            return Err(AnalysisError::NoAnalyzersConfigured);
        }

        let mut analyzer_ids = BTreeSet::new();
        for analyzer in &analyzers {
            let analyzer_id = analyzer.analyzer_id();
            if !is_valid_engine_identifier(analyzer_id) {
                return Err(AnalysisError::InvalidAnalyzerIdentifier {
                    analyzer_id: analyzer_id.to_owned(),
                });
            }
            if !analyzer_ids.insert(analyzer_id) {
                return Err(AnalysisError::DuplicateAnalyzerIdentifier {
                    analyzer_id: analyzer_id.to_owned(),
                });
            }
        }

        Ok(Self {
            ingestion_policy,
            policy_id,
            source_revision,
            analyzers,
        })
    }

    /// Analyze bounded bytes and assemble an attributable evidence bundle.
    ///
    /// # Errors
    ///
    /// Returns [`AnalysisError`] when request validation, ingestion, or
    /// generated-contract validation fails.
    pub fn analyze_bytes(
        &self,
        request: &AnalysisRequest,
        artifact_name: &str,
        bytes: &[u8],
    ) -> Result<EvidenceBundle, AnalysisError> {
        request.validate()?;
        let artifact = ingest_bytes(artifact_name, bytes, &self.ingestion_policy)?;
        let analysis_job_id = self.deterministic_job_id(request, artifact.descriptor());
        let mut records = Vec::new();

        push_record(
            &mut records,
            &analysis_job_id,
            EvidenceKind::ArtifactIdentity,
            "runtime_core",
            "Artifact identity established.",
            BTreeMap::from([
                (
                    "artifact_kind".to_owned(),
                    artifact.descriptor().artifact_kind.as_str().to_owned(),
                ),
                (
                    "artifact_name".to_owned(),
                    artifact.descriptor().artifact_name.clone(),
                ),
                (
                    "artifact_sha256".to_owned(),
                    artifact.descriptor().artifact_sha256.clone(),
                ),
                (
                    "artifact_size_bytes".to_owned(),
                    artifact.descriptor().artifact_size_bytes.to_string(),
                ),
            ]),
        );

        let mut analyzer_failed = false;
        for analyzer in &self.analyzers {
            match analyzer.analyze(&artifact) {
                Ok(findings) => {
                    for finding in findings {
                        if finding.evidence_kind != EvidenceKind::FileFormat
                            && finding.evidence_kind != EvidenceKind::StaticCapability
                        {
                            analyzer_failed = true;
                            push_record(
                                &mut records,
                                &analysis_job_id,
                                EvidenceKind::ToolFailure,
                                analyzer.analyzer_id(),
                                "Static analyzer returned a disallowed evidence kind.",
                                BTreeMap::from([
                                    (
                                        "failure_code".to_owned(),
                                        "disallowed_evidence_kind".to_owned(),
                                    ),
                                    (
                                        "reported_evidence_kind".to_owned(),
                                        finding.evidence_kind.as_str().to_owned(),
                                    ),
                                ]),
                            );
                            continue;
                        }

                        push_record(
                            &mut records,
                            &analysis_job_id,
                            finding.evidence_kind,
                            analyzer.analyzer_id(),
                            &finding.summary,
                            finding.attributes,
                        );
                    }
                }
                Err(failure) => {
                    analyzer_failed = true;
                    push_record(
                        &mut records,
                        &analysis_job_id,
                        EvidenceKind::ToolFailure,
                        failure.analyzer_id(),
                        "Static analyzer did not complete.",
                        BTreeMap::from([(
                            "failure_code".to_owned(),
                            failure.failure_code().to_owned(),
                        )]),
                    );
                }
            }
        }

        push_record(
            &mut records,
            &analysis_job_id,
            EvidenceKind::PolicyBoundary,
            "runtime_core",
            "Foundation runtime performed no execution, network access, or credential use.",
            BTreeMap::from([
                ("credentials_available".to_owned(), "false".to_owned()),
                ("dynamic_execution_performed".to_owned(), "false".to_owned()),
                ("network_access_performed".to_owned(), "false".to_owned()),
                ("policy_id".to_owned(), self.policy_id.clone()),
            ]),
        );

        let dynamic_unavailable = request.profile != AnalysisProfile::StaticOnly;
        let disposition = if analyzer_failed || dynamic_unavailable {
            RuntimeDisposition::Inconclusive
        } else {
            RuntimeDisposition::Completed
        };

        let mut limitations = vec!["runtime_does_not_determine_maliciousness".to_owned()];
        if dynamic_unavailable {
            limitations.push("dynamic_analysis_not_configured".to_owned());
        }
        if analyzer_failed {
            limitations.push("static_analyzer_failure".to_owned());
        }

        let bundle = EvidenceBundle {
            schema_version: crate::CONTRACT_SCHEMA_VERSION.to_owned(),
            analysis_job_id,
            request_id: request.request_id.clone(),
            artifact: artifact.descriptor().clone(),
            runtime: RuntimeManifest {
                runtime_name: env!("CARGO_PKG_NAME").to_owned(),
                runtime_version: env!("CARGO_PKG_VERSION").to_owned(),
                source_revision: self.source_revision.clone(),
                requested_profile: request.profile,
                dynamic_execution_performed: false,
                network_access_performed: false,
                credentials_available: false,
            },
            disposition,
            consumer_verdict_required: true,
            evidence: records,
            limitations,
        };
        bundle.validate()?;
        Ok(bundle)
    }

    fn deterministic_job_id(
        &self,
        request: &AnalysisRequest,
        descriptor: &crate::ArtifactDescriptor,
    ) -> String {
        let mut hasher = Sha256::new();
        for component in [
            request.request_id.as_str(),
            request.profile.as_str(),
            descriptor.artifact_sha256.as_str(),
            self.policy_id.as_str(),
            self.source_revision.as_str(),
        ] {
            hasher.update(component.as_bytes());
            hasher.update([0]);
        }
        for analyzer in &self.analyzers {
            hasher.update(analyzer.analyzer_id().as_bytes());
            hasher.update([0]);
        }
        let digest = format!("{:x}", hasher.finalize());
        format!("analysis_job_{}", &digest[..32])
    }
}

impl Default for AnalysisEngine {
    fn default() -> Self {
        Self {
            ingestion_policy: IngestionPolicy::default(),
            policy_id: "foundation_policy_v1".to_owned(),
            source_revision: "development".to_owned(),
            analyzers: vec![Box::new(FormatAnalyzer)],
        }
    }
}

/// Serialize a bundle as stable human-readable JSON.
///
/// # Errors
///
/// Returns [`SerializationError`] if serialization fails.
pub fn to_pretty_json(bundle: &EvidenceBundle) -> Result<String, SerializationError> {
    serde_json::to_string_pretty(bundle)
}

fn is_valid_engine_identifier(value: &str) -> bool {
    if value.is_empty() {
        return false;
    }
    if value.len() > MAX_ENGINE_IDENTIFIER_BYTES {
        return false;
    }
    !value.chars().any(char::is_control)
}

fn push_record(
    records: &mut Vec<EvidenceRecord>,
    analysis_job_id: &str,
    evidence_kind: EvidenceKind,
    producer_id: &str,
    summary: &str,
    attributes: BTreeMap<String, String>,
) {
    let sequence_number = records.len() + 1;
    records.push(EvidenceRecord {
        evidence_id: format!("{analysis_job_id}:evidence:{sequence_number:04}"),
        sequence_number,
        evidence_kind,
        producer_id: producer_id.to_owned(),
        summary: summary.to_owned(),
        attributes,
    });
}
