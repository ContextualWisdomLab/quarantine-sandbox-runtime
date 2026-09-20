//! RED contract for public EvidenceBundle foundation-cardinality schema parity.

use std::collections::BTreeMap;

use serde_json::{json, Map, Value};

const FOUNDATION_EVIDENCE_KINDS: [&str; 3] =
    ["artifact_identity", "file_format", "policy_boundary"];

const NON_FOUNDATION_EVIDENCE_KINDS: [&str; 4] = [
    "static_capability",
    "runtime_behavior",
    "network_attempt",
    "tool_failure",
];

const EVIDENCE_RECORD_FIELDS: [&str; 6] = [
    "evidence_id",
    "sequence_number",
    "evidence_kind",
    "producer_id",
    "summary",
    "attributes",
];

fn evidence_bundle_schema() -> Value {
    serde_json::from_str(include_str!("../schemas/evidence-bundle.schema.json"))
        .expect("checked-in EvidenceBundle schema must be valid JSON")
}

fn assert_evidence_record_items_contract(items: &Value) {
    let items = items
        .as_object()
        .expect("EvidenceBundle evidence items must remain an object schema");
    assert_eq!(
        items.get("type").and_then(Value::as_str),
        Some("object"),
        "EvidenceBundle evidence items must remain object-typed"
    );
    assert_eq!(
        items.get("additionalProperties").and_then(Value::as_bool),
        Some(false),
        "EvidenceBundle evidence records must remain closed to unknown members"
    );

    let required = items
        .get("required")
        .and_then(Value::as_array)
        .expect("EvidenceBundle evidence records must retain required fields");
    assert_eq!(
        required.len(),
        EVIDENCE_RECORD_FIELDS.len(),
        "EvidenceBundle evidence required-field surface must remain unchanged"
    );
    for field_name in EVIDENCE_RECORD_FIELDS {
        assert!(
            required.iter().any(|value| value.as_str() == Some(field_name)),
            "EvidenceBundle evidence records must keep required field {field_name}"
        );
    }

    let properties = items
        .get("properties")
        .and_then(Value::as_object)
        .expect("EvidenceBundle evidence records must retain their property surface");
    assert_eq!(
        properties.len(),
        EVIDENCE_RECORD_FIELDS.len(),
        "EvidenceBundle evidence property surface must remain unchanged"
    );
    for field_name in EVIDENCE_RECORD_FIELDS {
        assert!(
            properties.contains_key(field_name),
            "EvidenceBundle evidence records must keep property {field_name}"
        );
    }

    let evidence_kind_values = properties
        .get("evidence_kind")
        .and_then(|value| value.get("enum"))
        .and_then(Value::as_array)
        .expect("EvidenceBundle evidence_kind must retain its enum");
    let expected_evidence_kinds = FOUNDATION_EVIDENCE_KINDS
        .iter()
        .chain(NON_FOUNDATION_EVIDENCE_KINDS.iter())
        .copied()
        .collect::<Vec<_>>();
    assert_eq!(
        evidence_kind_values.len(),
        expected_evidence_kinds.len(),
        "EvidenceBundle evidence_kind enum size must remain unchanged"
    );
    for evidence_kind in expected_evidence_kinds {
        assert!(
            evidence_kind_values
                .iter()
                .any(|value| value.as_str() == Some(evidence_kind)),
            "EvidenceBundle evidence_kind enum must retain {evidence_kind}"
        );
    }

    assert_eq!(
        Value::Object(items.clone()),
        baseline_evidence_record_items(),
        "foundation-cardinality repair must preserve the complete existing EvidenceRecord item schema"
    );
}

fn assert_evidence_array_baseline_contract(evidence_schema: &Map<String, Value>) {
    assert_eq!(
        evidence_schema.get("type").and_then(Value::as_str),
        Some("array"),
        "foundation-cardinality repair must preserve the evidence array type"
    );
    assert_eq!(
        evidence_schema.get("minItems").and_then(Value::as_u64),
        Some(1),
        "foundation-cardinality repair must preserve the existing evidence minItems"
    );
    let items = evidence_schema
        .get("items")
        .expect("foundation-cardinality repair must retain evidence items");
    assert_evidence_record_items_contract(items);
}

