//! Rootless Podman infrastructure adapter for isolated application services.

use std::{
    net::{Ipv4Addr, SocketAddrV4, TcpStream},
    path::PathBuf,
    process::Output,
    thread,
    time::{Duration, Instant},
};

use sha2::{Digest, Sha256};

use super::bounded_command::{BoundedCommandError, BoundedCommandRunner};
use crate::{
    ApplicationServiceError, ApplicationServiceLease, ApplicationServiceRequest, CleanupReceipt,
    IsolationPolicy, ServiceEndpoint, sandbox_execution::RuntimeLeaseMetadata,
};

const PODMAN_BACKEND_ID: &str = "rootless_podman";
const DEFAULT_COMMAND_TIMEOUT: Duration = Duration::from_secs(30);
const DEFAULT_COMMAND_OUTPUT_LIMIT_BYTES: usize = 64 * 1024;

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
    /// Return the exact arguments used to prove the backend is rootless.
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
        let identity = sandbox_identity(request, policy, started_at_epoch_seconds);
        let sandbox_name = format!("qsr-app-{identity}");
        let network_name = format!("qsr-net-{identity}");

        let rootless_probe_args = vec![
            "info".to_owned(),
            "--format".to_owned(),
            "{{.Host.Security.Rootless}}".to_owned(),
        ];
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
    /// Returns [`ApplicationServiceError`] when Podman is unavailable, is not
    /// rootless, exceeds a command/output budget, rejects an isolation operation,
    /// returns a non-loopback port, readiness times out, or required cleanup fails.
    pub fn launch_at(
        &self,
        request: &ApplicationServiceRequest,
        policy: &IsolationPolicy,
        started_at_epoch_seconds: u64,
    ) -> Result<ApplicationServiceLease, ApplicationServiceError> {
        let plan = Self::plan_at(request, policy, started_at_epoch_seconds)?;
        let rootless = self.checked_output("rootless_probe", plan.rootless_probe_args())?;
        if String::from_utf8_lossy(&rootless.stdout).trim() != "true" {
            return Err(ApplicationServiceError::BackendNotRootless);
        }

        self.checked_output("network_create", plan.network_create_args())?;
        if let Err(error) = self.checked_output("container_create", plan.container_create_args()) {
            self.cleanup_network(&plan)?;
            return Err(error);
        }

        let start_args = ["start".to_owned(), plan.sandbox_name().to_owned()];
        if let Err(error) = self.checked_output("container_start", &start_args) {
            self.cleanup_created_container(&plan)?;
            return Err(error);
        }

        let port_args = [
            "port".to_owned(),
            plan.sandbox_name().to_owned(),
            format!("{}/tcp", request.container_port),
        ];
        let port_output = match self.checked_output("port_query", &port_args) {
            Ok(output) => output,
            Err(error) => {
                self.cleanup_started_container(&plan, policy.shutdown_grace_seconds)?;
                return Err(error);
            }
        };
        let host_port = match parse_loopback_port(&port_output.stdout) {
            Some(port) => port,
            None => {
                self.cleanup_started_container(&plan, policy.shutdown_grace_seconds)?;
                return Err(ApplicationServiceError::InvalidPortMapping);
            }
        };
        if wait_for_readiness(host_port, policy).is_err() {
            self.cleanup_started_container(&plan, policy.shutdown_grace_seconds)?;
            return Err(ApplicationServiceError::ReadinessTimeout);
        }

        Ok(ApplicationServiceLease::new(
            request,
            RuntimeLeaseMetadata {
                backend_id: PODMAN_BACKEND_ID,
                sandbox_id: plan.sandbox_name().to_owned(),
                network_id: plan.network_name().to_owned(),
                policy_id: policy.policy_id.clone(),
                policy_sha256: policy.effective_policy_sha256(),
                started_at_epoch_seconds,
                expires_at_epoch_seconds: plan.expires_at_epoch_seconds(),
                shutdown_grace_seconds: policy.shutdown_grace_seconds,
            },
            ServiceEndpoint::loopback(host_port, request.protocol),
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

fn sandbox_identity(
    request: &ApplicationServiceRequest,
    policy: &IsolationPolicy,
    started_at_epoch_seconds: u64,
) -> String {
    let mut hasher = Sha256::new();
    for component in [
        request.request_id.as_str(),
        request.image_reference.as_str(),
        policy.policy_id.as_str(),
    ] {
        hasher.update(component.as_bytes());
        hasher.update([0]);
    }
    hasher.update(started_at_epoch_seconds.to_be_bytes());
    let digest = format!("{:x}", hasher.finalize());
    digest[..16].to_owned()
}

fn cpu_limit(cpu_millicores: u32) -> String {
    format!("{}.{:03}", cpu_millicores / 1_000, cpu_millicores % 1_000)
}

fn parse_loopback_port(stdout: &[u8]) -> Option<u16> {
    let text = std::str::from_utf8(stdout).ok()?.trim();
    let port = text.strip_prefix("127.0.0.1:")?.parse::<u16>().ok()?;
    (port != 0).then_some(port)
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
