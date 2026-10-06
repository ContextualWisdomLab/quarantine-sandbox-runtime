//! Acceptance tests for offline artifact-binding verification (ADR 0012).
//!
//! Fixtures are synthetic bytes. No artifact is executed and no file, network
//! or container backend is used.

use proptest::prelude::*;
use quarantine_sandbox_runtime::{
    AnalysisEngine, AnalysisProfile, AnalysisRequest, ArtifactBindingError, ArtifactBindingOutcome,
    ArtifactKind, BoundedSourceContext, ContractError, EvidenceBundle, FieldBinding,
    verify_artifact_binding,
};

const PE_BYTES: &[u8] = b"MZ\x90\x00synthetic portable executable fixture";

fn request(profile: AnalysisProfile, original_file_name: Option<&str>) -> AnalysisRequest {
    AnalysisRequest {
        schema_version: "1.0.0".to_owned(),
        request_id: "binding_request_001".to_owned(),
        profile,
        bounded_source_context: original_file_name.map(|name| BoundedSourceContext {
            source_channel_code: None,
            original_file_name: Some(name.to_owned()),
            declared_media_type: None,
            host_artifact_reference: None,
            submitted_at: None,
        }),
    }
}

fn bundle_for(bytes: &[u8], original_file_name: Option<&str>) -> EvidenceBundle {
    AnalysisEngine::default()
        .analyze_bytes(
            &request(AnalysisProfile::StaticOnly, original_file_name),
            bytes,
        )
        .expect("synthetic fixture must analyze")
}

#[test]
fn engine_bundle_matches_the_bytes_it_describes() {
    let bundle = bundle_for(PE_BYTES, Some("sample.exe"));

    let report = verify_artifact_binding(&bundle, PE_BYTES).expect("valid bundle must verify");

    assert_eq!(report.outcome(), ArtifactBindingOutcome::Matched);
    assert!(report.is_matched());
    assert_eq!(report.artifact_sha256(), FieldBinding::Matched);
    assert_eq!(report.artifact_size_bytes(), FieldBinding::Matched);
    assert_eq!(report.artifact_kind(), FieldBinding::Matched);
    assert_eq!(
        report.computed_artifact_sha256(),
        bundle.artifact.artifact_sha256
    );
    assert_eq!(
        report.computed_artifact_size_bytes(),
        u64::try_from(PE_BYTES.len()).expect("fixture length fits u64")
    );
    assert_eq!(
        report.computed_artifact_kind(),
        ArtifactKind::PortableExecutable
    );
}

#[test]
fn one_changed_byte_mismatches_only_the_digest() {
    let bundle = bundle_for(PE_BYTES, None);
    let mut altered = PE_BYTES.to_vec();
    let last = altered.len() - 1;
    altered[last] ^= 0x01;

    let report = verify_artifact_binding(&bundle, &altered).expect("valid bundle must verify");

    assert_eq!(report.outcome(), ArtifactBindingOutcome::Mismatched);
    assert!(!report.is_matched());
    assert_eq!(report.artifact_sha256(), FieldBinding::Mismatched);
    assert_eq!(report.artifact_size_bytes(), FieldBinding::Matched);
    assert_eq!(report.artifact_kind(), FieldBinding::Matched);
    assert_ne!(
        report.computed_artifact_sha256(),
        bundle.artifact.artifact_sha256
    );
}

#[test]
fn truncated_bytes_mismatch_digest_and_size() {
    let bundle = bundle_for(PE_BYTES, None);

    let report =
        verify_artifact_binding(&bundle, &PE_BYTES[..8]).expect("valid bundle must verify");

    assert_eq!(report.outcome(), ArtifactBindingOutcome::Mismatched);
    assert_eq!(report.artifact_sha256(), FieldBinding::Mismatched);
    assert_eq!(report.artifact_size_bytes(), FieldBinding::Mismatched);
    assert_eq!(report.computed_artifact_size_bytes(), 8);
}

