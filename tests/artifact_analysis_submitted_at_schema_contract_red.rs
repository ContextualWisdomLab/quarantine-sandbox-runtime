//! Regression contract for executable Gregorian `submitted_at` validation.
//!
//! Stock Draft 2020-12 uses `format` as an annotation by default. The public
//! request contract therefore needs a fail-closed validation authority for the
//! stricter CWL UTC/Gregorian profile already enforced by the Rust validator.

use std::{fs, path::Path};

use quarantine_sandbox_runtime::{
    AnalysisProfile, AnalysisRequest, BoundedSourceContext, ContractError,
};
use serde_json::Value;

const STOCK_DRAFT_2020_12_DIALECT: &str = "https://json-schema.org/draft/2020-12/schema";
const CWL_DIALECT: &str = "https://contextualwisdomlab.org/schemas/quarantine/cwl-artifact-analysis-contract-dialect-1.0.0.schema.json";
const CWL_CONTRACT_VOCABULARY: &str =
    "https://contextualwisdomlab.org/vocab/quarantine-artifact-analysis-contract-1.0.0";
const CWL_RFC3339_PROFILE: &str =
    "utc_z_only_with_gregorian_day_validation_no_leap_second_notation";

fn schema(path: &str) -> Value {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join(path);
    let text = fs::read_to_string(path).expect("published schema must be readable");
    serde_json::from_str(&text).expect("published schema must be valid JSON")
}

fn analysis_request_schema() -> Value {
    schema("schemas/analysis-request.schema.json")
}

fn cwl_contract_dialect() -> Value {
    schema("schemas/cwl-artifact-analysis-contract-dialect-1.0.0.schema.json")
}

fn request_with_submitted_at(value: &str) -> AnalysisRequest {
    AnalysisRequest {
        schema_version: "1.0.0".to_owned(),
        request_id: format!("submitted-at-{value}"),
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

#[test]
fn runtime_gregorian_profile_rejects_impossible_dates_and_accepts_leap_day() {
    assert_eq!(
        request_with_submitted_at("2026-02-31T00:00:00Z").validate(),
        Err(ContractError::InvalidSubmittedAt)
    );
    assert_eq!(
        request_with_submitted_at("2023-02-29T00:00:00Z").validate(),
        Err(ContractError::InvalidSubmittedAt)
    );
    assert!(request_with_submitted_at("2024-02-29T23:59:59Z")
        .validate()
        .is_ok());
}

#[test]
fn published_schema_requires_fail_closed_gregorian_validation_authority() {
    let request_schema = analysis_request_schema();
    let submitted_at =
        &request_schema["properties"]["bounded_source_context"]["properties"]["submitted_at"];

    assert_eq!(
        submitted_at["x-cwl-rfc3339Profile"].as_str(),
        Some(CWL_RFC3339_PROFILE),
        "the published field must name the same UTC/Gregorian profile as the runtime contract"
    );
    assert_eq!(submitted_at["format"].as_str(), Some("date-time"));
    assert!(
        submitted_at["pattern"]
            .as_str()
            .is_some_and(|pattern| pattern.contains("([0-2][0-9]|3[01])")),
        "the current structural regex admits day 01..31 independently of month/year"
    );

    assert_eq!(
        request_schema["$schema"].as_str(),
        Some(CWL_DIALECT),
        "analysis-request must adopt the canonical #101/#102 CWL dialect; an arbitrary non-stock URI would not prove fail-closed vocabulary semantics"
    );
    assert_ne!(
        request_schema["$schema"].as_str(),
        Some(STOCK_DRAFT_2020_12_DIALECT)
    );

    let dialect = cwl_contract_dialect();
    assert_eq!(
        dialect["$schema"].as_str(),
        Some(STOCK_DRAFT_2020_12_DIALECT),
        "the CWL dialect itself must be defined over stock Draft 2020-12"
    );
    assert_eq!(
        dialect["$vocabulary"][CWL_CONTRACT_VOCABULARY].as_bool(),
        Some(true),
        "the CWL contract vocabulary must be required so unsupported validators fail closed"
    );
    assert_eq!(
        dialect["properties"]["x-cwl-rfc3339Profile"]["type"].as_str(),
        Some("string"),
        "the canonical dialect must recognize the RFC3339 profile keyword owned by the required CWL vocabulary"
    );
}
