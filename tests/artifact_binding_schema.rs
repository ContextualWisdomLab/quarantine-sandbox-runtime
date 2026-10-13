//! Structural checks of the packaged ArtifactBindingReport JSON Schema.
//! These tests are not a full Draft 2020-12 validator or authenticity checks.

use quarantine_sandbox_runtime::{
    AnalysisEngine, AnalysisProfile, AnalysisRequest, ArtifactKind, EvidenceBundle,
    verify_artifact_binding,
};
use serde_json::{Value, json};

const FIELDS: [&str; 7] = [
    "outcome",
    "artifact_sha256",
    "artifact_size_bytes",
    "artifact_kind",
    "computed_artifact_sha256",
    "computed_artifact_size_bytes",
    "computed_artifact_kind",
];
const COMPARISONS: [&str; 3] = ["artifact_sha256", "artifact_size_bytes", "artifact_kind"];
const KINDS: [ArtifactKind; 9] = [
    ArtifactKind::PortableExecutable,
    ArtifactKind::ElfExecutable,
    ArtifactKind::MachOExecutable,
    ArtifactKind::ZipArchive,
    ArtifactKind::PdfDocument,
    ArtifactKind::OleCompoundDocument,
    ArtifactKind::Script,
    ArtifactKind::Text,
    ArtifactKind::Unknown,
];

fn bundle_for(bytes: &[u8]) -> EvidenceBundle {
    AnalysisEngine::default()
        .analyze_bytes(
            &AnalysisRequest {
                schema_version: "1.0.0".to_owned(),
                request_id: "binding_schema_fixture".to_owned(),
                profile: AnalysisProfile::StaticOnly,
                bounded_source_context: None,
            },
            bytes,
        )
        .expect("synthetic bytes must analyze")
}

fn serialized_report() -> Value {
    let bytes = b"MZsynthetic schema fixture";
    serde_json::to_value(verify_artifact_binding(&bundle_for(bytes), bytes).unwrap()).unwrap()
}

// Evaluate only the schema keywords used by this contract. This is deliberately
// a structural acceptance harness, not a general Draft 2020-12 validator.
fn structurally_accepts(schema: &Value, value: &Value) -> bool {
    if let Some(kind) = schema.get("type") {
        let has_type = match kind.as_str().expect("single schema type") {
            "object" => value.is_object(),
            "string" => value.is_string(),
            "integer" => value.as_u64().is_some(),
            other => panic!("unsupported structural type: {other}"),
        };
        if !has_type {
            return false;
        }
    }
    if let Some(expected) = schema.get("const")
        && value != expected
    {
        return false;
    }
    if let Some(choices) = schema.get("enum")
        && !choices.as_array().unwrap().contains(value)
    {
        return false;
    }
    if let Some(object) = value.as_object() {
        if let Some(required) = schema.get("required")
            && !required
                .as_array()
                .unwrap()
                .iter()
                .all(|field| object.contains_key(field.as_str().expect("required field name")))
        {
            return false;
        }
        let properties = schema.get("properties").and_then(Value::as_object);
        for (field, child) in object {
            if let Some(property) = properties.and_then(|properties| properties.get(field)) {
                if !structurally_accepts(property, child) {
                    return false;
                }
            } else if schema.get("additionalProperties") == Some(&Value::Bool(false)) {
                return false;
            }
        }
    }
    if let Some(text) = value.as_str() {
        let length = u64::try_from(text.chars().count()).unwrap();
        if schema
            .get("minLength")
            .is_some_and(|minimum| length < minimum.as_u64().unwrap())
            || schema
                .get("maxLength")
                .is_some_and(|maximum| length > maximum.as_u64().unwrap())
        {
            return false;
        }
        if let Some(pattern) = schema.get("pattern") {
            assert_eq!(
                pattern, "^[0-9a-f]{64}$",
                "only the digest pattern is supported"
            );
            if text.len() != 64
                || !text
                    .bytes()
                    .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
            {
                return false;
            }
        }
    }
    if let Some(number) = value.as_u64()
        && (schema
            .get("minimum")
            .is_some_and(|minimum| number < minimum.as_u64().unwrap())
            || schema
                .get("maximum")
                .is_some_and(|maximum| number > maximum.as_u64().unwrap()))
    {
        return false;
    }
    if let Some(condition) = schema.get("if") {
        let branch = if structurally_accepts(condition, value) {
            "then"
        } else {
            "else"
        };
        if let Some(consequence) = schema.get(branch)
            && !structurally_accepts(consequence, value)
        {
            return false;
        }
    }
    true
}

