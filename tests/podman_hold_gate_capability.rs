//! Real rootless-Podman capability proof for the runtime-owned hold/attest/release gate.
//!
//! The dedicated Linux E2E lane proves that the runtime gate remains the running process until
//! explicit release, so effective process isolation can be observed before exact consumer argv
//! becomes executable.

#[cfg(target_os = "linux")]
mod linux {
    use std::{
        fs,
        io::Write,
        os::unix::fs::PermissionsExt,
        path::Path,
        process::{Command, Output, Stdio},
        thread,
        time::{Duration, Instant},
    };

    use sha2::{Digest, Sha256};

    struct ContainerGuard {
        container_id: Option<String>,
    }

    impl ContainerGuard {
        fn new(container_id: String) -> Self {
            Self {
                container_id: Some(container_id),
            }
        }

        fn remove(&mut self) {
            if let Some(container_id) = self.container_id.take() {
                let status = Command::new("podman")
                    .args(["rm", "--force", "--ignore", &container_id])
                    .status()
                    .expect("Podman cleanup command must be spawnable");
                assert!(status.success(), "real capability fixture must clean up");
            }
        }
    }

    impl Drop for ContainerGuard {
        fn drop(&mut self) {
            if let Some(container_id) = self.container_id.take() {
                let _ = Command::new("podman")
                    .args(["rm", "--force", "--ignore", &container_id])
                    .status();
            }
        }
    }

