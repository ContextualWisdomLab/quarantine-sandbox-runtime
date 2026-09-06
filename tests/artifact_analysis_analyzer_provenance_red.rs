//! Regression: deterministic analysis identity must bind analyzer provenance.
//!
//! Two materially different analyzer implementations can currently claim the
//! same `analyzer_id`. Because `AnalysisEngine::deterministic_job_id` hashes
//! only that human-readable identifier, both configurations can emit different
//! completed evidence under one identical analysis-job identity. A future
//! implementation may reject analyzers that lack stable provenance or bind a
//! versioned analyzer implementation/configuration identity into the job and
//! evidence contracts. Distinct provenance must not be implemented by adding
//! process-local randomness: identical analyzer configuration and input must
//! retain deterministic job and evidence identity across engine instances.

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

            assert_eq!(
                first_bundle.analysis_job_id, repeated_first_bundle.analysis_job_id,
                "identical analyzer provenance and input must retain deterministic job identity; process-local randomness is not provenance"
            );
            assert_eq!(
                first_bundle.evidence, repeated_first_bundle.evidence,
                "identical analyzer provenance and input must retain deterministic semantic evidence"
            );
            assert_ne!(
                first_bundle.evidence, second_bundle.evidence,
                "fixture must prove the analyzer implementations are materially different"
            );
            assert_ne!(
                first_bundle.analysis_job_id, second_bundle.analysis_job_id,
                "different analyzer implementations produced different evidence under one deterministic job identity; analyzer provenance must be bound to job/evidence identity"
            );
        }
        _ => panic!(
            "analyzer provenance admission must be deterministic: identical provenance requirements cannot accept only one of two otherwise-identical configurations"
        ),
    }
}