fn packaged_schema() -> Value {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("schemas/artifact-binding-report.schema.json");
    let source =
        std::fs::read_to_string(path).expect("packaged ArtifactBindingReport schema must exist");
    serde_json::from_str(&source).expect("packaged schema must be valid JSON")
}

#[test]
fn repository_validator_tracks_the_report_schema() {
    let validator = std::fs::read_to_string(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("scripts/validate_repository.py"),
    )
    .expect("repository validator must exist");
    let schema_inventory = validator
        .split("SCHEMA_FILES = (")
        .nth(1)
        .expect("schema inventory")
        .split(")\n")
        .next()
        .expect("schema inventory closing delimiter");
    assert!(
        schema_inventory.contains("\"schemas/artifact-binding-report.schema.json\""),
        "report schema must be required and checked by the repository validator"
    );
}

#[test]
fn structural_schema_has_versioned_identity_and_object_type() {
    let schema = packaged_schema();
    assert_eq!(
        schema["$schema"],
        "https://json-schema.org/draft/2020-12/schema"
    );
    assert_eq!(
        schema["$id"],
        "https://contextualwisdomlab.org/schemas/quarantine/artifact-binding-report-1.0.0.schema.json"
    );
    assert_eq!(schema["type"], "object");
}

#[test]
fn structural_schema_outcome_is_matched_if_and_only_if_every_comparison_matches() {
    let schema = packaged_schema();
    assert!(
        schema.get("if").is_some(),
        "outcome conditional must be published"
    );
    assert!(schema.get("then").is_some());
    assert!(schema.get("else").is_some());
    let bytes = b"MZsynthetic schema fixture";
    for mask in 0..8 {
        let mut bundle = bundle_for(bytes);
        if mask & 1 != 0 {
            bundle.artifact.artifact_sha256 = "0".repeat(64);
        }
        if mask & 2 != 0 {
            bundle.artifact.artifact_size_bytes += 1;
        }
        if mask & 4 != 0 {
            bundle.artifact.artifact_kind = ArtifactKind::Text;
        }
        let report =
            serde_json::to_value(verify_artifact_binding(&bundle, bytes).unwrap()).unwrap();
        let expected = if mask == 0 { "matched" } else { "mismatched" };
        assert_eq!(report["outcome"], expected);
        for (index, field) in COMPARISONS.iter().enumerate() {
            assert_eq!(
                report[field],
                if mask & (1 << index) == 0 {
                    "matched"
                } else {
                    "mismatched"
                }
            );
        }
        assert!(
            structurally_accepts(&schema, &report),
            "real report mask {mask}"
        );
        for outcome in ["matched", "mismatched"] {
            let mut changed = report.clone();
            changed["outcome"] = json!(outcome);
            assert_eq!(
                structurally_accepts(&schema, &changed),
                outcome == expected,
                "mask {mask}, outcome {outcome}"
            );
        }
    }
}

#[test]
fn structural_schema_bounds_computed_size_to_nonzero_u64() {
    let schema = packaged_schema();
    let size = &schema["properties"]["computed_artifact_size_bytes"];
    assert_eq!(size["type"], "integer");
    assert_eq!(size["minimum"].as_u64(), Some(1));
    assert_eq!(size["maximum"].as_u64(), Some(u64::MAX));
    let report = serialized_report();
    for valid in [1, 2, u64::MAX] {
        let mut changed = report.clone();
        changed["computed_artifact_size_bytes"] = json!(valid);
        assert!(structurally_accepts(&schema, &changed));
    }
    for invalid in [
        json!(0),
        json!(-1),
        json!(1.5),
        json!("1"),
        json!(null),
        json!(true),
        serde_json::from_str::<Value>("18446744073709551616").unwrap(),
    ] {
        let mut changed = report.clone();
        changed["computed_artifact_size_bytes"] = invalid;
        assert!(!structurally_accepts(&schema, &changed));
    }
}

