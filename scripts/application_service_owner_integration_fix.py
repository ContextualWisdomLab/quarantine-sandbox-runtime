#!/usr/bin/env python3
"""One-shot semantic repair for integrating canonical application-service owner truth.

This script is intentionally temporary and is removed by the workflow that executes it.
It refuses ambiguous source matches so a moved owner head cannot be rewritten accidentally.
"""

from pathlib import Path
import re


def replace_once(path: str, old: str, new: str) -> None:
    target = Path(path)
    text = target.read_text()
    count = text.count(old)
    if count != 1:
        raise SystemExit(f"{path}: expected exactly one literal match, found {count}")
    target.write_text(text.replace(old, new, 1))


def regex_once(path: str, pattern: str, replacement: str) -> None:
    target = Path(path)
    text = target.read_text()
    updated, count = re.subn(pattern, replacement, text, count=1, flags=re.S)
    if count != 1:
        raise SystemExit(f"{path}: expected exactly one regex match, found {count}")
    target.write_text(updated)


replace_once(
    "src/sandbox_execution/mod.rs",
    '''    /// Cleanup could not prove removal of all runtime-owned resources.\n    #[error("sandbox cleanup failed")]\n    CleanupFailed,\n''',
    '''    /// A caller-visible application-service lease lacks runtime-private cleanup authority.\n    #[error("sandbox cleanup authority unavailable")]\n    CleanupAuthorityUnavailable,\n    /// Cleanup could not prove removal of all runtime-owned resources.\n    #[error("sandbox cleanup failed")]\n    CleanupFailed,\n''',
)

replace_once(
    "src/application_service/mod.rs",
    '''/// Attested lease for one ready isolated application service.\n#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]\npub struct ApplicationServiceLease {\n''',
    '''/// Runtime-only capability selecting resources that this process may destroy.\n///\n/// Public lease fields remain correlation and evidence only. This capability is deliberately\n/// crate-private and non-serializable so a caller cannot forge destructive backend authority.\n#[derive(Clone, Debug, PartialEq, Eq)]\npub(crate) struct ApplicationServiceCleanupAuthority {\n    sandbox_id: String,\n    network_id: String,\n    shutdown_grace_seconds: u32,\n}\n\nimpl ApplicationServiceCleanupAuthority {\n    /// Return the exact runtime-acquired container identifier authorized for cleanup.\n    pub(crate) fn sandbox_id(&self) -> &str {\n        &self.sandbox_id\n    }\n\n    /// Return the runtime-owned network identifier authorized for cleanup.\n    pub(crate) fn network_id(&self) -> &str {\n        &self.network_id\n    }\n\n    /// Return the bounded stop grace period captured when the lease was issued.\n    pub(crate) const fn shutdown_grace_seconds(&self) -> u32 {\n        self.shutdown_grace_seconds\n    }\n}\n\n/// Attested lease for one ready isolated application service.\n#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]\npub struct ApplicationServiceLease {\n''',
)

