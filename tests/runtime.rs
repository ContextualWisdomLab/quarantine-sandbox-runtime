//! Integration tests for deterministic runtime evidence orchestration.

use std::collections::BTreeMap;

use quarantine_sandbox_runtime::{
    AnalysisContext, AnalysisEngine, AnalysisError, AnalysisProfile, AnalysisRequest,
    AnalyzerFailure, AnalyzerFinding, ArtifactKind, EvidenceKind, FormatAnalyzer, IngestionPolicy,
    RuntimeDisposition, StaticAnalyzer, to_pretty_json,
};

fn request(profile: AnalysisProfile) -> AnalysisRequest {
    AnalysisRequest {
        schema_version: "1.0.0".to_owned(),
        request_id: "request_runtime_001".to_owned(),
        profile,
        context: AnalysisContext {
            source_system: "integration_test".to_owned(),
            source_reference: "fixture_runtime_001".to_owned(),
            attributes: BTreeMap::new(),
        },
    }
}

#[test]
fn static_analysis_returns_attributable_evidence_without_a_verdict() {
    let engine = AnalysisEngine::new(
        IngestionPolicy::default(),
        "foundation_policy_v1",
        "revision_test",
        vec![Box::new(FormatAnalyzer)],
    )
    .expect("valid engine configuration must succeed");

    let bundle = engine
        .analyze_bytes(
            &request(AnalysisProfile::StaticOnly),
            "sample.exe",
            b"MZ\x90\x00",
        )
        .expect("static analysis must complete");

    assert_eq!(bundle.disposition, RuntimeDisposition::Completed);
    assert!(bundle.consumer_verdict_required);
    assert_eq!(
        bundle.artifact.artifact_kind,
        ArtifactKind::PortableExecutable
    );
    assert!(!bundle.runtime.dynamic_execution_performed);
    assert!(!bundle.runtime.network_access_performed);
    assert!(!bundle.runtime.credentials_available);
    assert_eq!(
        bundle
            .evidence
            .iter()
            .map(|record| record.sequence_number)
            .collect::<Vec<_>>(),
        vec![1, 2, 3]
    );
    assert_eq!(
        bundle.evidence[0].evidence_kind,
        EvidenceKind::ArtifactIdentity
    );
    assert_eq!(bundle.evidence[1].evidence_kind, EvidenceKind::FileFormat);
    assert_eq!(
        bundle.evidence[2].evidence_kind,
        EvidenceKind::PolicyBoundary
    );
    assert_eq!(
        bundle.limitations,
        vec!["runtime_does_not_determine_maliciousness"]
    );
    assert_eq!(bundle.validate(), Ok(()));
}

#[test]
fn unavailable_dynamic_profiles_fail_closed_as_inconclusive() {
    for profile in [
        AnalysisProfile::LinuxDynamic,
        AnalysisProfile::WindowsDynamic,
    ] {
        let engine = AnalysisEngine::default();
        let bundle = engine
            .analyze_bytes(&request(profile), "sample.bin", b"abc")
            .expect("bounded static foundation evidence must still be returned");

        assert_eq!(bundle.disposition, RuntimeDisposition::Inconclusive);
        assert!(
            bundle
                .limitations
                .contains(&"dynamic_analysis_not_configured".to_owned())
        );
        assert!(!bundle.runtime.dynamic_execution_performed);
    }
}

struct SuccessfulAnalyzer;

impl StaticAnalyzer for SuccessfulAnalyzer {
    fn analyzer_id(&self) -> &'static str {
        "successful_analyzer"
    }

    fn analyze(
        &self,
        _artifact: &quarantine_sandbox_runtime::IngestedArtifact,
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
        _artifact: &quarantine_sandbox_runtime::IngestedArtifact,
    ) -> Result<Vec<AnalyzerFinding>, AnalyzerFailure> {
        Err(AnalyzerFailure::new(self.analyzer_id(), "fixture_failure"))
    }
}

#[test]
fn analyzer_findings_are_ordered_and_failures_are_preserved_as_evidence() {
    let engine = AnalysisEngine::new(
        IngestionPolicy::default(),
        "foundation_policy_v1",
        "revision_test",
        vec![Box::new(SuccessfulAnalyzer), Box::new(FailingAnalyzer)],
    )
    .expect("valid engine configuration must succeed");

    let bundle = engine
        .analyze_bytes(
            &request(AnalysisProfile::StaticOnly),
            "sample.txt",
            b"safe text",
        )
        .expect("analyzer failures must not erase available evidence");

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
        .expect("tool failure evidence must be present");
    assert_eq!(failure.producer_id, "failing_analyzer");
    assert_eq!(
        failure.attributes.get("failure_code"),
        Some(&"fixture_failure".to_owned())
    );
    assert!(
        bundle
            .limitations
            .contains(&"static_analyzer_failure".to_owned())
    );
}

#[test]
fn runtime_rejects_invalid_requests_artifacts_and_engine_configuration() {
    let engine = AnalysisEngine::default();

    let mut invalid_request = request(AnalysisProfile::StaticOnly);
    invalid_request.request_id.clear();
    assert_eq!(
        engine.analyze_bytes(&invalid_request, "sample.bin", b"abc"),
        Err(AnalysisError::Contract(
            quarantine_sandbox_runtime::ContractError::EmptyField {
                field_name: "request_id"
            }
        ))
    );

    assert_eq!(
        engine.analyze_bytes(&request(AnalysisProfile::StaticOnly), "sample.bin", b""),
        Err(AnalysisError::Ingestion(
            quarantine_sandbox_runtime::IngestionError::EmptyArtifact
        ))
    );

    assert!(matches!(
        AnalysisEngine::new(
            IngestionPolicy::default(),
            "",
            "revision",
            vec![Box::new(FormatAnalyzer)]
        ),
        Err(AnalysisError::InvalidEngineConfiguration {
            field_name: "policy_id"
        })
    ));

    assert!(matches!(
        AnalysisEngine::new(
            IngestionPolicy::default(),
            "policy",
            "",
            vec![Box::new(FormatAnalyzer)]
        ),
        Err(AnalysisError::InvalidEngineConfiguration {
            field_name: "source_revision"
        })
    ));

    assert!(matches!(
        AnalysisEngine::new(IngestionPolicy::default(), "policy", "revision", Vec::new()),
        Err(AnalysisError::NoAnalyzersConfigured)
    ));
}

#[test]
fn job_and_evidence_identifiers_are_deterministic_and_json_is_pretty() {
    let engine = AnalysisEngine::default();
    let first = engine
        .analyze_bytes(&request(AnalysisProfile::StaticOnly), "sample.bin", b"abc")
        .expect("analysis must succeed");
    let second = engine
        .analyze_bytes(&request(AnalysisProfile::StaticOnly), "sample.bin", b"abc")
        .expect("analysis must succeed");

    assert_eq!(first.analysis_job_id, second.analysis_job_id);
    assert_eq!(first.evidence, second.evidence);

    let json = to_pretty_json(&first).expect("evidence must serialize");
    assert!(json.contains("\n  \"analysis_job_id\""));
    assert!(!json.contains("\"malicious\""));
    assert!(!json.contains("\"verdict\""));
}
