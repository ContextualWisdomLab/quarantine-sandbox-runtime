//! Regression: deterministic analysis identity must bind analyzer provenance.
//!
//! A human-readable `analyzer_id` is not executable provenance. Two materially
//! different analyzer implementations can currently claim the same ID, and the
//! runtime can therefore collapse distinct producers into one analysis-job and
//! evidence-producer identity. A valid repair must remain deterministic for the
//! same provenance/input, distinguish different analyzer provenance even when
//! semantic findings happen to be identical, and must not infer provenance from
//! findings, process-local randomness, Rust type identity, or addresses.

use std::collections::BTreeMap;

use quarantine_sandbox_runtime::{
    AnalysisEngine, AnalysisProfile, AnalysisRequest, AnalyzerFailure, AnalyzerFinding,
    CONTRACT_SCHEMA_VERSION, EvidenceKind, IngestedArtifact, IngestionPolicy, StaticAnalyzer,
};

struct AnalyzerVersionOne;

impl StaticAnalyzer for AnalyzerVersionOne {
    fn analyzer_id(&self) -> &'static str {
        "shared_semantic_analyzer"
    }

    fn analyze(
        &self,
        _artifact: &IngestedArtifact,
    ) -> Result<Vec<AnalyzerFinding>, AnalyzerFailure> {
        Ok(vec![AnalyzerFinding {
            evidence_kind: EvidenceKind::StaticCapability,
            summary: "Analyzer implementation one detected capability alpha.".to_owned(),
            attributes: BTreeMap::from([(
                "semantic_result".to_owned(),
                "alpha".to_owned(),
            )]),
        }])
    }
}

struct AnalyzerVersionTwo;

impl StaticAnalyzer for AnalyzerVersionTwo {
    fn analyzer_id(&self) -> &'static str {
        "shared_semantic_analyzer"
    }

    fn analyze(
        &self,
        _artifact: &IngestedArtifact,
    ) -> Result<Vec<AnalyzerFinding>, AnalyzerFailure> {
        Ok(vec![AnalyzerFinding {
            evidence_kind: EvidenceKind::StaticCapability,
            summary: "Analyzer implementation two detected capability beta.".to_owned(),
            attributes: BTreeMap::from([(
                "semantic_result".to_owned(),
                "beta".to_owned(),
            )]),
        }])
    }
}

/// Distinct implementation that intentionally emits the same semantic finding
/// as version one. This prevents a repair from deriving producer provenance
/// circularly from analyzer output rather than from stable executable/configuration identity.
struct AnalyzerVersionThree;

impl StaticAnalyzer for AnalyzerVersionThree {
    fn analyzer_id(&self) -> &'static str {
        "shared_semantic_analyzer"
    }

    fn analyze(
        &self,
        _artifact: &IngestedArtifact,
    ) -> Result<Vec<AnalyzerFinding>, AnalyzerFailure> {
        let semantic_result = ["al", "pha"].concat();
        Ok(vec![AnalyzerFinding {
            evidence_kind: EvidenceKind::StaticCapability,
            summary: "Analyzer implementation one detected capability alpha.".to_owned(),
            attributes: BTreeMap::from([("semantic_result".to_owned(), semantic_result)]),
        }])
    }
}

fn request() -> AnalysisRequest {
    AnalysisRequest {
        schema_version: CONTRACT_SCHEMA_VERSION.to_owned(),
        request_id: "artifact_analysis_analyzer_provenance_red".to_owned(),
        profile: AnalysisProfile::StaticOnly,
        bounded_source_context: None,
    }
}

