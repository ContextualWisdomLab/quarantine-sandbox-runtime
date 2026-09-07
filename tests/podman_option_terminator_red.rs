//! RED coverage for the Podman CLI option/data boundary.
//!
//! Direct argv avoids shell parsing, but consumer-controlled image/command data must
//! also be placed after Podman's option terminator so it cannot become CLI options.

use quarantine_sandbox_runtime::{
    ApplicationServiceRequest, IsolationPolicy, ResourceRequest, RootlessPodmanAdapter,
    ServiceProtocol,
};

fn policy() -> IsolationPolicy {
    IsolationPolicy {
        policy_id: "podman_option_terminator_v1".to_owned(),
        maximum_memory_bytes: 512 * 1024 * 1024,
        maximum_cpu_millicores: 2_000,
        maximum_processes: 128,
        maximum_lease_seconds: 900,
        maximum_tmpfs_bytes: 128 * 1024 * 1024,
        readiness_timeout_millis: 100,
        readiness_poll_interval_millis: 10,
        shutdown_grace_seconds: 2,
        run_as_user_id: 65_532,
        run_as_group_id: 65_532,
    }
}

#[test]
fn consumer_image_and_command_are_after_the_podman_option_terminator() {
    let image_reference = format!("-consumer/tool@sha256:{}", "a".repeat(64));
    let request = ApplicationServiceRequest {
        schema_version: "1.0.0".to_owned(),
        request_id: "podman_option_terminator_red".to_owned(),
        image_reference: image_reference.clone(),
        container_port: 8_080,
        protocol: ServiceProtocol::Tcp,
        command: vec!["--consumer-command".to_owned(), "value with spaces".to_owned()],
        resources: ResourceRequest {
            memory_bytes: 256 * 1024 * 1024,
            cpu_millicores: 1_000,
            maximum_processes: 32,
            lease_seconds: 300,
            tmpfs_bytes: 32 * 1024 * 1024,
        },
    };

    let plan = RootlessPodmanAdapter::plan_at(&request, &policy(), 1_780_000_000)
        .expect("the digest-pinned consumer image is valid domain intent");
    let create_args = plan.container_create_args();
    let image_index = create_args
        .iter()
        .position(|argument| argument == &image_reference)
        .expect("the exact consumer image must remain a positional operand");

    assert!(image_index > 0, "the image must not be the first create argument");
    assert_eq!(
        create_args[image_index - 1], "--",
        "Podman option parsing must terminate immediately before consumer image data"
    );
    assert_eq!(
        &create_args[image_index + 1..],
        request.command.as_slice(),
        "consumer command entries must remain exact argv after the image boundary"
    );
}
