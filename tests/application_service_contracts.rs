//! Contract and launch-plan tests for isolated application services.

use quarantine_sandbox_runtime::{
    ApplicationServiceError, ApplicationServiceRequest, IsolationPolicy, ResourceRequest,
    RootlessPodmanAdapter, ServiceProtocol,
};

fn digest_image() -> String {
    format!("localhost/cwl/tool@sha256:{}", "a".repeat(64))
}

fn policy() -> IsolationPolicy {
    IsolationPolicy {
        policy_id: "agent_application_policy_v1".to_owned(),
        maximum_memory_bytes: 512 * 1024 * 1024,
        maximum_cpu_millicores: 2_000,
        maximum_processes: 128,
        maximum_lease_seconds: 900,
        maximum_tmpfs_bytes: 128 * 1024 * 1024,
        readiness_timeout_millis: 2_000,
        readiness_poll_interval_millis: 25,
        shutdown_grace_seconds: 5,
        run_as_user_id: 65_532,
        run_as_group_id: 65_532,
    }
}

fn request() -> ApplicationServiceRequest {
    ApplicationServiceRequest {
        schema_version: "1.0.0".to_owned(),
        request_id: "agent_task_42".to_owned(),
        image_reference: digest_image(),
        container_port: 8_080,
        protocol: ServiceProtocol::Http,
        command: vec!["serve".to_owned(), "--port".to_owned(), "8080".to_owned()],
        resources: ResourceRequest {
            memory_bytes: 256 * 1024 * 1024,
            cpu_millicores: 1_000,
            maximum_processes: 64,
            lease_seconds: 300,
            tmpfs_bytes: 64 * 1024 * 1024,
        },
    }
}

#[test]
fn application_service_rejects_mutable_image_tags() {
    let mut request = request();
    request.image_reference = "docker.io/library/nginx:latest".to_owned();

    assert_eq!(
        request.validate(&policy()),
        Err(ApplicationServiceError::ImageReferenceNotDigestPinned)
    );
}

#[test]
fn application_service_rejects_resource_requests_above_policy() {
    let mut request = request();
    request.resources.memory_bytes = policy().maximum_memory_bytes + 1;

    assert_eq!(
        request.validate(&policy()),
        Err(ApplicationServiceError::ResourceLimitExceeded {
            resource_name: "memory_bytes",
        })
    );
}

#[test]
fn effective_policy_identity_changes_with_enforced_fields() {
    let first = policy();
    let mut second = first.clone();
    second.run_as_user_id += 1;

    let first_digest = first.effective_policy_sha256();
    assert_eq!(first_digest.len(), 64);
    assert_eq!(first_digest, first.clone().effective_policy_sha256());
    assert!(
        first_digest
            .bytes()
            .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
    );
    assert_ne!(first_digest, second.effective_policy_sha256());
}

#[test]
fn podman_plan_is_rootless_fail_closed_and_loopback_only() {
    let plan = RootlessPodmanAdapter::plan_at(&request(), &policy(), 1_780_000_000)
        .expect("valid application service request should create a launch plan");

    assert_eq!(
        plan.rootless_probe_args(),
        [
            "info".to_owned(),
            "--format".to_owned(),
            "{{.Host.Security.Rootless}}".to_owned(),
        ]
    );
    assert!(
        plan.network_create_args()
            .windows(2)
            .any(|pair| pair == ["--internal", "--disable-dns"])
    );
    assert!(
        !plan
            .network_create_args()
            .iter()
            .any(|argument| argument == "--ignore"),
        "per-sandbox networks must fail on a name collision instead of reusing an unverified pre-existing network"
    );

    let create = plan.container_create_args();
    for required in [
        "--pull=never",
        "--read-only",
        "--read-only-tmpfs=false",
        "--http-proxy=false",
        "--image-volume=ignore",
        "--no-hosts",
        "--no-hostname",
        "--systemd=false",
        "--sdnotify=ignore",
        "--cap-drop=all",
        "--security-opt=no-new-privileges",
        "--userns=auto",
        "--ipc=none",
        "--pid=private",
        "--restart=no",
        "--log-driver=none",
    ] {
        assert!(
            create.iter().any(|argument| argument == required),
            "missing {required}"
        );
    }
    assert!(create.windows(2).any(|pair| pair == ["--timeout", "300"]));
    assert!(create.windows(2).any(|pair| {
        pair == [
            "--label",
            &format!(
                "org.contextualwisdomlab.sandbox.policy_sha256={}",
                policy().effective_policy_sha256()
            ),
        ]
    }));
    assert!(create.iter().any(|argument| argument == "--publish"));
    assert!(
        create
            .iter()
            .any(|argument| argument == "127.0.0.1::8080/tcp")
    );
    assert!(!create.iter().any(|argument| argument == "--privileged"));
    assert!(!create.iter().any(|argument| argument == "--network=host"));
    assert!(
        !create
            .iter()
            .any(|argument| argument.contains("docker.sock"))
    );
    assert!(
        !create
            .iter()
            .any(|argument| argument.contains("podman.sock"))
    );
    assert_eq!(plan.expires_at_epoch_seconds(), 1_780_000_300);
}
