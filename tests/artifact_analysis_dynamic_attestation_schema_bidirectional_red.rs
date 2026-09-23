//! RED for public-schema parity of the inverse dynamic-attestation invariant.
//!
//! The schema already rejects `RuntimeBehavior` when execution is false. It
//! must also require observed runtime behavior whenever the manifest claims
//! dynamic execution, so the boolean alone cannot become wire-level execution
//! evidence.

use serde_json::{Map, Value, json};

fn object_has_only_keys(object: &Map<String, Value>, expected: &[&str]) -> bool {
    object.len() == expected.len() && expected.iter().all(|key| object.contains_key(*key))
}

fn is_exact_dynamic_execution_true_condition(value: &Value) -> bool {
    let Some(root) = value.as_object() else {
        return false;
    };
    if !object_has_only_keys(root, &["properties"]) {
        return false;
    }

    let Some(properties) = root.get("properties").and_then(Value::as_object) else {
        return false;
    };
    if !object_has_only_keys(properties, &["runtime"]) {
        return false;
    }

    let Some(runtime) = properties.get("runtime").and_then(Value::as_object) else {
        return false;
    };
    if !object_has_only_keys(runtime, &["properties"]) {
        return false;
    }

    let Some(runtime_properties) = runtime.get("properties").and_then(Value::as_object) else {
        return false;
    };
    if !object_has_only_keys(runtime_properties, &["dynamic_execution_performed"]) {
        return false;
    }

    let Some(execution) = runtime_properties
        .get("dynamic_execution_performed")
        .and_then(Value::as_object)
    else {
        return false;
    };

    object_has_only_keys(execution, &["const"])
        && execution.get("const") == Some(&Value::Bool(true))
}

fn requires_runtime_behavior(value: &Value) -> bool {
    let Some(root) = value.as_object() else {
        return false;
    };
    if !object_has_only_keys(root, &["properties"]) {
        return false;
    }

    let Some(properties) = root.get("properties").and_then(Value::as_object) else {
        return false;
    };
    if !object_has_only_keys(properties, &["evidence"]) {
        return false;
    }

    let Some(evidence) = properties.get("evidence").and_then(Value::as_object) else {
        return false;
    };
    if !object_has_only_keys(evidence, &["contains"]) {
        return false;
    }

    let Some(selector) = evidence.get("contains").and_then(Value::as_object) else {
        return false;
    };
    if !object_has_only_keys(selector, &["required", "properties"]) {
        return false;
    }

    let required_is_exact = selector
        .get("required")
        .and_then(Value::as_array)
        .is_some_and(|items| items.len() == 1 && items[0].as_str() == Some("evidence_kind"));
    let kind_is_exact = selector
        .get("properties")
        .and_then(Value::as_object)
        .filter(|properties| object_has_only_keys(properties, &["evidence_kind"]))
        .and_then(|properties| properties.get("evidence_kind"))
        .and_then(Value::as_object)
        .filter(|kind| object_has_only_keys(kind, &["const"]))
        .and_then(|kind| kind.get("const"))
        .and_then(Value::as_str)
        == Some("runtime_behavior");

    required_is_exact && kind_is_exact
}

#[test]
fn structural_witness_rejects_hidden_sibling_predicates() {
    let hidden_if_root_sibling = json!({
        "properties": {
            "runtime": {
                "properties": {
                    "dynamic_execution_performed": { "const": true }
                }
            },
            "disposition": { "const": "__never__" }
        }
    });
    assert!(
        !is_exact_dynamic_execution_true_condition(&hidden_if_root_sibling),
        "an impossible sibling condition must not masquerade as the execution=true trigger"
    );

    let hidden_if_runtime_sibling = json!({
        "properties": {
            "runtime": {
                "properties": {
                    "dynamic_execution_performed": { "const": true },
                    "requested_profile": { "const": "static_only" }
                }
            }
        }
    });
    assert!(
        !is_exact_dynamic_execution_true_condition(&hidden_if_runtime_sibling),
        "an additional runtime predicate must not narrow the execution=true trigger"
    );

    let hidden_then_root_sibling = json!({
        "properties": {
            "evidence": {
                "contains": {
                    "required": ["evidence_kind"],
                    "properties": {
                        "evidence_kind": { "const": "runtime_behavior" }
                    }
                }
            },
            "disposition": { "const": "__never__" }
        }
    });
    assert!(
        !requires_runtime_behavior(&hidden_then_root_sibling),
        "an impossible sibling consequence must not masquerade as a direct RuntimeBehavior requirement"
    );
}

#[test]
fn public_schema_requires_runtime_behavior_when_dynamic_execution_is_true() {
    let schema: Value =
        serde_json::from_str(include_str!("../schemas/evidence-bundle.schema.json"))
            .expect("checked-in evidence schema must be valid JSON");

    let rules = schema
        .get("allOf")
        .and_then(Value::as_array)
        .expect("evidence bundle schema must expose root allOf cross-field rules");

    assert!(
        rules.iter().any(|rule| {
            let Some(rule) = rule.as_object() else {
                return false;
            };
            object_has_only_keys(rule, &["if", "then"])
                && rule
                    .get("if")
                    .is_some_and(is_exact_dynamic_execution_true_condition)
                && rule.get("then").is_some_and(requires_runtime_behavior)
        }),
        "dynamic_execution_performed=true must directly require at least one RuntimeBehavior evidence record; a boolean alone is not execution evidence"
    );
}