replace_once(
    "src/application_service/mod.rs",
    '''    shutdown_grace_seconds: u32,\n    isolation_attestation: IsolationAttestation,\n}\n\nimpl ApplicationServiceLease {\n    pub(crate) fn new(\n        request: &ApplicationServiceRequest,\n        metadata: RuntimeLeaseMetadata,\n        endpoint: ServiceEndpoint,\n    ) -> Self {\n        Self {\n            schema_version: APPLICATION_SERVICE_LEASE_SCHEMA_VERSION.to_owned(),\n            request_id: request.request_id.clone(),\n            image_reference: request.image_reference.clone(),\n            backend_id: metadata.backend_id.to_owned(),\n            backend_version: metadata.backend_version,\n            sandbox_id: metadata.sandbox_id,\n            network_id: metadata.network_id,\n            policy_id: metadata.policy_id,\n            policy_sha256: metadata.policy_sha256,\n            endpoint,\n            started_at_epoch_seconds: metadata.started_at_epoch_seconds,\n            expires_at_epoch_seconds: metadata.expires_at_epoch_seconds,\n            shutdown_grace_seconds: metadata.shutdown_grace_seconds,\n            isolation_attestation: metadata.isolation_state,\n        }\n    }\n''',
    '''    shutdown_grace_seconds: u32,\n    isolation_attestation: IsolationAttestation,\n    #[serde(skip)]\n    cleanup_authority: Option<ApplicationServiceCleanupAuthority>,\n}\n\nimpl ApplicationServiceLease {\n    pub(crate) fn new_with_cleanup_sandbox_id(\n        request: &ApplicationServiceRequest,\n        metadata: RuntimeLeaseMetadata,\n        cleanup_sandbox_id: String,\n        endpoint: ServiceEndpoint,\n    ) -> Self {\n        let cleanup_authority = ApplicationServiceCleanupAuthority {\n            sandbox_id: cleanup_sandbox_id,\n            network_id: metadata.network_id.clone(),\n            shutdown_grace_seconds: metadata.shutdown_grace_seconds,\n        };\n        Self {\n            schema_version: APPLICATION_SERVICE_LEASE_SCHEMA_VERSION.to_owned(),\n            request_id: request.request_id.clone(),\n            image_reference: request.image_reference.clone(),\n            backend_id: metadata.backend_id.to_owned(),\n            backend_version: metadata.backend_version,\n            sandbox_id: metadata.sandbox_id,\n            network_id: metadata.network_id,\n            policy_id: metadata.policy_id,\n            policy_sha256: metadata.policy_sha256,\n            endpoint,\n            started_at_epoch_seconds: metadata.started_at_epoch_seconds,\n            expires_at_epoch_seconds: metadata.expires_at_epoch_seconds,\n            shutdown_grace_seconds: metadata.shutdown_grace_seconds,\n            isolation_attestation: metadata.isolation_state,\n            cleanup_authority: Some(cleanup_authority),\n        }\n    }\n''',
)

replace_once(
    "src/application_service/mod.rs",
    '''    pub(crate) const fn shutdown_grace_seconds(&self) -> u32 {\n        self.shutdown_grace_seconds\n    }\n\n    /// Return effective isolation evidence verified by the runtime backend.\n''',
    '''    pub(crate) const fn shutdown_grace_seconds(&self) -> u32 {\n        self.shutdown_grace_seconds\n    }\n\n    /// Return runtime-private cleanup authority when this lease originated in this process.\n    pub(crate) const fn cleanup_authority(&self) -> Option<&ApplicationServiceCleanupAuthority> {\n        self.cleanup_authority.as_ref()\n    }\n\n    /// Return effective isolation evidence verified by the runtime backend.\n''',
)