#[test]
fn edited_descriptor_digest_is_detected() {
    let mut bundle = bundle_for(PE_BYTES, None);
    bundle.artifact.artifact_sha256 = "0".repeat(64);

    let report = verify_artifact_binding(&bundle, PE_BYTES).expect("shape is still valid");

    assert_eq!(report.outcome(), ArtifactBindingOutcome::Mismatched);
    assert_eq!(report.artifact_sha256(), FieldBinding::Mismatched);
    assert_eq!(report.artifact_size_bytes(), FieldBinding::Matched);
    assert_eq!(report.artifact_kind(), FieldBinding::Matched);
}

#[test]
fn edited_descriptor_size_is_detected() {
    let mut bundle = bundle_for(PE_BYTES, None);
    bundle.artifact.artifact_size_bytes += 1;

    let report = verify_artifact_binding(&bundle, PE_BYTES).expect("shape is still valid");

    assert_eq!(report.outcome(), ArtifactBindingOutcome::Mismatched);
    assert_eq!(report.artifact_sha256(), FieldBinding::Matched);
    assert_eq!(report.artifact_size_bytes(), FieldBinding::Mismatched);
}

#[test]
fn edited_descriptor_kind_is_detected() {
    let mut bundle = bundle_for(PE_BYTES, None);
    bundle.artifact.artifact_kind = ArtifactKind::Text;

    let report = verify_artifact_binding(&bundle, PE_BYTES).expect("shape is still valid");

    assert_eq!(report.outcome(), ArtifactBindingOutcome::Mismatched);
    assert_eq!(report.artifact_sha256(), FieldBinding::Matched);
    assert_eq!(report.artifact_kind(), FieldBinding::Mismatched);
    assert_eq!(
        report.computed_artifact_kind(),
        ArtifactKind::PortableExecutable
    );
}

#[test]
fn kind_derivation_uses_the_descriptor_file_name_like_ingestion() {
    let script = b"print('synthetic')\n";
    let bundle = bundle_for(script, Some("tool.py"));
    assert_eq!(bundle.artifact.artifact_kind, ArtifactKind::Script);

    let report = verify_artifact_binding(&bundle, script).expect("valid bundle must verify");
    assert_eq!(report.outcome(), ArtifactBindingOutcome::Matched);

    let mut renamed = bundle.clone();
    renamed.artifact.original_file_name = None;
    let report = verify_artifact_binding(&renamed, script).expect("shape is still valid");
    assert_eq!(report.artifact_kind(), FieldBinding::Mismatched);
    assert_eq!(report.computed_artifact_kind(), ArtifactKind::Text);
    assert_eq!(report.outcome(), ArtifactBindingOutcome::Mismatched);
}

#[test]
fn empty_bytes_are_rejected() {
    let bundle = bundle_for(PE_BYTES, None);

    assert_eq!(
        verify_artifact_binding(&bundle, b""),
        Err(ArtifactBindingError::EmptyArtifact)
    );
}

#[test]
fn contract_violations_are_rejected_before_any_comparison() {
    let mut bundle = bundle_for(PE_BYTES, None);
    bundle.consumer_verdict_required = false;
    assert_eq!(
        verify_artifact_binding(&bundle, PE_BYTES),
        Err(ArtifactBindingError::Contract(
            ContractError::ConsumerVerdictMustBeRequired
        ))
    );
    assert_eq!(
        verify_artifact_binding(&bundle, b""),
        Err(ArtifactBindingError::Contract(
            ContractError::ConsumerVerdictMustBeRequired
        )),
        "contract validation runs before the empty-input check"
    );

    let mut bundle = bundle_for(PE_BYTES, None);
    bundle.artifact.artifact_sha256 = bundle.artifact.artifact_sha256.to_uppercase();
    assert_eq!(
        verify_artifact_binding(&bundle, PE_BYTES),
        Err(ArtifactBindingError::Contract(ContractError::InvalidSha256))
    );

    let mut bundle = bundle_for(PE_BYTES, None);
    bundle.runtime.dynamic_execution_performed = true;
    assert_eq!(
        verify_artifact_binding(&bundle, PE_BYTES),
        Err(ArtifactBindingError::Contract(
            ContractError::RuntimeBoundaryViolated {
                boundary_name: "dynamic_execution_performed"
            }
        ))
    );
}

