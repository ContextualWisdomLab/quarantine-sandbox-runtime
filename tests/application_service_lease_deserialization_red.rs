//! RED coverage for hostile deserialization of application-service lease evidence.
//!
//! Runtime-issued evidence constructors establish loopback, isolation and
//! lifecycle invariants. Public deserialization must not bypass those invariants.

use quarantine_sandbox_runtime::{ApplicationServiceLease, IsolationAttestation, ServiceEndpoint};
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
        "backend_id": "rootless_podman2",
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

    let mut zero_shutdown_grace = valid_lease();
    zero_shutdown_grace["shutdown_grace_seconds"] = json!(0);
    assert!(
        serde_json::from_value::<ApplicationServiceLease>(zero_shutdown_grace).is_err(),
        "zero shutdown grace must not materialize trusted lease evidence"
    );
}

#[test]
fn public_lease_deserialization_rejects_invalid_identifiers_and_digests() {
    for (field_name, invalid_value) in [
        ("request_id", json!("")),
        ("request_id", json!("r".repeat(129))),
        ("request_id", json!("request\ncontrol")),
        ("image_reference", json!("localhost/cwl/tool:latest")),
        (
            "image_reference",
            json!(format!("localhost/cwl/tool@sha256:{}", "A".repeat(64))),
        ),
        ("backend_id", json!("")),
        ("backend_id", json!("RootlessPodman")),
        ("backend_id", json!("a".repeat(65))),
        ("sandbox_id", json!("")),
        ("sandbox_id", json!("qsr_app_invalid")),
        ("sandbox_id", json!("a".repeat(65))),
        ("network_id", json!("")),
        ("network_id", json!("qsr_net_invalid")),
        ("network_id", json!("a".repeat(65))),
        ("policy_id", json!("")),
        ("policy_id", json!("p".repeat(129))),
        ("policy_id", json!("policy\ncontrol")),
        ("policy_sha256", json!("b".repeat(63))),
        ("policy_sha256", json!("B".repeat(64))),
        ("policy_sha256", json!("g".repeat(64))),
    ] {
        let mut forged = valid_lease();
        forged[field_name] = invalid_value;
        assert!(
            serde_json::from_value::<ApplicationServiceLease>(forged).is_err(),
            "invalid {field_name} must fail closed at the public lease wire boundary"
        );
    }
}

#[test]
fn public_deserialization_rejects_unknown_members_for_closed_wire_objects() {
    let endpoint = json!({
        "host": "127.0.0.1",
        "port": 8080,
        "protocol": "http",
        "unexpected": true
    });
    assert!(
        serde_json::from_value::<ServiceEndpoint>(endpoint).is_err(),
        "endpoint additionalProperties=false must be enforced by Rust deserialization"
    );

    let mut attestation = valid_attestation();
    attestation["unexpected"] = json!(true);
    assert!(
        serde_json::from_value::<IsolationAttestation>(attestation).is_err(),
        "attestation additionalProperties=false must be enforced by Rust deserialization"
    );

    let mut lease = valid_lease();
    lease["unexpected"] = json!(true);
    assert!(
        serde_json::from_value::<ApplicationServiceLease>(lease).is_err(),
        "lease additionalProperties=false must be enforced by Rust deserialization"
    );
}
