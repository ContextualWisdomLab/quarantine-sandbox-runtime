//! RED contract for binding ArtifactIdentity evidence to the top-level subject.

use std::collections::BTreeMap;

use quarantine_sandbox_runtime::{
    AnalysisEngine, AnalysisProfile, AnalysisRequest, BoundedSourceContext, EvidenceBundle,
    EvidenceKind,
};

fn named_static_request(request_id: &str, original_file_name: &str) -> AnalysisRequest {
    AnalysisRequest {
        schema_version: "1.0.0".to_owned(),
        request_id: request_id.to_owned(),
        profile: AnalysisProfile::StaticOnly,
        bounded_source_context: Some(BoundedSourceContext {
            source_channel_code: None,
            original_file_name: Some(original_file_name.to_owned()),
            declared_media_type: None,
            host_artifact_reference: None,
            submitted_at: None,
        }),
    }
}

fn subject_controls() -> (EvidenceBundle, EvidenceBundle) {
    let engine = AnalysisEngine::default();
    let primary = engine
        .analyze_bytes(
            &named_static_request("artifact_identity_binding_red_primary", "primary.txt"),
            b"subject-binding-fixture",
        )
        .expect("foundation static analysis must produce a valid primary control bundle");
    let alternate = engine
        .analyze_bytes(
            &named_static_request("artifact_identity_binding_red_alternate", "alternate.pdf"),
            b"%PDF-1.7\nalternate-subject-binding-fixture",
        )
        .expect("foundation static analysis must produce a valid alternate control bundle");

    assert_eq!(primary.validate(), Ok(()));
    assert_eq!(alternate.validate(), Ok(()));
    (primary, alternate)
}

fn artifact_identity_attributes(bundle: &EvidenceBundle) -> BTreeMap<String, String> {
    bundle
        .evidence
        .iter()
        .find(|record| record.evidence_kind == EvidenceKind::ArtifactIdentity)
        .expect("engine-produced evidence must include ArtifactIdentity")
        .attributes
        .clone()
}

fn assert_subject_attribute_mismatch_rejected(attribute_name: &str) {
    let (mut primary, alternate) = subject_controls();
    let primary_attributes = artifact_identity_attributes(&primary);
    let alternate_attributes = artifact_identity_attributes(&alternate);
    let primary_value = primary_attributes
        .get(attribute_name)
        .expect("primary ArtifactIdentity must contain the selected subject attribute");
    let alternate_value = alternate_attributes
        .get(attribute_name)
        .expect("alternate ArtifactIdentity must contain the selected subject attribute");

    assert_ne!(
        primary_value, alternate_value,
        "hostile control must use a materially different valid {attribute_name} value"
    );

    let identity_record = primary
        .evidence
        .iter_mut()
        .find(|record| record.evidence_kind == EvidenceKind::ArtifactIdentity)
        .expect("engine-produced evidence must include ArtifactIdentity");
    let previous = identity_record
        .attributes
        .insert(attribute_name.to_owned(), alternate_value.clone());
    assert_eq!(previous.as_ref(), Some(primary_value));

    assert!(
        primary.validate().is_err(),
        "a receipt must not validate when ArtifactIdentity {attribute_name} contradicts the top-level ArtifactDescriptor"
    );
}

#[test]
fn artifact_identity_name_must_match_top_level_artifact_name() {
    assert_subject_attribute_mismatch_rejected("artifact_name");
}

#[test]
fn artifact_identity_original_name_must_match_top_level_original_file_name() {
    assert_subject_attribute_mismatch_rejected("original_file_name");
}

#[test]
fn artifact_identity_sha256_must_match_top_level_artifact_sha256() {
    assert_subject_attribute_mismatch_rejected("artifact_sha256");
}

#[test]
fn artifact_identity_size_must_match_top_level_artifact_size_bytes() {
    assert_subject_attribute_mismatch_rejected("artifact_size_bytes");
}

#[test]
fn artifact_identity_kind_must_match_top_level_artifact_kind() {
    assert_subject_attribute_mismatch_rejected("artifact_kind");
}
