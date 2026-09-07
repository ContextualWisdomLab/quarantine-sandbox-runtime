//! RED contract for strict cleanup-receipt wire admission.
//!
//! A cleanup receipt is runtime-issued evidence. Public deserialization must not
//! bypass the invariants already published by `application-service-cleanup-1.0.0`.

use quarantine_sandbox_runtime::CleanupReceipt;
use serde_json::{Value, json};

fn valid_cleanup_wire() -> Value {
    json!({
        "schema_version": "1.0.0",
        "sandbox_id": "qsr-app-cleanup-red",
        "network_id": "qsr-net-cleanup-red",
        "container_removed": true,
        "network_removed": true,
        "terminated_at_epoch_seconds": 1_780_000_123_u64
    })
}

fn assert_cleanup_wire_rejected(value: Value) {
    assert!(
        serde_json::from_value::<CleanupReceipt>(value).is_err(),
        "hostile cleanup wire value must fail before materializing runtime cleanup evidence"
    );
}

#[test]
fn schema_valid_cleanup_receipt_still_deserializes() {
    let receipt: CleanupReceipt = serde_json::from_value(valid_cleanup_wire())
        .expect("schema-valid cleanup receipt should remain wire-compatible");

    assert_eq!(receipt.schema_version(), "1.0.0");
    assert_eq!(receipt.sandbox_id(), "qsr-app-cleanup-red");
    assert_eq!(receipt.network_id(), "qsr-net-cleanup-red");
    assert!(receipt.container_removed());
    assert!(receipt.network_removed());
    assert_eq!(receipt.terminated_at_epoch_seconds(), 1_780_000_123);
}

#[test]
fn cleanup_receipt_rejects_schema_and_removal_claims_not_issued_by_runtime() {
    let mut unsupported_schema = valid_cleanup_wire();
    unsupported_schema["schema_version"] = json!("9.9.9");
    assert_cleanup_wire_rejected(unsupported_schema);

    let mut container_not_removed = valid_cleanup_wire();
    container_not_removed["container_removed"] = json!(false);
    assert_cleanup_wire_rejected(container_not_removed);

    let mut network_not_removed = valid_cleanup_wire();
    network_not_removed["network_removed"] = json!(false);
    assert_cleanup_wire_rejected(network_not_removed);
}

#[test]
fn cleanup_receipt_rejects_runtime_identifiers_outside_the_published_schema() {
    for field_name in ["sandbox_id", "network_id"] {
        for invalid_value in [
            String::new(),
            "Foreign-Resource".to_owned(),
            "foreign/resource".to_owned(),
            "a".repeat(65),
        ] {
            let mut value = valid_cleanup_wire();
            value[field_name] = json!(invalid_value);
            assert_cleanup_wire_rejected(value);
        }
    }
}

#[test]
fn cleanup_receipt_rejects_unknown_top_level_members() {
    let mut value = valid_cleanup_wire();
    value
        .as_object_mut()
        .expect("cleanup receipt fixture must be an object")
        .insert("controller_claim".to_owned(), json!("forged"));
    assert_cleanup_wire_rejected(value);
}
