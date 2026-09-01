//! Rootless Podman infrastructure adapter for isolated application services.

use std::{
    net::{Ipv4Addr, SocketAddrV4, TcpStream},
    path::PathBuf,
    process::{Command, Output},
    thread,
    time::{Duration, Instant},
};

use serde::Deserialize;
use sha2::{Digest, Sha256};

use crate::{
    ApplicationServiceError, ApplicationServiceLease, ApplicationServiceRequest, CleanupReceipt,
    IsolationControlStatus, IsolationPolicy, ServiceEndpoint, VerifiedIsolationState,
    sandbox_execution::RuntimeLeaseMetadata,
};

const PODMAN_BACKEND_ID: &str = "rootless_podman";
const MAX_BACKEND_OUTPUT_BYTES: usize = 65_536;

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
    #[serde(default, rename = "EffectiveCaps")]
    effective_caps: Vec<String>,
    #[serde(rename = "Config")]
    config: ContainerConfig,
    #[serde(rename = "HostConfig")]
    host_config: ContainerHostConfig,
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
}

impl RootlessPodmanAdapter {
    /// Construct an adapter for a specific Podman-compatible executable.
    ///
    /// The executable is invoked directly with argv; this adapter never uses a shell.
    #[must_use]
    pub fn new(program: impl Into<PathBuf>) -> Self {
        Self {
            program: program.into(),
        }
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
        let info_output = self.checked_output("backend_security_info", plan.rootless_probe_args())?;
        let info: PodmanInfo = parse_json("backend_security_info", &info_output.stdout)?;
        validate_backend_security(&info)?;

        self.checked_output("network_create", plan.network_create_args())?;
        let create_output = match self.checked_output("container_create", plan.container_create_args()) {
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

        let verified = match self.verify_effective_isolation(
            &plan,
            request,
            policy,
            &info,
            &container_id,
        ) {
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
    /// Cleanup attempts every required operation before returning failure.
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

        require_control("read_only_root_filesystem", container.host_config.readonly_rootfs)?;
        require_control("unprivileged_container", !container.host_config.privileged)?;
        require_control("all_capabilities_dropped", container.effective_caps.is_empty())?;
        let security_options = container.host_config.security_opt.unwrap_or_default();
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
                .any(|option| option == "seccomp=unconfined"),
        )?;
        require_control(
            "isolated_user_namespace",
            container.host_config.userns_mode == "auto",
        )?;
        require_control("isolated_pid_namespace", container.host_config.pid_mode == "private")?;
        require_control("isolated_ipc_namespace", container.host_config.ipc_mode == "none")?;
        require_control(
            "non_root_identity",
            container.config.user
                == format!("{}:{}", policy.run_as_user_id, policy.run_as_group_id),
        )?;
        require_control(
            "resource_limits",
            resource_limits_match(&container.host_config, request),
        )?;
        require_control("lsm", effective_lsm_verified(info, &container))?;

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
        require_control("external_egress_denied", network.internal && !network.dns_enabled)?;

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

    fn checked_output(
        &self,
        operation: &'static str,
        args: &[String],
    ) -> Result<Output, ApplicationServiceError> {
        let output = Command::new(&self.program)
            .args(args)
            .output()
            .map_err(|_| ApplicationServiceError::BackendInvocationFailed { operation })?;
        let output_bytes = output.stdout.len().saturating_add(output.stderr.len());
        if output_bytes > MAX_BACKEND_OUTPUT_BYTES {
            return Err(ApplicationServiceError::BackendOutputTooLarge {
                operation,
                maximum_bytes: MAX_BACKEND_OUTPUT_BYTES,
            });
        }
        if !output.status.success() {
            return Err(ApplicationServiceError::BackendCommandFailed { operation });
        }
        Ok(output)
    }

    fn command_succeeded(&self, args: &[String]) -> bool {
        Command::new(&self.program)
            .args(args)
            .status()
            .is_ok_and(|status| status.success())
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

fn effective_lsm_verified(info: &PodmanInfo, container: &ContainerInspection) -> bool {
    (info.host.security.apparmor_enabled && !container.apparmor_profile.is_empty())
        || (info.host.security.selinux_enabled && !container.process_label.is_empty())
}

fn resource_limits_match(
    effective: &ContainerHostConfig,
    request: &ApplicationServiceRequest,
) -> bool {
    let requested_nano_cpus = u64::from(request.resources.cpu_millicores) * 1_000_000;
    effective.memory > 0
        && effective.memory <= request.resources.memory_bytes
        && effective.nano_cpus > 0
        && effective.nano_cpus <= requested_nano_cpus
        && effective.pids_limit > 0
        && u64::try_from(effective.pids_limit)
            .is_ok_and(|value| value <= u64::from(request.resources.maximum_processes))
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
