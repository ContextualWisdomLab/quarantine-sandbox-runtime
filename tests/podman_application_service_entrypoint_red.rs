//! RED: a non-empty application-service direct argv must replace image ENTRYPOINT semantics.
//!
//! `ApplicationServiceRequest.command` is a direct-argv contract. Podman treats
//! command arguments after the image as arguments to an image-defined ENTRYPOINT,
//! so merely appending the request after the image can execute a different binary.
//! Production remains unchanged until this RED executes for that exact cause.

use quarantine_sandbox_runtime::{
    ApplicationServiceRequest, IsolationPolicy, ResourceRequest, RootlessPodmanAdapter,
    ServiceProtocol,
};

const STARTED_AT_EPOCH_SECONDS: u64 = 1_780_000_400;

fn policy() -> IsolationPolicy {
    IsolationPolicy {
        policy_id: "application_entrypoint_red_v1".to_owned(),
        maximum_memory_bytes: 256 * 1024 * 1024,
        maximum_cpu_millicores: 500,
        maximum_processes: 32,
        maximum_lease_seconds: 60,
        maximum_tmpfs_bytes: 32 * 1024 * 1024,
        readiness_timeout_millis: 500,
        readiness_poll_interval_millis: 10,
        shutdown_grace_seconds: 1,
        run_as_user_id: 65_532,
        run_as_group_id: 65_532,
    }
}

fn request() -> ApplicationServiceRequest {
    ApplicationServiceRequest {
        schema_version: "1.0.0".to_owned(),
        request_id: "application_entrypoint_red".to_owned(),
        image_reference: concat!(
            "localhost/cwl/tool@sha256:",
            "eeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeee"
        )
        .to_owned(),
        container_port: 8_080,
        protocol: ServiceProtocol::Http,
        command: vec![
            "/usr/local/bin/cwl-service".to_owned(),
            "--profile".to_owned(),
            "argument with spaces".to_owned(),
        ],
        resources: ResourceRequest {
            memory_bytes: 128 * 1024 * 1024,
            cpu_millicores: 250,
            maximum_processes: 16,
            lease_seconds: 30,
            tmpfs_bytes: 16 * 1024 * 1024,
        },
    }
}

#[test]
fn non_empty_direct_argv_replaces_image_entrypoint_without_a_second_command_layer() {
    let request = request();
    let plan = RootlessPodmanAdapter::plan_at(&request, &policy(), STARTED_AT_EPOCH_SECONDS)
        .expect("valid request and policy must yield a launch plan");
    let args = plan.container_create_args();
    let image_index = args
        .iter()
        .position(|argument| argument == &request.image_reference)
        .expect("immutable image reference must remain the image operand");
    let before_image = &args[..image_index];
    let expected_entrypoint_json =
        serde_json::to_string(&request.command).expect("validated argv must serialize as JSON");
    let separate_entrypoint = before_image.windows(2).any(|window| {
        window[0] == "--entrypoint" && window[1] == expected_entrypoint_json
    });
    let inline_entrypoint = before_image
        .iter()
        .any(|argument| argument == &format!("--entrypoint={expected_entrypoint_json}"));

    assert!(
        separate_entrypoint || inline_entrypoint,
        "non-empty direct argv must replace the image ENTRYPOINT as one exact JSON array: {args:?}"
    );
    assert_eq!(
        &args[image_index + 1..],
        &[] as &[String],
        "the same consumer argv must not also be appended after the image as COMMAND arguments: {args:?}"
    );
}
