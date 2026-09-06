//! Static-analyzer orchestration and deterministic evidence assembly.

use std::collections::{BTreeMap, BTreeSet};

use serde_json::Error as SerializationError;
use sha2::{Digest, Sha256};
use thiserror::Error;

use crate::{
    AnalysisProfile, AnalysisRequest, ContractError, EvidenceKind, EvidenceRecord,
    IngestedArtifact, IngestionError, IngestionPolicy, RuntimeDisposition, RuntimeManifest,
};

use super::{contracts::EvidenceBundle, ingestion::ingest_bytes_with_optional_name};

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
    pub fn new(analyzer_id: &str, failure_code: &str) -> Self {
        Self {
            analyzer_id: analyzer_id.to_owned(),
            failure_code: failure_code.to_owned(),
        }
    }

    fn analyzer_id(&self) -> &str {
        &self.analyzer_id
    }

    fn failure_code(&self) -> &str {
        &self.failure_code
    }
}

/// Pluggable analyzer contract retained for compatibility while isolated workers are introduced.
///
/// Implementations supplied through [`AnalysisEngine::new`] are not invoked in the runtime host
/// process. Production execution requires an isolated analyzer-worker path; until that port is
/// available, externally supplied analyzers fail closed before invocation.
pub trait StaticAnalyzer: Send + Sync {
    /// Return a stable producer identifier.
    fn analyzer_id(&self) -> &'static str;

    /// Inspect an immutable artifact and return typed findings.
    ///
    /// # Errors
    ///
    /// Returns [`AnalyzerFailure`] when analysis cannot complete. The engine preserves a worker
    /// failure as attributable evidence once isolated worker execution is available.
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

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum AnalyzerExecutionPath {
    BundledRuntime,
    IsolatedWorkerRequired,
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
    /// Externally supplied analyzer code requires an enforceable isolated worker before invocation.
    #[error("isolated analyzer worker required before invoking externally supplied analyzer code")]
    IsolatedAnalyzerWorkerRequired,
}

/// Ordered static-analysis engine.
pub struct AnalysisEngine {
    ingestion_policy: IngestionPolicy,
    policy_id: String,
    source_revision: String,
    analyzers: Vec<Box<dyn StaticAnalyzer>>,
    analyzer_execution_path: AnalyzerExecutionPath,
}

impl AnalysisEngine {
    /// Construct an engine with an explicit policy, revision, and analyzer order.
    ///
    /// This compatibility constructor validates externally supplied analyzers but does not execute
    /// them in the runtime host process. [`Self::analyze_bytes`] fails closed with
    /// [`AnalysisError::IsolatedAnalyzerWorkerRequired`] until an enforceable isolated-worker port
    /// owns their execution.
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
        Self::validate_configuration(&ingestion_policy, policy_id, source_revision, &analyzers)?;

        Ok(Self {
            ingestion_policy,
            policy_id: policy_id.to_owned(),
            source_revision: source_revision.to_owned(),
            analyzers,
            analyzer_execution_path: AnalyzerExecutionPath::IsolatedWorkerRequired,
        })
    }

    /// Construct the runtime-owned foundation engine with only bundled non-executing analyzers.
    ///
    /// This path is limited to analyzers compiled into this crate and does not authorize arbitrary
    /// analyzer implementations to run in process.
    ///
    /// # Errors
    ///
    /// Returns [`AnalysisError`] when the ingestion policy or engine identifiers are invalid.
    pub fn with_bundled_static_analyzers(
        ingestion_policy: IngestionPolicy,
        policy_id: &str,
        source_revision: &str,
    ) -> Result<Self, AnalysisError> {
        let analyzers: Vec<Box<dyn StaticAnalyzer>> = vec![Box::new(FormatAnalyzer)];
        Self::validate_configuration(&ingestion_policy, policy_id, source_revision, &analyzers)?;

        Ok(Self {
            ingestion_policy,
            policy_id: policy_id.to_owned(),
            source_revision: source_revision.to_owned(),
            analyzers,
            analyzer_execution_path: AnalyzerExecutionPath::BundledRuntime,
        })
    }

