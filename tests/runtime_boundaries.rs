//! Boundary tests for analyzer admission and deterministic identities.

use quarantine_sandbox_runtime::{
    AnalysisEngine, AnalysisError, AnalysisProfile, AnalysisRequest, AnalyzerFailure,
    AnalyzerFinding, BoundedSourceContext, FormatAnalyzer, IngestionError, IngestionPolicy,
    StaticAnalyzer,
};

fn request(profile: AnalysisProfile) -> AnalysisRequest {
    AnalysisRequest {
        schema_version: "1.0.0".to_owned(),
        request_id: "request_boundary_runtime".to_owned(),
        profile,
        bounded_source_context: Some(BoundedSourceContext {
            source_channel_code: Some("runtime_boundary_test".to_owned()),
            original_file_name: Some("sample.bin".to_owned()),
            declared_media_type: None,
            host_artifact_reference: Some("fixture_runtime_boundary".to_owned()),
            submitted_at: None,
        }),
    }
}

struct EmptyAnalyzer;

impl StaticAnalyzer for EmptyAnalyzer {
    fn analyzer_id(&self) -> &'static str {
        "empty_analyzer"
    }

    fn analyze(
        &self,
        _artifact: &quarantine_sandbox_runtime::IngestedArtifact,
    ) -> Result<Vec<AnalyzerFinding>, AnalyzerFailure> {
        panic!("external analyzer must not be invoked before isolated worker execution exists")
    }
}

struct InvalidIdentifierAnalyzer;

impl StaticAnalyzer for InvalidIdentifierAnalyzer {
    fn analyzer_id(&self) -> &'static str {
        ""
    }

    fn analyze(
        &self,
        _artifact: &quarantine_sandbox_runtime::IngestedArtifact,
    ) -> Result<Vec<AnalyzerFinding>, AnalyzerFailure> {
        Ok(Vec::new())
    }
}

struct DuplicateIdentifierAnalyzer;

impl StaticAnalyzer for DuplicateIdentifierAnalyzer {
    fn analyzer_id(&self) -> &'static str {
        "duplicate_analyzer"
    }

    fn analyze(
        &self,
        _artifact: &quarantine_sandbox_runtime::IngestedArtifact,
    ) -> Result<Vec<AnalyzerFinding>, AnalyzerFailure> {
        Ok(Vec::new())
    }
}

#[test]
fn external_analyzer_configuration_fails_closed_before_invocation() {
    let engine = AnalysisEngine::new(
        IngestionPolicy::default(),
        "foundation_policy_v1",
        "revision_test",
        vec![Box::new(EmptyAnalyzer)],
    )
    .expect("valid external analyzer identity must remain representable");

    assert_eq!(
        engine.analyze_bytes(&request(AnalysisProfile::StaticOnly), b"abc"),
        Err(AnalysisError::IsolatedAnalyzerWorkerRequired)
    );
    assert_eq!(
        engine.analyze_bytes(&request(AnalysisProfile::LinuxDynamic), b"abc"),
        Err(AnalysisError::IsolatedAnalyzerWorkerRequired)
    );
}

#[test]
fn engine_rejects_invalid_ingestion_policy_before_analysis() {
    let invalid_policy = IngestionPolicy {
        maximum_artifact_bytes: 0,
        maximum_artifact_name_bytes: 255,
    };
    assert_eq!(
        AnalysisEngine::new(
            invalid_policy,
            "policy",
            "revision",
            vec![Box::new(FormatAnalyzer)],
        )
        .err(),
        Some(AnalysisError::Ingestion(IngestionError::InvalidPolicy {
            policy_field: "maximum_artifact_bytes",
        }))
    );
}

#[test]
fn engine_rejects_invalid_and_duplicate_identifiers() {
    let oversized_policy_id = "x".repeat(129);
    assert_eq!(
        AnalysisEngine::new(
            IngestionPolicy::default(),
            &oversized_policy_id,
            "revision",
            vec![Box::new(FormatAnalyzer)],
        )
        .err(),
        Some(AnalysisError::InvalidEngineConfiguration {
            field_name: "policy_id",
        })
    );
    assert_eq!(
        AnalysisEngine::new(
            IngestionPolicy::default(),
            "policy",
            "bad\nrevision",
            vec![Box::new(FormatAnalyzer)],
        )
        .err(),
        Some(AnalysisError::InvalidEngineConfiguration {
            field_name: "source_revision",
        })
    );
    assert_eq!(
        AnalysisEngine::new(
            IngestionPolicy::default(),
            "policy",
            "revision",
            vec![Box::new(InvalidIdentifierAnalyzer)],
        )
        .err(),
        Some(AnalysisError::InvalidAnalyzerIdentifier {
            analyzer_id: String::new(),
        })
    );
    assert_eq!(
        AnalysisEngine::new(
            IngestionPolicy::default(),
            "policy",
            "revision",
            vec![
                Box::new(DuplicateIdentifierAnalyzer),
                Box::new(DuplicateIdentifierAnalyzer),
            ],
        )
        .err(),
        Some(AnalysisError::DuplicateAnalyzerIdentifier {
            analyzer_id: "duplicate_analyzer".to_owned(),
        })
    );
}

#[test]
fn deterministic_identifiers_change_when_identity_or_bundled_engine_inputs_change() {
    let engine = AnalysisEngine::default();
    let baseline = engine
        .analyze_bytes(&request(AnalysisProfile::StaticOnly), b"abc")
        .expect("baseline analysis must succeed");

    let mut changed_request = request(AnalysisProfile::StaticOnly);
    changed_request.request_id = "request_boundary_runtime_002".to_owned();
    let changed_id = engine
        .analyze_bytes(&changed_request, b"abc")
        .expect("changed request analysis must succeed");
    assert_ne!(baseline.analysis_job_id, changed_id.analysis_job_id);

    let changed_profile = engine
        .analyze_bytes(&request(AnalysisProfile::LinuxDynamic), b"abc")
        .expect("changed profile analysis must return evidence");
    assert_ne!(baseline.analysis_job_id, changed_profile.analysis_job_id);

    let changed_artifact = engine
        .analyze_bytes(&request(AnalysisProfile::StaticOnly), b"abd")
        .expect("changed artifact analysis must succeed");
    assert_ne!(baseline.analysis_job_id, changed_artifact.analysis_job_id);

    for changed_engine in [
        AnalysisEngine::with_bundled_static_analyzers(
            IngestionPolicy::default(),
            "changed_policy",
            "development",
        )
        .expect("changed policy engine must be valid"),
        AnalysisEngine::with_bundled_static_analyzers(
            IngestionPolicy::default(),
            "foundation_policy_v1",
            "changed_revision",
        )
        .expect("changed revision engine must be valid"),
    ] {
        let changed = changed_engine
            .analyze_bytes(&request(AnalysisProfile::StaticOnly), b"abc")
            .expect("changed bundled engine analysis must succeed");
        assert_ne!(baseline.analysis_job_id, changed.analysis_job_id);
    }
}
