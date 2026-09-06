//! RED for truthful dynamic artifact-analysis execution attestation.
//!
//! Issue #52 is independent from worker containment (#49) and bounded result
//! ingestion (#50). The public contract already models dynamic profiles and
//! runtime-behavior evidence, so an isolated worker must be able to attest that
//! dynamic execution actually occurred without weakening the static-only
//! boundary or allowing an unavailable worker to be reported as completed.

use std::collections::BTreeMap;

use quarantine_sandbox_runtime::{
    AnalysisProfile, ArtifactDescriptor, ArtifactKind, ContractError, EvidenceBundle, EvidenceKind,
    EvidenceRecord, RuntimeDisposition, RuntimeManifest,
};
use serde_json::Value;

fn dynamic_bundle(profile: AnalysisProfile) -> EvidenceBundle {
    EvidenceBundle {
        schema_version: "1.0.0".to_owned(),
        analysis_job_id: "analysis_job_dynamic_attestation_red".to_owned(),
        request_id: "dynamic-attestation-red-001".to_owned(),
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
            source_revision: "dynamic-attestation-red".to_owned(),
            requested_profile: profile,
            dynamic_execution_performed: true,
            network_access_performed: false,
            credentials_available: false,
        },
        disposition: RuntimeDisposition::Completed,
        consumer_verdict_required: true,
        evidence: vec![
            EvidenceRecord {
                evidence_id: "analysis_job_dynamic_attestation_red:evidence:0001".to_owned(),
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
                evidence_id: "analysis_job_dynamic_attestation_red:evidence:0002".to_owned(),
                sequence_number: 2,
                evidence_kind: EvidenceKind::RuntimeBehavior,
                producer_id: "isolated_dynamic_worker".to_owned(),
                summary: "Approved dynamic probe executed in the isolated worker.".to_owned(),
                attributes: BTreeMap::from([(
                    "worker_invocation_id".to_owned(),
                    "worker_invocation_dynamic_attestation_red".to_owned(),
                )]),
            },
        ],
        limitations: vec!["runtime_does_not_determine_maliciousness".to_owned()],
    }
}

fn unavailable_dynamic_bundle(profile: AnalysisProfile) -> EvidenceBundle {
    let mut bundle = dynamic_bundle(profile);
    bundle.runtime.dynamic_execution_performed = false;
    bundle.disposition = RuntimeDisposition::Inconclusive;
    bundle
        .evidence
        .retain(|record| record.evidence_kind != EvidenceKind::RuntimeBehavior);
    bundle
}

/// Evaluate the Draft 2020-12 assertion/applicator vocabulary needed by this
/// cross-field contract. Unrelated annotations and independent bounds are
/// intentionally ignored; the representative fixtures already satisfy those
/// constraints and this helper exists to execute the profile/execution/
/// disposition logic instead of merely searching schema text for keywords.
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
                if required
                    .iter()
                    .filter_map(Value::as_str)
                    .any(|property| !instance_object.contains_key(property))
                {
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
fn approved_dynamic_profiles_can_truthfully_attest_isolated_execution() {
    for profile in [
        AnalysisProfile::LinuxDynamic,
        AnalysisProfile::WindowsDynamic,
    ] {
        let bundle = dynamic_bundle(profile);
        bundle.validate().unwrap_or_else(|error| {
            panic!(
                "a completed dynamic profile must be able to attest actual isolated execution: profile={profile:?}, error={error:?}"
            )
        });
    }
}

#[test]
fn unavailable_dynamic_profiles_remain_valid_inconclusive_receipts() {
    for profile in [
        AnalysisProfile::LinuxDynamic,
        AnalysisProfile::WindowsDynamic,
    ] {
        let bundle = unavailable_dynamic_bundle(profile);
        bundle.validate().unwrap_or_else(|error| {
            panic!(
                "an unavailable dynamic worker must remain representable as inconclusive without fabricated execution: profile={profile:?}, error={error:?}"
            )
        });
    }
}

#[test]
fn dynamic_profile_cannot_claim_completion_when_execution_did_not_occur() {
    for profile in [
        AnalysisProfile::LinuxDynamic,
        AnalysisProfile::WindowsDynamic,
    ] {
        let mut bundle = unavailable_dynamic_bundle(profile);
        bundle.disposition = RuntimeDisposition::Completed;

        assert!(
            bundle.validate().is_err(),
            "a dynamic receipt with execution=false must not validate as completed: profile={profile:?}"
        );
    }
}

#[test]
fn static_only_profile_still_rejects_dynamic_execution_attestation() {
    let bundle = dynamic_bundle(AnalysisProfile::StaticOnly);

    assert_eq!(
        bundle.validate(),
        Err(ContractError::RuntimeBoundaryViolated {
            boundary_name: "dynamic_execution_performed",
        }),
        "StaticOnly must remain non-executing even after dynamic-profile attestation becomes representable"
    );
}

#[test]
fn evidence_bundle_schema_executes_dynamic_completeness_semantics() {
    let schema: Value =
        serde_json::from_str(include_str!("../schemas/evidence-bundle.schema.json"))
            .expect("checked-in evidence schema must be valid JSON");

    for profile in [
        AnalysisProfile::LinuxDynamic,
        AnalysisProfile::WindowsDynamic,
    ] {
        let completed = serde_json::to_value(dynamic_bundle(profile)).expect("fixture must serialize");
        let inconclusive = serde_json::to_value(unavailable_dynamic_bundle(profile))
            .expect("fixture must serialize");
        let mut false_completion = unavailable_dynamic_bundle(profile);
        false_completion.disposition = RuntimeDisposition::Completed;
        let false_completion =
            serde_json::to_value(false_completion).expect("fixture must serialize");

        assert!(
            relevant_schema_accepts(&schema, &completed),
            "the wire schema must admit truthful completed dynamic execution: profile={profile:?}"
        );
        assert!(
            relevant_schema_accepts(&schema, &inconclusive),
            "the wire schema must admit unavailable dynamic execution as Inconclusive: profile={profile:?}"
        );
        assert!(
            !relevant_schema_accepts(&schema, &false_completion),
            "the wire schema must reject execution=false + Completed: profile={profile:?}"
        );
    }

    let static_execution = serde_json::to_value(dynamic_bundle(AnalysisProfile::StaticOnly))
        .expect("fixture must serialize");
    assert!(
        !relevant_schema_accepts(&schema, &static_execution),
        "the wire schema must reject StaticOnly + dynamic_execution_performed=true"
    );
}