launch = '''    pub fn launch_at(
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

        let create_receipt_directory = tempfile::Builder::new()
            .prefix("qsr-application-create-")
            .tempdir()
            .map_err(|_| ApplicationServiceError::BackendInvocationFailed {
                operation: "container_create_receipt",
            })?;
        let create_receipt_path = create_receipt_directory.path().join("container-id");
        let create_receipt_path_text = create_receipt_path.to_str().ok_or(
            ApplicationServiceError::BackendInvocationFailed {
                operation: "container_create_receipt",
            },
        )?;
        let mut create_args = plan.container_create_args().to_vec();
        create_args.insert(3, format!("--cidfile={create_receipt_path_text}"));

        self.checked_output("network_create", plan.network_create_args())?;
        let create_output = match self.checked_output("container_create", &create_args) {
            Ok(output) => output,
            Err(error) => {
                let receipt = match read_container_create_receipt(&create_receipt_path) {
                    Ok(receipt) => receipt,
                    Err(receipt_error) => {
                        self.cleanup_network(&plan)?;
                        return Err(receipt_error);
                    }
                };
                if let Some(container_id) = receipt {
                    self.cleanup_acquired_container(&plan, &container_id)?;
                } else {
                    self.cleanup_network(&plan)?;
                }
                return Err(error);
            }
        };
        let stdout_container_id = match parse_backend_identifier(&create_output.stdout) {
            Some(identifier) => identifier,
            None => {
                let original = ApplicationServiceError::MalformedIsolationInspection {
                    operation: "container_create",
                };
                let receipt = match read_container_create_receipt(&create_receipt_path) {
                    Ok(receipt) => receipt,
                    Err(receipt_error) => {
                        self.cleanup_network(&plan)?;
                        return Err(receipt_error);
                    }
                };
                if let Some(container_id) = receipt {
                    self.cleanup_acquired_container(&plan, &container_id)?;
                    return Err(original);
                }
                self.cleanup_network(&plan)?;
                return Err(ApplicationServiceError::MalformedIsolationInspection {
                    operation: "container_create_receipt",
                });
            }
        };
        let container_id = match read_container_create_receipt(&create_receipt_path) {
            Ok(Some(receipt_container_id)) if receipt_container_id == stdout_container_id => {
                receipt_container_id
            }
            Ok(Some(receipt_container_id)) => {
                self.cleanup_acquired_container(&plan, &receipt_container_id)?;
                return Err(ApplicationServiceError::MalformedIsolationInspection {
                    operation: "container_create_receipt",
                });
            }
            Ok(None) => stdout_container_id,
            Err(receipt_error) => {
                self.cleanup_acquired_container(&plan, &stdout_container_id)?;
                return Err(receipt_error);
            }
        };

        let start_args = ["start".to_owned(), container_id.clone()];
        if let Err(error) = self.checked_output("container_start", &start_args) {
            self.cleanup_acquired_container(&plan, &container_id)?;
            return Err(error);
        }

        let verified =
            match self.verify_effective_isolation(&plan, request, policy, &info, &container_id) {
                Ok(value) => value,
                Err(error) => {
                    self.cleanup_started_container(
                        &plan,
                        &container_id,
                        policy.shutdown_grace_seconds,
                    )?;
                    return Err(error);
                }
            };

        if wait_for_readiness(verified.host_port, policy).is_err() {
            self.cleanup_started_container(&plan, &container_id, policy.shutdown_grace_seconds)?;
            return Err(ApplicationServiceError::ReadinessTimeout);
        }

        Ok(ApplicationServiceLease::new_with_cleanup_sandbox_id(
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
            container_id,
            ServiceEndpoint::loopback(verified.host_port, request.protocol),
        ))
    }

'''
regex_once(
    "src/infrastructure/podman.rs",
    r"    pub fn launch_at\(.*?\n    \}\n\n(?=    /// Stop a leased service)",
    launch,
)

terminate = '''    pub fn terminate_at(
        &self,
        lease: &ApplicationServiceLease,
        terminated_at_epoch_seconds: u64,
    ) -> Result<CleanupReceipt, ApplicationServiceError> {
        let authority = lease
            .cleanup_authority()
            .ok_or(ApplicationServiceError::CleanupAuthorityUnavailable)?;
        let stop_args = [
            "stop".to_owned(),
            "--time".to_owned(),
            authority.shutdown_grace_seconds().to_string(),
            authority.sandbox_id().to_owned(),
        ];
        let remove_args = [
            "rm".to_owned(),
            "--force".to_owned(),
            authority.sandbox_id().to_owned(),
        ];
        let network_args = [
            "network".to_owned(),
            "rm".to_owned(),
            "--force".to_owned(),
            authority.network_id().to_owned(),
        ];
        let stop_ok = self.command_succeeded(&stop_args);
        let remove_ok = self.command_succeeded(&remove_args);
        let network_ok = self.command_succeeded(&network_args);
        if !(stop_ok && remove_ok && network_ok) {
            return Err(ApplicationServiceError::CleanupFailed);
        }
        Ok(CleanupReceipt::complete(lease, terminated_at_epoch_seconds))
    }

'''
regex_once(
    "src/infrastructure/podman.rs",
    r"    pub fn terminate_at\(.*?\n    \}\n\n(?=    /// Run one bounded command)",
    terminate,
)

