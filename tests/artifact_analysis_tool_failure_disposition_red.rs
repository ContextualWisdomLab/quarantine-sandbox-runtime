//! RED for artifact-analysis completeness when attributable tool failure exists.
//!
//! Issue #56 owns the cross-field invariant between `ToolFailure` evidence and
//! runtime disposition. A receipt may preserve useful partial evidence as
//! inconclusive, but it must not claim every required analyzer completed while
//! simultaneously recording an analyzer or worker failure.

use std::collections::BTreeMap;

use quarantine_sandbox_runtime::{
    AnalysisProfile, ArtifactDescriptor, ArtifactKind, EvidenceBundle, EvidenceKind, EvidenceRecord,
    RuntimeDisposition, RuntimeManifest,
};
use serde_json::Value;

fn bundle_with_tool_failure(disposition: RuntimeDisposition) -> EvidenceBundle {
    EvidenceBundle {
        schema_version: "1.0.0".to_owned(),
        analysis_job_id: "analysis_job_tool_failure_disposition_red".to_owned(),
        request_id: "tool-failure-disposition-red-001".to_owned(),
        artifact: ArtifactDescriptor {
            artifact_name: "artifact.bin".to_owned(),
            original_file_name: None,
            artifact_size_bytes: 3,
            artifact_sha256: "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
                .to_owned(),
            artifact_kind: ArtifactKind::Unknown,
        },
        runtime: RuntimeManifest {
            runtime_name: "quarantine-sandbox-runtime".to_owned(),
            runtime_version: "0.1.0".to_owned(),
            source_revision: "tool-failure-disposition-red".to_owned(),
            requested_profile: AnalysisProfile::StaticOnly,
            dynamic_execution_performed: false,
            network_access_performed: false,
            credentials_available: false,
        },
        disposition,
        consumer_verdict_required: true,
        evidence: vec![
            EvidenceRecord {
                evidence_id: "analysis_job_tool_failure_disposition_red:evidence:0001".to_owned(),
                sequence_number: 1,
                evidence_kind: EvidenceKind::ArtifactIdentity,
                producer_id: "runtime_core".to_owned(),
                summary: "Artifact identity established.".to_owned(),
                attributes: BTreeMap::from([(
                    "artifact_sha256".to_owned(),
                    "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad".to_owned(),
                )]),
            },
            EvidenceRecord {
                evidence_id: "analysis_job_tool_failure_disposition_red:evidence:0002".to_owned(),
                sequence_number: 2,
                evidence_kind: EvidenceKind::ToolFailure,
                producer_id: "fixture_analyzer".to_owned(),
                summary: "Static analyzer did not complete.".to_owned(),
                attributes: BTreeMap::from([(
                    "failure_code".to_owned(),
                    "fixture_failure".to_owned(),
                )]),
            },
        ],
        limitations: vec![
            "runtime_does_not_determine_maliciousness".to_owned(),
            "static_analyzer_failure".to_owned(),
        ],
    }
}

