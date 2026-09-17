//! CI contract for real production-gated command-runtime acceptance.
//!
//! The dedicated positive-LSM lane is release evidence only when it executes the same
//! [`RuntimeGatePodmanAdapter`](quarantine_sandbox_runtime::RuntimeGatePodmanAdapter) boundary
//! exposed to consumers. Legacy rootless-Podman probes remain useful diagnostics, but they cannot
//! substitute for an exact production hold/attest/release/timeout lifecycle witness.

#[test]
fn positive_lsm_lane_executes_the_production_gated_timeout_witness() {
    let workflow = include_str!("../.github/workflows/ci.yml");

    assert!(
        workflow.contains("--test podman_command_execution_e2e"),
        "positive-LSM CI must execute the real command-runtime E2E binary"
    );
    assert!(
        workflow.contains(
            "--exact production_gated_command_execution_kills_and_reports_a_command_that_exceeds_its_timeout"
        ),
        "positive-LSM CI must execute the production RuntimeGatePodmanAdapter timeout witness"
    );
}