    fn successful_output(command: &mut Command, operation: &str) -> Output {
        let output = command
            .output()
            .unwrap_or_else(|error| panic!("{operation} must be spawnable: {error}"));
        assert!(
            output.status.success(),
            "{operation} failed: stdout={} stderr={}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
        output
    }

    fn sha256(path: &Path) -> String {
        let bytes = fs::read(path).expect("gate fixture bytes must remain readable");
        format!("{:x}", Sha256::digest(bytes))
    }

    fn wait_for_gate_ready(container_id: &str) -> String {
        let deadline = Instant::now() + Duration::from_secs(5);
        loop {
            let output = successful_output(
                Command::new("podman").args(["logs", container_id]),
                "podman logs while gate is held",
            );
            let logs = String::from_utf8(output.stdout).expect("gate logs must be UTF-8");
            if logs.contains("QSR_GATE_READY") {
                return logs;
            }
            assert!(
                Instant::now() < deadline,
                "runtime-owned gate did not become ready before the capability-test deadline"
            );
            thread::sleep(Duration::from_millis(50));
        }
    }

    fn podman_top_value(container_id: &str, descriptor: &str) -> String {
        let operation = format!("podman top held gate {descriptor} evidence");
        let output = successful_output(
            Command::new("podman").args(["top", container_id, descriptor]),
            &operation,
        );
        let text = String::from_utf8(output.stdout).expect("podman top output must be UTF-8");
        let mut lines = text.lines().filter(|line| !line.trim().is_empty());
        let _header = lines.next().expect("podman top must return a header");
        let value = lines
            .next()
            .expect("podman top must return the held gate process")
            .trim()
            .to_owned();
        assert!(
            !value.is_empty(),
            "podman top {descriptor} must not be empty"
        );
        assert!(
            lines.next().is_none(),
            "held gate fixture must expose exactly one running process before release"
        );
        value
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

    fn proc_status_field(host_pid: &str, field_name: &str) -> String {
        assert!(
            host_pid.chars().all(|character| character.is_ascii_digit()),
            "Podman hpid evidence must be a decimal host PID"
        );
        let status = fs::read_to_string(format!("/proc/{host_pid}/status"))
            .expect("held gate host process status must remain readable");
        let prefix = format!("{field_name}:");
        status
            .lines()
            .find_map(|line| line.strip_prefix(&prefix))
            .map(str::trim)
            .map(str::to_owned)
            .unwrap_or_else(|| panic!("held gate process status must expose {field_name}"))
    }

    fn release_gate(container_id: &str, release_token: &str) {
        let mut child = Command::new("podman")
            .args(["attach", "--sig-proxy=false", container_id])
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .expect("podman attach must be spawnable");
        child
            .stdin
            .as_mut()
            .expect("attach stdin must be piped")
            .write_all(format!("{release_token}\n").as_bytes())
            .expect("release token must be writable to the held gate");
        drop(child.stdin.take());

        let deadline = Instant::now() + Duration::from_secs(5);
        loop {
            match child.try_wait().expect("podman attach wait must succeed") {
                Some(status) => {
                    assert!(status.success(), "gate release/consumer exec must succeed");
                    return;
                }
                None if Instant::now() < deadline => thread::sleep(Duration::from_millis(25)),
                None => {
                    let _ = child.kill();
                    let _ = child.wait();
                    panic!("gate release did not complete before the capability-test deadline");
                }
            }
        }
    }

    #[test]
    #[ignore = "requires real rootless Podman plus the musl target installed by the dedicated E2E lane"]
    fn rootless_podman_supports_a_runtime_owned_held_gate_before_consumer_exec() {
        assert_eq!(
            std::env::consts::ARCH,
            "x86_64",
            "this hosted capability fixture currently builds an x86_64 musl gate explicitly"
        );
        let image = std::env::var("QSR_PODMAN_E2E_IMAGE")
            .expect("E2E lane must provide one pre-pulled digest-pinned image");
        assert!(
            image.contains("@sha256:"),
            "capability fixture image must be digest pinned"
        );

        let workspace = tempfile::tempdir().expect("capability workspace must be creatable");
        let gate_path = workspace.path().join("qsr-hold-gate");
        let gate_source =
            Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/qsr_hold_gate.rs");
        successful_output(
            Command::new("rustc").args([
                "--edition=2024",
                "--target=x86_64-unknown-linux-musl",
                "-C",
                "opt-level=2",
                "-C",
                "strip=symbols",
                "-o",
                gate_path
                    .to_str()
                    .expect("temporary gate path must be Unicode on the hosted runner"),
                gate_source
                    .to_str()
                    .expect("repository fixture path must be Unicode on the hosted runner"),
            ]),
            "compile static runtime-owned hold gate fixture",
        );
        let mut permissions = fs::metadata(&gate_path)
            .expect("compiled gate metadata must be available")
            .permissions();
        permissions.set_mode(0o555);
        fs::set_permissions(&gate_path, permissions)
            .expect("gate fixture must be read/execute only");
        let gate_digest_before = sha256(&gate_path);

        let release_token = format!("qsr-release-{}", std::process::id());
        let exact_consumer_argument = "exact-argv-sentinel";
        let volume = format!(
            "{}:/qsr-runtime-gate:ro",
            gate_path
                .to_str()
                .expect("temporary gate path must be Unicode on the hosted runner")
        );
        let consumer_script = "import sys,time; print('QSR_CONSUMER_RAN:' + sys.argv[1], flush=True); time.sleep(0.05)";
        let create = successful_output(
            Command::new("podman").args([
                "create",
                "--pull=never",
                "--read-only",
                "--read-only-tmpfs=false",
                "--network=none",
                "--cap-drop=all",
                "--security-opt=no-new-privileges",
                "--userns=auto",
                "--ipc=none",
                "--pid=private",
                "--uts=private",
                "--cgroupns=private",
                "--user=65532:65532",
                "--interactive",
                "--log-driver=k8s-file",
                "--tmpfs=/tmp:rw,noexec,nosuid,nodev,size=1048576",
                "--volume",
                &volume,
                "--entrypoint=/qsr-runtime-gate",
                &image,
                &release_token,
                "/usr/local/bin/python",
                "-c",
                consumer_script,
                exact_consumer_argument,
            ]),
            "podman create held gate container",
        );
        let container_id = String::from_utf8(create.stdout)
            .expect("podman create ID must be UTF-8")
            .trim()
            .to_owned();
        assert!(
            !container_id.is_empty(),
            "podman create must return an exact ID"
        );
        let mut cleanup = ContainerGuard::new(container_id.clone());

        successful_output(
            Command::new("podman").args(["start", &container_id]),
            "podman start runtime-owned gate",
        );
        let held_logs = wait_for_gate_ready(&container_id);
        assert!(
            !held_logs.contains("QSR_CONSUMER_RAN:"),
            "consumer output must not exist while the runtime-owned gate is held"
        );

        let seccomp = podman_top_value(&container_id, "seccomp");
        assert!(
            matches!(seccomp.as_str(), "filter" | "strict"),
            "held gate must expose an effective seccomp mode"
        );
        for capability_column in ["capeff", "capbnd", "capinh", "capprm"] {
            assert!(
                capability_set_is_empty(&podman_top_value(&container_id, capability_column)),
                "held gate must expose an empty {capability_column} set"
            );
        }
        // Podman 4.9.3 has no `capamb` top descriptor; passing it triggers host-ps fallback.
        // Preserve ambient-capability proof by resolving the documented `hpid` descriptor and
        // reading the kernel's process status instead of weakening the capability assertion.
        let host_pid = podman_top_value(&container_id, "hpid");
        assert_eq!(
            proc_status_field(&host_pid, "CapAmb"),
            "0000000000000000",
            "held gate must expose an empty ambient capability set"
        );
        assert!(
            !podman_top_value(&container_id, "label").is_empty(),
            "held gate must expose the backend's process label even when the hosted lane cannot accept it as a positive LSM"
        );

        let process_args = successful_output(
            Command::new("podman").args(["top", &container_id, "args"]),
            "podman top held gate argv",
        );
        let process_args = String::from_utf8(process_args.stdout).expect("gate argv must be UTF-8");
        assert!(
            process_args.contains("/qsr-runtime-gate"),
            "the running process before release must be the runtime-owned gate"
        );
        assert_eq!(
            sha256(&gate_path),
            gate_digest_before,
            "the runtime-owned gate fixture must not change while it is mounted and held"
        );

        release_gate(&container_id, &release_token);
        let released_logs = successful_output(
            Command::new("podman").args(["logs", &container_id]),
            "podman logs after trusted capability release",
        );
        let released_logs =
            String::from_utf8(released_logs.stdout).expect("released consumer logs must be UTF-8");
        assert!(
            released_logs.contains(&format!("QSR_CONSUMER_RAN:{exact_consumer_argument}")),
            "the held gate must exec the exact consumer argv only after explicit release"
        );
        assert_eq!(
            sha256(&gate_path),
            gate_digest_before,
            "gate fixture identity must remain unchanged through the exec transition"
        );

        cleanup.remove();
    }
}
