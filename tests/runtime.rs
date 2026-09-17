//! Integration tests for deterministic runtime evidence orchestration.

use quarantine_sandbox_runtime::{
    AnalysisEngine, AnalysisError, AnalysisProfile, AnalysisRequest, AnalyzerFailure,
    AnalyzerFinding, ArtifactKind, BoundedSourceContext, EvidenceKind, FormatAnalyzer,
    IngestionPolicy, RuntimeDisposition, StaticAnalyzer, to_pretty_json,
};

fn request(profile: AnalysisProfile) -> AnalysisRequest {
    AnalysisRequest {
        schema_version: "1.0.0".to_owned(),
        request_id: "request_runtime_001".to_owned(),
        profile,
        bounded_source_context: Some(BoundedSourceContext {
            source_channel_code: Some("integration_test".to_owned()),
            original_file_name: Some("sample.bin".to_owned()),
            declared_media_type: None,
            host_artifact_reference: Some("fixture_runtime_001".to_owned()),
            submitted_at: None,
        }),
    }
}

#[test]
fn static_analysis_returns_attributable_evidence_without_a_verdict() {
    let engine = AnalysisEngine::with_bundled_static_analyzers(
        IngestionPolicy::default(),
        "foundation_policy_v1",
        "revision_test",
    )
    .expect("valid bundled engine configuration must succeed");

    let bundle = engine
        .analyze_bytes(&request(AnalysisProfile::StaticOnly), b"MZ\x90\x00")
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
            .analyze_bytes(&request(profile), b"abc")
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

struct ExternalAnalyzer;

impl StaticAnalyzer for ExternalAnalyzer {
    fn analyzer_id(&self) -> &'static str {
        "external_analyzer"
    }

    fn analyze(
        &self,
        _artifact: &quarantine_sandbox_runtime::IngestedArtifact,
    ) -> Result<Vec<AnalyzerFinding>, AnalyzerFailure> {
        panic!("externally supplied analyzer must not execute in the runtime host process")
    }
}

#[test]
fn externally_supplied_analyzers_require_isolated_worker_execution() {
    let engine = AnalysisEngine::new(
        IngestionPolicy::default(),
        "foundation_policy_v1",
        "revision_test",
        vec![Box::new(ExternalAnalyzer)],
    )
    .expect("otherwise-valid external analyzer configuration must remain representable");

    assert_eq!(
        engine.analyze_bytes(&request(AnalysisProfile::StaticOnly), b"safe text"),
        Err(AnalysisError::IsolatedAnalyzerWorkerRequired)
    );
}

#[test]
fn runtime_rejects_invalid_requests_artifacts_and_engine_configuration() {
    let engine = AnalysisEngine::default();

    let mut invalid_request = request(AnalysisProfile::StaticOnly);
    invalid_request.request_id.clear();
    assert_eq!(
        engine.analyze_bytes(&invalid_request, b"abc"),
        Err(AnalysisError::Contract(
            quarantine_sandbox_runtime::ContractError::EmptyField {
                field_name: "request_id"
            }
        ))
    );

    assert_eq!(
        engine.analyze_bytes(&request(AnalysisProfile::StaticOnly), b""),
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
        .analyze_bytes(&request(AnalysisProfile::StaticOnly), b"abc")
        .expect("analysis must succeed");
    let second = engine
        .analyze_bytes(&request(AnalysisProfile::StaticOnly), b"abc")
        .expect("analysis must succeed");

    assert_eq!(first.analysis_job_id, second.analysis_job_id);
    assert_eq!(first.evidence, second.evidence);

    let json = to_pretty_json(&first).expect("evidence must serialize");
    assert!(json.contains("\n  \"analysis_job_id\""));
    assert!(!json.contains("\"malicious\""));
    assert!(!json.contains("\"verdict\""));
}
