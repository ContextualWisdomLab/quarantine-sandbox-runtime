//! RED contract for matching bounded source-context schema admission to the Rust invariant.

use std::collections::BTreeSet;

use serde_json::Value;

const ANALYSIS_REQUEST_SCHEMA: &str = include_str!("../schemas/analysis-request.schema.json");
const CONTEXT_FIELDS: [&str; 5] = [
    "source_channel_code",
    "original_file_name",
    "declared_media_type",
    "host_artifact_reference",
    "submitted_at",
];

fn schema() -> Value {
    serde_json::from_str(ANALYSIS_REQUEST_SCHEMA).expect("analysis request schema must be JSON")
}

#[test]
fn bounded_source_context_wire_fields_remain_individually_nullable() {
    let schema = schema();
    let context = &schema["properties"]["bounded_source_context"];

    assert_eq!(context["type"], serde_json::json!(["object", "null"]));
    for field_name in CONTEXT_FIELDS {
        assert_eq!(
            context["properties"][field_name]["type"],
            serde_json::json!(["string", "null"]),
            "{field_name} must remain nullable when another context field carries the value"
        );
    }
}

#[test]
fn published_schema_requires_one_supported_context_field_to_be_a_string() {
    let schema = schema();
    let context = &schema["properties"]["bounded_source_context"];
    let any_of = context["anyOf"]
        .as_array()
        .expect("bounded source context must reject present all-null objects with anyOf");

    assert_eq!(
        any_of.len(),
        CONTEXT_FIELDS.len(),
        "every supported context field must be able to satisfy the non-empty invariant"
    );

    let expected: BTreeSet<_> = CONTEXT_FIELDS.into_iter().collect();
    let mut covered = BTreeSet::new();

    for branch in any_of {
        let required = branch["required"]
            .as_array()
            .expect("each non-empty branch must require one supported field");
        assert_eq!(
            required.len(),
            1,
            "each branch must cover exactly one field"
        );
        let field_name = required[0]
            .as_str()
            .expect("required field name must be a string");
        assert!(
            expected.contains(field_name),
            "anyOf must not introduce an unknown context field"
        );
        assert_eq!(
            branch["properties"][field_name]["type"], "string",
            "a required-but-null field must not satisfy the non-empty invariant"
        );
        assert!(
            covered.insert(field_name),
            "each supported context field needs one canonical branch"
        );
    }

    assert_eq!(covered, expected);
}
