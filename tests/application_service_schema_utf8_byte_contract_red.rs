//! Regression contract for application-service UTF-8 byte bounds.
//!
//! Draft 2020-12 `maxLength` counts JSON-string characters, while the
//! application-service runtime bounds request identifiers and argv entries in
//! UTF-8 octets. The published validation path must therefore use a versioned
//! assertion vocabulary rather than silently treating `x-cwl-maxUtf8Bytes` as
//! an ignored extension keyword.

use std::{fs, path::Path};

use quarantine_sandbox_runtime::{
    ApplicationServiceError, ApplicationServiceRequest, IsolationPolicy, ResourceRequest,
    ServiceProtocol,
};
use serde_json::Value;

const STOCK_DRAFT_2020_12_DIALECT: &str = "https://json-schema.org/draft/2020-12/schema";

fn schema() -> Value {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("schemas/application-service-request.schema.json");
    let text = fs::read_to_string(path).expect("application-service schema must be readable");
    serde_json::from_str(&text).expect("application-service schema must be valid JSON")
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
        image_reference: format!("localhost/cwl/tool@sha256:{}", "a".repeat(64)),
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

fn stock_string_assertions_accept(property: &Value, value: &str) -> bool {
    let minimum_characters = property["minLength"]
        .as_u64()
        .expect("minLength must be an integer") as usize;
    let maximum_characters = property["maxLength"]
        .as_u64()
        .expect("maxLength must be an integer") as usize;
    let character_count = value.chars().count();

    character_count >= minimum_characters
        && character_count <= maximum_characters
        && !value
            .chars()
            .any(|character| matches!(character as u32, 0x00..=0x1f | 0x7f..=0x9f))
}

#[test]
fn public_schema_must_enforce_runtime_utf8_byte_bounds_for_request_id_and_command() {
    let schema = schema();
    let request_id_schema = &schema["properties"]["request_id"];
    let command_item_schema = &schema["properties"]["command"]["items"];

    let request_id = "é".repeat(65);
    assert_eq!(request_id.chars().count(), 65);
    assert_eq!(request_id.len(), 130);
    assert_eq!(
        request_id_schema["x-cwl-maxUtf8Bytes"].as_u64(),
        Some(128)
    );
    assert!(stock_string_assertions_accept(request_id_schema, &request_id));

    let mut request_id_candidate = request();
    request_id_candidate.request_id = request_id;
    assert_eq!(
        request_id_candidate.validate(&policy()),
        Err(ApplicationServiceError::InvalidRequestId)
    );

    let command_argument = "é".repeat(513);
    assert_eq!(command_argument.chars().count(), 513);
    assert_eq!(command_argument.len(), 1_026);
    assert_eq!(
        command_item_schema["x-cwl-maxUtf8Bytes"].as_u64(),
        Some(1_024)
    );
    assert!(stock_string_assertions_accept(
        command_item_schema,
        &command_argument
    ));

    let mut command_candidate = request();
    command_candidate.command = vec![command_argument];
    assert_eq!(
        command_candidate.validate(&policy()),
        Err(ApplicationServiceError::InvalidCommandArgument { argument_index: 0 })
    );

    assert_ne!(
        schema["$schema"].as_str(),
        Some(STOCK_DRAFT_2020_12_DIALECT),
        "stock Draft 2020-12 does not make x-cwl-maxUtf8Bytes an executable assertion"
    );
}
