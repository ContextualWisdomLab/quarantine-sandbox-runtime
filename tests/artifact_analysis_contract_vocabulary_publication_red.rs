//! Release contract for the required CWL artifact-analysis vocabulary.
//!
//! A required JSON Schema vocabulary needs independently implementable keyword
//! semantics, not only meta-schema syntax. These publication artifacts stay
//! owner-local and are not a substitute for the runtime validator.

use std::{fs, path::Path};

use serde_json::Value;

const CWL_CONTRACT_VOCABULARY: &str =
    "https://contextualwisdomlab.org/vocab/quarantine-artifact-analysis-contract-1.0.0";
const VOCABULARY_SPEC_PATH: &str =
    "docs/contracts/cwl_artifact_analysis_contract_vocabulary_1_0_0.md";
const CONFORMANCE_VECTORS_PATH: &str =
    "tests/fixtures/cwl_artifact_analysis_contract_vocabulary_1_0_0_vectors.json";
const BOUNDED_SOURCE_CONTEXT_FIELDS: [&str; 5] = [
    "source_channel_code",
    "original_file_name",
    "declared_media_type",
    "host_artifact_reference",
    "submitted_at",
];
const REQUIRED_SPECIFICATION_CLAUSES: [&str; 6] = [
    "MUST count UTF-8 octets of the JSON string value",
    "MUST accept the instance if and only if the UTF-8 octet count is less than or equal to the keyword value",
    "MUST serialize the normalized instance as compact JSON encoded as UTF-8",
    "MUST materialize every missing nullable BoundedSourceContext property as JSON null before serialization",
    "MUST count the UTF-8 octets of that compact serialization",
    "MUST accept the instance if and only if the serialized UTF-8 octet count is less than or equal to the keyword value",
];

fn repository_path(relative_path: &str) -> std::path::PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join(relative_path)
}

fn read_text(relative_path: &str) -> String {
    fs::read_to_string(repository_path(relative_path)).unwrap_or_else(|error| {
        panic!(
            "required vocabulary publication artifact {relative_path} must be readable: {error}"
        )
    })
}

fn vector_by_id<'a>(vectors: &'a Value, vector_id: &str) -> &'a Value {
    vectors["cases"]
        .as_array()
        .expect("vocabulary conformance publication must contain a cases array")
        .iter()
        .find(|case| case["id"].as_str() == Some(vector_id))
        .unwrap_or_else(|| {
            panic!("missing required vocabulary conformance vector {vector_id}")
        })
}

fn canonical_bounded_source_context(instance: &Value) -> Value {
    let mut object = instance
        .as_object()
        .expect("serialized-byte vector instance must be a JSON object")
        .clone();
    for field_name in BOUNDED_SOURCE_CONTEXT_FIELDS {
        object.entry(field_name.to_owned()).or_insert(Value::Null);
    }
    Value::Object(object)
}

fn assert_utf8_vector(
    vectors: &Value,
    vector_id: &str,
    expected_characters: usize,
    expected_bytes: usize,
    expected_valid: bool,
) {
    let vector = vector_by_id(vectors, vector_id);
    assert_eq!(vector["keyword"].as_str(), Some("x-cwl-maxUtf8Bytes"));
    assert_eq!(vector["keyword_value"].as_u64(), Some(128));
    let instance = vector["instance"]
        .as_str()
        .expect("UTF-8 byte vector must contain a string instance");
    assert_eq!(instance.chars().count(), expected_characters);
    assert_eq!(instance.len(), expected_bytes);
    assert_eq!(vector["valid"].as_bool(), Some(expected_valid));
}

