//! Branch-coverage regression tests for bounded source-context validation.

use quarantine_sandbox_runtime::{
    AnalysisProfile, AnalysisRequest, BoundedSourceContext, ContractError,
};

fn empty_context() -> BoundedSourceContext {
    BoundedSourceContext {
        source_channel_code: None,
        original_file_name: None,
        declared_media_type: None,
        host_artifact_reference: None,
        submitted_at: None,
    }
}

fn request(context: BoundedSourceContext) -> AnalysisRequest {
    AnalysisRequest {
        schema_version: "1.0.0".to_owned(),
        request_id: "request_source_context_coverage".to_owned(),
        profile: AnalysisProfile::StaticOnly,
        bounded_source_context: Some(context),
    }
}

#[test]
fn each_optional_context_field_is_independently_usable() {
    let mut original_name_only = empty_context();
    original_name_only.original_file_name = Some("sample.bin".to_owned());
    assert_eq!(request(original_name_only).validate(), Ok(()));

    let mut media_type_only = empty_context();
    media_type_only.declared_media_type = Some("text/plain;\tcharset=utf-8".to_owned());
    assert_eq!(request(media_type_only).validate(), Ok(()));

    let mut host_reference_only = empty_context();
    host_reference_only.host_artifact_reference = Some("artifact:1".to_owned());
    assert_eq!(request(host_reference_only).validate(), Ok(()));

    let mut submitted_at_only = empty_context();
    submitted_at_only.submitted_at = Some("2026-08-18T12:00:00Z".to_owned());
    assert_eq!(request(submitted_at_only).validate(), Ok(()));

    let mut numeric_channel = empty_context();
    numeric_channel.source_channel_code = Some("api_1".to_owned());
    assert_eq!(request(numeric_channel).validate(), Ok(()));
}

#[test]
fn media_type_and_host_reference_cover_all_bounded_rejection_paths() {
    let mut oversized_media_type = empty_context();
    oversized_media_type.declared_media_type = Some("a".repeat(256));
    assert_eq!(
        request(oversized_media_type).validate(),
        Err(ContractError::InvalidDeclaredMediaType)
    );

    let mut oversized_media_token = empty_context();
    oversized_media_token.declared_media_type = Some(format!("{}/plain", "a".repeat(128)));
    assert_eq!(
        request(oversized_media_token).validate(),
        Err(ContractError::InvalidDeclaredMediaType)
    );

    let mut non_ascii_host_reference = empty_context();
    non_ascii_host_reference.host_artifact_reference = Some("artifact_한글".to_owned());
    assert_eq!(
        request(non_ascii_host_reference).validate(),
        Err(ContractError::InvalidHostArtifactReference)
    );
}

#[test]
fn submitted_at_rejects_every_delimiter_and_numeric_component_boundary() {
    for invalid_timestamp in [
        "2026x08-18T12:00:00Z",
        "2026-08x18T12:00:00Z",
        "2026-08-18T12x00:00Z",
        "2026-08-18T12:00x00Z",
        "2026-0x-18T12:00:00Z",
        "2026-08-1xT12:00:00Z",
        "2026-08-18T1x:00:00Z",
        "2026-08-18T12:0x:00Z",
        "2026-08-18T12:00:0xZ",
        "2026-08-18T12:00:00AZ",
        "2026-08-18T12:00:00.1234567890Z",
        "1900-02-29T00:00:00Z",
    ] {
        let mut context = empty_context();
        context.submitted_at = Some(invalid_timestamp.to_owned());
        assert_eq!(
            request(context).validate(),
            Err(ContractError::InvalidSubmittedAt),
            "timestamp unexpectedly accepted: {invalid_timestamp}"
        );
    }

    let mut divisible_by_four_hundred = empty_context();
    divisible_by_four_hundred.submitted_at = Some("2000-02-29T00:00:00Z".to_owned());
    assert_eq!(request(divisible_by_four_hundred).validate(), Ok(()));
}