fn assert_narrow_foundation_contains_selector(evidence_kind: &str, contains: &Value) {
    let contains = contains
        .as_object()
        .expect("foundation contains selector must be an object");
    assert_eq!(
        contains.len(),
        2,
        "foundation contains selector must not constrain unrelated evidence fields"
    );

    let required = contains
        .get("required")
        .and_then(Value::as_array)
        .expect("foundation contains selector must explicitly require evidence_kind");
    assert_eq!(
        required.len(),
        1,
        "foundation contains selector must require only evidence_kind"
    );
    assert_eq!(
        required[0].as_str(),
        Some("evidence_kind"),
        "foundation contains selector must require evidence_kind"
    );

    let properties = contains
        .get("properties")
        .and_then(Value::as_object)
        .expect("foundation contains selector must constrain evidence_kind");
    assert_eq!(
        properties.len(),
        1,
        "foundation contains selector must not constrain unrelated evidence properties"
    );

    let evidence_kind_selector = properties
        .get("evidence_kind")
        .and_then(Value::as_object)
        .expect("foundation contains selector must constrain evidence_kind");
    assert_eq!(
        evidence_kind_selector.len(),
        1,
        "foundation evidence_kind selector must contain only const"
    );
    assert_eq!(
        evidence_kind_selector.get("const").and_then(Value::as_str),
        Some(evidence_kind),
        "foundation evidence_kind selector must bind the counted evidence kind"
    );
}

fn assert_narrow_foundation_occurrence_constraint(
    evidence_kind: &str,
    constraint: &Value,
    contains: &Value,
) {
    let constraint = constraint
        .as_object()
        .expect("foundation occurrence constraint must be an object");
    assert_eq!(
        constraint.len(),
        3,
        "foundation occurrence constraint must not add unrelated assertions or applicators"
    );
    for keyword in ["contains", "minContains", "maxContains"] {
        assert!(
            constraint.contains_key(keyword),
            "foundation occurrence constraint must contain only the exact-count keywords"
        );
    }
    assert_narrow_foundation_contains_selector(evidence_kind, contains);
}

fn direct_evidence_occurrence_constraints_from(
    schema: &Value,
) -> BTreeMap<String, (u64, u64)> {
    assert_eq!(
        schema.get("$schema").and_then(Value::as_str),
        Some("https://json-schema.org/draft/2020-12/schema"),
        "EvidenceBundle schema must remain on the declared Draft 2020-12 contract"
    );

    let evidence_schema = schema
        .pointer("/properties/evidence")
        .expect("EvidenceBundle schema must declare the evidence array");
    let evidence_schema = evidence_schema
        .as_object()
        .expect("EvidenceBundle evidence schema must be an object");

    let allowed_parent_keywords = ["type", "minItems", "items", "allOf"];
    assert_eq!(
        evidence_schema.len(),
        allowed_parent_keywords.len(),
        "foundation-cardinality repair must not add evidence-array sibling restrictions"
    );
    for keyword in allowed_parent_keywords {
        assert!(
            evidence_schema.contains_key(keyword),
            "foundation-cardinality repair must retain evidence-array {keyword}"
        );
    }
    assert_evidence_array_baseline_contract(evidence_schema);

    let all_of = evidence_schema
        .get("allOf")
        .and_then(Value::as_array)
        .expect("evidence cardinality constraints must apply directly through allOf");
    assert_eq!(
        all_of.len(),
        FOUNDATION_EVIDENCE_KINDS.len(),
        "evidence allOf must contain only the three foundation occurrence constraints"
    );

    let mut constraints = BTreeMap::new();
    for constraint in all_of {
        let Some(contains) = constraint.get("contains") else {
            continue;
        };
        let Some(evidence_kind) = contains
            .pointer("/properties/evidence_kind/const")
            .and_then(Value::as_str)
        else {
            continue;
        };

        if FOUNDATION_EVIDENCE_KINDS
            .iter()
            .any(|candidate| *candidate == evidence_kind)
        {
            assert_narrow_foundation_occurrence_constraint(evidence_kind, constraint, contains);
        }

        let min_contains = constraint.get("minContains").and_then(Value::as_u64);
        let max_contains = constraint.get("maxContains").and_then(Value::as_u64);

        if let (Some(min_contains), Some(max_contains)) = (min_contains, max_contains) {
            let previous =
                constraints.insert(evidence_kind.to_owned(), (min_contains, max_contains));
            assert!(
                previous.is_none(),
                "schema must not declare conflicting direct occurrence bounds for {evidence_kind}"
            );
        }
    }

    constraints
}

fn direct_evidence_occurrence_constraints() -> BTreeMap<String, (u64, u64)> {
    direct_evidence_occurrence_constraints_from(&evidence_bundle_schema())
}

fn exact_foundation_constraint(evidence_kind: &str) -> Value {
    json!({
        "contains": {
            "required": ["evidence_kind"],
            "properties": {
                "evidence_kind": { "const": evidence_kind }
            }
        },
        "minContains": 1,
        "maxContains": 1
    })
}

