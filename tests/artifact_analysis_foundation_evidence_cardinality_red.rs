//! RED contract for foundation-record cardinality in v1 bundled-runtime receipts.

use quarantine_sandbox_runtime::{AnalysisEngine, AnalysisProfile, AnalysisRequest, EvidenceKind};

fn static_request() -> AnalysisRequest {
    AnalysisRequest {
        schema_version: "1.0.0".to_owned(),
        request_id: "foundation_evidence_cardinality_red".to_owned(),
        profile: AnalysisProfile::StaticOnly,
        bounded_source_context: None,
    }
}

fn canonicalize_record_identities(
    bundle: &mut quarantine_sandbox_runtime::EvidenceBundle,
) {
    let analysis_job_id = bundle.analysis_job_id.clone();
    for (index, record) in bundle.evidence.iter_mut().enumerate() {
        let sequence_number = index + 1;
        record.sequence_number = sequence_number;
        record.evidence_id = format!("{analysis_job_id}:evidence:{sequence_number:04}");
    }
}

fn foundation_index(
    bundle: &quarantine_sandbox_runtime::EvidenceBundle,
    evidence_kind: EvidenceKind,
) -> usize {
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
    matching_indices[0]
}

#[test]
fn v1_foundation_evidence_records_must_exist_exactly_once() {
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
        let foundation_index = foundation_index(&bundle, evidence_kind);

        let mut missing = bundle.clone();
        missing.evidence.remove(foundation_index);
        canonicalize_record_identities(&mut missing);
        assert!(
            missing.validate().is_err(),
            "bundle must reject missing {evidence_kind:?} foundation evidence"
        );

        let mut duplicated = bundle.clone();
        duplicated
            .evidence
            .push(bundle.evidence[foundation_index].clone());
        canonicalize_record_identities(&mut duplicated);
        assert!(
            duplicated.validate().is_err(),
            "bundle must reject a duplicated canonical {evidence_kind:?} foundation record"
        );
    }
}