    fn validate_configuration(
        ingestion_policy: &IngestionPolicy,
        policy_id: &str,
        source_revision: &str,
        analyzers: &[Box<dyn StaticAnalyzer>],
    ) -> Result<(), AnalysisError> {
        ingestion_policy.validate()?;

        if !is_valid_engine_identifier(policy_id) {
            return Err(AnalysisError::InvalidEngineConfiguration {
                field_name: "policy_id",
            });
        }
        if !is_valid_engine_identifier(source_revision) {
            return Err(AnalysisError::InvalidEngineConfiguration {
                field_name: "source_revision",
            });
        }
        if analyzers.is_empty() {
            return Err(AnalysisError::NoAnalyzersConfigured);
        }

        let mut analyzer_ids = BTreeSet::new();
        for analyzer in analyzers {
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

        Ok(())
    }

    /// Analyze bounded bytes and assemble an attributable evidence bundle.
    ///
    /// The runtime never requires a file name. If bounded source context includes one, it is used
    /// only as untrusted classification metadata. Externally supplied analyzers are rejected before
    /// invocation until an isolated worker can provide enforceable capability evidence.
    ///
    /// # Errors
    ///
    /// Returns [`AnalysisError`] when request validation, ingestion, analyzer isolation, or
    /// generated-contract validation fails.
    pub fn analyze_bytes(
        &self,
        request: &AnalysisRequest,
        bytes: &[u8],
    ) -> Result<EvidenceBundle, AnalysisError> {
        request.validate()?;

        if self.analyzer_execution_path == AnalyzerExecutionPath::IsolatedWorkerRequired {
            return Err(AnalysisError::IsolatedAnalyzerWorkerRequired);
        }

        let original_file_name = request
            .bounded_source_context
            .as_ref()
            .and_then(|context| context.original_file_name.as_deref());
        let artifact =
            ingest_bytes_with_optional_name(original_file_name, bytes, &self.ingestion_policy)?;
        let analysis_job_id = self.deterministic_job_id(request, artifact.descriptor());
        let mut records = Vec::new();

        let mut identity_attributes = BTreeMap::from([
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
        ]);
        if let Some(original_file_name) = &artifact.descriptor().original_file_name {
            identity_attributes.insert("original_file_name".to_owned(), original_file_name.clone());
        }

        push_record(
            &mut records,
            &analysis_job_id,
            EvidenceKind::ArtifactIdentity,
            "runtime_core",
            "Artifact identity established.",
            identity_attributes,
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
                    let mut attributes = BTreeMap::from([(
                        "failure_code".to_owned(),
                        failure.failure_code().to_owned(),
                    )]);
                    if failure.analyzer_id() != analyzer.analyzer_id() {
                        attributes.insert(
                            "reported_analyzer_id".to_owned(),
                            failure.analyzer_id().to_owned(),
                        );
                    }
                    push_record(
                        &mut records,
                        &analysis_job_id,
                        EvidenceKind::ToolFailure,
                        analyzer.analyzer_id(),
                        "Static analyzer did not complete.",
                        attributes,
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
        Self::with_bundled_static_analyzers(
            IngestionPolicy::default(),
            "foundation_policy_v1",
            "development",
        )
        .expect("built-in analysis engine configuration must remain valid")
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

#[cfg(test)]
mod tests {
    use super::*;

    fn request(profile: AnalysisProfile) -> AnalysisRequest {
        AnalysisRequest {
            schema_version: "1.0.0".to_owned(),
            request_id: "runtime_internal_fixture".to_owned(),
            profile,
            bounded_source_context: None,
        }
    }

    fn trusted_fixture_engine(analyzers: Vec<Box<dyn StaticAnalyzer>>) -> AnalysisEngine {
        let ingestion_policy = IngestionPolicy::default();
        AnalysisEngine::validate_configuration(
            &ingestion_policy,
            "foundation_policy_v1",
            "unit_test",
            &analyzers,
        )
        .expect("private unit fixture must satisfy engine configuration");

        AnalysisEngine {
            ingestion_policy,
            policy_id: "foundation_policy_v1".to_owned(),
            source_revision: "unit_test".to_owned(),
            analyzers,
            analyzer_execution_path: AnalyzerExecutionPath::BundledRuntime,
        }
    }

    struct SuccessfulAnalyzer;

    impl StaticAnalyzer for SuccessfulAnalyzer {
        fn analyzer_id(&self) -> &'static str {
            "successful_analyzer"
        }

        fn analyze(
            &self,
            _artifact: &IngestedArtifact,
        ) -> Result<Vec<AnalyzerFinding>, AnalyzerFailure> {
            Ok(vec![AnalyzerFinding {
                evidence_kind: EvidenceKind::StaticCapability,
                summary: "Fixture capability detected.".to_owned(),
                attributes: BTreeMap::from([("capability_code".to_owned(), "test".to_owned())]),
            }])
        }
    }

    struct FailingAnalyzer;

    impl StaticAnalyzer for FailingAnalyzer {
        fn analyzer_id(&self) -> &'static str {
            "failing_analyzer"
        }

        fn analyze(
            &self,
            _artifact: &IngestedArtifact,
        ) -> Result<Vec<AnalyzerFinding>, AnalyzerFailure> {
            Err(AnalyzerFailure::new(
                "reported_failure_analyzer",
                "fixture_failure",
            ))
        }
    }

    struct EmptyAnalyzer;

    impl StaticAnalyzer for EmptyAnalyzer {
        fn analyzer_id(&self) -> &'static str {
            "empty_analyzer"
        }

        fn analyze(
            &self,
            _artifact: &IngestedArtifact,
        ) -> Result<Vec<AnalyzerFinding>, AnalyzerFailure> {
            Ok(Vec::new())
        }
    }

    struct InvalidFindingAnalyzer;

    impl StaticAnalyzer for InvalidFindingAnalyzer {
        fn analyzer_id(&self) -> &'static str {
            "invalid_finding_analyzer"
        }

        fn analyze(
            &self,
            _artifact: &IngestedArtifact,
        ) -> Result<Vec<AnalyzerFinding>, AnalyzerFailure> {
            Ok(vec![AnalyzerFinding {
                evidence_kind: EvidenceKind::StaticCapability,
                summary: String::new(),
                attributes: BTreeMap::new(),
            }])
        }
    }

    struct DisallowedEvidenceAnalyzer;

    impl StaticAnalyzer for DisallowedEvidenceAnalyzer {
        fn analyzer_id(&self) -> &'static str {
            "disallowed_evidence_analyzer"
        }

        fn analyze(
            &self,
            _artifact: &IngestedArtifact,
        ) -> Result<Vec<AnalyzerFinding>, AnalyzerFailure> {
            Ok(vec![AnalyzerFinding {
                evidence_kind: EvidenceKind::RuntimeBehavior,
                summary: "A static analyzer must not claim runtime behavior.".to_owned(),
                attributes: BTreeMap::new(),
            }])
        }
    }

