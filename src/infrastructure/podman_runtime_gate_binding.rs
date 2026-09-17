//! Runtime-owned Podman hold-gate binding and one-time release control.

use std::{
    io::{self, Read, Write},
    process::{Child, Command, Stdio},
    sync::mpsc::{self, RecvTimeoutError},
    thread,
};

use sha2::{Digest, Sha256};

use super::{
    podman::{RootlessPodmanAdapter, classify_spawn_failure},
    runtime_gate_artifact::RuntimeGateArtifact,
};
use crate::{
    ApplicationServiceError, CommandExecutionError, CommandExecutionRequest, IsolationPolicy,
};

const RUNTIME_GATE_CONTAINER_PATH: &str = "/qsr-runtime-gate";
const RUNTIME_GATE_RELEASE_ACK: &[u8] = b"QSR_GATE_RELEASED\n";
const RUNTIME_GATE_RELEASE_ACK_BUDGET_BYTES: usize = 64;
const RUNTIME_GATE_RELEASE_IDENTITY_OPERATION: &str = "runtime_gate_release_identity";
const RUNTIME_GATE_RELEASE_ATTACH_OPERATION: &str = "runtime_gate_release_attach";
const RUNTIME_GATE_RELEASE_WRITE_OPERATION: &str = "runtime_gate_release_write";
const RUNTIME_GATE_RELEASE_ACK_OPERATION: &str = "runtime_gate_release_ack";
const RUNTIME_GATE_RELEASE_DETACH_OPERATION: &str = "runtime_gate_release_detach";

/// Immutable command-create fragment for a release-authorized runtime gate.
///
/// This plan is deliberately narrower than a complete Podman launch plan. It carries only the
/// arguments that must replace the hostile consumer as OCI PID 1: the verified read-only gate
/// bind, the runtime-owned gate entrypoint, the immutable image reference, a fresh one-time release
/// token, and the exact consumer argv behind that token. Existing command isolation flags remain
/// owned by the canonical Podman adapter and are not copied into this value.
#[derive(Debug, PartialEq, Eq)]
pub struct RuntimeGateCommandBindingPlan {
    container_create_binding_args: Vec<String>,
    runtime_gate_sha256: String,
    runtime_gate_architecture: String,
    release_token: String,
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
/// Command execution through this adapter composes the verified gate into the canonical Podman
/// lifecycle: the gate is OCI PID 1, effective process isolation is attested while consumer argv
/// remains held, and the one-time release channel opens only after those checks succeed.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RuntimeGatePodmanAdapter {
    _inner: RootlessPodmanAdapter,
    runtime_gate_artifact: RuntimeGateArtifact,
}