#[test]
fn structural_schema_bounds_computed_sha256_to_lowercase_hex() {
    let schema = packaged_schema();
    let digest = &schema["properties"]["computed_artifact_sha256"];
    assert_eq!(digest["type"], "string");
    assert_eq!(digest["pattern"], "^[0-9a-f]{64}$");
    assert_eq!(digest["minLength"], 64);
    assert_eq!(digest["maxLength"], 64);
    let report = serialized_report();
    for valid in ["0".repeat(64), "f".repeat(64), "0123456789abcdef".repeat(4)] {
        let mut changed = report.clone();
        changed["computed_artifact_sha256"] = json!(valid);
        assert!(structurally_accepts(&schema, &changed));
    }
    for invalid in [
        json!(""),
        json!("a".repeat(63)),
        json!("a".repeat(65)),
        json!("A".repeat(64)),
        json!("g".repeat(64)),
        json!(format!("{}\n", "a".repeat(64))),
        json!("é".repeat(64)),
        json!(null),
        json!(64),
        json!(false),
    ] {
        let mut changed = report.clone();
        changed["computed_artifact_sha256"] = invalid;
        assert!(!structurally_accepts(&schema, &changed));
    }
}

#[test]
fn structural_schema_binds_wire_enums_to_serialized_rust_values() {
    let schema = packaged_schema();
    let report = serialized_report();
    for field in [
        "outcome",
        "artifact_sha256",
        "artifact_size_bytes",
        "artifact_kind",
    ] {
        assert_eq!(schema["properties"][field]["type"], "string");
        assert_eq!(
            schema["properties"][field]["enum"],
            json!(["matched", "mismatched"])
        );
        for invalid in [
            json!("unknown"),
            json!("Matched"),
            json!(true),
            json!(0),
            json!(null),
        ] {
            let mut changed = report.clone();
            changed[field] = invalid;
            assert!(!structurally_accepts(&schema, &changed), "invalid {field}");
        }
    }
    let expected: Vec<Value> = KINDS
        .iter()
        .map(|kind| serde_json::to_value(kind).unwrap())
        .collect();
    assert_eq!(
        schema["properties"]["computed_artifact_kind"]["type"],
        "string"
    );
    assert_eq!(
        schema["properties"]["computed_artifact_kind"]["enum"],
        json!(expected)
    );
    for kind in expected {
        let mut changed = report.clone();
        changed["computed_artifact_kind"] = kind;
        assert!(structurally_accepts(&schema, &changed));
    }
    for invalid in [
        json!("binary"),
        json!("PortableExecutable"),
        json!(null),
        json!(9),
    ] {
        let mut changed = report.clone();
        changed["computed_artifact_kind"] = invalid;
        assert!(!structurally_accepts(&schema, &changed));
    }
    let fixtures: [&[u8]; 9] = [
        b"MZsynthetic",
        b"\x7fELFsynthetic",
        b"\xfe\xed\xfa\xcesynthetic",
        b"PK\x03\x04synthetic",
        b"%PDF-synthetic",
        b"\xd0\xcf\x11\xe0\xa1\xb1\x1a\xe1synthetic",
        b"#!/bin/sh\n# synthetic",
        b"synthetic plain text",
        b"\xff\x00",
    ];
    for (kind, bytes) in KINDS.iter().zip(fixtures) {
        let report = verify_artifact_binding(&bundle_for(bytes), bytes).unwrap();
        assert_eq!(report.computed_artifact_kind(), *kind);
        assert!(structurally_accepts(
            &schema,
            &serde_json::to_value(report).unwrap()
        ));
    }
}

#[test]
fn structural_schema_requires_exactly_the_serialized_report_fields() {
    let schema = packaged_schema();
    let report = serialized_report();
    assert_eq!(schema["additionalProperties"], false);
    let mut required: Vec<&str> = schema["required"]
        .as_array()
        .unwrap()
        .iter()
        .map(|field| field.as_str().unwrap())
        .collect();
    let mut expected = FIELDS.to_vec();
    required.sort_unstable();
    expected.sort_unstable();
    assert_eq!(required, expected);
    let properties = schema["properties"].as_object().unwrap();
    assert_eq!(properties.len(), FIELDS.len());
    assert!(FIELDS.iter().all(|field| properties.contains_key(*field)));
    assert_eq!(report.as_object().unwrap().len(), FIELDS.len());
    assert!(structurally_accepts(&schema, &report));
    for field in FIELDS {
        let mut incomplete = report.clone();
        incomplete.as_object_mut().unwrap().remove(field);
        assert!(
            !structurally_accepts(&schema, &incomplete),
            "missing {field}"
        );
    }
    for field in ["schema_version", "verdict", "provenance", "unexpected"] {
        let mut extended = report.clone();
        extended[field] = json!("extra");
        assert!(
            !structurally_accepts(&schema, &extended),
            "unexpected {field}"
        );
    }
    for not_an_object in [
        json!(null),
        json!([]),
        json!("report"),
        json!(1),
        json!(false),
    ] {
        assert!(!structurally_accepts(&schema, &not_an_object));
    }
}
