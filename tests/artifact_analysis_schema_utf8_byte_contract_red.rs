//! Causal RED for the published analysis-request UTF-8 byte-bound contract.
//!
//! Draft 2020-12 `maxLength` counts JSON-string characters. This witness stays
//! within that standard assertion while exceeding the runtime's documented
//! UTF-8 byte budget. An unknown `x-cwl-*` keyword must not be treated as if it
//! were a stock Draft 2020-12 assertion.

use std::{fs, path::Path};

use quarantine_sandbox_runtime::{AnalysisProfile, AnalysisRequest, ContractError};
use serde_json::Value;

const STOCK_DRAFT_2020_12_DIALECT: &str = "https://json-schema.org/draft/2020-12/schema";

fn analysis_request_schema() -> Value {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("schemas/analysis-request.schema.json");
    let text =
        fs::read_to_string(path).expect("published analysis-request schema must be readable");
    serde_json::from_str(&text).expect("published analysis-request schema must be valid JSON")
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

    let advertises_only_stock_dialect =
        schema["$schema"].as_str() == Some(STOCK_DRAFT_2020_12_DIALECT);
    assert!(
        !advertises_only_stock_dialect
            || !stock_request_id_assertions_accept(request_id_schema, &request.request_id),
        "stock Draft 2020-12 accepts the witness because maxLength counts characters; the published contract must either make the CWL byte vocabulary required/fail-closed or use another executable validation path that rejects the same wire value"
    );
}