impl RootlessPodmanAdapter {
    /// Bind an independently digest- and architecture-verified runtime gate artifact.
    ///
    /// The returned adapter is the release-authorized command boundary. It keeps the underlying
    /// service backend unchanged while requiring gated command composition for its own execution
    /// method.
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
    /// Run one bounded command through the verified hold/attest/release gate lifecycle.
    ///
    /// # Errors
    ///
    /// Returns [`CommandExecutionError`] if request validation, Podman isolation, gate release,
    /// completion evidence, output collection, or exact-ID cleanup fails. Consumer argv is not
    /// released until live effective process isolation has been verified.
    pub fn run_command_at(
        &self,
        request: &CommandExecutionRequest,
        policy: &IsolationPolicy,
        started_at_epoch_seconds: u64,
    ) -> Result<crate::CommandExecutionResult, CommandExecutionError> {
        let plan = self.plan_command_binding(request, policy)?;
        let binding_args = plan.container_create_binding_args().to_vec();
        self._inner.run_runtime_gate_command_at(
            request,
            policy,
            started_at_epoch_seconds,
            &binding_args,
            |container_id| self.release_command_gate(container_id, plan),
        )
    }

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
            "--interactive".to_owned(),
            "--volume".to_owned(),
            format!(
                "{}:{RUNTIME_GATE_CONTAINER_PATH}:ro",
                self.runtime_gate_artifact.path().display()
            ),
            format!("--entrypoint={RUNTIME_GATE_CONTAINER_PATH}"),
            "--".to_owned(),
            request.image_reference.clone(),
            release_token.clone(),
        ];
        container_create_binding_args.extend(request.command.iter().cloned());

        Ok(RuntimeGateCommandBindingPlan {
            container_create_binding_args,
            runtime_gate_sha256: self.runtime_gate_artifact.sha256().to_owned(),
            runtime_gate_architecture: self.runtime_gate_artifact.architecture().to_owned(),
            release_token,
        })
    }

    /// Deliver this plan's one-time release token to one exact acquired container.
    ///
    /// The attach client is an infrastructure control channel only. The method writes the token
    /// once, closes stdin so the consumer cannot inherit controller input authority, requires an
    /// explicit bounded gate acknowledgement, and then terminates only the local attach client.
    /// `--sig-proxy=false` prevents that local termination from being forwarded to the container.
    ///
    /// # Errors
    ///
    /// Returns [`CommandExecutionError`] for malformed acquired identity, attach spawn/write/read
    /// failures, acknowledgement timeout or contradiction, and attach-client reap failure.
    pub fn release_command_gate(
        &self,
        container_id: &str,
        plan: RuntimeGateCommandBindingPlan,
    ) -> Result<(), CommandExecutionError> {
        if !is_exact_container_id(container_id) {
            return Err(CommandExecutionError::Backend(
                ApplicationServiceError::MalformedIsolationInspection {
                    operation: RUNTIME_GATE_RELEASE_IDENTITY_OPERATION,
                },
            ));
        }

        let mut child = Command::new(self._inner.command_program())
            .args(["attach", "--sig-proxy=false", container_id])
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .map_err(|error| {
                CommandExecutionError::Backend(ApplicationServiceError::BackendSpawnFailed {
                    operation: RUNTIME_GATE_RELEASE_ATTACH_OPERATION,
                    failure_kind: classify_spawn_failure(error.kind()),
                })
            })?;

        let Some(stdout) = child.stdout.take() else {
            let original = release_invocation_error(RUNTIME_GATE_RELEASE_ACK_OPERATION);
            return Err(fail_after_release_client_cleanup(&mut child, original));
        };
        let Some(mut stdin) = child.stdin.take() else {
            let original = release_invocation_error(RUNTIME_GATE_RELEASE_WRITE_OPERATION);
            return Err(fail_after_release_client_cleanup(&mut child, original));
        };

        let (sender, receiver) = mpsc::sync_channel(1);
        let _reader = thread::spawn(move || {
            let _ = sender.send(read_release_ack(stdout));
        });

        let release_payload = format!("{}\n", plan.release_token);
        if stdin
            .write_all(release_payload.as_bytes())
            .and_then(|()| stdin.flush())
            .is_err()
        {
            drop(stdin);
            let original = release_invocation_error(RUNTIME_GATE_RELEASE_WRITE_OPERATION);
            return Err(fail_after_release_client_cleanup(&mut child, original));
        }
        drop(stdin);

        let acknowledgement = receiver.recv_timeout(self._inner.command_timeout());
        terminate_release_client(&mut child)?;

        match acknowledgement {
            Ok(Ok(())) => Ok(()),
            Ok(Err(_)) | Err(RecvTimeoutError::Disconnected) => {
                Err(release_invocation_error(RUNTIME_GATE_RELEASE_ACK_OPERATION))
            }
            Err(RecvTimeoutError::Timeout) => Err(CommandExecutionError::Backend(
                ApplicationServiceError::BackendCommandTimedOut {
                    operation: RUNTIME_GATE_RELEASE_ACK_OPERATION,
                },
            )),
        }
    }
}

