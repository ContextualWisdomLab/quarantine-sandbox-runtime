//! Edge-case conformance RED for the required CWL RFC 3339 profile.
//!
//! The human-readable vocabulary already requires Gregorian month/day bounds,
//! the 4/100/400 leap-year rule, and an upper-case UTC `Z`. This witness keeps
//! those independent semantic branches executable so a partial implementation
//! cannot claim vocabulary 1.0.0 conformance from the smaller happy-path set.

use std::{fs, path::Path};

use quarantine_sandbox_runtime::{
    AnalysisProfile, AnalysisRequest, BoundedSourceContext, ContractError,
};
use serde_json::Value;

const CWL_CONTRACT_VOCABULARY: &str =
    "https://contextualwisdomlab.org/vocab/quarantine-artifact-analysis-contract-1.0.0";
const CWL_RFC3339_PROFILE: &str =
    "utc_z_only_with_gregorian_day_validation_no_leap_second_notation";
const CONFORMANCE_VECTORS_PATH: &str =
    "tests/fixtures/cwl_artifact_analysis_contract_vocabulary_1_0_0_vectors.json";

fn request_with_submitted_at(value: &str) -> AnalysisRequest {
    AnalysisRequest {
        schema_version: "1.0.0".to_owned(),
        request_id: format!("rfc3339-edge-{value}"),
        profile: AnalysisProfile::StaticOnly,
        bounded_source_context: Some(BoundedSourceContext {
            source_channel_code: Some("direct_api".to_owned()),
            original_file_name: None,
            declared_media_type: None,
            host_artifact_reference: None,
            submitted_at: Some(value.to_owned()),
        }),
    }
}

fn published_vectors() -> Value {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join(CONFORMANCE_VECTORS_PATH);
    let text = fs::read_to_string(path).expect("vocabulary conformance vectors must be readable");
    serde_json::from_str(&text).expect("vocabulary conformance vectors must be valid JSON")
}

fn vector_by_id<'a>(vectors: &'a Value, vector_id: &str) -> &'a Value {
    vectors["cases"]
        .as_array()
        .expect("vocabulary conformance publication must contain a cases array")
        .iter()
        .find(|case| case["id"].as_str() == Some(vector_id))
        .unwrap_or_else(|| panic!("missing required Gregorian edge vector {vector_id}"))
}

fn assert_profile_vector(vectors: &Value, vector_id: &str, instance: &str, valid: bool) {
    let vector = vector_by_id(vectors, vector_id);
    assert_eq!(vector["keyword"].as_str(), Some("x-cwl-rfc3339Profile"));
    assert_eq!(vector["keyword_value"].as_str(), Some(CWL_RFC3339_PROFILE));
    assert_eq!(vector["instance"].as_str(), Some(instance));
    assert_eq!(vector["valid"].as_bool(), Some(valid));
}

#[test]
fn runtime_profile_covers_gregorian_century_month_length_and_uppercase_z_edges() {
    assert_eq!(
        request_with_submitted_at("1900-02-29T00:00:00Z").validate(),
        Err(ContractError::InvalidSubmittedAt),
        "a century year not divisible by 400 is not a leap year"
    );
    assert!(
        request_with_submitted_at("2000-02-29T00:00:00Z")
            .validate()
            .is_ok(),
        "a century year divisible by 400 remains a leap year"
    );
    assert_eq!(
        request_with_submitted_at("2026-04-31T00:00:00Z").validate(),
        Err(ContractError::InvalidSubmittedAt),
        "April has only 30 days"
    );
    assert_eq!(
        request_with_submitted_at("2024-02-29T23:59:59z").validate(),
        Err(ContractError::InvalidSubmittedAt),
        "the CWL profile requires upper-case Z"
    );
}

#[test]
fn published_profile_vectors_cover_independent_gregorian_edge_branches() {
    let vectors = published_vectors();
    assert_eq!(
        vectors["vocabulary_id"].as_str(),
        Some(CWL_CONTRACT_VOCABULARY),
        "edge vectors must bind the exact required vocabulary identity"
    );

    assert_profile_vector(
        &vectors,
        "rfc3339_profile_century_non_400_february_29_invalid",
        "1900-02-29T00:00:00Z",
        false,
    );
    assert_profile_vector(
        &vectors,
        "rfc3339_profile_century_400_february_29_valid",
        "2000-02-29T00:00:00Z",
        true,
    );
    assert_profile_vector(
        &vectors,
        "rfc3339_profile_thirty_day_month_31_invalid",
        "2026-04-31T00:00:00Z",
        false,
    );
    assert_profile_vector(
        &vectors,
        "rfc3339_profile_lowercase_z_invalid",
        "2024-02-29T23:59:59z",
        false,
    );
}
