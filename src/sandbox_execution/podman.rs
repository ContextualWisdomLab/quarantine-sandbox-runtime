//! Rootless Podman infrastructure adapter for isolated application-service plans.

use sha2::{Digest, Sha256};

use crate::{ApplicationServiceError, ApplicationServiceRequest, IsolationPolicy};

const PODMAN_BACKEND_ID: &str = "rootless_podman";

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
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct RootlessPodmanAdapter;

impl RootlessPodmanAdapter {
    /// Return the stable backend identifier emitted in future runtime receipts.
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
            "--ignore".to_owned(),
            network_name.clone(),
        ];
        let mut container_create_args = vec![
            "create".to_owned(),
            "--name".to_owned(),
            sandbox_name.clone(),
            "--pull=never".to_owned(),
            "--read-only".to_owned(),
            "--cap-drop=all".to_owned(),
            "--security-opt=no-new-privileges".to_owned(),
            "--userns=auto".to_owned(),
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
            format!("org.contextualwisdomlab.sandbox.policy={}", policy.policy_id),
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