/// Accept only the immutable 64-character lowercase Podman container ID as release authority.
fn is_exact_container_id(container_id: &str) -> bool {
    container_id.len() == 64
        && container_id
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

/// Read until the trusted gate's exact acknowledgement or the fixed control-byte budget is spent.
fn read_release_ack(mut stdout: impl Read) -> io::Result<()> {
    let mut line = Vec::with_capacity(RUNTIME_GATE_RELEASE_ACK.len());
    let mut byte = [0_u8; 1];
    for _ in 0..RUNTIME_GATE_RELEASE_ACK_BUDGET_BYTES {
        if stdout.read(&mut byte)? == 0 {
            return Err(io::Error::new(
                io::ErrorKind::UnexpectedEof,
                "runtime gate closed before release acknowledgement",
            ));
        }
        line.push(byte[0]);
        if byte[0] == b'\n' {
            if line == RUNTIME_GATE_RELEASE_ACK {
                return Ok(());
            }
            line.clear();
        }
    }
    Err(io::Error::new(
        io::ErrorKind::InvalidData,
        "runtime gate release acknowledgement exceeded the bounded control budget",
    ))
}

/// Convert an internal release-control failure into the stable provider-neutral error taxonomy.
fn release_invocation_error(operation: &'static str) -> CommandExecutionError {
    CommandExecutionError::Backend(ApplicationServiceError::BackendInvocationFailed { operation })
}

/// Terminate and reap only the local attach client, never the released container process.
///
/// `release_command_gate` always starts attach with `--sig-proxy=false`; killing this child is
/// therefore local control-channel cleanup rather than workload termination authority.
fn terminate_release_client(child: &mut Child) -> Result<(), CommandExecutionError> {
    match child.try_wait() {
        Ok(Some(_)) => return Ok(()),
        Ok(None) => {}
        Err(_) => {
            let _ = child.kill();
            let _ = child.wait();
            return Err(release_invocation_error(
                RUNTIME_GATE_RELEASE_DETACH_OPERATION,
            ));
        }
    }

    if child.kill().is_err() {
        if child.try_wait().is_ok_and(|status| status.is_some()) {
            return Ok(());
        }
        return Err(release_invocation_error(
            RUNTIME_GATE_RELEASE_DETACH_OPERATION,
        ));
    }
    child
        .wait()
        .map(|_| ())
        .map_err(|_| release_invocation_error(RUNTIME_GATE_RELEASE_DETACH_OPERATION))
}

/// Preserve cleanup failure precedence when a release-control operation already failed.
fn fail_after_release_client_cleanup(
    child: &mut Child,
    original: CommandExecutionError,
) -> CommandExecutionError {
    terminate_release_client(child).err().unwrap_or(original)
}

/// Generate an unpredictable one-time release token without a deterministic fallback.
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

    const ELF_HEADER_BYTES: usize = 64;
    const PROGRAM_HEADER_BYTES: usize = 56;
    const FILE_BYTES: usize = 512;

    fn self_contained_gate_bytes() -> Vec<u8> {
        let machine = match std::env::consts::ARCH {
            "x86_64" => 62_u16,
            "aarch64" => 183_u16,
            other => panic!("runtime-gate test fixture does not support architecture {other}"),
        };
        let mut bytes = vec![0_u8; FILE_BYTES];
        bytes[..7].copy_from_slice(b"\x7fELF\x02\x01\x01");
        bytes[16..18].copy_from_slice(&2_u16.to_le_bytes());
        bytes[18..20].copy_from_slice(&machine.to_le_bytes());
        bytes[20..24].copy_from_slice(&1_u32.to_le_bytes());
        bytes[24..32].copy_from_slice(&0x400100_u64.to_le_bytes());
        bytes[32..40].copy_from_slice(&(ELF_HEADER_BYTES as u64).to_le_bytes());
        bytes[52..54].copy_from_slice(&(ELF_HEADER_BYTES as u16).to_le_bytes());
        bytes[54..56].copy_from_slice(&(PROGRAM_HEADER_BYTES as u16).to_le_bytes());
        bytes[56..58].copy_from_slice(&1_u16.to_le_bytes());
        let header = ELF_HEADER_BYTES;
        bytes[header..header + 4].copy_from_slice(&1_u32.to_le_bytes());
        bytes[header + 4..header + 8].copy_from_slice(&5_u32.to_le_bytes());
        bytes[header + 16..header + 24].copy_from_slice(&0x400000_u64.to_le_bytes());
        bytes[header + 32..header + 40].copy_from_slice(&(FILE_BYTES as u64).to_le_bytes());
        bytes[header + 40..header + 48].copy_from_slice(&(FILE_BYTES as u64).to_le_bytes());
        bytes[header + 48..header + 56].copy_from_slice(&4096_u64.to_le_bytes());
        bytes
    }

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
        let directory = tempfile::tempdir().expect("runtime-gate fixture directory should exist");
        let source = directory.path().join("self-contained-runtime-gate");
        let bytes = self_contained_gate_bytes();
        fs::write(&source, &bytes).expect("runtime-gate fixture should be writable");
        let expected_sha256 = format!("{:x}", Sha256::digest(&bytes));
        let artifact =
            RuntimeGateArtifact::stage(&source, &expected_sha256, std::env::consts::ARCH)
                .expect("matching runtime gate artifact should stage");
        let adapter = RootlessPodmanAdapter::new("podman").with_runtime_gate_artifact(artifact);
        let plan = adapter
            .plan_command_binding(&request(), &policy())
            .expect("valid gate binding should plan");

        assert_eq!(
            plan.container_create_binding_args()
                .first()
                .map(String::as_str),
            Some("--interactive"),
            "the runtime gate reads its one-time release token from stdin, so Podman must keep container stdin open until the controller releases the held gate"
        );
    }
}
