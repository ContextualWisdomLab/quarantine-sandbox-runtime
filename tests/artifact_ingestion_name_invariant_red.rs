//! Regression contract for artifact-ingestion name admission.
//!
//! Ingestion is an admission boundary: it must not return an artifact whose
//! canonical descriptor rejects the same caller-supplied identity.

use quarantine_sandbox_runtime::{IngestionError, IngestionPolicy, ingest_bytes};

#[test]
fn ingestion_rejects_name_policy_wider_than_descriptor_contract() {
    let policy = IngestionPolicy {
        maximum_artifact_bytes: 1,
        maximum_artifact_name_bytes: 256,
    };
    let name = "a".repeat(256);

    assert_eq!(
        ingest_bytes(&name, b"x", &policy),
        Err(IngestionError::InvalidPolicy {
            policy_field: "maximum_artifact_name_bytes",
        })
    );
}

#[test]
fn ingestion_rejects_names_that_are_not_leaf_file_names() {
    for invalid_name in [
        ".",
        "..",
        "folder/sample.bin",
        "folder\\sample.bin",
        "../sample.bin",
    ] {
        assert!(
            ingest_bytes(invalid_name, b"x", &IngestionPolicy::default()).is_err(),
            "ingestion accepted non-leaf artifact name {invalid_name:?}"
        );
    }
}

#[test]
fn accepted_unicode_leaf_name_produces_a_valid_descriptor() {
    let artifact = ingest_bytes("검증-자료.bin", b"x", &IngestionPolicy::default())
        .expect("valid Unicode leaf name within the UTF-8 byte bound must be accepted");

    artifact
        .descriptor()
        .validate()
        .expect("ingestion must not manufacture a descriptor that violates its domain invariant");
}
