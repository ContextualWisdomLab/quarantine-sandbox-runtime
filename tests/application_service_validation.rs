//! Exhaustive validation and deterministic-plan tests for application-service contracts.

use quarantine_sandbox_runtime::{
    ApplicationServiceError, ApplicationServiceRequest, IsolationPolicy, ResourceRequest,
    RootlessPodmanAdapter, SandboxExecutionError, ServiceProtocol,
};

fn digest_image() -> String {
    format!("localhost/cwl/tool@sha256:{}", "a".repeat(64))
}

fn policy() -> IsolationPolicy {
    IsolationPolicy {
        policy_id: "application_policy_v1".to_owned(),
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

fn request() -> ApplicationServiceRequest {
    ApplicationServiceRequest {
        schema_version: "1.0.0".to_owned(),
        request_id: "request_001".to_owned(),
        image_reference: digest_image(),
        container_port: 8_080,
        protocol: ServiceProtocol::Tcp,
        command: vec!["serve".to_owned()],
        resources: ResourceRequest {
            memory_bytes: 512,
            cpu_millicores: 1_000,
            maximum_processes: 32,
            lease_seconds: 60,
            tmpfs_bytes: 256,
        },
    }
}

fn assert_resource_error(mut candidate: ApplicationServiceRequest, expected: &'static str) {
    assert_eq!(
        candidate.validate(&policy()),
        Err(ApplicationServiceError::ResourceLimitExceeded {
            resource_name: expected,
        })
    );
    candidate.resources = request().resources;
    assert_eq!(candidate.validate(&policy()), Ok(()));
}

#[test]
fn stable_wire_codes_and_backend_defaults_are_exposed() {
    assert_eq!(ServiceProtocol::Tcp.as_str(), "tcp");
    assert_eq!(ServiceProtocol::Http.as_str(), "http");
    assert_eq!(RootlessPodmanAdapter::backend_id(), "rootless_podman");
    let _adapter = RootlessPodmanAdapter::default();
}

#[test]
fn request_validation_rejects_schema_identity_and_port_errors() {
    let mut candidate = request();
    candidate.schema_version = "2.0.0".to_owned();
    assert_eq!(
        candidate.validate(&policy()),
        Err(ApplicationServiceError::UnsupportedSchemaVersion {
            actual_version: "2.0.0".to_owned(),
        })
    );

    for invalid_id in [String::new(), "x".repeat(129), "bad\nid".to_owned()] {
        let mut candidate = request();
        candidate.request_id = invalid_id;
        assert_eq!(
            candidate.validate(&policy()),
            Err(ApplicationServiceError::InvalidRequestId)
        );
    }

    let mut candidate = request();
    candidate.container_port = 0;
    assert_eq!(
        candidate.validate(&policy()),
        Err(ApplicationServiceError::InvalidContainerPort)
    );
}

#[test]
fn request_validation_rejects_every_mutable_or_malformed_image_identity_class() {
    let invalid_images = [
        String::new(),
        "x".repeat(513),
        format!("localhost/tool @sha256:{}", "a".repeat(64)),
        format!("localhost/tool\n@sha256:{}", "a".repeat(64)),
        "localhost/tool:latest".to_owned(),
        format!("@sha256:{}", "a".repeat(64)),
        format!("localhost/tool@sha256:{}", "a".repeat(63)),
        format!("localhost/tool@sha256:{}", "A".repeat(64)),
        format!("localhost/tool@sha256:{}", "g".repeat(64)),
    ];
    for image_reference in invalid_images {
        let mut candidate = request();
        candidate.image_reference = image_reference;
        assert_eq!(
            candidate.validate(&policy()),
            Err(ApplicationServiceError::ImageReferenceNotDigestPinned)
        );
    }
    assert_eq!(request().validate(&policy()), Ok(()));
}

#[test]
fn request_validation_rejects_unbounded_or_control_command_arguments() {
    let mut candidate = request();
    candidate.command = (0..65).map(|index| format!("arg_{index}")).collect();
    assert_eq!(
        candidate.validate(&policy()),
        Err(ApplicationServiceError::TooManyCommandArguments {
            maximum_arguments: 64,
        })
    );

    for argument in [String::new(), "x".repeat(1_025), "bad\narg".to_owned()] {
        let mut candidate = request();
        candidate.command = vec![argument];
        assert_eq!(
            candidate.validate(&policy()),
            Err(ApplicationServiceError::InvalidCommandArgument { argument_index: 0 })
        );
    }
}

#[test]
fn isolation_policy_rejects_every_invalid_bound() {
    let mut candidate = policy();
    candidate.policy_id.clear();
    assert_eq!(
        candidate.validate(),
        Err(SandboxExecutionError::InvalidPolicy {
            field_name: "policy_id",
        })
    );
    let mut candidate = policy();
    candidate.policy_id = "x".repeat(129);
    assert!(candidate.validate().is_err());
    let mut candidate = policy();
    candidate.policy_id = "bad\npolicy".to_owned();
    assert!(candidate.validate().is_err());

    let field_cases: [(&str, fn(&mut IsolationPolicy)); 11] = [
        ("maximum_memory_bytes", |value| value.maximum_memory_bytes = 0),
        ("maximum_cpu_millicores", |value| value.maximum_cpu_millicores = 0),
        ("maximum_processes", |value| value.maximum_processes = 0),
        ("maximum_lease_seconds", |value| value.maximum_lease_seconds = 0),
        ("maximum_tmpfs_bytes", |value| value.maximum_tmpfs_bytes = 0),
        ("readiness_timeout_millis", |value| value.readiness_timeout_millis = 0),
        ("readiness_poll_interval_millis", |value| value.readiness_poll_interval_millis = 0),
        ("readiness_poll_interval_millis", |value| {
            value.readiness_poll_interval_millis = value.readiness_timeout_millis + 1;
        }),
        ("shutdown_grace_seconds", |value| value.shutdown_grace_seconds = 0),
        ("run_as_user_id", |value| value.run_as_user_id = 0),
        ("run_as_group_id", |value| value.run_as_group_id = 0),
    ];
    for (field_name, mutate) in field_cases {
        let mut candidate = policy();
        mutate(&mut candidate);
        assert_eq!(
            candidate.validate(),
            Err(SandboxExecutionError::InvalidPolicy { field_name })
        );
    }
    assert_eq!(policy().validate(), Ok(()));
}

#[test]
fn resource_request_rejects_zero_and_over_policy_for_every_dimension() {
    for value in [0, policy().maximum_memory_bytes + 1] {
        let mut candidate = request();
        candidate.resources.memory_bytes = value;
        assert_resource_error(candidate, "memory_bytes");
    }
    for value in [0, policy().maximum_cpu_millicores + 1] {
        let mut candidate = request();
        candidate.resources.cpu_millicores = value;
        assert_resource_error(candidate, "cpu_millicores");
    }
    for value in [0, policy().maximum_processes + 1] {
        let mut candidate = request();
        candidate.resources.maximum_processes = value;
        assert_resource_error(candidate, "maximum_processes");
    }
    for value in [0, policy().maximum_lease_seconds + 1] {
        let mut candidate = request();
        candidate.resources.lease_seconds = value;
        assert_resource_error(candidate, "lease_seconds");
    }
    for value in [0, policy().maximum_tmpfs_bytes + 1] {
        let mut candidate = request();
        candidate.resources.tmpfs_bytes = value;
        assert_resource_error(candidate, "tmpfs_bytes");
    }
}

#[test]
fn launch_plan_is_deterministic_and_rejects_expiry_overflow() {
    let first = RootlessPodmanAdapter::plan_at(&request(), &policy(), 1_000)
        .expect("valid plan should be created");
    let second = RootlessPodmanAdapter::plan_at(&request(), &policy(), 1_000)
        .expect("same inputs should create a plan");
    assert_eq!(first, second);
    assert!(first.sandbox_name().starts_with("qsr-app-"));
    assert!(first.network_name().starts_with("qsr-net-"));

    let different_time = RootlessPodmanAdapter::plan_at(&request(), &policy(), 1_001)
        .expect("different start time should still create a plan");
    assert_ne!(first.sandbox_name(), different_time.sandbox_name());
    assert_ne!(first.network_name(), different_time.network_name());

    assert_eq!(
        RootlessPodmanAdapter::plan_at(&request(), &policy(), u64::MAX),
        Err(ApplicationServiceError::LeaseExpiryOverflow)
    );
}
