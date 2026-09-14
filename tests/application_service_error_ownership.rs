//! Architecture fitness: application-service request/error vocabulary stays in its Supporting context.

use std::{fs, path::Path};

#[test]
fn application_service_error_vocabulary_stays_in_supporting_context() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let supporting = fs::read_to_string(root.join("src/application_service/mod.rs"))
        .expect("application_service source should be readable");
    let core = fs::read_to_string(root.join("src/sandbox_execution/mod.rs"))
        .expect("sandbox_execution source should be readable");
    let facade = fs::read_to_string(root.join("src/lib.rs"))
        .expect("crate facade source should be readable");
    let gate = fs::read_to_string(root.join("src/infrastructure/podman_runtime_gate_binding.rs"))
        .expect("runtime-gate adapter source should be readable");

    assert!(
        supporting.contains("pub enum ApplicationServiceError"),
        "Supporting application_service must own its public failure vocabulary"
    );
    assert!(
        !supporting.contains("SandboxRuntimeError as ApplicationServiceError"),
        "application_service must not alias its domain errors to a Core runtime enum"
    );
    assert!(
        facade.contains("ApplicationServiceError,"),
        "the public facade must export the Supporting-context error type"
    );
    assert!(
        !facade.contains("SandboxRuntimeError as ApplicationServiceError"),
        "the public facade must not recreate the Supporting error as a Core alias"
    );
    assert!(
        !gate.contains("ApplicationServiceError"),
        "Core bounded-command gate infrastructure must not name Supporting-context errors"
    );

    let runtime_error = core
        .split_once("pub enum SandboxRuntimeError {")
        .map(|(_, tail)| tail)
        .and_then(|tail| tail.split_once("\n}\n\nimpl From<SandboxExecutionError>"))
        .map(|(body, _)| body)
        .expect("Core runtime error enum should remain structurally identifiable");
    for supporting_only_variant in [
        "UnsupportedSchemaVersion",
        "InvalidRequestId",
        "ImageReferenceNotDigestPinned",
        "InvalidContainerPort",
        "TooManyCommandArguments",
        "InvalidCommandArgument",
        "LeaseExpiryOverflow",
        "RuntimeIdentityUnavailable",
        "InvalidPortMapping",
        "ReadinessTimeout",
        "CleanupAuthorityUnavailable",
    ] {
        assert!(
            !runtime_error.contains(supporting_only_variant),
            "Core SandboxRuntimeError absorbed Supporting application-service variant {supporting_only_variant}"
        );
    }
}
