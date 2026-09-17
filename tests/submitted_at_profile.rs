//! RFC 3339 profile regression tests for source submission timestamps.

use quarantine_sandbox_runtime::{
    AnalysisProfile, AnalysisRequest, BoundedSourceContext, ContractError,
};

fn request(submitted_at: &str) -> AnalysisRequest {
    AnalysisRequest {
        schema_version: "1.0.0".to_owned(),
        request_id: "request_timestamp_profile".to_owned(),
        profile: AnalysisProfile::StaticOnly,
        bounded_source_context: Some(BoundedSourceContext {
            source_channel_code: None,
            original_file_name: None,
            declared_media_type: None,
            host_artifact_reference: None,
            submitted_at: Some(submitted_at.to_owned()),
        }),
    }
}

#[test]
fn deterministic_utc_profile_rejects_unverified_leap_second_notation() {
    assert_eq!(
        request("2024-02-29T23:59:60Z").validate(),
        Err(ContractError::InvalidSubmittedAt)
    );
    assert_eq!(request("2024-02-29T23:59:59Z").validate(), Ok(()));
}
