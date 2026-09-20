//! RED contract for public EvidenceBundle foundation-cardinality schema parity.

use std::collections::BTreeMap;

use serde_json::{json, Value};

const FOUNDATION_EVIDENCE_KINDS: [&str; 3] =
    ["artifact_identity", "file_format", "policy_boundary"];

const NON_FOUNDATION_EVIDENCE_KINDS: [&str; 4] = [
    "static_capability",
    "runtime_behavior",
    "network_attempt",
    "tool_failure",
];

fn evidence_bundle_schema() -> Value {
    serde_json::from_str(include_str!("../schemas/evidence-bundle.schema.json"))
        .expect("checked-in EvidenceBundle schema must be valid JSON")
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
    let all_of = evidence_schema
        .get("allOf")
        .and_then(Value::as_array)
        .expect("evidence cardinality constraints must apply directly through allOf");

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
            assert_narrow_foundation_contains_selector(evidence_kind, contains);
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

#[test]
fn cardinality_witness_rejects_hidden_contains_predicates() {
    let malformed_repair = json!({
        "$schema": "https://json-schema.org/draft/2020-12/schema",
        "properties": {
            "evidence": {
                "allOf": [
                    {
                        "contains": {
                            "required": ["evidence_kind"],
                            "properties": {
                                "evidence_kind": { "const": "artifact_identity" },
                                "producer_id": { "const": "__never__" }
                            }
                        },
                        "minContains": 1,
                        "maxContains": 1
                    }
                ]
            }
        }
    });

    let result = std::panic::catch_unwind(|| {
        direct_evidence_occurrence_constraints_from(&malformed_repair)
    });
    assert!(
        result.is_err(),
        "cardinality witness must reject contains selectors with hidden predicates"
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
