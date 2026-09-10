//! Core isolation-status observation coverage.

use quarantine_sandbox_runtime::{IsolationControlStatus, VerifiedIsolationState};

#[test]
fn verified_isolation_status_accessors_preserve_observed_tri_state() {
    let state: VerifiedIsolationState = serde_json::from_value(serde_json::json!({
        "rootless": "verified",
        "read_only_root_filesystem": "not_applicable",
        "all_capabilities_dropped": "unavailable",
        "no_new_privileges": "verified",
        "isolated_user_namespace": "not_applicable",
        "external_egress_denied": "unavailable",
        "loopback_only_publication": "verified",
        "seccomp_enforced": "not_applicable",
        "lsm_enforced": "unavailable",
        "resource_limits_verified": "verified",
        "credentials_available": false
    }))
    .expect("valid isolation observation should deserialize");

    assert_eq!(state.rootless_status(), IsolationControlStatus::Verified);
    assert_eq!(
        state.read_only_root_filesystem_status(),
        IsolationControlStatus::NotApplicable
    );
    assert_eq!(
        state.all_capabilities_dropped_status(),
        IsolationControlStatus::Unavailable
    );
    assert_eq!(
        state.no_new_privileges_status(),
        IsolationControlStatus::Verified
    );
    assert_eq!(
        state.isolated_user_namespace_status(),
        IsolationControlStatus::NotApplicable
    );
    assert_eq!(
        state.external_egress_denied_status(),
        IsolationControlStatus::Unavailable
    );
    assert_eq!(
        state.loopback_only_publication_status(),
        IsolationControlStatus::Verified
    );
    assert_eq!(
        state.seccomp_status(),
        IsolationControlStatus::NotApplicable
    );
    assert_eq!(state.lsm_status(), IsolationControlStatus::Unavailable);
    assert_eq!(
        state.resource_limits_status(),
        IsolationControlStatus::Verified
    );

    assert!(state.rootless());
    assert!(!state.read_only_root_filesystem());
    assert!(!state.all_capabilities_dropped());
    assert!(state.no_new_privileges());
    assert!(!state.isolated_user_namespace());
    assert!(!state.external_egress_denied());
    assert!(state.loopback_only_publication());
    assert!(!state.seccomp_enforced());
    assert!(!state.lsm_enforced());
    assert!(state.resource_limits_verified());
    assert!(!state.credentials_available());
}