/// Evaluate only the Draft 2020-12 assertion/applicator vocabulary needed by
/// this cross-field contract. Unknown annotation/format/bound keywords are
/// deliberately ignored because the fixtures are already valid under those
/// independent constraints; this helper exists only to prove that the checked-
/// in logical guard actually accepts/rejects representative wire instances.
fn relevant_schema_accepts(schema: &Value, instance: &Value) -> bool {
    match schema {
        Value::Bool(accepted) => *accepted,
        Value::Object(object) => {
            if let Some(expected) = object.get("const")
                && expected != instance
            {
                return false;
            }

            if let Some(allowed) = object.get("enum").and_then(Value::as_array)
                && !allowed.iter().any(|candidate| candidate == instance)
            {
                return false;
            }

            if let Some(required) = object.get("required").and_then(Value::as_array) {
                let Some(instance_object) = instance.as_object() else {
                    return false;
                };
                if required.iter().filter_map(Value::as_str).any(|property| {
                    !instance_object.contains_key(property)
                }) {
                    return false;
                }
            }

            if let Some(properties) = object.get("properties").and_then(Value::as_object)
                && let Some(instance_object) = instance.as_object()
            {
                for (property, property_schema) in properties {
                    if let Some(property_value) = instance_object.get(property)
                        && !relevant_schema_accepts(property_schema, property_value)
                    {
                        return false;
                    }
                }
            }

            if let Some(item_schema) = object.get("items")
                && let Some(items) = instance.as_array()
                && items
                    .iter()
                    .any(|item| !relevant_schema_accepts(item_schema, item))
            {
                return false;
            }

            if let Some(contains_schema) = object.get("contains")
                && let Some(items) = instance.as_array()
                && !items
                    .iter()
                    .any(|item| relevant_schema_accepts(contains_schema, item))
            {
                return false;
            }

            if let Some(all_of) = object.get("allOf").and_then(Value::as_array)
                && all_of
                    .iter()
                    .any(|subschema| !relevant_schema_accepts(subschema, instance))
            {
                return false;
            }

            if let Some(any_of) = object.get("anyOf").and_then(Value::as_array)
                && !any_of
                    .iter()
                    .any(|subschema| relevant_schema_accepts(subschema, instance))
            {
                return false;
            }

            if let Some(one_of) = object.get("oneOf").and_then(Value::as_array)
                && one_of
                    .iter()
                    .filter(|subschema| relevant_schema_accepts(subschema, instance))
                    .count()
                    != 1
            {
                return false;
            }

            if let Some(not_schema) = object.get("not")
                && relevant_schema_accepts(not_schema, instance)
            {
                return false;
            }

            if let Some(if_schema) = object.get("if") {
                let branch = if relevant_schema_accepts(if_schema, instance) {
                    object.get("then")
                } else {
                    object.get("else")
                };
                if let Some(branch_schema) = branch
                    && !relevant_schema_accepts(branch_schema, instance)
                {
                    return false;
                }
            }

            if let Some(dependent_schemas) = object.get("dependentSchemas").and_then(Value::as_object)
                && let Some(instance_object) = instance.as_object()
            {
                for (property, dependent_schema) in dependent_schemas {
                    if instance_object.contains_key(property)
                        && !relevant_schema_accepts(dependent_schema, instance)
                    {
                        return false;
                    }
                }
            }

            if let Some(dependent_required) =
                object.get("dependentRequired").and_then(Value::as_object)
                && let Some(instance_object) = instance.as_object()
            {
                for (property, dependencies) in dependent_required {
                    if !instance_object.contains_key(property) {
                        continue;
                    }
                    let Some(dependencies) = dependencies.as_array() else {
                        return false;
                    };
                    if dependencies
                        .iter()
                        .filter_map(Value::as_str)
                        .any(|dependency| !instance_object.contains_key(dependency))
                    {
                        return false;
                    }
                }
            }

            true
        }
        _ => false,
    }
}

#[test]
fn tool_failure_receipt_remains_representable_as_inconclusive() {
    let bundle = bundle_with_tool_failure(RuntimeDisposition::Inconclusive);

    bundle.validate().unwrap_or_else(|error| {
        panic!("partial evidence with an attributable tool failure must remain representable as inconclusive: {error:?}")
    });
}

#[test]
fn tool_failure_evidence_cannot_claim_completed_disposition() {
    let bundle = bundle_with_tool_failure(RuntimeDisposition::Completed);

    assert!(
        bundle.validate().is_err(),
        "a receipt containing ToolFailure evidence must not validate as Completed"
    );
}

#[test]
fn evidence_bundle_schema_executes_tool_failure_disposition_semantics() {
    let schema: Value =
        serde_json::from_str(include_str!("../schemas/evidence-bundle.schema.json"))
            .expect("checked-in evidence schema must be valid JSON");
    let inconclusive = serde_json::to_value(bundle_with_tool_failure(
        RuntimeDisposition::Inconclusive,
    ))
    .expect("inconclusive fixture must serialize");
    let completed = serde_json::to_value(bundle_with_tool_failure(RuntimeDisposition::Completed))
        .expect("completed fixture must serialize");

    assert!(
        relevant_schema_accepts(&schema, &inconclusive),
        "the wire schema must continue to admit an otherwise-valid ToolFailure + Inconclusive receipt"
    );
    assert!(
        !relevant_schema_accepts(&schema, &completed),
        "the wire schema must execute a logical guard that rejects the same receipt when disposition is Completed"
    );
}
