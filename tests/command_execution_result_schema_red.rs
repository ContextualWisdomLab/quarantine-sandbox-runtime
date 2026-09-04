//! RED contract for the command-execution result JSON Schema.

use std::collections::BTreeSet;

use serde_json::{Map, Value};

fn command_result_schema() -> Value {
    serde_json::from_str(include_str!(
        "../schemas/command-execution-result.schema.json"
    ))
    .expect("checked-in command result schema must be valid JSON")
}

fn receipt_object_schema(receipt_schema: &Value) -> &Map<String, Value> {
    if receipt_schema.get("type").and_then(Value::as_str) == Some("object") {
        return receipt_schema
            .as_object()
            .expect("an object schema must be represented as a JSON object");
    }

    if let Some(types) = receipt_schema.get("type").and_then(Value::as_array) {
        let admitted: BTreeSet<_> = types.iter().filter_map(Value::as_str).collect();
        assert_eq!(
            admitted,
            BTreeSet::from(["null", "object"]),
            "source_artifact_receipt must admit exactly null or object"
        );
        return receipt_schema
            .as_object()
            .expect("a typed receipt schema must be a JSON object");
    }

    let variants = receipt_schema
        .get("oneOf")
        .and_then(Value::as_array)
        .expect("source_artifact_receipt must admit null and object via type or oneOf");
    let null_variants = variants
        .iter()
        .filter(|variant| variant.get("type").and_then(Value::as_str) == Some("null"))
        .count();
    assert_eq!(null_variants, 1, "receipt schema must admit exactly one null variant");
    variants
        .iter()
        .find(|variant| variant.get("type").and_then(Value::as_str) == Some("object"))
        .and_then(Value::as_object)
        .expect("receipt schema must admit an object variant")
}

#[test]
fn command_result_schema_declares_optional_strict_source_artifact_receipt() {
    let schema = command_result_schema();
    assert_eq!(schema.get("additionalProperties"), Some(&Value::Bool(false)));

    let properties = schema
        .get("properties")
        .and_then(Value::as_object)
        .expect("command result schema must declare properties");
    let receipt_schema = properties
        .get("source_artifact_receipt")
        .expect("serialized CommandExecutionResult.source_artifact_receipt must be declared");

    let top_level_required: BTreeSet<_> = schema
        .get("required")
        .and_then(Value::as_array)
        .expect("command result schema must declare required fields")
        .iter()
        .filter_map(Value::as_str)
        .collect();
    assert!(
        !top_level_required.contains("source_artifact_receipt"),
        "the Option receipt remains optional at the top level"
    );

    let object_schema = receipt_object_schema(receipt_schema);
    assert_eq!(
        object_schema.get("additionalProperties"),
        Some(&Value::Bool(false)),
        "a populated receipt must reject undeclared members"
    );

    let expected_fields = BTreeSet::from([
        "executable_files_stripped",
        "mounted_noexec",
        "mounted_read_only",
        "regular_file_count",
        "revision_sha",
        "total_bytes",
        "tree_sha256",
    ]);
    let receipt_properties: BTreeSet<_> = object_schema
        .get("properties")
        .and_then(Value::as_object)
        .expect("receipt object must declare properties")
        .keys()
        .map(String::as_str)
        .collect();
    assert_eq!(receipt_properties, expected_fields);

    let receipt_required: BTreeSet<_> = object_schema
        .get("required")
        .and_then(Value::as_array)
        .expect("receipt object must require its serialized fields")
        .iter()
        .filter_map(Value::as_str)
        .collect();
    assert_eq!(receipt_required, expected_fields);
}
