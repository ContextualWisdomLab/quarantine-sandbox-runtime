//! Boundary tests for analyzer completion and deterministic identities.

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

struct DisallowedEvidenceAnalyzer;

impl StaticAnalyzer for DisallowedEvidenceAnalyzer {
    fn analyzer_id(&self) -> &'static str {
        "disallowed_evidence_analyzer"
    }

    fn analyze(
        &self,
        _artifact: &quarantine_sandbox_runtime::IngestedArtifact,
    ) -> Result<Vec<AnalyzerFinding>, AnalyzerFailure> {
        Ok(vec![AnalyzerFinding {
            evidence_kind: EvidenceKind::RuntimeBehavior,
            summary: "A static analyzer must not claim runtime behavior.".to_owned(),
            attributes: BTreeMap::new(),
        }])
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

struct SpoofingFailureAnalyzer;

impl StaticAnalyzer for SpoofingFailureAnalyzer {
    fn analyzer_id(&self) -> &'static str {
        "configured_analyzer"
    }

    fn analyze(
        &self,
        _artifact: &quarantine_sandbox_runtime::IngestedArtifact,
    ) -> Result<Vec<AnalyzerFinding>, AnalyzerFailure> {
        Err(AnalyzerFailure::new(
            "spoofed_analyzer",
            "fixture_failure",
        ))
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
        .analyze_bytes(&request(AnalysisProfile::StaticOnly), "sample.bin", b"abc")
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
        invalid_engine.analyze_bytes(&request(AnalysisProfile::StaticOnly), "sample.bin", b"abc"),
        Err(AnalysisError::Contract(
            quarantine_sandbox_runtime::ContractError::EmptyField {
                field_name: "summary",
            }
        ))
    );
}

#[test]
fn disallowed_static_evidence_fails_closed_without_claiming_runtime_behavior() {
    let engine = AnalysisEngine::new(
        IngestionPolicy::default(),
        "foundation_policy_v1",
        "revision_test",
        vec![Box::new(DisallowedEvidenceAnalyzer)],
    )
    .expect("analyzer identifier is valid");
    let bundle = engine
        .analyze_bytes(
            &request(AnalysisProfile::LinuxDynamic),
            "sample.bin",
            b"abc",
        )
        .expect("the runtime must preserve attributable failure evidence");

    assert_eq!(bundle.disposition, RuntimeDisposition::Inconclusive);
    assert!(
        bundle
            .limitations
            .contains(&"static_analyzer_failure".to_owned())
    );
    assert!(
        bundle
            .limitations
            .contains(&"dynamic_analysis_not_configured".to_owned())
    );
    let failure = bundle
        .evidence
        .iter()
        .find(|record| record.evidence_kind == EvidenceKind::ToolFailure)
        .expect("disallowed evidence must become a tool failure");
    assert_eq!(
        failure.attributes.get("failure_code"),
        Some(&"disallowed_evidence_kind".to_owned())
    );
    assert_eq!(
        failure.attributes.get("reported_evidence_kind"),
        Some(&"runtime_behavior".to_owned())
    );
    assert!(
        bundle
            .evidence
            .iter()
            .all(|record| record.evidence_kind != EvidenceKind::RuntimeBehavior)
    );
}

#[test]
fn analyzer_failure_attribution_uses_the_configured_analyzer_identity() {
    let engine = AnalysisEngine::new(
        IngestionPolicy::default(),
        "foundation_policy_v1",
        "revision_test",
        vec![Box::new(SpoofingFailureAnalyzer)],
    )
    .expect("configured analyzer identifier is valid");
    let bundle = engine
        .analyze_bytes(&request(AnalysisProfile::StaticOnly), "sample.bin", b"abc")
        .expect("analyzer failure must remain an evidence bundle");

    let failure = bundle
        .evidence
        .iter()
        .find(|record| record.evidence_kind == EvidenceKind::ToolFailure)
        .expect("failure evidence must be present");
    assert_eq!(failure.producer_id, "configured_analyzer");
    assert_eq!(
        failure.attributes.get("reported_analyzer_id"),
        Some(&"spoofed_analyzer".to_owned())
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
fn deterministic_identifiers_change_when_identity_or_engine_inputs_change() {
    let engine = AnalysisEngine::default();
    let baseline = engine
        .analyze_bytes(&request(AnalysisProfile::StaticOnly), "sample.bin", b"abc")
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
        .analyze_bytes(&request(AnalysisProfile::StaticOnly), "sample.bin", b"abd")
        .expect("changed artifact analysis must succeed");
    assert_ne!(baseline.analysis_job_id, changed_artifact.analysis_job_id);

    for changed_engine in [
        AnalysisEngine::new(
            IngestionPolicy::default(),
            "changed_policy",
            "development",
            vec![Box::new(FormatAnalyzer)],
        )
        .expect("changed policy engine must be valid"),
        AnalysisEngine::new(
            IngestionPolicy::default(),
            "foundation_policy_v1",
            "changed_revision",
            vec![Box::new(FormatAnalyzer)],
        )
        .expect("changed revision engine must be valid"),
        AnalysisEngine::new(
            IngestionPolicy::default(),
            "foundation_policy_v1",
            "development",
            vec![Box::new(EmptyAnalyzer)],
        )
        .expect("changed analyzer engine must be valid"),
    ] {
        let changed = changed_engine
            .analyze_bytes(&request(AnalysisProfile::StaticOnly), "sample.bin", b"abc")
            .expect("changed engine analysis must succeed");
        assert_ne!(baseline.analysis_job_id, changed.analysis_job_id);
    }
}
