//! Public lease-owner identity validation edge coverage.
//!
//! Each short-circuit condition is exercised independently so caller scoping
//! cannot depend on one representative invalid identity.

use quarantine_sandbox_runtime::{ApplicationServiceCoordinatorError, LeaseOwnerId};

#[test]
fn lease_owner_identity_bounds_fail_closed_independently() {
    for invalid in [
        "".to_owned(),
        "x".repeat(129),
        "owner with space".to_owned(),
    ] {
        assert_eq!(
            LeaseOwnerId::new(&invalid),
            Err(ApplicationServiceCoordinatorError::InvalidLeaseOwnerId),
        );
    }

    let non_ascii = "owner-é";
    assert_eq!(
        LeaseOwnerId::new(non_ascii),
        Err(ApplicationServiceCoordinatorError::InvalidLeaseOwnerId),
    );

    let maximum = "x".repeat(128);
    assert_eq!(
        LeaseOwnerId::new(&maximum)
            .expect("the exact maximum bounded ASCII owner should be admitted")
            .as_str(),
        maximum,
    );
}

#[test]
fn lease_owner_identity_accepts_each_supported_separator() {
    for owner in [
        "agent.alpha",
        "agent_alpha",
        "urn:agent",
        "team/agent",
        "agent@example",
        "agent-alpha",
    ] {
        assert_eq!(
            LeaseOwnerId::new(owner)
                .expect("documented separator should remain part of the public owner grammar")
                .as_str(),
            owner,
        );
    }
}