    struct AlternateEmptyAnalyzer;

    impl StaticAnalyzer for AlternateEmptyAnalyzer {
        fn analyzer_id(&self) -> &'static str {
            "alternate_empty_analyzer"
        }

        fn analyze(
            &self,
            _artifact: &IngestedArtifact,
        ) -> Result<Vec<AnalyzerFinding>, AnalyzerFailure> {
            Ok(Vec::new())
        }
    }

    #[test]
    fn private_bundled_fixture_preserves_findings_and_failure_attribution() {
        let engine = trusted_fixture_engine(vec![
            Box::new(SuccessfulAnalyzer),
            Box::new(FailingAnalyzer),
        ]);
        let bundle = engine
            .analyze_bytes(&request(AnalysisProfile::StaticOnly), b"safe text")
            .expect("private fixture normalization must complete");

        assert_eq!(bundle.disposition, RuntimeDisposition::Inconclusive);
        assert!(
            bundle
                .evidence
                .iter()
                .any(|record| record.evidence_kind == EvidenceKind::StaticCapability)
        );
        let failure = bundle
            .evidence
            .iter()
            .find(|record| record.evidence_kind == EvidenceKind::ToolFailure)
            .expect("failure evidence must be preserved");
        assert_eq!(failure.producer_id, "failing_analyzer");
        assert_eq!(
            failure.attributes.get("failure_code"),
            Some(&"fixture_failure".to_owned())
        );
        assert_eq!(
            failure.attributes.get("reported_analyzer_id"),
            Some(&"reported_failure_analyzer".to_owned())
        );
        assert!(
            bundle
                .limitations
                .contains(&"static_analyzer_failure".to_owned())
        );
    }

    #[test]
    fn private_bundled_fixture_covers_empty_invalid_and_disallowed_findings() {
        let empty = trusted_fixture_engine(vec![Box::new(EmptyAnalyzer)])
            .analyze_bytes(&request(AnalysisProfile::StaticOnly), b"abc")
            .expect("empty analyzer output is valid");
        assert_eq!(empty.disposition, RuntimeDisposition::Completed);
        assert_eq!(empty.evidence.len(), 2);

        let invalid = trusted_fixture_engine(vec![Box::new(InvalidFindingAnalyzer)])
            .analyze_bytes(&request(AnalysisProfile::StaticOnly), b"abc");
        assert_eq!(
            invalid,
            Err(AnalysisError::Contract(ContractError::EmptyField {
                field_name: "summary",
            }))
        );

        let disallowed = trusted_fixture_engine(vec![Box::new(DisallowedEvidenceAnalyzer)])
            .analyze_bytes(&request(AnalysisProfile::LinuxDynamic), b"abc")
            .expect("disallowed evidence must become attributable failure evidence");
        assert_eq!(disallowed.disposition, RuntimeDisposition::Inconclusive);
        assert!(
            disallowed
                .limitations
                .contains(&"dynamic_analysis_not_configured".to_owned())
        );
        let failure = disallowed
            .evidence
            .iter()
            .find(|record| record.evidence_kind == EvidenceKind::ToolFailure)
            .expect("disallowed evidence must become ToolFailure");
        assert_eq!(
            failure.attributes.get("failure_code"),
            Some(&"disallowed_evidence_kind".to_owned())
        );
        assert_eq!(
            failure.attributes.get("reported_evidence_kind"),
            Some(&"runtime_behavior".to_owned())
        );
        assert!(
            disallowed
                .evidence
                .iter()
                .all(|record| record.evidence_kind != EvidenceKind::RuntimeBehavior)
        );
    }

    #[test]
    fn private_bundled_fixture_keeps_deterministic_identity_sensitive_to_analyzer_set() {
        let first_engine = trusted_fixture_engine(vec![Box::new(EmptyAnalyzer)]);
        let second_engine = trusted_fixture_engine(vec![Box::new(AlternateEmptyAnalyzer)]);
        let first = first_engine
            .analyze_bytes(&request(AnalysisProfile::StaticOnly), b"abc")
            .expect("first private fixture analysis must complete");
        let first_repeat = first_engine
            .analyze_bytes(&request(AnalysisProfile::StaticOnly), b"abc")
            .expect("repeat private fixture analysis must complete");
        let second = second_engine
            .analyze_bytes(&request(AnalysisProfile::StaticOnly), b"abc")
            .expect("second private fixture analysis must complete");

        assert_eq!(first.analysis_job_id, first_repeat.analysis_job_id);
        assert_eq!(first.evidence, first_repeat.evidence);
        assert_ne!(first.analysis_job_id, second.analysis_job_id);
    }
}
