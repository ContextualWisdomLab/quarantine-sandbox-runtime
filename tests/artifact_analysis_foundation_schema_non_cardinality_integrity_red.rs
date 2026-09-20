//! RED guard that foundation-cardinality work preserves the rest of the public schema.

use serde_json::{json, Value};
use sha2::{Digest, Sha256};

const NON_CARDINALITY_SCHEMA_SHA256: &str =
    "e92a6562641ed7f4fc21f1d247cb97f78709a6dd8aaa34e8820c6343c1b90206";
const FOUNDATION_EVIDENCE_KINDS: [&str; 3] =
    ["artifact_identity", "file_format", "policy_boundary"];

fn evidence_bundle_schema() -> Value {
    serde_json::from_str(include_str!("../schemas/evidence-bundle.schema.json"))
        .expect("checked-in EvidenceBundle schema must be valid JSON")
}

fn schema_without_foundation_cardinality(mut schema: Value) -> Value {
    let evidence = schema
        .pointer_mut("/properties/evidence")
        .and_then(Value::as_object_mut)
        .expect("EvidenceBundle schema must declare an evidence object schema");
    evidence.remove("allOf");
    schema
}

fn non_cardinality_schema_sha256(schema: Value) -> String {
    let baseline = schema_without_foundation_cardinality(schema);
    let canonical = serde_json::to_vec(&baseline)
        .expect("EvidenceBundle non-cardinality schema must serialize deterministically");
    let digest = Sha256::digest(canonical);
    digest.iter().map(|byte| format!("{byte:02x}")).collect()
}

fn assert_non_cardinality_schema_unchanged(schema: Value) {
    assert_eq!(
        non_cardinality_schema_sha256(schema),
        NON_CARDINALITY_SCHEMA_SHA256,
        "foundation-cardinality work may change only properties.evidence.allOf; every other public EvidenceBundle schema contract must remain byte-semantically unchanged after JSON normalization"
    );
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

fn schema_with_target_cardinality() -> Value {
    let mut schema = evidence_bundle_schema();
    let evidence = schema
        .pointer_mut("/properties/evidence")
        .and_then(Value::as_object_mut)
        .expect("EvidenceBundle schema must declare an evidence object schema");
    evidence.insert(
        "allOf".to_owned(),
        Value::Array(
            FOUNDATION_EVIDENCE_KINDS
                .iter()
                .map(|evidence_kind| exact_foundation_constraint(evidence_kind))
                .collect(),
        ),
    );
    schema
}

#[test]
fn current_schema_matches_the_pinned_non_cardinality_contract() {
    assert_non_cardinality_schema_unchanged(evidence_bundle_schema());
}

#[test]
fn intended_foundation_cardinality_delta_preserves_the_pinned_contract() {
    assert_non_cardinality_schema_unchanged(schema_with_target_cardinality());
}

#[test]
fn witness_rejects_root_level_false_green_predicates() {
    let mut malformed_repair = schema_with_target_cardinality();
    malformed_repair["not"] = json!({});

    let result = std::panic::catch_unwind(|| assert_non_cardinality_schema_unchanged(malformed_repair));
    assert!(
        result.is_err(),
        "a root-level assertion that can invalidate every bundle must not coexist with a cardinality GREEN"
    );
}

#[test]
fn witness_rejects_unrelated_field_contract_drift() {
    let mut malformed_repair = schema_with_target_cardinality();
    malformed_repair["properties"]["analysis_job_id"]["maxLength"] = json!(4096);

    let result = std::panic::catch_unwind(|| assert_non_cardinality_schema_unchanged(malformed_repair));
    assert!(
        result.is_err(),
        "foundation-cardinality work must not weaken an unrelated public field contract"
    );
}
