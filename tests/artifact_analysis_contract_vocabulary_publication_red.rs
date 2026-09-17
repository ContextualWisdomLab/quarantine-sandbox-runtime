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

    let vectors: Value = serde_json::from_str(&read_text(CONFORMANCE_VECTORS_PATH))
        .expect("vocabulary conformance vectors must be valid JSON");
    assert_eq!(
        vectors["vocabulary_id"].as_str(),
        Some(CWL_CONTRACT_VOCABULARY),
        "conformance vectors must bind the exact required vocabulary identity"
    );

    let multibyte = vector_by_id(&vectors, "max_utf8_bytes_multibyte_overflow");
    assert_eq!(
        multibyte["keyword"].as_str(),
        Some("x-cwl-maxUtf8Bytes")
    );
    assert_eq!(multibyte["keyword_value"].as_u64(), Some(128));
    let multibyte_instance = multibyte["instance"]
        .as_str()
        .expect("UTF-8 overflow vector must contain a string instance");
    assert_eq!(multibyte_instance.chars().count(), 65);
    assert_eq!(multibyte_instance.len(), 130);
    assert_eq!(multibyte["valid"].as_bool(), Some(false));

    let serialized = vector_by_id(
        &vectors,
        "max_serialized_utf8_bytes_missing_nullable_normalization",
    );
    assert_eq!(
        serialized["keyword"].as_str(),
        Some("x-cwl-maxSerializedUtf8Bytes")
    );
    assert_eq!(serialized["keyword_value"].as_u64(), Some(1_024));

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

    let instance_bytes = serde_json::to_vec(instance)
        .expect("serialized-byte source instance must be serializable")
        .len();
    assert_eq!(
        instance_bytes, 1_005,
        "the source JSON data model fits before CWL domain normalization"
    );

    let canonical_instance = canonical_bounded_source_context(instance);
    let canonical_bytes = serde_json::to_vec(&canonical_instance)
        .expect("canonical bounded source context must be serializable")
        .len();
    assert_eq!(
        canonical_bytes, 1_025,
        "CWL normalization must materialize the omitted nullable property before measuring bytes"
    );
    assert!(canonical_bytes > 1_024);
    assert_eq!(serialized["valid"].as_bool(), Some(false));
}