verify = '''    fn verify_effective_isolation(
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
            container_id.to_owned(),
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
            container_id.to_owned(),
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
            container_id.to_owned(),
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

'''
regex_once(
    "src/infrastructure/podman.rs",
    r"    fn verify_effective_isolation\(.*?\n    \}\n\n(?=    fn command_runner\(&self\))",
    verify,
)

cleanup = '''    fn cleanup_network(&self, plan: &PodmanLaunchPlan) -> Result<(), ApplicationServiceError> {
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

    fn cleanup_acquired_container(
        &self,
        plan: &PodmanLaunchPlan,
        container_id: &str,
    ) -> Result<(), ApplicationServiceError> {
        let remove_args = [
            "rm".to_owned(),
            "--force".to_owned(),
            container_id.to_owned(),
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
        container_id: &str,
        shutdown_grace_seconds: u32,
    ) -> Result<(), ApplicationServiceError> {
        let stop_args = [
            "stop".to_owned(),
            "--time".to_owned(),
            shutdown_grace_seconds.to_string(),
            container_id.to_owned(),
        ];
        let remove_args = [
            "rm".to_owned(),
            "--force".to_owned(),
            container_id.to_owned(),
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
'''
regex_once(
    "src/infrastructure/podman.rs",
    r"    fn cleanup_network\(.*?\n    \}\n(?=\}\n\nimpl Default for RootlessPodmanAdapter)",
    cleanup,
)

podman = Path("src/infrastructure/podman.rs")
podman_text = podman.read_text()
receipt_refs = podman_text.count("read_command_create_receipt")
if receipt_refs < 2:
    raise SystemExit(
        f"podman.rs: expected command receipt helper references, found {receipt_refs}"
    )
podman.write_text(
    podman_text.replace("read_command_create_receipt", "read_container_create_receipt")
)

baseline = Path("docs/product-technical-gap-baseline.md")
baseline_text = baseline.read_text()
marker = "<!-- current-authority-2026-09-14-application-service-owner-integration -->"
if marker in baseline_text:
    raise SystemExit("baseline already contains application-service owner integration authority")
authority = f'''{marker}
# Current authority supersession — application-service owner integration (2026-09-14)

Draft #112 exact `cc82dac196d133cb614aebc2ab7f37be945f48d1` keeps bounded command execution in the `sandbox_execution` Core context and the application-service profile in its Supporting context. Direct owner-ancestry PR #117 proves the canonical #21 lineage is genuinely divergent and cannot be adopted by a blind merge. Draft #118 therefore replays #21's runtime-identity, exact cleanup, live capability-column, and inspection-cardinality invariants against the current command-runtime architecture before changing production source.

The #118 RED predecessor `59c8d21c15dbfd818d27970c211abe0340c6dcbb` adapts only stale fake backend-version evidence required by the newer #112 parser. The hostile assertions remain unchanged. This descendant adopts the minimum service-owner semantics without copying command-domain truth: each service create provisions a runtime-owned cidfile receipt; destructive post-create container operations use only the exact acquired container ID; generated `qsr-app-*` remains correlation metadata; cleanup authority is crate-private and non-serializable; forged/deserialized public lease evidence cannot recreate destructive authority; all five live capability columns and single-record network evidence remain fail-closed.

No predecessor GREEN transfers to this moved candidate. Native exact-head verify, public/private rustdoc, complete production coverage, dedicated positive effective-LSM, #35/#43 real-runtime enforcement, qualifying independent review/security, ordinary reconciliation with the retained #21 ancestry, protected-head verification, and immutable release/SBOM/provenance/reproducibility/rollback remain mandatory before merge or release authority.

---

'''
baseline.write_text(authority + baseline_text)
