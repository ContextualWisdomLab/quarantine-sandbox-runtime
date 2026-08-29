//! Regression tests for the published bounded source-context contract.

use quarantine_sandbox_runtime::{
    AnalysisEngine, AnalysisProfile, AnalysisRequest, ArtifactKind, BoundedSourceContext,
    ContractError,
};

fn valid_context() -> BoundedSourceContext {
    BoundedSourceContext {
        source_channel_code: Some("direct_api".to_owned()),
        original_file_name: Some("sample.py".to_owned()),
        declared_media_type: Some("text/x-python".to_owned()),
        host_artifact_reference: Some("artifact_01K2M7W5V5K3".to_owned()),
        submitted_at: Some("2026-08-18T12:00:00Z".to_owned()),
    }
}

fn request(context: Option<BoundedSourceContext>) -> AnalysisRequest {
    AnalysisRequest {
        schema_version: "1.0.0".to_owned(),
        request_id: "request_bounded_context".to_owned(),
        profile: AnalysisProfile::StaticOnly,
        bounded_source_context: context,
    }
}

#[test]
fn bounded_source_context_is_optional_closed_and_round_trippable() {
    let without_context = request(None);
    assert_eq!(without_context.validate(), Ok(()));

    let with_context = request(Some(valid_context()));
    assert_eq!(with_context.validate(), Ok(()));
    let encoded = serde_json::to_string(&with_context).expect("request serialization must succeed");
    let decoded: AnalysisRequest =
        serde_json::from_str(&encoded).expect("request deserialization must succeed");
    assert_eq!(decoded, with_context);

    let unknown_field = r#"{
      "schema_version":"1.0.0",
      "request_id":"request_unknown_field",
      "profile":"static_only",
      "bounded_source_context":{
        "source_channel_code":"direct_api",
        "free_form_note":"must fail closed"
      }
    }"#;
    assert!(serde_json::from_str::<AnalysisRequest>(unknown_field).is_err());
}

#[test]
fn bounded_source_context_rejects_empty_malformed_and_sensitive_shapes() {
    let mut empty = valid_context();
    empty.source_channel_code = None;
    empty.original_file_name = None;
    empty.declared_media_type = None;
    empty.host_artifact_reference = None;
    empty.submitted_at = None;
    assert_eq!(
        request(Some(empty)).validate(),
        Err(ContractError::EmptyBoundedSourceContext)
    );

    for invalid_channel in ["Email/Attachment", ""] {
        let mut context = valid_context();
        context.source_channel_code = Some(invalid_channel.to_owned());
        assert_eq!(
            request(Some(context)).validate(),
            Err(ContractError::InvalidSourceChannelCode)
        );
    }
    let mut long_channel = valid_context();
    long_channel.source_channel_code = Some("a".repeat(65));
    assert_eq!(
        request(Some(long_channel)).validate(),
        Err(ContractError::InvalidSourceChannelCode)
    );

    for invalid_name in [
        "../sample.py",
        "folder/sample.py",
        "folder\\sample.py",
        ".",
        "..",
        "",
        "bad\nname.py",
    ] {
        let mut context = valid_context();
        context.original_file_name = Some(invalid_name.to_owned());
        assert_eq!(
            request(Some(context)).validate(),
            Err(ContractError::InvalidOriginalFileName)
        );
    }
    let mut long_name = valid_context();
    long_name.original_file_name = Some("x".repeat(256));
    assert_eq!(
        request(Some(long_name)).validate(),
        Err(ContractError::InvalidOriginalFileName)
    );

    for invalid_media_type in [
        "not a media type",
        "",
        "/plain",
        "text/",
        "text/plain;",
        "text/pl ain",
    ] {
        let mut context = valid_context();
        context.declared_media_type = Some(invalid_media_type.to_owned());
        assert_eq!(
            request(Some(context)).validate(),
            Err(ContractError::InvalidDeclaredMediaType)
        );
    }
    let mut non_ascii_media_type = valid_context();
    non_ascii_media_type.declared_media_type = Some("text/한글".to_owned());
    assert_eq!(
        request(Some(non_ascii_media_type)).validate(),
        Err(ContractError::InvalidDeclaredMediaType)
    );

    for invalid_reference in [
        "person@example.com",
        "https://example.com/artifact/1",
        "github_pat_11AA_secret",
        "Bearer_secret",
        "",
    ] {
        let mut context = valid_context();
        context.host_artifact_reference = Some(invalid_reference.to_owned());
        assert_eq!(
            request(Some(context)).validate(),
            Err(ContractError::InvalidHostArtifactReference)
        );
    }
    let mut long_reference = valid_context();
    long_reference.host_artifact_reference = Some("a".repeat(129));
    assert_eq!(
        request(Some(long_reference)).validate(),
        Err(ContractError::InvalidHostArtifactReference)
    );

    for invalid_timestamp in [
        "2026-08-18T12:00:00+09:00",
        "2026-02-30T12:00:00Z",
        "2026-08-18 12:00:00Z",
        "2026-13-18T12:00:00Z",
        "2026-08-00T12:00:00Z",
        "2026-08-18T24:00:00Z",
        "2026-08-18T12:60:00Z",
        "2026-08-18T12:00:61Z",
        "2026-08-18T12:00:00.Z",
        "2026-08-18T12:00:00.AZ",
        "xxxx-08-18T12:00:00Z",
        "",
    ] {
        let mut context = valid_context();
        context.submitted_at = Some(invalid_timestamp.to_owned());
        assert_eq!(
            request(Some(context)).validate(),
            Err(ContractError::InvalidSubmittedAt)
        );
    }

    for valid_timestamp in [
        "2024-02-29T23:59:60Z",
        "2026-04-30T00:00:00Z",
        "2026-08-18T12:00:00.1Z",
        "2026-08-18T12:00:00.123456789Z",
    ] {
        let mut context = valid_context();
        context.submitted_at = Some(valid_timestamp.to_owned());
        assert_eq!(request(Some(context)).validate(), Ok(()));
    }
}

#[test]
fn aggregate_serialized_context_limit_is_enforced_after_field_validation() {
    let mut context = valid_context();
    context.source_channel_code = Some(format!("a{}", "b".repeat(62)));
    context.original_file_name = Some("\"".repeat(255));
    context.declared_media_type = Some(format!(
        "application/octet-stream;x=\"{}\"",
        "\\\"".repeat(45)
    ));
    context.host_artifact_reference = Some("a".repeat(128));
    context.submitted_at = Some("2026-08-18T12:00:00.123456789Z".to_owned());

    assert_eq!(
        request(Some(context)).validate(),
        Err(ContractError::BoundedSourceContextTooLarge {
            maximum_bytes: 1_024,
        })
    );
}

#[test]
fn runtime_uses_only_optional_context_filename_and_never_requires_it() {
    let engine = AnalysisEngine::default();

    let unnamed = engine
        .analyze_bytes(&request(None), b"plain text")
        .expect("artifact bytes must be analyzable without source context");
    assert_eq!(unnamed.artifact.original_file_name, None);
    assert_eq!(unnamed.artifact.artifact_kind, ArtifactKind::Text);

    let named = engine
        .analyze_bytes(&request(Some(valid_context())), b"print('ok')")
        .expect("bounded context filename must be accepted");
    assert_eq!(
        named.artifact.original_file_name.as_deref(),
        Some("sample.py")
    );
    assert_eq!(named.artifact.artifact_kind, ArtifactKind::Script);
}
