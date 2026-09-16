//! RED contract for binding normalized evidence identifiers to one analysis job.

use quarantine_sandbox_runtime::{
    AnalysisEngine, AnalysisProfile, AnalysisRequest, EvidenceBundle,
};

fn static_request() -> AnalysisRequest {
    AnalysisRequest {
        schema_version: "1.0.0".to_owned(),
        request_id: "evidence_id_binding_red".to_owned(),
        profile: AnalysisProfile::StaticOnly,
        bounded_source_context: None,
    }
}

fn control_bundle() -> EvidenceBundle {
    let bundle = AnalysisEngine::default()
        .analyze_bytes(&static_request(), b"evidence-id-binding-fixture")
        .expect("foundation static analysis must produce a valid control bundle");

    assert_eq!(bundle.validate(), Ok(()));
    assert!(
        bundle.evidence.len() >= 2,
        "the control bundle must contain at least two records to test identity binding"
    );
    bundle
}

#[test]
fn emitted_evidence_ids_match_the_documented_job_and_sequence_identity() {
    let bundle = control_bundle();

    for record in &bundle.evidence {
        assert_eq!(
            record.evidence_id,
            format!(
                "{}:evidence:{:04}",
                bundle.analysis_job_id, record.sequence_number
            ),
            "runtime assembly must keep the existing deterministic public evidence identity form"
        );
    }
}

#[test]
fn foreign_job_evidence_identifier_fails_closed() {
    let mut bundle = control_bundle();
    bundle.evidence[0].evidence_id = "analysis_job_foreign:evidence:0001".to_owned();

    assert!(
        bundle.validate().is_err(),
        "an evidence record must not validate with an identifier unrelated to the enclosing analysis_job_id"
    );
}

#[test]
fn duplicate_sequence_evidence_identifier_fails_closed() {
    let mut bundle = control_bundle();
    let first_evidence_id = bundle.evidence[0].evidence_id.clone();
    bundle.evidence[1].evidence_id = first_evidence_id;

    assert!(
        bundle.validate().is_err(),
        "two different sequence positions must not validate with the same evidence_id"
    );
}
