use sha2::{Digest, Sha256};

use super::{podman::RootlessPodmanAdapter, runtime_gate_artifact::RuntimeGateArtifact};
use crate::{
    ApplicationServiceError, CommandExecutionError, CommandExecutionRequest, IsolationPolicy,
};

const RUNTIME_GATE_CONTAINER_PATH: &str = "/qsr-runtime-gate";

/// Immutable command-create fragment for a release-authorized runtime gate.
///
/// This plan is deliberately narrower than a complete Podman launch plan. It carries only the
/// arguments that must replace the hostile consumer as OCI PID 1: the verified read-only gate
/// bind, the runtime-owned gate entrypoint, the immutable image reference, a fresh one-time release
/// token, and the exact consumer argv behind that token. Existing command isolation flags remain
/// owned by the canonical Podman adapter and are not copied into this value.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RuntimeGateCommandBindingPlan {
    container_create_binding_args: Vec<String>,
    runtime_gate_sha256: String,
    runtime_gate_architecture: String,
}

impl RuntimeGateCommandBindingPlan {
    /// Return the exact create arguments that bind and select the runtime-owned gate.
    #[must_use]
    pub fn container_create_binding_args(&self) -> &[String] {
        &self.container_create_binding_args
    }

    /// Return the independently verified gate digest attached to this plan.
    #[must_use]
    pub fn runtime_gate_sha256(&self) -> &str {
        &self.runtime_gate_sha256
    }

    /// Return the independently verified gate architecture attached to this plan.
    #[must_use]
    pub fn runtime_gate_architecture(&self) -> &str {
        &self.runtime_gate_architecture
    }
}

/// A Podman command adapter carrying one independently verified runtime gate artifact.
///
/// This is an intentionally incomplete issue #25 integration boundary. The wrapped canonical
/// adapter retains ownership of command execution; this type only admits construction of the
/// immutable gate-binding fragment. It does not start a container or claim that the bounded
/// release channel exists.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RuntimeGatePodmanAdapter {
    _inner: RootlessPodmanAdapter,
    runtime_gate_artifact: RuntimeGateArtifact,
}

impl RootlessPodmanAdapter {
    /// Bind an independently digest- and architecture-verified runtime gate artifact.
    ///
    /// The returned adapter exposes only the gate-binding plan until the canonical command runtime
    /// owns a bounded attest-and-release channel. The legacy command execution method remains
    /// unchanged and must not be treated as issue #25 GREEN.
    #[must_use]
    pub fn with_runtime_gate_artifact(
        self,
        runtime_gate_artifact: RuntimeGateArtifact,
    ) -> RuntimeGatePodmanAdapter {
        RuntimeGatePodmanAdapter {
            _inner: self,
            runtime_gate_artifact,
        }
    }
}

impl RuntimeGatePodmanAdapter {
    /// Build the runtime-owned gate fragment without invoking Podman or releasing consumer code.
    ///
    /// # Errors
    ///
    /// Returns [`CommandExecutionError`] when the request violates policy or a cryptographically
    /// random one-time release token cannot be generated. A plan is never produced with a guessed
    /// or deterministic fallback token.
    pub fn plan_command_binding(
        &self,
        request: &CommandExecutionRequest,
        policy: &IsolationPolicy,
    ) -> Result<RuntimeGateCommandBindingPlan, CommandExecutionError> {
        request.validate(policy)?;
        let release_token = runtime_gate_release_token()?;
        let mut container_create_binding_args = vec![
            "--volume".to_owned(),
            format!(
                "{}:{RUNTIME_GATE_CONTAINER_PATH}:ro",
                self.runtime_gate_artifact.path().display()
            ),
            format!("--entrypoint={RUNTIME_GATE_CONTAINER_PATH}"),
            request.image_reference.clone(),
            release_token,
        ];
        container_create_binding_args.extend(request.command.iter().cloned());

        Ok(RuntimeGateCommandBindingPlan {
            container_create_binding_args,
            runtime_gate_sha256: self.runtime_gate_artifact.sha256().to_owned(),
            runtime_gate_architecture: self.runtime_gate_artifact.architecture().to_owned(),
        })
    }
}

fn runtime_gate_release_token() -> Result<String, CommandExecutionError> {
    let mut nonce = [0_u8; 32];
    getrandom::fill(&mut nonce).map_err(|_| {
        CommandExecutionError::Backend(ApplicationServiceError::BackendInvocationFailed {
            operation: "runtime_gate_release_token",
        })
    })?;
    Ok(format!("{:x}", Sha256::digest(nonce)))
}

#[cfg(all(test, target_os = "linux"))]
mod tests {
    use std::fs;

    use sha2::{Digest, Sha256};

    use super::RootlessPodmanAdapter;
    use crate::{CommandExecutionRequest, IsolationPolicy, ResourceRequest, RuntimeGateArtifact};

    fn policy() -> IsolationPolicy {
        IsolationPolicy {
            policy_id: "runtime_gate_stdin_policy_v1".to_owned(),
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
            schema_version: "1.0.0".to_owned(),
            request_id: "runtime-gate-stdin-request".to_owned(),
            image_reference: format!("localhost/cwl/tool@sha256:{}", "a".repeat(64)),
            command: vec!["payload-sentinel".to_owned()],
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
    fn gate_binding_keeps_container_stdin_open_for_bounded_release() {
        let source = std::env::current_exe().expect("current test executable should exist");
        let bytes = fs::read(&source).expect("current test executable should be readable");
        let expected_sha256 = format!("{:x}", Sha256::digest(bytes));
        let artifact = RuntimeGateArtifact::stage(&source, &expected_sha256, std::env::consts::ARCH)
            .expect("matching runtime gate artifact should stage");
        let adapter = RootlessPodmanAdapter::new("podman").with_runtime_gate_artifact(artifact);
        let plan = adapter
            .plan_command_binding(&request(), &policy())
            .expect("valid gate binding should plan");

        assert_eq!(
            plan.container_create_binding_args().first().map(String::as_str),
            Some("--interactive"),
            "the runtime gate reads its one-time release token from stdin, so Podman must keep container stdin open until the controller releases the held gate"
        );
    }
}
