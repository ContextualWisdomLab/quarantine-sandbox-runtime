//! RED contract for binding normalized evidence identifiers to one analysis job.

use quarantine_sandbox_runtime::{AnalysisEngine, AnalysisProfile, AnalysisRequest};

fn static_request() -> AnalysisRequest {
    AnalysisRequest {
        schema_version: "1.0.0".to_owned(),
        request_id: "evidence_id_binding_red".to_owned(),
        profile: AnalysisProfile::StaticOnly,
        bounded_source_context: None,
    }
}

#[test]
fn evidence_ids_must_be_bound_to_the_enclosing_analysis_job_and_sequence() {
    let engine = AnalysisEngine::default();
    let bundle = engine
        .analyze_bytes(&static_request(), b"evidence-id-binding-fixture")
        .expect("foundation static analysis must produce a valid control bundle");

    assert_eq!(bundle.validate(), Ok(()));
    assert!(
        bundle.evidence.len() >= 2,
        "the control bundle must contain at least two records to test duplicate identity"
    );

    let mut foreign_job_identity = bundle.clone();
    foreign_job_identity.evidence[0].evidence_id =
        "analysis_job_foreign:evidence:0001".to_owned();
    assert!(
        foreign_job_identity.validate().is_err(),
        "an evidence record must not validate with an identifier unrelated to the enclosing analysis_job_id"
    );

    let mut duplicate_identity = bundle.clone();
    let first_evidence_id = duplicate_identity.evidence[0].evidence_id.clone();
    duplicate_identity.evidence[1].evidence_id = first_evidence_id;
    assert!(
        duplicate_identity.validate().is_err(),
        "two different sequence positions must not validate with the same evidence_id"
    );
}
