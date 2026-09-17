//! Causal RED for application-service Podman option/operand separation.

use quarantine_sandbox_runtime::{
    ApplicationServiceRequest, IsolationPolicy, ResourceRequest, RootlessPodmanAdapter,
    ServiceProtocol,
};

fn policy() -> IsolationPolicy {
    IsolationPolicy {
        policy_id: "option_delimiter_policy_v1".to_owned(),
        maximum_memory_bytes: 1_024,
        maximum_cpu_millicores: 2_000,
        maximum_processes: 64,
        maximum_lease_seconds: 120,
        maximum_tmpfs_bytes: 512,
        readiness_timeout_millis: 100,
        readiness_poll_interval_millis: 10,
        shutdown_grace_seconds: 5,
        run_as_user_id: 65_532,
        run_as_group_id: 65_532,
    }
}

#[test]
fn plan_terminates_podman_options_before_untrusted_image_operand() {
    let digest = "a".repeat(64);
    let hostile_image_reference = format!("--annotation=qsr.boundary=value@sha256:{digest}");
    let fallback_image_if_reparsed = format!("localhost/cwl/fallback@sha256:{digest}");
    let request = ApplicationServiceRequest {
        schema_version: "1.0.0".to_owned(),
        request_id: "option_delimiter_request".to_owned(),
        image_reference: hostile_image_reference.clone(),
        container_port: 8_080,
        protocol: ServiceProtocol::Tcp,
        command: vec![fallback_image_if_reparsed, "serve".to_owned()],
        resources: ResourceRequest {
            memory_bytes: 512,
            cpu_millicores: 1_000,
            maximum_processes: 32,
            lease_seconds: 60,
            tmpfs_bytes: 256,
        },
    };

    assert_eq!(request.validate(&policy()), Ok(()));

    let plan = RootlessPodmanAdapter::plan_at(&request, &policy(), 1_000)
        .expect("otherwise-valid request must reach the Podman launch plan");
    let argv = plan.container_create_args();
    let image_index = argv
        .iter()
        .position(|argument| argument == &hostile_image_reference)
        .expect("validated image reference must remain one exact argv entry");

    assert!(image_index > 0, "image operand must follow Podman create options");
    assert_eq!(
        argv[image_index - 1], "--",
        "an accepted leading-dash image token must not be reinterpretable as a Podman option"
    );
    assert_eq!(
        argv.iter().filter(|argument| argument.as_str() == "--").count(),
        1,
        "the create plan needs one unambiguous option terminator"
    );
    assert_eq!(
        &argv[image_index + 1..],
        request.command.as_slice(),
        "consumer command argv must remain entry-preserving after the image operand"
    );
}
