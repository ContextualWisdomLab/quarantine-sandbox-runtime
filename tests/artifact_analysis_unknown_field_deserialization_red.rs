//! RED contract for strict deserialization of published artifact evidence JSON.

use quarantine_sandbox_runtime::EvidenceBundle;
use serde_json::{Value, json};

fn valid_bundle_json() -> Value {
    json!({
        "schema_version": "1.0.0",
        "analysis_job_id": "analysis_job_alpha",
        "request_id": "request_alpha",
        "artifact": {
            "artifact_name": "sample.bin",
            "original_file_name": "sample.bin",
            "artifact_size_bytes": 3,
            "artifact_sha256": "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad",
            "artifact_kind": "unknown"
        },
        "runtime": {
            "runtime_name": "quarantine-sandbox-runtime",
            "runtime_version": "0.1.0",
            "source_revision": "revision_alpha",
            "requested_profile": "static_only",
            "dynamic_execution_performed": false,
            "network_access_performed": false,
            "credentials_available": false
        },
        "disposition": "completed",
        "consumer_verdict_required": true,
        "evidence": [{
            "evidence_id": "evidence_0001",
            "sequence_number": 1,
            "evidence_kind": "artifact_identity",
            "producer_id": "runtime_core",
            "summary": "Artifact identity established.",
            "attributes": {}
        }],
        "limitations": ["runtime_does_not_determine_maliciousness"]
    })
}

fn assert_unknown_field_rejected(value: Value) {
    assert!(
        serde_json::from_value::<EvidenceBundle>(value).is_err(),
        "published evidence JSON must reject unknown fields instead of silently discarding them"
    );
}

#[test]
fn valid_published_bundle_still_deserializes() {
    let bundle = serde_json::from_value::<EvidenceBundle>(valid_bundle_json())
        .expect("published evidence shape must deserialize");
    assert_eq!(bundle.validate(), Ok(()));
}

#[test]
fn top_level_unknown_field_is_rejected() {
    let mut value = valid_bundle_json();
    value
        .as_object_mut()
        .expect("bundle fixture must be an object")
        .insert("unexpected_contract_field".to_owned(), json!(true));
    assert_unknown_field_rejected(value);
}

#[test]
fn artifact_unknown_field_is_rejected() {
    let mut value = valid_bundle_json();
    value["artifact"]
        .as_object_mut()
        .expect("artifact fixture must be an object")
        .insert("unexpected_artifact_field".to_owned(), json!(true));
    assert_unknown_field_rejected(value);
}

#[test]
fn runtime_unknown_field_is_rejected() {
    let mut value = valid_bundle_json();
    value["runtime"]
        .as_object_mut()
        .expect("runtime fixture must be an object")
        .insert("unexpected_runtime_field".to_owned(), json!(true));
    assert_unknown_field_rejected(value);
}

#[test]
fn evidence_record_unknown_field_is_rejected() {
    let mut value = valid_bundle_json();
    value["evidence"][0]
        .as_object_mut()
        .expect("evidence fixture must be an object")
        .insert("unexpected_evidence_field".to_owned(), json!(true));
    assert_unknown_field_rejected(value);
}
