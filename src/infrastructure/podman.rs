//! Rootless Podman infrastructure adapter for isolated application services.

use std::{
    net::{Ipv4Addr, SocketAddrV4, TcpStream},
    path::PathBuf,
    process::Output,
    thread,
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

use serde::{Deserialize, Deserializer};
use sha2::{Digest, Sha256};

use super::bounded_command::{BoundedCommandError, BoundedCommandRunner};
use crate::{
    ApplicationServiceError, ApplicationServiceLease, ApplicationServiceRequest, CleanupReceipt,
    CommandExecutionError, CommandExecutionRequest, CommandExecutionResult, IsolationControlStatus,
    IsolationPolicy, ResourceRequest, ServiceEndpoint, VerifiedIsolationState,
    application_service::CommandExecutionOutcome, sandbox_execution::RuntimeLeaseMetadata,
};

const PODMAN_BACKEND_ID: &str = "rootless_podman";
const DEFAULT_COMMAND_TIMEOUT: Duration = Duration::from_secs(30);
const DEFAULT_COMMAND_OUTPUT_LIMIT_BYTES: usize = 64 * 1024;

#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
struct PodmanInfo {
    host: PodmanHost,
    version: PodmanVersion,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
struct PodmanHost {
    security: PodmanSecurity,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
struct PodmanSecurity {
    rootless: bool,
    #[serde(rename = "seccompEnabled")]
    seccomp_enabled: bool,
    #[serde(rename = "seccompProfilePath")]
    seccomp_profile_path: String,
    #[serde(default, rename = "apparmorEnabled")]
    apparmor_enabled: bool,
    #[serde(default, rename = "selinuxEnabled")]
    selinux_enabled: bool,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
struct PodmanVersion {
    #[serde(rename = "Version", alias = "version")]
    version: String,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
struct ContainerInspection {
    #[serde(rename = "Id")]
    id: String,
    #[serde(default, rename = "AppArmorProfile")]
    apparmor_profile: String,
    #[serde(default, rename = "ProcessLabel")]
    process_label: String,
    #[serde(
        default,
        rename = "EffectiveCaps",
        deserialize_with = "null_as_default"
    )]
    effective_caps: Vec<String>,
    #[serde(default, rename = "BoundingCaps", deserialize_with = "null_as_default")]
    bounding_caps: Vec<String>,
    #[serde(rename = "Config")]
    config: ContainerConfig,
    #[serde(rename = "HostConfig")]
    host_config: ContainerHostConfig,
}

/// Treat an explicit JSON `null` the same as a missing key: fall back to `T::default()`.
///
/// `#[serde(default)]` alone only covers a *missing* key. Podman has been
/// observed (6.1.0, unlike the CI-pinned 5.8.4) to emit an explicit JSON
/// `null` for `EffectiveCaps`/`BoundingCaps` once every capability is
/// dropped, which `#[serde(default)]` alone does not tolerate for a
/// non-`Option` field. Both representations mean the same fact -- no
/// capabilities -- so both must deserialize to the same empty value.
fn null_as_default<'de, D, T>(deserializer: D) -> Result<T, D::Error>
where
    D: Deserializer<'de>,
    T: Deserialize<'de> + Default,
{
    Ok(Option::deserialize(deserializer)?.unwrap_or_default())
}

#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
struct ContainerConfig {
    #[serde(rename = "User")]
    user: String,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
struct ContainerHostConfig {
    #[serde(rename = "ReadonlyRootfs")]
    readonly_rootfs: bool,
    #[serde(rename = "Privileged")]
    privileged: bool,
    #[serde(default, rename = "SecurityOpt")]
    security_opt: Option<Vec<String>>,
    #[serde(rename = "UsernsMode")]
    userns_mode: String,
    // Podman 6.1.0 was observed to leave `HostConfig.UsernsMode` empty for a
    // container created with `--userns=auto`, unlike the CI-pinned 5.8.4,
    // recording the effective mode only as this annotation instead.
    #[serde(default, rename = "Annotations")]
    annotations: std::collections::BTreeMap<String, String>,
    #[serde(rename = "PidMode")]
    pid_mode: String,
    #[serde(rename = "IpcMode")]
    ipc_mode: String,
    #[serde(rename = "Memory")]
    memory: u64,
    #[serde(rename = "NanoCpus")]
    nano_cpus: u64,
    #[serde(rename = "PidsLimit")]
    pids_limit: i64,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
struct NetworkInspection {
    internal: bool,
    #[serde(default)]
    dns_enabled: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct ProcessSecurityEvidence {
    seccomp: String,
    effective_caps: String,
    bounding_caps: String,
    inheritable_caps: String,
    permitted_caps: String,
    ambient_caps: String,
    lsm_label: String,
}

/// Deterministic, auditable rootless Podman command plan.
///
/// Creating a plan performs no process execution. It exists so the domain
/// contract and isolation controls can be reviewed before a process adapter is
/// allowed to execute them.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PodmanLaunchPlan {
    rootless_probe_args: Vec<String>,
    network_create_args: Vec<String>,
    container_create_args: Vec<String>,
    sandbox_name: String,
    network_name: String,
    expires_at_epoch_seconds: u64,
}

impl PodmanLaunchPlan {
    /// Return the exact arguments used to inspect backend security capability.
    #[must_use]
    pub fn rootless_probe_args(&self) -> &[String] {
        &self.rootless_probe_args
    }

    /// Return the exact arguments used to create the per-sandbox internal network.
    #[must_use]
    pub fn network_create_args(&self) -> &[String] {
        &self.network_create_args
    }

    /// Return the exact arguments used to create the isolated service container.
    #[must_use]
    pub fn container_create_args(&self) -> &[String] {
        &self.container_create_args
    }

    /// Return the deterministic sandbox identifier used by the backend adapter.
    #[must_use]
    pub fn sandbox_name(&self) -> &str {
        &self.sandbox_name
    }

    /// Return the deterministic per-sandbox network identifier.
    #[must_use]
    pub fn network_name(&self) -> &str {
        &self.network_name
    }

    /// Return the absolute lease expiry timestamp used by runtime supervision.
    #[must_use]
    pub const fn expires_at_epoch_seconds(&self) -> u64 {
        self.expires_at_epoch_seconds
    }
}

/// First OCI infrastructure adapter for the application-service profile.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RootlessPodmanAdapter {
    program: PathBuf,
    command_timeout: Duration,
    command_output_limit_bytes: usize,
}

impl RootlessPodmanAdapter {
    /// Construct an adapter for a specific Podman-compatible executable.
    ///
    /// The executable is invoked directly with argv; this adapter never uses a shell.
    /// Individual CLI calls have a 30-second wall-clock budget and retain at most
    /// 64 KiB from each output stream unless an operator-facing builder overrides it.
    #[must_use]
    pub fn new(program: impl Into<PathBuf>) -> Self {
        Self {
            program: program.into(),
            command_timeout: DEFAULT_COMMAND_TIMEOUT,
            command_output_limit_bytes: DEFAULT_COMMAND_OUTPUT_LIMIT_BYTES,
        }
    }

    /// Override the wall-clock budget for each Podman CLI operation.
    ///
    /// This is primarily useful for environment-specific runtime profiles and
    /// deterministic tests. A zero duration intentionally makes every command
    /// fail closed immediately rather than disabling the deadline.
    #[must_use]
    pub const fn with_command_timeout(mut self, timeout: Duration) -> Self {
        self.command_timeout = timeout;
        self
    }

    /// Override the maximum retained bytes for each Podman stdout/stderr stream.
    ///
    /// The reader always drains the pipe; bytes beyond this budget are discarded
    /// and the command is terminated with a typed fail-closed error.
    #[must_use]
    pub const fn with_command_output_limit_bytes(mut self, output_limit_bytes: usize) -> Self {
        self.command_output_limit_bytes = output_limit_bytes;
        self
    }

    /// Return the stable backend identifier emitted in runtime receipts.
    #[must_use]
    pub const fn backend_id() -> &'static str {
        PODMAN_BACKEND_ID
    }

    /// Build a deterministic fail-closed Podman launch plan without executing it.
    ///
    /// # Errors
    ///
    /// Returns [`ApplicationServiceError`] when the request or policy is invalid,
    /// or when the requested lease cannot be represented as an absolute expiry.
    pub fn plan_at(
        request: &ApplicationServiceRequest,
        policy: &IsolationPolicy,
        started_at_epoch_seconds: u64,
    ) -> Result<PodmanLaunchPlan, ApplicationServiceError> {
        request.validate(policy)?;
        let expires_at_epoch_seconds = started_at_epoch_seconds
            .checked_add(u64::from(request.resources.lease_seconds))
            .ok_or(ApplicationServiceError::LeaseExpiryOverflow)?;
        let identity = sandbox_identity(
            &request.request_id,
            &request.image_reference,
            &policy.policy_id,
            started_at_epoch_seconds,
        );
        let sandbox_name = format!("qsr-app-{identity}");
        let network_name = format!("qsr-net-{identity}");

        let rootless_probe_args = vec!["info".to_owned(), "--format".to_owned(), "json".to_owned()];
        let network_create_args = vec![
            "network".to_owned(),
            "create".to_owned(),
            "--internal".to_owned(),
            "--disable-dns".to_owned(),
            network_name.clone(),
        ];
        let mut container_create_args = vec![
            "create".to_owned(),
            "--name".to_owned(),
            sandbox_name.clone(),
            "--pull=never".to_owned(),
            "--read-only".to_owned(),
            "--read-only-tmpfs=false".to_owned(),
            "--http-proxy=false".to_owned(),
            "--image-volume=ignore".to_owned(),
            "--no-hosts".to_owned(),
            "--no-hostname".to_owned(),
            "--systemd=false".to_owned(),
            "--sdnotify=ignore".to_owned(),
            "--cap-drop=all".to_owned(),
            "--security-opt=no-new-privileges".to_owned(),
            "--userns=auto".to_owned(),
            "--ipc=none".to_owned(),
            "--pid=private".to_owned(),
            "--uts=private".to_owned(),
            "--cgroupns=private".to_owned(),
            "--restart=no".to_owned(),
            "--log-driver=none".to_owned(),
            "--timeout".to_owned(),
            request.resources.lease_seconds.to_string(),
            "--user".to_owned(),
            format!("{}:{}", policy.run_as_user_id, policy.run_as_group_id),
            "--pids-limit".to_owned(),
            request.resources.maximum_processes.to_string(),
            "--memory".to_owned(),
            request.resources.memory_bytes.to_string(),
            "--cpus".to_owned(),
            cpu_limit(request.resources.cpu_millicores),
            "--tmpfs".to_owned(),
            format!(
                "/tmp:rw,noexec,nosuid,nodev,size={}",
                request.resources.tmpfs_bytes
            ),
            "--network".to_owned(),
            network_name.clone(),
            "--publish".to_owned(),
            format!("127.0.0.1::{}/tcp", request.container_port),
            "--label".to_owned(),
            format!("org.contextualwisdomlab.sandbox.identity={identity}"),
            "--label".to_owned(),
            format!(
                "org.contextualwisdomlab.sandbox.policy={}",
                policy.policy_id
            ),
            "--label".to_owned(),
            format!(
                "org.contextualwisdomlab.sandbox.policy_sha256={}",
                policy.effective_policy_sha256()
            ),
            request.image_reference.clone(),
        ];
        container_create_args.extend(request.command.iter().cloned());

        Ok(PodmanLaunchPlan {
            rootless_probe_args,
            network_create_args,
            container_create_args,
            sandbox_name,
            network_name,
            expires_at_epoch_seconds,
        })
    }

    /// Launch one isolated application service and wait for bounded readiness.
    ///
    /// # Errors
    ///
    /// Returns [`ApplicationServiceError`] when Podman cannot prove every P0
    /// isolation invariant, launch or inspection fails, readiness times out, or
    /// cleanup after a partial launch cannot be proven complete.
    pub fn launch_at(
        &self,
        request: &ApplicationServiceRequest,
        policy: &IsolationPolicy,
        started_at_epoch_seconds: u64,
    ) -> Result<ApplicationServiceLease, ApplicationServiceError> {
        let plan = Self::plan_at(request, policy, started_at_epoch_seconds)?;
        let info_output =
            self.checked_output("backend_security_info", plan.rootless_probe_args())?;
        let info: PodmanInfo = parse_json("backend_security_info", &info_output.stdout)?;
        validate_backend_security(&info)?;

        self.checked_output("network_create", plan.network_create_args())?;
        let create_output =
            match self.checked_output("container_create", plan.container_create_args()) {
                Ok(output) => output,
                Err(error) => {
                    self.cleanup_network(&plan)?;
                    return Err(error);
                }
            };
        let container_id = match parse_backend_identifier(&create_output.stdout) {
            Some(identifier) => identifier,
            None => {
                self.cleanup_created_container(&plan)?;
                return Err(ApplicationServiceError::MalformedIsolationInspection {
                    operation: "container_create",
                });
            }
        };

        let start_args = ["start".to_owned(), plan.sandbox_name().to_owned()];
        if let Err(error) = self.checked_output("container_start", &start_args) {
            self.cleanup_created_container(&plan)?;
            return Err(error);
        }

        let verified =
            match self.verify_effective_isolation(&plan, request, policy, &info, &container_id) {
                Ok(value) => value,
                Err(error) => {
                    self.cleanup_started_container(&plan, policy.shutdown_grace_seconds)?;
                    return Err(error);
                }
            };

        if wait_for_readiness(verified.host_port, policy).is_err() {
            self.cleanup_started_container(&plan, policy.shutdown_grace_seconds)?;
            return Err(ApplicationServiceError::ReadinessTimeout);
        }

        Ok(ApplicationServiceLease::new(
            request,
            RuntimeLeaseMetadata {
                backend_id: PODMAN_BACKEND_ID,
                backend_version: info.version.version,
                sandbox_id: plan.sandbox_name().to_owned(),
                network_id: plan.network_name().to_owned(),
                policy_id: policy.policy_id.clone(),
                policy_sha256: policy.effective_policy_sha256(),
                started_at_epoch_seconds,
                expires_at_epoch_seconds: plan.expires_at_epoch_seconds(),
                shutdown_grace_seconds: policy.shutdown_grace_seconds,
                isolation_state: verified.state,
            },
            ServiceEndpoint::loopback(verified.host_port, request.protocol),
        ))
    }

    /// Stop a leased service and remove all runtime-owned isolation resources.
    ///
    /// Cleanup attempts every required operation before returning failure. Cleanup
    /// commands use the same wall-clock and output bounds as launch commands.
    ///
    /// # Errors
    ///
    /// Returns [`ApplicationServiceError::CleanupFailed`] when container or
    /// network removal cannot be proven successful.
    pub fn terminate_at(
        &self,
        lease: &ApplicationServiceLease,
        terminated_at_epoch_seconds: u64,
    ) -> Result<CleanupReceipt, ApplicationServiceError> {
        let stop_args = [
            "stop".to_owned(),
            "--time".to_owned(),
            lease.shutdown_grace_seconds().to_string(),
            lease.sandbox_id().to_owned(),
        ];
        let remove_args = [
            "rm".to_owned(),
            "--force".to_owned(),
            lease.sandbox_id().to_owned(),
        ];
        let network_args = [
            "network".to_owned(),
            "rm".to_owned(),
            "--force".to_owned(),
            lease.network_id().to_owned(),
        ];
        let stop_ok = self.command_succeeded(&stop_args);
        let remove_ok = self.command_succeeded(&remove_args);
        let network_ok = self.command_succeeded(&network_args);
        if !(stop_ok && remove_ok && network_ok) {
            return Err(ApplicationServiceError::CleanupFailed);
        }
        Ok(CleanupReceipt::complete(lease, terminated_at_epoch_seconds))
    }

    /// Run one bounded command to completion inside a fresh isolated sandbox.
    ///
    /// Unlike [`Self::launch_at`], this does not create a per-sandbox network
    /// or publish a port: the sandbox runs with no network namespace
    /// attachment at all (`--network none`), which is a strictly stronger
    /// egress denial than the service profile's internal, DNS-disabled
    /// network and needs no separate network object to create or clean up.
    ///
    /// The sandbox is started detached, not attached (unlike a plain
    /// `podman run`/`start --attach`), because `Self::verify_command_isolation`
    /// must inspect the container's *running* process (`podman top`, which
    /// errors on a not-yet-started container) before this method trusts it
    /// enough to let the requested command actually execute. Completion is
    /// observed with `podman wait` (bounded by the requested lease) and
    /// output with `podman logs` against a `k8s-file`-backed log driver,
    /// rather than by attaching to the workload's live pipes: a detached
    /// `podman start --attach` on an already-running container does not
    /// reliably mirror the container's own exit code (observed directly
    /// against Podman 6.1.0), while `podman wait`'s stdout is the
    /// authoritative post-exit code for both the ordinary and the
    /// forcibly-killed-on-timeout path.
    ///
    /// # Errors
    ///
    /// Returns [`CommandExecutionError`] when Podman cannot prove the
    /// required P0 isolation invariants, the sandbox cannot be created, or
    /// cleanup after the run cannot be proven complete.
    pub fn run_command_at(
        &self,
        request: &CommandExecutionRequest,
        policy: &IsolationPolicy,
        started_at_epoch_seconds: u64,
    ) -> Result<CommandExecutionResult, CommandExecutionError> {
        request.validate(policy)?;
        let info_output = self.checked_output(
            "backend_security_info",
            &["info".to_owned(), "--format".to_owned(), "json".to_owned()],
        )?;
        let info: PodmanInfo = parse_json("backend_security_info", &info_output.stdout)?;
        validate_backend_security(&info)?;

        let identity = command_sandbox_identity(
            &request.request_id,
            &request.image_reference,
            &policy.policy_id,
            started_at_epoch_seconds,
        )?;
        let sandbox_name = format!("qsr-cmd-{identity}");

        let mut create_args = vec![
            "create".to_owned(),
            "--name".to_owned(),
            sandbox_name.clone(),
            "--pull=never".to_owned(),
            "--read-only".to_owned(),
            "--read-only-tmpfs=false".to_owned(),
            "--http-proxy=false".to_owned(),
            "--image-volume=ignore".to_owned(),
            "--no-hosts".to_owned(),
            "--systemd=false".to_owned(),
            "--sdnotify=ignore".to_owned(),
            "--cap-drop=all".to_owned(),
            "--security-opt=no-new-privileges".to_owned(),
            "--userns=auto".to_owned(),
            "--ipc=none".to_owned(),
            "--pid=private".to_owned(),
            "--uts=private".to_owned(),
            "--cgroupns=private".to_owned(),
            "--restart=no".to_owned(),
            // Unlike the service profile's `--log-driver=none`: bounded
            // command output is retrieved after the fact with `podman logs`,
            // which needs a log driver that actually retains output.
            "--log-driver=k8s-file".to_owned(),
            "--network".to_owned(),
            "none".to_owned(),
            "--timeout".to_owned(),
            request.resources.lease_seconds.to_string(),
            "--user".to_owned(),
            format!("{}:{}", policy.run_as_user_id, policy.run_as_group_id),
            "--pids-limit".to_owned(),
            request.resources.maximum_processes.to_string(),
            "--memory".to_owned(),
            request.resources.memory_bytes.to_string(),
            "--cpus".to_owned(),
            cpu_limit(request.resources.cpu_millicores),
            "--tmpfs".to_owned(),
            format!(
                "/tmp:rw,noexec,nosuid,nodev,size={}",
                request.resources.tmpfs_bytes
            ),
            "--label".to_owned(),
            format!("org.contextualwisdomlab.sandbox.identity={identity}"),
            "--label".to_owned(),
            format!(
                "org.contextualwisdomlab.sandbox.policy={}",
                policy.policy_id
            ),
            "--label".to_owned(),
            format!(
                "org.contextualwisdomlab.sandbox.policy_sha256={}",
                policy.effective_policy_sha256()
            ),
            request.image_reference.clone(),
        ];
        create_args.extend(request.command.iter().cloned());

        let create_output = match self.checked_output("container_create", &create_args) {
            Ok(output) => output,
            Err(error) => return Err(self.cleanup_or_report(&sandbox_name, error.into())),
        };
        let container_id = match parse_backend_identifier(&create_output.stdout) {
            Some(identifier) => identifier,
            None => {
                return Err(self.cleanup_or_report(
                    &sandbox_name,
                    CommandExecutionError::Backend(
                        ApplicationServiceError::MalformedIsolationInspection {
                            operation: "container_create",
                        },
                    ),
                ));
            }
        };

        // Once a command container might exist, every later failure must attempt
        // cleanup. If cleanup cannot be proven, leak risk takes precedence over
        // the earlier backend/isolation/log error and the call fails closed with
        // `CleanupFailed`.
        if let Err(error) = self.checked_output(
            "container_start",
            &["start".to_owned(), sandbox_name.clone()],
        ) {
            return Err(self.cleanup_or_report(&sandbox_name, error.into()));
        }

        if let Err(error) =
            self.verify_command_isolation(&sandbox_name, request, policy, &info, &container_id)
        {
            return Err(self.cleanup_or_report(&sandbox_name, error.into()));
        }

        let (exit_code, timed_out) = match self.wait_for_command(&sandbox_name, request) {
            Ok(outcome) => outcome,
            Err(error) => return Err(self.cleanup_or_report(&sandbox_name, error)),
        };

        let logs_runner =
            BoundedCommandRunner::new(self.command_timeout, self.command_output_limit_bytes);
        let logs_outcome = match logs_runner
            .run_to_completion(&self.program, &["logs".to_owned(), sandbox_name.clone()])
        {
            Ok(outcome) => outcome,
            Err(_) => {
                return Err(self.cleanup_or_report(
                    &sandbox_name,
                    CommandExecutionError::Backend(
                        ApplicationServiceError::BackendInvocationFailed {
                            operation: "container_logs",
                        },
                    ),
                ));
            }
        };
        if logs_outcome.timed_out {
            // The log driver failing to hand back already-written output
            // promptly is an infrastructure fault, not a fact about the
            // workload (whose own timeout is already captured above).
            return Err(self.cleanup_or_report(
                &sandbox_name,
                CommandExecutionError::Backend(ApplicationServiceError::BackendCommandTimedOut {
                    operation: "container_logs",
                }),
            ));
        }
        // A `None` status only occurs on the timeout/output-budget kill
        // paths already handled above (or by the truncation flags on
        // success); anything else is `podman logs` itself failing (e.g. the
        // log driver or container state is broken) and is an infrastructure
        // fault, not empty workload output.
        if !logs_outcome.status.is_none_or(|status| status.success()) {
            return Err(self.cleanup_or_report(
                &sandbox_name,
                CommandExecutionError::Backend(ApplicationServiceError::BackendCommandFailed {
                    operation: "container_logs",
                }),
            ));
        }

        let cleanup_ok = self
            .cleanup_created_command_container(&sandbox_name)
            .is_ok();
        if !cleanup_ok {
            return Err(CommandExecutionError::Backend(
                ApplicationServiceError::CleanupFailed,
            ));
        }

        let finished_at_epoch_seconds = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_or(started_at_epoch_seconds, |duration| duration.as_secs());

        Ok(CommandExecutionResult::new(
            request,
            CommandExecutionOutcome {
                backend_id: PODMAN_BACKEND_ID,
                backend_version: info.version.version,
                sandbox_id: sandbox_name,
                exit_code,
                timed_out,
                stdout: String::from_utf8_lossy(&logs_outcome.stdout).into_owned(),
                stdout_truncated: logs_outcome.stdout_truncated,
                stderr: String::from_utf8_lossy(&logs_outcome.stderr).into_owned(),
                stderr_truncated: logs_outcome.stderr_truncated,
                started_at_epoch_seconds,
                finished_at_epoch_seconds,
            },
        ))
    }

    /// Wait for the running sandbox to exit, bounded by its requested lease,
    /// and return its authoritative `(exit_code, timed_out)`.
    ///
    /// On a wall-clock timeout the sandbox is force-killed and waited on
    /// again (bounded by the adapter's ordinary short administrative
    /// timeout) to obtain the post-kill exit code. The `podman wait` process
    /// itself must succeed and stay within its tiny output budget before its
    /// stdout is accepted as workload evidence; wrapper failure is a backend
    /// failure, never a workload exit status.
    fn wait_for_command(
        &self,
        sandbox_name: &str,
        request: &CommandExecutionRequest,
    ) -> Result<(i32, bool), CommandExecutionError> {
        let run_timeout = Duration::from_secs(u64::from(request.resources.lease_seconds));
        let wait_runner = BoundedCommandRunner::new(run_timeout, self.command_output_limit_bytes);
        let wait_outcome = wait_runner
            .run_to_completion(&self.program, &["wait".to_owned(), sandbox_name.to_owned()])
            .map_err(|_| {
                CommandExecutionError::Backend(ApplicationServiceError::BackendInvocationFailed {
                    operation: "command_wait",
                })
            })?;

        if !wait_outcome.timed_out {
            if wait_outcome.stdout_truncated || wait_outcome.stderr_truncated {
                return Err(CommandExecutionError::Backend(
                    ApplicationServiceError::BackendOutputLimitExceeded {
                        operation: "command_wait",
                    },
                ));
            }
            match wait_outcome.status {
                Some(status) if status.success() => {
                    return Ok((
                        parse_wait_exit_code(&wait_outcome.stdout, "command_wait")?,
                        false,
                    ));
                }
                Some(_) => {
                    return Err(CommandExecutionError::Backend(
                        ApplicationServiceError::BackendCommandFailed {
                            operation: "command_wait",
                        },
                    ));
                }
                None => {
                    return Err(CommandExecutionError::Backend(
                        ApplicationServiceError::BackendInvocationFailed {
                            operation: "command_wait",
                        },
                    ));
                }
            }
        }

        // Best effort: the container may already be exiting on its own.
        let _ = self.command_succeeded(&["kill".to_owned(), sandbox_name.to_owned()]);
        let post_kill = self
            .checked_output(
                "command_wait_after_kill",
                &["wait".to_owned(), sandbox_name.to_owned()],
            )
            .map_err(CommandExecutionError::Backend)?;
        Ok((
            parse_wait_exit_code(&post_kill.stdout, "command_wait_after_kill")?,
            true,
        ))
    }

    /// Verify the same P0 isolation invariants as [`Self::verify_effective_isolation`]
    /// for a command-execution sandbox, minus the network/port controls that
    /// do not apply to a `--network none` one-shot command.
    ///
    /// A bounded command may exit before `podman top` samples its live PID 1.
    /// That race does not authorize weaker evidence: static `container inspect`
    /// records configured/requested state, not the effective per-process
    /// seccomp/LSM/capability state required at this security boundary. When
    /// live process evidence is unavailable this method therefore fails closed.
    /// Supporting extremely short-lived commands in the future requires a
    /// reviewed start/hold/attest/release handshake (or an equivalent stronger
    /// backend primitive), not a static-only attestation fallback.
    fn verify_command_isolation(
        &self,
        sandbox_name: &str,
        request: &CommandExecutionRequest,
        policy: &IsolationPolicy,
        info: &PodmanInfo,
        container_id: &str,
    ) -> Result<(), ApplicationServiceError> {
        let container_args = [
            "container".to_owned(),
            "inspect".to_owned(),
            "--format".to_owned(),
            "json".to_owned(),
            sandbox_name.to_owned(),
        ];
        let container_output = self.checked_output("container_inspect", &container_args)?;
        let container: ContainerInspection =
            parse_single_inspection("container_inspect", &container_output.stdout)?;
        if container.id != container_id {
            return Err(ApplicationServiceError::MalformedIsolationInspection {
                operation: "container_inspect",
            });
        }

        require_control(
            "read_only_root_filesystem",
            container.host_config.readonly_rootfs,
        )?;
        require_control("unprivileged_container", !container.host_config.privileged)?;
        let security_options = container
            .host_config
            .security_opt
            .as_deref()
            .unwrap_or_default();
        require_control(
            "no_new_privileges",
            security_options
                .iter()
                .any(|option| option == "no-new-privileges" || option == "no-new-privileges=true"),
        )?;
        require_control(
            "isolated_user_namespace",
            isolated_user_namespace_verified(&container.host_config),
        )?;
        require_control(
            "isolated_pid_namespace",
            container.host_config.pid_mode == "private",
        )?;
        require_control(
            "isolated_ipc_namespace",
            container.host_config.ipc_mode == "none",
        )?;
        require_control(
            "non_root_identity",
            container.config.user
                == format!("{}:{}", policy.run_as_user_id, policy.run_as_group_id),
        )?;
        require_control(
            "resource_limits",
            resource_limits_match(&container.host_config, &request.resources),
        )?;

        let process_args = [
            "top".to_owned(),
            sandbox_name.to_owned(),
            "pid".to_owned(),
            "seccomp".to_owned(),
            "capeff".to_owned(),
            "capbnd".to_owned(),
            "capinh".to_owned(),
            "capprm".to_owned(),
            "capamb".to_owned(),
            "label".to_owned(),
        ];
        // A one-shot command can exit before `top` samples it, unlike the
        // long-lived service-lease path this mirrors (`verify_effective_isolation`),
        // which never falls back either. Static `container inspect` configuration
        // proves what was requested, not the effective per-process seccomp/LSM/
        // capability state the release boundary requires positive proof of --
        // fail closed instead of downgrading to it when live evidence is gone.
        let live_process_output = self.checked_output("process_security_top", &process_args)?;
        let process = parse_process_security_top(&live_process_output.stdout)?;

        require_control(
            "all_capabilities_dropped",
            container.effective_caps.is_empty()
                && container.bounding_caps.is_empty()
                && process_capabilities_empty(&process),
        )?;
        require_control(
            "seccomp",
            !security_options
                .iter()
                .any(|option| option == "seccomp=unconfined")
                && matches!(
                    process.seccomp.to_ascii_lowercase().as_str(),
                    "filter" | "strict"
                ),
        )?;
        require_control("lsm", effective_lsm_verified(info, &container, &process))?;
        Ok(())
    }

    fn cleanup_created_command_container(
        &self,
        sandbox_name: &str,
    ) -> Result<(), ApplicationServiceError> {
        // `--ignore` makes cleanup idempotent: a failed create may or may not
        // have persisted the deterministic name, and absence is also a proven
        // leak-free terminal state. Podman documents `rm --ignore` for exactly
        // this absent-container case.
        let remove_args = [
            "rm".to_owned(),
            "--force".to_owned(),
            "--ignore".to_owned(),
            sandbox_name.to_owned(),
        ];
        if self.command_succeeded(&remove_args) {
            Ok(())
        } else {
            Err(ApplicationServiceError::CleanupFailed)
        }
    }

    /// Attempt cleanup for a failed command-execution attempt and return the
    /// error to surface: a failed cleanup means a container is leaked, which
    /// must never be hidden behind `original`, the earlier backend error
    /// that triggered this cleanup in the first place.
    fn cleanup_or_report(
        &self,
        sandbox_name: &str,
        original: CommandExecutionError,
    ) -> CommandExecutionError {
        match self.cleanup_created_command_container(sandbox_name) {
            Ok(()) => original,
            Err(_) => CommandExecutionError::Backend(ApplicationServiceError::CleanupFailed),
        }
    }

    fn verify_effective_isolation(
        &self,
        plan: &PodmanLaunchPlan,
        request: &ApplicationServiceRequest,
        policy: &IsolationPolicy,
        info: &PodmanInfo,
        container_id: &str,
    ) -> Result<VerifiedLaunch, ApplicationServiceError> {
        let container_args = [
            "container".to_owned(),
            "inspect".to_owned(),
            "--format".to_owned(),
            "json".to_owned(),
            plan.sandbox_name().to_owned(),
        ];
        let container_output = self.checked_output("container_inspect", &container_args)?;
        let container: ContainerInspection =
            parse_single_inspection("container_inspect", &container_output.stdout)?;
        if container.id != container_id {
            return Err(ApplicationServiceError::MalformedIsolationInspection {
                operation: "container_inspect",
            });
        }

        let process_args = [
            "top".to_owned(),
            plan.sandbox_name().to_owned(),
            "pid".to_owned(),
            "seccomp".to_owned(),
            "capeff".to_owned(),
            "capbnd".to_owned(),
            "capinh".to_owned(),
            "capprm".to_owned(),
            "capamb".to_owned(),
            "label".to_owned(),
        ];
        let process_output = self.checked_output("process_security_top", &process_args)?;
        let process = parse_process_security_top(&process_output.stdout)?;

        require_control(
            "read_only_root_filesystem",
            container.host_config.readonly_rootfs,
        )?;
        require_control("unprivileged_container", !container.host_config.privileged)?;
        require_control(
            "all_capabilities_dropped",
            container.effective_caps.is_empty()
                && container.bounding_caps.is_empty()
                && process_capabilities_empty(&process),
        )?;
        let security_options = container
            .host_config
            .security_opt
            .as_deref()
            .unwrap_or_default();
        require_control(
            "no_new_privileges",
            security_options
                .iter()
                .any(|option| option == "no-new-privileges" || option == "no-new-privileges=true"),
        )?;
        require_control(
            "seccomp",
            !security_options
                .iter()
                .any(|option| option == "seccomp=unconfined")
                && matches!(
                    process.seccomp.to_ascii_lowercase().as_str(),
                    "filter" | "strict"
                ),
        )?;
        require_control(
            "isolated_user_namespace",
            isolated_user_namespace_verified(&container.host_config),
        )?;
        require_control(
            "isolated_pid_namespace",
            container.host_config.pid_mode == "private",
        )?;
        require_control(
            "isolated_ipc_namespace",
            container.host_config.ipc_mode == "none",
        )?;
        require_control(
            "non_root_identity",
            container.config.user
                == format!("{}:{}", policy.run_as_user_id, policy.run_as_group_id),
        )?;
        require_control(
            "resource_limits",
            resource_limits_match(&container.host_config, &request.resources),
        )?;
        require_control("lsm", effective_lsm_verified(info, &container, &process))?;

        let network_args = [
            "network".to_owned(),
            "inspect".to_owned(),
            "--format".to_owned(),
            "json".to_owned(),
            plan.network_name().to_owned(),
        ];
        let network_output = self.checked_output("network_inspect", &network_args)?;
        let network: NetworkInspection =
            parse_single_inspection("network_inspect", &network_output.stdout)?;
        require_control(
            "external_egress_denied",
            network.internal && !network.dns_enabled,
        )?;

        let port_args = [
            "port".to_owned(),
            plan.sandbox_name().to_owned(),
            format!("{}/tcp", request.container_port),
        ];
        let port_output = self.checked_output("port_query", &port_args)?;
        let host_port = parse_loopback_port(&port_output.stdout)
            .ok_or(ApplicationServiceError::InvalidPortMapping)?;

        Ok(VerifiedLaunch {
            host_port,
            state: VerifiedIsolationState {
                rootless: IsolationControlStatus::Verified,
                read_only_root_filesystem: IsolationControlStatus::Verified,
                all_capabilities_dropped: IsolationControlStatus::Verified,
                no_new_privileges: IsolationControlStatus::Verified,
                isolated_user_namespace: IsolationControlStatus::Verified,
                external_egress_denied: IsolationControlStatus::Verified,
                loopback_only_publication: IsolationControlStatus::Verified,
                seccomp_enforced: IsolationControlStatus::Verified,
                lsm_enforced: IsolationControlStatus::Verified,
                resource_limits_verified: IsolationControlStatus::Verified,
                credentials_available: false,
            },
        })
    }

    fn command_runner(&self) -> BoundedCommandRunner {
        BoundedCommandRunner::new(self.command_timeout, self.command_output_limit_bytes)
    }

    fn checked_output(
        &self,
        operation: &'static str,
        args: &[String],
    ) -> Result<Output, ApplicationServiceError> {
        let output =
            self.command_runner()
                .run(&self.program, args)
                .map_err(|error| match error {
                    BoundedCommandError::Timeout => {
                        ApplicationServiceError::BackendCommandTimedOut { operation }
                    }
                    BoundedCommandError::OutputLimit => {
                        ApplicationServiceError::BackendOutputLimitExceeded { operation }
                    }
                    BoundedCommandError::Spawn
                    | BoundedCommandError::Wait
                    | BoundedCommandError::Capture => {
                        ApplicationServiceError::BackendInvocationFailed { operation }
                    }
                })?;
        if !output.status.success() {
            return Err(ApplicationServiceError::BackendCommandFailed { operation });
        }
        Ok(output)
    }

    fn command_succeeded(&self, args: &[String]) -> bool {
        self.command_runner()
            .run(&self.program, args)
            .is_ok_and(|output| output.status.success())
    }

    fn cleanup_network(&self, plan: &PodmanLaunchPlan) -> Result<(), ApplicationServiceError> {
        let args = [
            "network".to_owned(),
            "rm".to_owned(),
            "--force".to_owned(),
            plan.network_name().to_owned(),
        ];
        if self.command_succeeded(&args) {
            Ok(())
        } else {
            Err(ApplicationServiceError::CleanupFailed)
        }
    }

    fn cleanup_created_container(
        &self,
        plan: &PodmanLaunchPlan,
    ) -> Result<(), ApplicationServiceError> {
        let remove_args = [
            "rm".to_owned(),
            "--force".to_owned(),
            plan.sandbox_name().to_owned(),
        ];
        let container_removed = self.command_succeeded(&remove_args);
        let network_removed = self.cleanup_network(plan).is_ok();
        if container_removed && network_removed {
            Ok(())
        } else {
            Err(ApplicationServiceError::CleanupFailed)
        }
    }

    fn cleanup_started_container(
        &self,
        plan: &PodmanLaunchPlan,
        shutdown_grace_seconds: u32,
    ) -> Result<(), ApplicationServiceError> {
        let stop_args = [
            "stop".to_owned(),
            "--time".to_owned(),
            shutdown_grace_seconds.to_string(),
            plan.sandbox_name().to_owned(),
        ];
        let remove_args = [
            "rm".to_owned(),
            "--force".to_owned(),
            plan.sandbox_name().to_owned(),
        ];
        let stopped = self.command_succeeded(&stop_args);
        let container_removed = self.command_succeeded(&remove_args);
        let network_removed = self.cleanup_network(plan).is_ok();
        if stopped && container_removed && network_removed {
            Ok(())
        } else {
            Err(ApplicationServiceError::CleanupFailed)
        }
    }
}

impl Default for RootlessPodmanAdapter {
    fn default() -> Self {
        Self::new("podman")
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct VerifiedLaunch {
    host_port: u16,
    state: VerifiedIsolationState,
}

fn validate_backend_security(info: &PodmanInfo) -> Result<(), ApplicationServiceError> {
    if !info.host.security.rootless {
        return Err(ApplicationServiceError::BackendNotRootless);
    }
    require_control(
        "seccomp",
        info.host.security.seccomp_enabled && !info.host.security.seccomp_profile_path.is_empty(),
    )?;
    require_control(
        "lsm",
        info.host.security.apparmor_enabled || info.host.security.selinux_enabled,
    )?;
    if info.version.version.is_empty()
        || info.version.version.len() > 128
        || info.version.version.chars().any(char::is_control)
    {
        return Err(ApplicationServiceError::MalformedIsolationInspection {
            operation: "backend_security_info",
        });
    }
    Ok(())
}

fn effective_lsm_verified(
    info: &PodmanInfo,
    container: &ContainerInspection,
    process: &ProcessSecurityEvidence,
) -> bool {
    let runtime_label = process.lsm_label.trim();
    if runtime_label.is_empty() || runtime_label.eq_ignore_ascii_case("unconfined") {
        return false;
    }

    if info.host.security.selinux_enabled {
        let inspect_label = container.process_label.trim();
        return !inspect_label.is_empty()
            && !inspect_label.eq_ignore_ascii_case("unconfined")
            && inspect_label == runtime_label;
    }

    if info.host.security.apparmor_enabled {
        let inspect_profile = container.apparmor_profile.trim();
        // `/proc/<pid>/attr/current` exposes the current task's AppArmor
        // context as `<profile> (<mode>)`. A bare profile name is not positive
        // enforcement evidence, and complain mode audits without enforcing.
        let Some((runtime_profile, mode_with_suffix)) = runtime_label.rsplit_once(" (") else {
            return false;
        };
        let Some(mode) = mode_with_suffix.strip_suffix(')') else {
            return false;
        };
        let runtime_profile = runtime_profile.trim();
        return mode.eq_ignore_ascii_case("enforce")
            && !runtime_profile.is_empty()
            && !inspect_profile.is_empty()
            && !inspect_profile.eq_ignore_ascii_case("unconfined")
            && inspect_profile == runtime_profile;
    }

    false
}

fn process_capabilities_empty(process: &ProcessSecurityEvidence) -> bool {
    [
        process.effective_caps.as_str(),
        process.bounding_caps.as_str(),
        process.inheritable_caps.as_str(),
        process.permitted_caps.as_str(),
        process.ambient_caps.as_str(),
    ]
    .into_iter()
    .all(capability_set_is_empty)
}

fn capability_set_is_empty(value: &str) -> bool {
    let normalized = value.trim();
    if normalized.is_empty()
        || normalized == "-"
        || normalized.eq_ignore_ascii_case("none")
        || normalized == "0"
        || normalized.eq_ignore_ascii_case("0x0")
    {
        return true;
    }
    let hexadecimal = normalized.strip_prefix("0x").unwrap_or(normalized);
    !hexadecimal.is_empty() && hexadecimal.chars().all(|character| character == '0')
}

fn parse_process_security_top(
    bytes: &[u8],
) -> Result<ProcessSecurityEvidence, ApplicationServiceError> {
    let text = std::str::from_utf8(bytes).map_err(|_| {
        ApplicationServiceError::MalformedIsolationInspection {
            operation: "process_security_top",
        }
    })?;
    let mut matched = None;
    for line in text.lines().skip(1).filter(|line| !line.trim().is_empty()) {
        let fields: Vec<&str> = line.split_whitespace().collect();
        if fields.first().copied() != Some("1") {
            continue;
        }
        if fields.len() < 8 || matched.is_some() {
            return Err(ApplicationServiceError::MalformedIsolationInspection {
                operation: "process_security_top",
            });
        }
        matched = Some(ProcessSecurityEvidence {
            seccomp: fields[1].to_owned(),
            effective_caps: fields[2].to_owned(),
            bounding_caps: fields[3].to_owned(),
            inheritable_caps: fields[4].to_owned(),
            permitted_caps: fields[5].to_owned(),
            ambient_caps: fields[6].to_owned(),
            // Podman 6.1.0 was observed to pass through a trailing NUL byte
            // from the underlying NUL-terminated `/proc/<pid>/attr/current`
            // kernel interface into this column; a real LSM label never
            // legitimately contains one, so it is always safe to strip.
            lsm_label: fields[7..].join(" ").replace('\0', ""),
        });
    }
    matched.ok_or(ApplicationServiceError::MalformedIsolationInspection {
        operation: "process_security_top",
    })
}

/// Confirm the container was created with an isolated (non-host) user namespace.
///
/// Checks both representations Podman has been observed to use across
/// versions: the CI-pinned 5.8.4 reports `HostConfig.UsernsMode == "auto"`
/// directly, while 6.1.0 leaves that field empty and records the same fact
/// only as the `io.podman.annotations.userns` annotation.
fn isolated_user_namespace_verified(host_config: &ContainerHostConfig) -> bool {
    host_config.userns_mode == "auto"
        || host_config
            .annotations
            .get("io.podman.annotations.userns")
            .is_some_and(|value| value == "auto")
}

fn resource_limits_match(effective: &ContainerHostConfig, resources: &ResourceRequest) -> bool {
    let requested_nano_cpus = u64::from(resources.cpu_millicores) * 1_000_000;
    effective.memory > 0
        && effective.memory <= resources.memory_bytes
        && effective.nano_cpus > 0
        && effective.nano_cpus <= requested_nano_cpus
        && effective.pids_limit > 0
        && u64::try_from(effective.pids_limit)
            .is_ok_and(|value| value <= u64::from(resources.maximum_processes))
}

fn require_control(
    control_name: &'static str,
    verified: bool,
) -> Result<(), ApplicationServiceError> {
    if verified {
        Ok(())
    } else {
        Err(ApplicationServiceError::IsolationVerificationFailed { control_name })
    }
}

fn parse_json<T>(operation: &'static str, bytes: &[u8]) -> Result<T, ApplicationServiceError>
where
    T: for<'de> Deserialize<'de>,
{
    serde_json::from_slice(bytes)
        .map_err(|_| ApplicationServiceError::MalformedIsolationInspection { operation })
}

fn parse_single_inspection<T>(
    operation: &'static str,
    bytes: &[u8],
) -> Result<T, ApplicationServiceError>
where
    T: for<'de> Deserialize<'de>,
{
    let mut values: Vec<T> = parse_json(operation, bytes)?;
    if values.len() != 1 {
        return Err(ApplicationServiceError::MalformedIsolationInspection { operation });
    }
    values
        .pop()
        .ok_or(ApplicationServiceError::MalformedIsolationInspection { operation })
}

fn parse_backend_identifier(bytes: &[u8]) -> Option<String> {
    let identifier = std::str::from_utf8(bytes).ok()?.trim();
    if identifier.is_empty()
        || identifier.len() > 128
        || identifier
            .chars()
            .any(|character| character.is_control() || character.is_whitespace())
    {
        return None;
    }
    Some(identifier.to_owned())
}

fn sandbox_identity(
    request_id: &str,
    image_reference: &str,
    policy_id: &str,
    started_at_epoch_seconds: u64,
) -> String {
    let mut hasher = Sha256::new();
    for component in [request_id, image_reference, policy_id] {
        hasher.update(component.as_bytes());
        hasher.update([0]);
    }
    hasher.update(started_at_epoch_seconds.to_be_bytes());
    let digest = format!("{:x}", hasher.finalize());
    digest[..16].to_owned()
}

fn command_sandbox_identity(
    request_id: &str,
    image_reference: &str,
    policy_id: &str,
    started_at_epoch_seconds: u64,
) -> Result<String, CommandExecutionError> {
    let mut execution_nonce = [0_u8; 16];
    getrandom::fill(&mut execution_nonce).map_err(|_| {
        CommandExecutionError::Backend(ApplicationServiceError::BackendInvocationFailed {
            operation: "execution_identity",
        })
    })?;

    let mut hasher = Sha256::new();
    for component in [request_id, image_reference, policy_id] {
        hasher.update(component.as_bytes());
        hasher.update([0]);
    }
    hasher.update(started_at_epoch_seconds.to_be_bytes());
    hasher.update(execution_nonce);
    let digest = format!("{:x}", hasher.finalize());
    Ok(digest[..16].to_owned())
}

fn cpu_limit(cpu_millicores: u32) -> String {
    format!("{}.{:03}", cpu_millicores / 1_000, cpu_millicores % 1_000)
}

fn parse_loopback_port(stdout: &[u8]) -> Option<u16> {
    let text = std::str::from_utf8(stdout).ok()?.trim();
    let port = text.strip_prefix("127.0.0.1:")?.parse::<u16>().ok()?;
    (port != 0).then_some(port)
}

/// Parse one successful `podman wait` stdout payload into the workload exit code.
///
/// Malformed or missing stdout is runtime evidence corruption, not a synthetic
/// workload result. The caller supplies the exact wait operation so ordinary
/// and post-kill failures retain distinct provenance.
fn parse_wait_exit_code(
    stdout: &[u8],
    operation: &'static str,
) -> Result<i32, CommandExecutionError> {
    std::str::from_utf8(stdout)
        .ok()
        .map(str::trim)
        .filter(|text| !text.is_empty())
        .and_then(|text| text.parse::<i32>().ok())
        .ok_or(CommandExecutionError::Backend(
            ApplicationServiceError::MalformedIsolationInspection { operation },
        ))
}

fn wait_for_readiness(
    host_port: u16,
    policy: &IsolationPolicy,
) -> Result<(), ApplicationServiceError> {
    let deadline = Instant::now() + Duration::from_millis(policy.readiness_timeout_millis);
    let address = SocketAddrV4::new(Ipv4Addr::LOCALHOST, host_port);
    let poll = Duration::from_millis(policy.readiness_poll_interval_millis);
    loop {
        let now = Instant::now();
        if now >= deadline {
            return Err(ApplicationServiceError::ReadinessTimeout);
        }
        let remaining = deadline.saturating_duration_since(now);
        if TcpStream::connect_timeout(&address.into(), poll.min(remaining)).is_ok() {
            return Ok(());
        }
        let after_probe = Instant::now();
        if after_probe >= deadline {
            return Err(ApplicationServiceError::ReadinessTimeout);
        }
        thread::sleep(poll.min(deadline.saturating_duration_since(after_probe)));
    }
}
