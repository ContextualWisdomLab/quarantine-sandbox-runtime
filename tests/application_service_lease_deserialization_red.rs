//! RED coverage for hostile deserialization of application-service lease evidence.
//!
//! Runtime-issued evidence constructors establish loopback, isolation and
//! lifecycle invariants. Public deserialization must not bypass those invariants.

use quarantine_sandbox_runtime::{
    ApplicationServiceLease, IsolationAttestation, ServiceEndpoint,
};
use serde_json::{Value, json};

fn valid_attestation() -> Value {
    json!({
        "rootless": true,
        "read_only_root_filesystem": true,
        "all_capabilities_dropped": true,
        "no_new_privileges": true,
        "isolated_user_namespace": true,
        "external_egress_denied": true,
        "loopback_only_publication": true,
        "credentials_available": false
    })
}

fn valid_lease() -> Value {
    json!({
        "schema_version": "1.1.0",
        "request_id": "lease_deserialization_request",
        "image_reference": format!("localhost/cwl/tool@sha256:{}", "a".repeat(64)),
        "backend_id": "rootless_podman",
        "sandbox_id": "qsr-app-0123456789abcdef",
        "network_id": "qsr-net-0123456789abcdef",
        "policy_id": "application_policy_v1",
        "policy_sha256": "b".repeat(64),
        "endpoint": {
            "host": "127.0.0.1",
            "port": 8080,
            "protocol": "http"
        },
        "started_at_epoch_seconds": 1_780_000_000_u64,
        "expires_at_epoch_seconds": 1_780_000_300_u64,
        "shutdown_grace_seconds": 2,
        "isolation_attestation": valid_attestation()
    })
}

#[test]
fn public_endpoint_deserialization_rejects_non_loopback_authority() {
    let forged = json!({
        "host": "0.0.0.0",
        "port": 8080,
        "protocol": "http"
    });

    assert!(
        serde_json::from_value::<ServiceEndpoint>(forged).is_err(),
        "deserialization must not bypass the loopback-only endpoint invariant"
    );
}

#[test]
fn public_attestation_deserialization_rejects_false_p0_controls() {
    for field_name in [
        "rootless",
        "read_only_root_filesystem",
        "all_capabilities_dropped",
        "no_new_privileges",
        "isolated_user_namespace",
        "external_egress_denied",
        "loopback_only_publication",
    ] {
        let mut forged = valid_attestation();
        forged[field_name] = json!(false);
        assert!(
            serde_json::from_value::<IsolationAttestation>(forged).is_err(),
            "{field_name} must not become trusted P0 evidence through Deserialize"
        );
    }

    let mut forged = valid_attestation();
    forged["credentials_available"] = json!(true);
    assert!(
        serde_json::from_value::<IsolationAttestation>(forged).is_err(),
        "ambient credentials must not become trusted P0 evidence through Deserialize"
    );
}

#[test]
fn public_lease_deserialization_rejects_invalid_schema_endpoint_and_chronology() {
    serde_json::from_value::<ApplicationServiceLease>(valid_lease())
        .expect("the currently published valid lease shape must remain readable");

    let mut unsupported_schema = valid_lease();
    unsupported_schema["schema_version"] = json!("9.9.9");
    assert!(
        serde_json::from_value::<ApplicationServiceLease>(unsupported_schema).is_err(),
        "unsupported lease schema must fail closed"
    );

    let mut zero_port = valid_lease();
    zero_port["endpoint"]["port"] = json!(0);
    assert!(
        serde_json::from_value::<ApplicationServiceLease>(zero_port).is_err(),
        "zero-port service evidence must fail closed"
    );

    let mut impossible_chronology = valid_lease();
    impossible_chronology["expires_at_epoch_seconds"] = json!(1_780_000_000_u64);
    assert!(
        serde_json::from_value::<ApplicationServiceLease>(impossible_chronology).is_err(),
        "lease expiry must be strictly later than the observed start"
    );
}
