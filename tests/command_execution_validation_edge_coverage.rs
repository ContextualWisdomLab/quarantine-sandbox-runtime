//! Public command-execution validation edge coverage.
//!
//! These cases exercise each short-circuit branch independently so the contract
//! does not rely on one representative invalid value for a compound predicate.

use quarantine_sandbox_runtime::{
    CONTRACT_SCHEMA_VERSION, CommandExecutionError, CommandExecutionRequest, IsolationPolicy,
    ResourceRequest,
};

fn policy() -> IsolationPolicy {
    IsolationPolicy {
        policy_id: "command_validation_edge_coverage".to_owned(),
        maximum_memory_bytes: 512 * 1024 * 1024,
        maximum_cpu_millicores: 2_000,
        maximum_processes: 64,
        maximum_lease_seconds: 300,
        maximum_tmpfs_bytes: 64 * 1024 * 1024,
        readiness_timeout_millis: 1_000,
        readiness_poll_interval_millis: 10,
        shutdown_grace_seconds: 2,
        run_as_user_id: 65_532,
        run_as_group_id: 65_532,
    }
}

fn request() -> CommandExecutionRequest {
    CommandExecutionRequest {
        schema_version: CONTRACT_SCHEMA_VERSION.to_owned(),
        request_id: "command-validation-edge".to_owned(),
        image_reference: format!("localhost/cwl/tool@sha256:{}", "a".repeat(64)),
        command: vec!["payload".to_owned()],
        source_artifact: None,
        resources: ResourceRequest {
            memory_bytes: 256 * 1024 * 1024,
            cpu_millicores: 1_000,
            maximum_processes: 16,
            lease_seconds: 20,
            tmpfs_bytes: 16 * 1024 * 1024,
        },
    }
}

#[test]
fn request_identifier_short_circuit_edges_fail_closed_independently() {
    for invalid_request_id in [String::new(), "x".repeat(129), "bad\nid".to_owned()] {
        let mut candidate = request();
        candidate.request_id = invalid_request_id;
        assert_eq!(
            candidate.validate(&policy()),
            Err(CommandExecutionError::InvalidRequestId)
        );
    }

    let mut maximum_boundary = request();
    maximum_boundary.request_id = "x".repeat(128);
    assert_eq!(maximum_boundary.validate(&policy()), Ok(()));
}

#[test]
fn command_shape_short_circuit_edges_fail_closed_independently() {
    let mut empty_command = request();
    empty_command.command.clear();
    assert_eq!(
        empty_command.validate(&policy()),
        Err(CommandExecutionError::EmptyCommand)
    );

    let mut too_many_arguments = request();
    too_many_arguments.command = (0..65).map(|index| format!("arg-{index}")).collect();
    assert_eq!(
        too_many_arguments.validate(&policy()),
        Err(CommandExecutionError::TooManyCommandArguments {
            maximum_arguments: 64,
        })
    );

    for invalid_argument in [String::new(), "x".repeat(1_025), "bad\narg".to_owned()] {
        let mut candidate = request();
        candidate.command = vec!["first".to_owned(), invalid_argument];
        assert_eq!(
            candidate.validate(&policy()),
            Err(CommandExecutionError::InvalidCommandArgument { argument_index: 1 })
        );
    }

    let mut maximum_boundaries = request();
    maximum_boundaries.command = (0..64).map(|_| "x".repeat(1_024)).collect();
    assert_eq!(maximum_boundaries.validate(&policy()), Ok(()));
}

#[test]
fn schema_and_image_rejections_remain_distinct_from_argument_validation() {
    let mut unsupported_schema = request();
    unsupported_schema.schema_version = "2.0.0".to_owned();
    assert_eq!(
        unsupported_schema.validate(&policy()),
        Err(CommandExecutionError::UnsupportedSchemaVersion {
            actual_version: "2.0.0".to_owned(),
        })
    );

    let mut mutable_image = request();
    mutable_image.image_reference = "localhost/cwl/tool:latest".to_owned();
    assert_eq!(
        mutable_image.validate(&policy()),
        Err(CommandExecutionError::ImageReferenceNotDigestPinned)
    );
}