#[test]
fn materially_different_analyzers_cannot_share_one_deterministic_job_identity() {
    let first_engine = AnalysisEngine::new(
        IngestionPolicy::default(),
        "analyzer_provenance_policy_v1",
        "artifact_provenance_red",
        vec![Box::new(AnalyzerVersionOne)],
    );
    let second_engine = AnalysisEngine::new(
        IngestionPolicy::default(),
        "analyzer_provenance_policy_v1",
        "artifact_provenance_red",
        vec![Box::new(AnalyzerVersionTwo)],
    );

    match (first_engine, second_engine) {
        (Err(_), Err(_)) => {
            // A future contract may fail closed when stable analyzer provenance
            // is unavailable rather than accepting an ambiguous producer ID.
        }
        (Ok(first_engine), Ok(second_engine)) => {
            let first_bundle = first_engine
                .analyze_bytes(&request(), b"same hostile artifact bytes")
                .expect("first otherwise-valid analyzer must produce evidence");
            let second_bundle = second_engine
                .analyze_bytes(&request(), b"same hostile artifact bytes")
                .expect("second otherwise-valid analyzer must produce evidence");

            let repeated_first_engine = AnalysisEngine::new(
                IngestionPolicy::default(),
                "analyzer_provenance_policy_v1",
                "artifact_provenance_red",
                vec![Box::new(AnalyzerVersionOne)],
            )
            .expect("an identical analyzer configuration must be admitted deterministically");
            let repeated_first_bundle = repeated_first_engine
                .analyze_bytes(&request(), b"same hostile artifact bytes")
                .expect("identical analyzer configuration must produce evidence deterministically");

            let same_output_different_engine = AnalysisEngine::new(
                IngestionPolicy::default(),
                "analyzer_provenance_policy_v1",
                "artifact_provenance_red",
                vec![Box::new(AnalyzerVersionThree)],
            )
            .expect("a distinct implementation with otherwise-valid configuration must be admitted consistently");
            let same_output_different_bundle = same_output_different_engine
                .analyze_bytes(&request(), b"same hostile artifact bytes")
                .expect("distinct implementation fixture must reach provenance assertions");

            let analyzer_semantics = |bundle: &quarantine_sandbox_runtime::EvidenceBundle| {
                bundle
                    .evidence
                    .iter()
                    .filter(|record| record.evidence_kind == EvidenceKind::StaticCapability)
                    .map(|record| {
                        (
                            record.evidence_kind,
                            record.summary.clone(),
                            record.attributes.clone(),
                        )
                    })
                    .collect::<Vec<_>>()
            };
            let analyzer_producer_ids = |bundle: &quarantine_sandbox_runtime::EvidenceBundle| {
                bundle
                    .evidence
                    .iter()
                    .filter(|record| record.evidence_kind == EvidenceKind::StaticCapability)
                    .map(|record| record.producer_id.clone())
                    .collect::<Vec<_>>()
            };

            assert_eq!(
                first_bundle.analysis_job_id, repeated_first_bundle.analysis_job_id,
                "identical analyzer provenance and input must retain deterministic job identity; process-local randomness is not provenance"
            );
            assert_eq!(
                first_bundle.evidence, repeated_first_bundle.evidence,
                "identical analyzer provenance and input must retain deterministic evidence identity"
            );
            assert_eq!(
                analyzer_producer_ids(&first_bundle),
                analyzer_producer_ids(&repeated_first_bundle),
                "identical analyzer provenance must retain the same serialized producer identity"
            );

            assert_ne!(
                analyzer_semantics(&first_bundle),
                analyzer_semantics(&second_bundle),
                "fixture must prove analyzer versions one and two are semantically different"
            );
            assert_ne!(
                first_bundle.analysis_job_id, second_bundle.analysis_job_id,
                "different analyzer implementations produced different evidence under one deterministic job identity; analyzer provenance must be bound to job identity"
            );
            assert_ne!(
                analyzer_producer_ids(&first_bundle),
                analyzer_producer_ids(&second_bundle),
                "different analyzer provenance must be distinguishable in serialized producer identity, not only in the job digest"
            );

            assert_eq!(
                analyzer_semantics(&first_bundle),
                analyzer_semantics(&same_output_different_bundle),
                "fixture must prove two distinct implementations can emit identical semantic findings"
            );
            assert_ne!(
                first_bundle.analysis_job_id, same_output_different_bundle.analysis_job_id,
                "analyzer output is not provenance: distinct implementations with identical findings still require distinct deterministic job identity or fail-closed admission"
            );
            assert_ne!(
                analyzer_producer_ids(&first_bundle),
                analyzer_producer_ids(&same_output_different_bundle),
                "analyzer output is not producer provenance: identical findings from distinct implementations must remain attributable to distinct stable producer identities"
            );
        }
        _ => panic!(
            "analyzer provenance admission must be deterministic: identical provenance requirements cannot accept only one of two otherwise-identical configurations"
        ),
    }
}
