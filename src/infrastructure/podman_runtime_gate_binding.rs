/// A Podman command adapter with one independently verified runtime gate artifact bound to it.
///
/// This is an intentionally fail-closed integration slice for issue #25. It proves that the
/// release-authorized host artifact, rather than hostile image content, becomes the initial OCI
/// process. Until the bounded release channel is implemented, this adapter never starts the
/// initialized container and therefore cannot release consumer code.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RuntimeGatePodmanAdapter {
    inner: RootlessPodmanAdapter,
    runtime_gate_artifact: super::runtime_gate_artifact::RuntimeGateArtifact,
}

impl RootlessPodmanAdapter {
    /// Bind an independently digest- and architecture-verified runtime gate artifact.
    ///
    /// The returned adapter remains fail closed before `podman start` until the runtime-owned
    /// bounded release channel is implemented. This builder does not weaken the legacy command
    /// path or claim completion of issue #25.
    #[must_use]
    pub fn with_runtime_gate_artifact(
        self,
        runtime_gate_artifact: super::runtime_gate_artifact::RuntimeGateArtifact,
    ) -> RuntimeGatePodmanAdapter {
        RuntimeGatePodmanAdapter {
            inner: self,
            runtime_gate_artifact,
        }
    }
}

impl RuntimeGatePodmanAdapter {
    /// Create and initialize one command container with the verified gate as PID 1.
    ///
    /// The verified gate is mounted read-only at `/qsr-runtime-gate`; the consumer argv is passed
    /// behind a controller-generated release token and is never installed as the OCI entrypoint.
    /// After `podman init` succeeds this transitional adapter deliberately cleans up and returns a
    /// fail-closed `runtime_gate_release` backend error because no authoritative release channel is
    /// admitted yet.
    ///
    /// # Errors
    ///
    /// Returns [`CommandExecutionError`] on request, backend, create/init, cleanup, or the expected
    /// not-yet-implemented release boundary. A successful consumer execution is impossible in this
    /// slice by design.
    pub fn run_command_at(
        &self,
        request: &CommandExecutionRequest,
        policy: &IsolationPolicy,
        started_at_epoch_seconds: u64,
    ) -> Result<CommandExecutionResult, CommandExecutionError> {
        request.validate(policy)?;
        let staged_source = request
            .source_artifact
            .as_ref()
            .map(stage_pr_source_artifact)
            .transpose()?;
        let info_output = self.inner.checked_output(
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
        let create_receipt_directory = tempfile::Builder::new()
            .prefix("qsr-command-create-")
            .tempdir()
            .map_err(|_| {
                CommandExecutionError::Backend(ApplicationServiceError::BackendInvocationFailed {
                    operation: "container_create_receipt",
                })
            })?;
        let create_receipt_path = create_receipt_directory.path().join("container-id");
        let create_receipt_path_text = create_receipt_path.to_str().ok_or(
            CommandExecutionError::Backend(ApplicationServiceError::BackendInvocationFailed {
                operation: "container_create_receipt",
            }),
        )?;
        let release_token = runtime_gate_release_token()?;

        let mut create_args = vec![
            "create".to_owned(),
            "--name".to_owned(),
            sandbox_name.clone(),
            format!("--cidfile={create_receipt_path_text}"),
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
            "--log-driver=k8s-file".to_owned(),
            "--log-opt".to_owned(),
            format!("max-size={DEFAULT_COMMAND_LOG_STORAGE_LIMIT_BYTES}b"),
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
        ];
        if let Some(staged) = &staged_source {
            create_args.push("--volume".to_owned());
            create_args.push(format!(
                "{}:/workspace:ro,noexec,nosuid,nodev,Z",
                staged.path().display()
            ));
            create_args.push("--workdir".to_owned());
            create_args.push("/workspace".to_owned());
        }
        create_args.push("--volume".to_owned());
        create_args.push(format!(
            "{}:/qsr-runtime-gate:ro",
            self.runtime_gate_artifact.path().display()
        ));
        create_args.push("--entrypoint=/qsr-runtime-gate".to_owned());
        create_args.push(request.image_reference.clone());
        create_args.push(release_token);
        create_args.extend(request.command.iter().cloned());

        let create_output = match self.inner.checked_output("container_create", &create_args) {
            Ok(output) => output,
            Err(error) => {
                let original = CommandExecutionError::Backend(error);
                let owned_container_id = read_command_create_receipt(&create_receipt_path)
                    .map_err(CommandExecutionError::Backend)?;
                return match owned_container_id {
                    Some(container_id) => Err(self
                        .inner
                        .cleanup_owned_command_container_or_report(&container_id, original)),
                    None => Err(original),
                };
            }
        };
        let container_id = match parse_backend_identifier(&create_output.stdout) {
            Some(identifier) => identifier,
            None => {
                return Err(self.inner.cleanup_or_report(
                    &sandbox_name,
                    CommandExecutionError::Backend(
                        ApplicationServiceError::MalformedIsolationInspection {
                            operation: "container_create",
                        },
                    ),
                ));
            }
        };

        if let Err(error) = self.inner.checked_output(
            "container_init",
            &["init".to_owned(), container_id.clone()],
        ) {
            return Err(self
                .inner
                .cleanup_owned_command_container_or_report(&container_id, error.into()));
        }

        Err(self.inner.cleanup_owned_command_container_or_report(
            &container_id,
            CommandExecutionError::Backend(ApplicationServiceError::BackendInvocationFailed {
                operation: "runtime_gate_release",
            }),
        ))
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
