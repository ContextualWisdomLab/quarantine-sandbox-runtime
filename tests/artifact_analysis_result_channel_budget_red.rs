//! RED for bounded artifact-analyzer result ingestion.
//!
//! Issue #50 is independent from the host-capability isolation in #49. Even an
//! isolated analyzer worker can still exhaust the trusted controller if its
//! findings are materialized and accumulated without an operator-owned output
//! budget. The 64 KiB fixture budget mirrors issue #17's versioned
//! `maximum_output_bytes` contract; it is test authority, not a proposed global
//! production constant.

use std::collections::BTreeMap;

use quarantine_sandbox_runtime::{
    AnalysisEngine, AnalysisProfile, AnalysisRequest, AnalyzerFailure, AnalyzerFinding,
    EvidenceKind, IngestedArtifact, IngestionPolicy, RuntimeDisposition, StaticAnalyzer,
};

const DECLARED_PROFILE_OUTPUT_BUDGET_BYTES: usize = 65_536;
const AMPLIFIED_FINDING_COUNT: usize = 512;
const AMPLIFIED_SUMMARY_BYTES: usize = 256;

struct AmplifyingAnalyzer;

impl StaticAnalyzer for AmplifyingAnalyzer {
    fn analyzer_id(&self) -> &'static str {
        "amplifying_analyzer"
    }

    fn analyze(
        &self,
        _artifact: &IngestedArtifact,
    ) -> Result<Vec<AnalyzerFinding>, AnalyzerFailure> {
        let mut findings = Vec::with_capacity(AMPLIFIED_FINDING_COUNT);
        for index in 0..AMPLIFIED_FINDING_COUNT {
            findings.push(AnalyzerFinding {
                evidence_kind: EvidenceKind::StaticCapability,
                summary: "x".repeat(AMPLIFIED_SUMMARY_BYTES),
                attributes: BTreeMap::from([(
                    "capability_code".to_owned(),
                    format!("amplified_finding_{index:04}"),
                )]),
            });
        }
        Ok(findings)
    }
}

#[test]
fn analyzer_result_amplification_cannot_complete_past_declared_profile_budget() {
    let engine = AnalysisEngine::new(
        IngestionPolicy::default(),
        "artifact_analysis_result_budget_v1",
        "result-budget-red",
        vec![Box::new(AmplifyingAnalyzer)],
    )
    .expect("fixture engine is valid");
    let request = AnalysisRequest {
        schema_version: "1.0.0".to_owned(),
        request_id: "result-budget-red-001".to_owned(),
        profile: AnalysisProfile::StaticOnly,
        bounded_source_context: None,
    };

    let result = engine.analyze_bytes(&request, b"purpose-built-safe-fixture");

    match result {
        Err(_) => {
            // Fail-closed rejection is an acceptable future GREEN once the
            // versioned profile budget is bound to the worker/result channel.
        }
        Ok(bundle) => {
            let serialized = serde_json::to_vec(&bundle).expect("evidence bundle serializes");
            assert!(
                bundle.disposition != RuntimeDisposition::Completed
                    && serialized.len() <= DECLARED_PROFILE_OUTPUT_BUDGET_BYTES,
                "an over-budget analyzer result must not be materialized as Completed evidence: disposition={:?}, serialized_bytes={}, declared_budget_bytes={}",
                bundle.disposition,
                serialized.len(),
                DECLARED_PROFILE_OUTPUT_BUDGET_BYTES
            );
        }
    }
}