fn baseline_evidence_record_items() -> Value {
    json!({
        "type": "object",
        "additionalProperties": false,
        "required": EVIDENCE_RECORD_FIELDS,
        "properties": {
            "evidence_id": {
                "type": "string",
                "minLength": 1,
                "maxLength": 256,
                "pattern": "^[^\\u0000-\\u001F\\u007F-\\u009F]*$",
                "x-cwl-maxUtf8Bytes": 256
            },
            "sequence_number": {
                "type": "integer",
                "minimum": 1
            },
            "evidence_kind": {
                "enum": [
                    "artifact_identity",
                    "file_format",
                    "policy_boundary",
                    "static_capability",
                    "runtime_behavior",
                    "network_attempt",
                    "tool_failure"
                ]
            },
            "producer_id": {
                "type": "string",
                "minLength": 1,
                "maxLength": 128,
                "pattern": "^[^\\u0000-\\u001F\\u007F-\\u009F]*$",
                "x-cwl-maxUtf8Bytes": 128
            },
            "summary": {
                "type": "string",
                "minLength": 1,
                "maxLength": 4096,
                "pattern": "^[^\\u0000-\\u001F\\u007F-\\u009F]*$",
                "x-cwl-maxUtf8Bytes": 4096
            },
            "attributes": {
                "type": "object",
                "maxProperties": 32,
                "propertyNames": {
                    "minLength": 1,
                    "maxLength": 128,
                    "pattern": "^[^\\u0000-\\u001F\\u007F-\\u009F]*$",
                    "x-cwl-maxUtf8Bytes": 128
                },
                "additionalProperties": {
                    "type": "string",
                    "minLength": 1,
                    "maxLength": 1024,
                    "pattern": "^[^\\u0000-\\u001F\\u007F-\\u009F]*$",
                    "x-cwl-maxUtf8Bytes": 1024
                }
            }
        }
    })
}

fn baseline_evidence_schema(all_of: Vec<Value>) -> Value {
    json!({
        "$schema": "https://json-schema.org/draft/2020-12/schema",
        "properties": {
            "evidence": {
                "type": "array",
                "minItems": 1,
                "items": baseline_evidence_record_items(),
                "allOf": all_of
            }
        }
    })
}

#[test]
fn cardinality_witness_accepts_unchanged_evidence_record_contract() {
    let baseline = baseline_evidence_schema(
        FOUNDATION_EVIDENCE_KINDS
            .iter()
            .map(|evidence_kind| exact_foundation_constraint(evidence_kind))
            .collect(),
    );
    let constraints = direct_evidence_occurrence_constraints_from(&baseline);
    for evidence_kind in FOUNDATION_EVIDENCE_KINDS {
        assert_eq!(constraints.get(evidence_kind), Some(&(1, 1)));
    }
}

#[test]
fn cardinality_witness_rejects_hidden_contains_predicates() {
    let malformed_repair = baseline_evidence_schema(vec![
        json!({
            "contains": {
                "required": ["evidence_kind"],
                "properties": {
                    "evidence_kind": { "const": "artifact_identity" },
                    "producer_id": { "const": "__never__" }
                }
            },
            "minContains": 1,
            "maxContains": 1
        }),
        exact_foundation_constraint("file_format"),
        exact_foundation_constraint("policy_boundary"),
    ]);

    let result = std::panic::catch_unwind(|| {
        direct_evidence_occurrence_constraints_from(&malformed_repair)
    });
    assert!(
        result.is_err(),
        "cardinality witness must reject contains selectors with hidden predicates"
    );
}

#[test]
fn cardinality_witness_rejects_hidden_occurrence_predicates() {
    let malformed_repair = baseline_evidence_schema(vec![
        json!({
            "contains": {
                "required": ["evidence_kind"],
                "properties": {
                    "evidence_kind": { "const": "artifact_identity" }
                }
            },
            "minContains": 1,
            "maxContains": 1,
            "maxItems": 0
        }),
        exact_foundation_constraint("file_format"),
        exact_foundation_constraint("policy_boundary"),
    ]);

    let result = std::panic::catch_unwind(|| {
        direct_evidence_occurrence_constraints_from(&malformed_repair)
    });
    assert!(
        result.is_err(),
        "cardinality witness must reject outer occurrence constraints with hidden predicates"
    );
}

#[test]
fn cardinality_witness_rejects_hidden_evidence_array_predicates() {
    let mut malformed_repair = baseline_evidence_schema(
        FOUNDATION_EVIDENCE_KINDS
            .iter()
            .map(|evidence_kind| exact_foundation_constraint(evidence_kind))
            .collect(),
    );
    malformed_repair["properties"]["evidence"]["maxItems"] = json!(3);

    let result = std::panic::catch_unwind(|| {
        direct_evidence_occurrence_constraints_from(&malformed_repair)
    });
    assert!(
        result.is_err(),
        "cardinality witness must reject evidence-array siblings that globally cap optional evidence"
    );
}

