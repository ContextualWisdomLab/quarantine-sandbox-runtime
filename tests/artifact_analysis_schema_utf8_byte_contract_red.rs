//! Regression contract for the published analysis-request UTF-8 byte bound.
//!
//! Draft 2020-12 `maxLength` counts JSON-string characters. The CWL dialect
//! therefore requires an explicit vocabulary for UTF-8 byte assertions so an
//! implementation that does not understand those semantics must fail closed.

use std::{fs, path::Path};

use quarantine_sandbox_runtime::{AnalysisProfile, AnalysisRequest, ContractError};
use serde_json::Value;

const STOCK_DRAFT_2020_12_DIALECT: &str = "https://json-schema.org/draft/2020-12/schema";
const CWL_DIALECT: &str = "https://contextualwisdomlab.org/schemas/quarantine/cwl-artifact-analysis-contract-dialect-1.0.0.schema.json";
const CWL_BYTE_VOCABULARY: &str =
    "https://contextualwisdomlab.org/vocab/quarantine-artifact-analysis-contract-1.0.0";

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

fn stock_request_id_assertions_accept(property: &Value, value: &str) -> bool {
    let minimum_characters = property["minLength"]
        .as_u64()
        .expect("request_id minLength must be an integer") as usize;
    let maximum_characters = property["maxLength"]
        .as_u64()
        .expect("request_id maxLength must be an integer") as usize;
    let character_count = value.chars().count();

    character_count >= minimum_characters
        && character_count <= maximum_characters
        && !value
            .chars()
            .any(|character| matches!(character as u32, 0x00..=0x1f | 0x7f..=0x9f))
}

#[test]
fn published_schema_does_not_silently_accept_request_id_beyond_runtime_byte_bound() {
    let schema = analysis_request_schema();
    let request_id_schema = &schema["properties"]["request_id"];
    let maximum_utf8_bytes = request_id_schema["x-cwl-maxUtf8Bytes"]
        .as_u64()
        .expect("request_id must publish its UTF-8 byte bound")
        as usize;

    let request_id = "é".repeat(65);
    assert_eq!(request_id.chars().count(), 65);
    assert_eq!(request_id.len(), 130);
    assert!(request_id.len() > maximum_utf8_bytes);
    assert!(stock_request_id_assertions_accept(
        request_id_schema,
        &request_id
    ));

    let request = AnalysisRequest {
        schema_version: "1.0.0".to_owned(),
        request_id,
        profile: AnalysisProfile::StaticOnly,
        bounded_source_context: None,
    };
    assert_eq!(
        request.validate(),
        Err(ContractError::FieldTooLong {
            field_name: "request_id",
            maximum_bytes: 128,
        })
    );

    assert_eq!(schema["$schema"].as_str(), Some(CWL_DIALECT));
    assert_ne!(
        schema["$schema"].as_str(),
        Some(STOCK_DRAFT_2020_12_DIALECT)
    );

    let dialect = cwl_contract_dialect();
    assert_eq!(
        dialect["$schema"].as_str(),
        Some(STOCK_DRAFT_2020_12_DIALECT),
        "the CWL dialect itself must be defined using the stock Draft 2020-12 meta-schema"
    );
    assert_eq!(
        dialect["$vocabulary"][CWL_BYTE_VOCABULARY].as_bool(),
        Some(true),
        "UTF-8 byte semantics must be a required vocabulary so unsupported validators fail closed"
    );
}
