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

fn logical_schema_binds_tool_failure_to_disposition(value: &Value) -> bool {
    const LOGICAL_KEYWORDS: [&str; 10] = [
        "if",
        "then",
        "else",
        "allOf",
        "anyOf",
        "oneOf",
        "not",
        "contains",
        "dependentSchemas",
        "dependentRequired",
    ];

    match value {
        Value::Object(object) => {
            for keyword in LOGICAL_KEYWORDS {
                if let Some(logic) = object.get(keyword) {
                    let rendered = logic.to_string();
                    if rendered.contains("tool_failure") && rendered.contains("completed") {
                        return true;
                    }
                }
            }
            object
                .values()
                .any(logical_schema_binds_tool_failure_to_disposition)
        }
        Value::Array(items) => items
            .iter()
            .any(logical_schema_binds_tool_failure_to_disposition),
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
fn evidence_bundle_schema_binds_tool_failure_to_non_completed_disposition() {
    let schema: Value =
        serde_json::from_str(include_str!("../schemas/evidence-bundle.schema.json"))
            .expect("checked-in evidence schema must be valid JSON");

    assert!(
        logical_schema_binds_tool_failure_to_disposition(&schema),
        "the wire schema needs a logical guard preventing tool_failure evidence from coexisting with completed disposition"
    );
}
