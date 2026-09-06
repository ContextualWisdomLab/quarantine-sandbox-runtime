//! RED contract for requiring exactly one runtime-owned foundation evidence record per kind.

use quarantine_sandbox_runtime::{AnalysisEngine, AnalysisProfile, AnalysisRequest, EvidenceKind};

fn static_request() -> AnalysisRequest {
    AnalysisRequest {
        schema_version: "1.0.0".to_owned(),
        request_id: "foundation_evidence_cardinality_red".to_owned(),
        profile: AnalysisProfile::StaticOnly,
        bounded_source_context: None,
    }
}

#[test]
fn runtime_owned_foundation_evidence_must_exist_exactly_once() {
    let engine = AnalysisEngine::default();
    let bundle = engine
        .analyze_bytes(
            &static_request(),
            b"foundation-evidence-cardinality-fixture",
        )
        .expect("foundation static analysis must produce a valid control bundle");

    assert_eq!(bundle.validate(), Ok(()));

    for evidence_kind in [
        EvidenceKind::ArtifactIdentity,
        EvidenceKind::FileFormat,
        EvidenceKind::PolicyBoundary,
    ] {
        let matching_indices: Vec<_> = bundle
            .evidence
            .iter()
            .enumerate()
            .filter_map(|(index, record)| {
                (record.evidence_kind == evidence_kind).then_some(index)
            })
            .collect();
        assert_eq!(
            matching_indices.len(),
            1,
            "control bundle must contain exactly one {evidence_kind:?} foundation record"
        );

        let foundation_index = matching_indices[0];

        let mut missing = bundle.clone();
        missing.evidence.remove(foundation_index);
        let missing_job_id = missing.analysis_job_id.clone();
        for (index, record) in missing.evidence.iter_mut().enumerate() {
            let sequence_number = index + 1;
            record.sequence_number = sequence_number;
            record.evidence_id = format!("{missing_job_id}:evidence:{sequence_number:04}");
        }
        assert!(
            missing.validate().is_err(),
            "bundle must reject missing {evidence_kind:?} foundation evidence"
        );

        let mut duplicated = bundle.clone();
        duplicated
            .evidence
            .push(bundle.evidence[foundation_index].clone());
        let duplicated_job_id = duplicated.analysis_job_id.clone();
        for (index, record) in duplicated.evidence.iter_mut().enumerate() {
            let sequence_number = index + 1;
            record.sequence_number = sequence_number;
            record.evidence_id = format!("{duplicated_job_id}:evidence:{sequence_number:04}");
        }
        assert!(
            duplicated.validate().is_err(),
            "bundle must reject duplicate {evidence_kind:?} foundation evidence"
        );
    }
}
