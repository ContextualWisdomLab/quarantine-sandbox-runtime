//! RED contract for public EvidenceBundle foundation-cardinality schema parity.

use std::collections::BTreeMap;

use serde_json::Value;

const FOUNDATION_EVIDENCE_KINDS: [&str; 3] = [
    "artifact_identity",
    "file_format",
    "policy_boundary",
];

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

fn collect_contains_cardinality(
    schema: &Value,
    constraints: &mut BTreeMap<String, (u64, u64)>,
) {
    match schema {
        Value::Object(object) => {
            if let Some(contains) = object.get("contains") {
                let evidence_kind = contains
                    .pointer("/properties/evidence_kind/const")
                    .and_then(Value::as_str);
                let min_contains = object.get("minContains").and_then(Value::as_u64);
                let max_contains = object.get("maxContains").and_then(Value::as_u64);

                if let (Some(evidence_kind), Some(min_contains), Some(max_contains)) =
                    (evidence_kind, min_contains, max_contains)
                {
                    let previous = constraints.insert(
                        evidence_kind.to_owned(),
                        (min_contains, max_contains),
                    );
                    assert!(
                        previous.is_none(),
                        "schema must not declare conflicting occurrence bounds for {evidence_kind}"
                    );
                }
            }

            for child in object.values() {
                collect_contains_cardinality(child, constraints);
            }
        }
        Value::Array(items) => {
            for item in items {
                collect_contains_cardinality(item, constraints);
            }
        }
        _ => {}
    }
}

fn evidence_occurrence_constraints() -> BTreeMap<String, (u64, u64)> {
    let schema = evidence_bundle_schema();
    assert_eq!(
        schema.get("$schema").and_then(Value::as_str),
        Some("https://json-schema.org/draft/2020-12/schema"),
        "EvidenceBundle schema must remain on the declared Draft 2020-12 contract"
    );

    let evidence_schema = schema
        .pointer("/properties/evidence")
        .expect("EvidenceBundle schema must declare the evidence array");
    let mut constraints = BTreeMap::new();
    collect_contains_cardinality(evidence_schema, &mut constraints);
    constraints
}

#[test]
fn evidence_schema_requires_exactly_one_of_each_foundation_kind() {
    let constraints = evidence_occurrence_constraints();

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
    let constraints = evidence_occurrence_constraints();

    for evidence_kind in NON_FOUNDATION_EVIDENCE_KINDS {
        assert!(
            !constraints.contains_key(evidence_kind),
            "foundation cardinality must not globally singleton {evidence_kind}"
        );
    }
}