#[test]
fn verification_never_upgrades_an_inconclusive_bundle() {
    let bundle = AnalysisEngine::default()
        .analyze_bytes(&request(AnalysisProfile::LinuxDynamic, None), PE_BYTES)
        .expect("dynamic request still yields a bundle");
    let before = bundle.clone();

    let report = verify_artifact_binding(&bundle, PE_BYTES).expect("valid bundle must verify");

    assert_eq!(report.outcome(), ArtifactBindingOutcome::Matched);
    assert_eq!(bundle, before, "verification must not mutate the bundle");
    assert!(
        bundle
            .limitations
            .iter()
            .any(|limitation| limitation == "dynamic_analysis_not_configured")
    );
}

#[test]
fn report_serializes_closed_snake_case_wire_values_without_verdict_fields() {
    let bundle = bundle_for(PE_BYTES, None);
    let mut altered = PE_BYTES.to_vec();
    altered.push(b'!');
    let report = verify_artifact_binding(&bundle, &altered).expect("valid bundle must verify");

    let value = serde_json::to_value(&report).expect("report must serialize");
    let object = value.as_object().expect("report is a JSON object");
    let mut keys = object.keys().map(String::as_str).collect::<Vec<_>>();
    keys.sort_unstable();
    assert_eq!(
        keys,
        [
            "artifact_kind",
            "artifact_sha256",
            "artifact_size_bytes",
            "computed_artifact_kind",
            "computed_artifact_sha256",
            "computed_artifact_size_bytes",
            "outcome",
        ]
    );
    assert_eq!(object["outcome"], "mismatched");
    assert_eq!(object["artifact_sha256"], "mismatched");
    assert_eq!(object["artifact_size_bytes"], "mismatched");
    assert_eq!(object["artifact_kind"], "matched");
    assert_eq!(object["computed_artifact_kind"], "portable_executable");
    assert_eq!(
        object["computed_artifact_size_bytes"],
        u64::try_from(altered.len()).expect("fixture length fits u64")
    );
}

#[test]
fn wire_codes_are_stable() {
    assert_eq!(ArtifactBindingOutcome::Matched.as_str(), "matched");
    assert_eq!(ArtifactBindingOutcome::Mismatched.as_str(), "mismatched");
    assert_eq!(FieldBinding::Matched.as_str(), "matched");
    assert_eq!(FieldBinding::Mismatched.as_str(), "mismatched");
    assert_eq!(
        ArtifactBindingError::EmptyArtifact.to_string(),
        "artifact bytes must not be empty"
    );
    assert_eq!(
        ArtifactBindingError::Contract(ContractError::InvalidSha256).to_string(),
        ContractError::InvalidSha256.to_string()
    );
}

proptest! {
    #[test]
    fn engine_bundles_match_their_own_bytes_and_reject_other_bytes(
        bytes in prop::collection::vec(any::<u8>(), 1..=512),
        other in prop::collection::vec(any::<u8>(), 1..=512),
        name_index in 0usize..5,
    ) {
        let names = [None, Some("sample.bin"), Some("run.sh"), Some("notes.txt"), Some("tool.PY")];
        let bundle = bundle_for(&bytes, names[name_index]);

        let report = verify_artifact_binding(&bundle, &bytes)
            .expect("engine bundle must satisfy the contract");
        prop_assert_eq!(report.outcome(), ArtifactBindingOutcome::Matched);
        prop_assert_eq!(report.computed_artifact_kind(), bundle.artifact.artifact_kind);

        let other_report = verify_artifact_binding(&bundle, &other)
            .expect("engine bundle must satisfy the contract");
        prop_assert_eq!(other_report.is_matched(), bytes == other);
    }
}
