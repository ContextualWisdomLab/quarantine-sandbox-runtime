use std::collections::BTreeMap;

use quarantine_sandbox_runtime::{
    AnalysisContext, AnalysisEngine, AnalysisError, AnalysisProfile, AnalysisRequest,
    AnalyzerFailure, AnalyzerFinding, EvidenceKind, FormatAnalyzer, IngestionError,
    IngestionPolicy, RuntimeDisposition, StaticAnalyzer,
};

fn request(profile: AnalysisProfile) -> AnalysisRequest {
    AnalysisRequest {
        schema_version: "1.0.0".to_owned(),
        request_id: "request_boundary_runtime".to_owned(),
        profile,
        context: AnalysisContext {
            source_system: "runtime_boundary_test".to_owned(),
            source_reference: "fixture_runtime_boundary".to_owned(),
            attributes: BTreeMap::new(),
        },
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
        _artifact: &quarantine_sandbox_runtime::IngestedArtifact,
    ) -> Result<Vec<AnalyzerFinding>, AnalyzerFailure> {
        Ok(vec![AnalyzerFinding {
            evidence_kind: EvidenceKind::StaticCapability,
            summary: String::new(),
            attributes: BTreeMap::new(),
        }])
    }
}

#[test]
fn empty_output_is_valid_but_malformed_findings_fail_contract_validation() {
    let empty_engine = AnalysisEngine::new(
        IngestionPolicy::default(),
        "foundation_policy_v1",
        "revision_test",
        vec![Box::new(EmptyAnalyzer)],
    )
    .expect("empty findings are a valid analyzer result");
    let empty_bundle = empty_engine
        .analyze_bytes(
            &request(AnalysisProfile::StaticOnly),
            "sample.bin",
            b"abc",
        )
        .expect("identity and boundary evidence must remain sufficient");
    assert_eq!(empty_bundle.disposition, RuntimeDisposition::Completed);
    assert_eq!(empty_bundle.evidence.len(), 2);

    let invalid_engine = AnalysisEngine::new(
        IngestionPolicy::default(),
        "foundation_policy_v1",
        "revision_test",
        vec![Box::new(InvalidFindingAnalyzer)],
    )
    .expect("engine configuration itself is valid");
    assert_eq!(
        invalid_engine.analyze_bytes(
            &request(AnalysisProfile::StaticOnly),
            "sample.bin",
            b"abc",
        ),
        Err(AnalysisError::Contract(
            quarantine_sandbox_runtime::ContractError::EmptyField {
                field_name: "summary",
            }
        ))
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
fn deterministic_identifiers_change_when_identity_inputs_change() {
    let engine = AnalysisEngine::default();
    let baseline = engine
        .analyze_bytes(
            &request(AnalysisProfile::StaticOnly),
            "sample.bin",
            b"abc",
        )
        .expect("baseline analysis must succeed");

    let mut changed_request = request(AnalysisProfile::StaticOnly);
    changed_request.request_id = "request_boundary_runtime_002".to_owned();
    let changed_id = engine
        .analyze_bytes(&changed_request, "sample.bin", b"abc")
        .expect("changed request analysis must succeed");
    assert_ne!(baseline.analysis_job_id, changed_id.analysis_job_id);

    let changed_profile = engine
        .analyze_bytes(
            &request(AnalysisProfile::LinuxDynamic),
            "sample.bin",
            b"abc",
        )
        .expect("changed profile analysis must return evidence");
    assert_ne!(baseline.analysis_job_id, changed_profile.analysis_job_id);

    let changed_artifact = engine
        .analyze_bytes(
            &request(AnalysisProfile::StaticOnly),
            "sample.bin",
            b"abd",
        )
        .expect("changed artifact analysis must succeed");
    assert_ne!(baseline.analysis_job_id, changed_artifact.analysis_job_id);
}
