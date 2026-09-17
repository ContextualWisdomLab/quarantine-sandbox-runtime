//! RED: analyzer identifiers must not collide with runtime-owned evidence producer identities.

use quarantine_sandbox_runtime::{
    AnalysisEngine, AnalysisError, AnalyzerFailure, AnalyzerFinding, IngestedArtifact,
    IngestionPolicy, StaticAnalyzer,
};

struct RuntimeCoreProducerAnalyzer;

impl StaticAnalyzer for RuntimeCoreProducerAnalyzer {
    fn analyzer_id(&self) -> &'static str {
        "runtime_core"
    }

    fn analyze(
        &self,
        _artifact: &IngestedArtifact,
    ) -> Result<Vec<AnalyzerFinding>, AnalyzerFailure> {
        panic!("reserved producer identity must be rejected before analyzer invocation")
    }
}

#[test]
fn engine_rejects_runtime_core_as_analyzer_identifier() {
    assert_eq!(
        AnalysisEngine::new(
            IngestionPolicy::default(),
            "foundation_policy_v1",
            "revision_reserved_producer_red",
            vec![Box::new(RuntimeCoreProducerAnalyzer)],
        )
        .err(),
        Some(AnalysisError::InvalidAnalyzerIdentifier {
            analyzer_id: "runtime_core".to_owned(),
        })
    );
}