#[test]
fn cardinality_witness_rejects_extra_all_of_predicates() {
    let mut all_of: Vec<Value> = FOUNDATION_EVIDENCE_KINDS
        .iter()
        .map(|evidence_kind| exact_foundation_constraint(evidence_kind))
        .collect();
    all_of.push(json!({ "maxItems": 3 }));
    let malformed_repair = baseline_evidence_schema(all_of);

    let result = std::panic::catch_unwind(|| {
        direct_evidence_occurrence_constraints_from(&malformed_repair)
    });
    assert!(
        result.is_err(),
        "cardinality witness must reject unrelated evidence-array predicates hidden in allOf"
    );
}

#[test]
fn cardinality_witness_rejects_weakened_evidence_array_minimum() {
    let mut malformed_repair = baseline_evidence_schema(
        FOUNDATION_EVIDENCE_KINDS
            .iter()
            .map(|evidence_kind| exact_foundation_constraint(evidence_kind))
            .collect(),
    );
    malformed_repair["properties"]["evidence"]["minItems"] = json!(0);

    let result = std::panic::catch_unwind(|| {
        direct_evidence_occurrence_constraints_from(&malformed_repair)
    });
    assert!(
        result.is_err(),
        "cardinality witness must reject weakening the pre-existing evidence minItems contract"
    );
}

#[test]
fn cardinality_witness_rejects_weakened_evidence_record_items() {
    let mut malformed_repair = baseline_evidence_schema(
        FOUNDATION_EVIDENCE_KINDS
            .iter()
            .map(|evidence_kind| exact_foundation_constraint(evidence_kind))
            .collect(),
    );
    malformed_repair["properties"]["evidence"]["items"] = json!({});

    let result = std::panic::catch_unwind(|| {
        direct_evidence_occurrence_constraints_from(&malformed_repair)
    });
    assert!(
        result.is_err(),
        "cardinality witness must reject weakening the pre-existing evidence-record item contract"
    );
}

#[test]
fn cardinality_witness_rejects_weakened_evidence_record_field_semantics() {
    let mut malformed_repair = baseline_evidence_schema(
        FOUNDATION_EVIDENCE_KINDS
            .iter()
            .map(|evidence_kind| exact_foundation_constraint(evidence_kind))
            .collect(),
    );
    malformed_repair["properties"]["evidence"]["items"]["properties"]["evidence_id"]
        ["maxLength"] = json!(4096);

    let result = std::panic::catch_unwind(|| {
        direct_evidence_occurrence_constraints_from(&malformed_repair)
    });
    assert!(
        result.is_err(),
        "cardinality witness must reject weakening nested EvidenceRecord field semantics"
    );
}

#[test]
fn cardinality_witness_rejects_hidden_evidence_kind_predicates() {
    let mut malformed_repair = baseline_evidence_schema(
        FOUNDATION_EVIDENCE_KINDS
            .iter()
            .map(|evidence_kind| exact_foundation_constraint(evidence_kind))
            .collect(),
    );
    malformed_repair["properties"]["evidence"]["items"]["properties"]["evidence_kind"]
        ["const"] = json!("artifact_identity");

    let result = std::panic::catch_unwind(|| {
        direct_evidence_occurrence_constraints_from(&malformed_repair)
    });
    assert!(
        result.is_err(),
        "cardinality witness must reject hidden evidence_kind predicates that make other required kinds impossible"
    );
}

#[test]
fn cardinality_witness_rejects_hidden_evidence_record_predicates() {
    let mut malformed_repair = baseline_evidence_schema(
        FOUNDATION_EVIDENCE_KINDS
            .iter()
            .map(|evidence_kind| exact_foundation_constraint(evidence_kind))
            .collect(),
    );
    malformed_repair["properties"]["evidence"]["items"]["maxProperties"] = json!(0);

    let result = std::panic::catch_unwind(|| {
        direct_evidence_occurrence_constraints_from(&malformed_repair)
    });
    assert!(
        result.is_err(),
        "cardinality witness must reject item-level sibling predicates that invalidate ordinary evidence records"
    );
}

#[test]
fn evidence_schema_requires_exactly_one_of_each_foundation_kind() {
    let constraints = direct_evidence_occurrence_constraints();

    for evidence_kind in FOUNDATION_EVIDENCE_KINDS {
        assert_eq!(
            constraints.get(evidence_kind),
            Some(&(1, 1)),
            "public schema must require exactly one {evidence_kind} record"
        );
    }
}

#[test]
fn evidence_schema_does_not_singleton_non_foundation_kinds() {
    let constraints = direct_evidence_occurrence_constraints();

    for evidence_kind in NON_FOUNDATION_EVIDENCE_KINDS {
        assert!(
            !constraints.contains_key(evidence_kind),
            "foundation cardinality must not globally singleton {evidence_kind}"
        );
    }
}