fn assert_serialized_vector(
    vectors: &Value,
    vector_id: &str,
    expected_raw_bytes: usize,
    expected_canonical_bytes: usize,
    expected_valid: bool,
) -> Value {
    let vector = vector_by_id(vectors, vector_id);
    assert_eq!(
        vector["keyword"].as_str(),
        Some("x-cwl-maxSerializedUtf8Bytes")
    );
    assert_eq!(vector["keyword_value"].as_u64(), Some(1_024));

    let instance = &vector["instance"];
    let raw_bytes = serde_json::to_vec(instance)
        .expect("serialized-byte source instance must be serializable")
        .len();
    assert_eq!(raw_bytes, expected_raw_bytes);

    let canonical_instance = canonical_bounded_source_context(instance);
    let canonical_bytes = serde_json::to_vec(&canonical_instance)
        .expect("canonical bounded source context must be serializable")
        .len();
    assert_eq!(canonical_bytes, expected_canonical_bytes);
    assert_eq!(vector["valid"].as_bool(), Some(expected_valid));

    canonical_instance
}

#[test]
fn required_cwl_vocabulary_publishes_semantics_and_conformance_vectors() {
    let specification = read_text(VOCABULARY_SPEC_PATH);
    for required_term in [
        CWL_CONTRACT_VOCABULARY,
        "x-cwl-maxUtf8Bytes",
        "x-cwl-maxSerializedUtf8Bytes",
    ] {
        assert!(
            specification.contains(required_term),
            "vocabulary specification must define {required_term}"
        );
    }
    for required_clause in REQUIRED_SPECIFICATION_CLAUSES {
        assert!(
            specification.contains(required_clause),
            "vocabulary specification must normatively define: {required_clause}"
        );
    }

    let vectors: Value = serde_json::from_str(&read_text(CONFORMANCE_VECTORS_PATH))
        .expect("vocabulary conformance vectors must be valid JSON");
    assert_eq!(
        vectors["vocabulary_id"].as_str(),
        Some(CWL_CONTRACT_VOCABULARY),
        "conformance vectors must bind the exact required vocabulary identity"
    );

    assert_utf8_vector(
        &vectors,
        "max_utf8_bytes_multibyte_boundary",
        64,
        128,
        true,
    );
    assert_utf8_vector(
        &vectors,
        "max_utf8_bytes_multibyte_overflow",
        65,
        130,
        false,
    );

    let boundary = assert_serialized_vector(
        &vectors,
        "max_serialized_utf8_bytes_normalized_boundary",
        1_004,
        1_024,
        true,
    );
    let boundary_object = boundary
        .as_object()
        .expect("serialized-byte boundary vector must normalize to an object");
    assert_eq!(
        boundary_object.get("submitted_at"),
        Some(&Value::Null),
        "boundary vector must prove missing nullable fields are materialized before counting"
    );

    let serialized = vector_by_id(
        &vectors,
        "max_serialized_utf8_bytes_missing_nullable_normalization",
    );
    let instance = &serialized["instance"];
    let instance_object = instance
        .as_object()
        .expect("serialized-byte vector instance must be a JSON object");
    assert_eq!(
        instance_object
            .get("source_channel_code")
            .and_then(Value::as_str)
            .map(str::len),
        Some(64)
    );
    let original_file_name = instance_object
        .get("original_file_name")
        .and_then(Value::as_str)
        .expect("normalization vector must contain original_file_name");
    assert_eq!(original_file_name.len(), 227);
    assert!(original_file_name.bytes().all(|byte| byte == b'"'));
    assert_eq!(
        instance_object
            .get("declared_media_type")
            .and_then(Value::as_str)
            .map(str::len),
        Some(255)
    );
    assert_eq!(
        instance_object
            .get("host_artifact_reference")
            .and_then(Value::as_str)
            .map(str::len),
        Some(128)
    );
    assert!(!instance_object.contains_key("submitted_at"));

    let canonical_instance = assert_serialized_vector(
        &vectors,
        "max_serialized_utf8_bytes_missing_nullable_normalization",
        1_005,
        1_025,
        false,
    );
    assert!(
        serde_json::to_vec(&canonical_instance)
            .expect("canonical overflow vector must be serializable")
            .len()
            > 1_024
    );
}
