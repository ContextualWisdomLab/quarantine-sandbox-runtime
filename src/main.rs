//! Command-line entrypoint for running one bounded command inside a rootless-Podman sandbox.
//!
//! This is the first consumer-facing transport for
//! [`quarantine_sandbox_runtime::execute_command`]: a synchronous CLI a CI script (or a human)
//! can invoke directly, with no long-running service to start, poll, or shut down. See
//! `docs/adr/0008-podman-backed-command-execution-and-cli.md` for why a synchronous CLI, not an
//! HTTP service, is the right first transport for the bounded command-execution contract.
//!
//! There is deliberately no `--network-policy` flag: [`CommandExecutionRequest`] has no network
//! field (see ADR-0007), and the backend always runs the sandbox with no network namespace
//! attachment at all, denying all egress unconditionally. A configurable network policy would
//! require extending the accepted domain contract, which is out of scope here.

use std::{
    env,
    process::ExitCode,
    time::{SystemTime, UNIX_EPOCH},
};

use quarantine_sandbox_runtime::{
    CommandExecutionRequest, IsolationPolicy, ResourceRequest, RootlessPodmanAdapter,
    execute_command,
};

const SCHEMA_VERSION: &str = "1.0.0";

/// Operator ceiling every CLI invocation validates against.
///
/// The CLI has no config file yet, so this is a conservative, fixed policy
/// that a caller's resource flags are validated against (never exceeded).
/// Replace with a loaded config file once an operator actually needs a
/// different ceiling (tracked in `docs/product-technical-gap-baseline.md`).
fn default_policy() -> IsolationPolicy {
    IsolationPolicy {
        policy_id: "cli_default_policy_v1".to_owned(),
        maximum_memory_bytes: 1024 * 1024 * 1024,
        maximum_cpu_millicores: 4_000,
        maximum_processes: 256,
        maximum_lease_seconds: 900,
        maximum_tmpfs_bytes: 256 * 1024 * 1024,
        readiness_timeout_millis: 1_000,
        readiness_poll_interval_millis: 50,
        shutdown_grace_seconds: 5,
        run_as_user_id: 65_532,
        run_as_group_id: 65_532,
    }
}

#[derive(Debug)]
struct RunArgs {
    image_reference: String,
    request_id: Option<String>,
    memory_bytes: u64,
    cpu_millicores: u32,
    maximum_processes: u32,
    lease_seconds: u32,
    tmpfs_bytes: u64,
    podman_program: String,
    command: Vec<String>,
}

fn print_usage() {
    eprintln!(
        "usage: quarantine-sandbox-runtime run --image <repo@sha256:digest> [--request-id ID]\n\
         \x20      [--memory-bytes N] [--cpu-millicores N] [--max-processes N]\n\
         \x20      [--timeout-seconds N] [--tmpfs-bytes N] [--podman PATH] -- <command> [args...]\n\n\
         Runs one bounded command to completion inside a rootless-Podman sandbox with no network\n\
         namespace attachment (all egress denied) and prints its exit status plus bounded\n\
         stdout/stderr as JSON on stdout. This process's own exit code mirrors the sandboxed\n\
         command's exit code (137 if it was killed for exceeding --timeout-seconds)."
    );
}

fn parse_number<T: std::str::FromStr>(value: &str, flag: &str) -> Result<T, String> {
    value
        .parse()
        .map_err(|_| format!("{flag} must be a non-negative integer, got {value:?}"))
}

fn parse_args(
    policy: &IsolationPolicy,
    mut args: impl Iterator<Item = String>,
) -> Result<RunArgs, String> {
    match args.next().as_deref() {
        Some("run") => {}
        Some(other) => return Err(format!("unknown subcommand: {other}")),
        None => return Err("missing subcommand: expected `run`".to_owned()),
    }

    let mut image_reference = None;
    let mut request_id = None;
    let mut memory_bytes = policy.maximum_memory_bytes;
    let mut cpu_millicores = policy.maximum_cpu_millicores;
    let mut maximum_processes = policy.maximum_processes;
    let mut lease_seconds = policy.maximum_lease_seconds;
    let mut tmpfs_bytes = policy.maximum_tmpfs_bytes;
    let mut podman_program = "podman".to_owned();
    let mut command = Vec::new();

    while let Some(flag) = args.next() {
        if flag == "--" {
            command.extend(args);
            break;
        }
        let mut next_value = || {
            args.next()
                .ok_or_else(|| format!("{flag} requires a value"))
        };
        match flag.as_str() {
            "--image" => image_reference = Some(next_value()?),
            "--request-id" => request_id = Some(next_value()?),
            "--memory-bytes" => memory_bytes = parse_number(&next_value()?, "--memory-bytes")?,
            "--cpu-millicores" => {
                cpu_millicores = parse_number(&next_value()?, "--cpu-millicores")?;
            }
            "--max-processes" => {
                maximum_processes = parse_number(&next_value()?, "--max-processes")?;
            }
            "--timeout-seconds" => {
                lease_seconds = parse_number(&next_value()?, "--timeout-seconds")?;
            }
            "--tmpfs-bytes" => tmpfs_bytes = parse_number(&next_value()?, "--tmpfs-bytes")?,
            "--podman" => podman_program = next_value()?,
            other => return Err(format!("unknown flag: {other}")),
        }
    }

    let image_reference = image_reference.ok_or_else(|| "--image is required".to_owned())?;
    if command.is_empty() {
        return Err("no command given after `--`".to_owned());
    }

    Ok(RunArgs {
        image_reference,
        request_id,
        memory_bytes,
        cpu_millicores,
        maximum_processes,
        lease_seconds,
        tmpfs_bytes,
        podman_program,
        command,
    })
}

fn epoch_seconds() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |duration| duration.as_secs())
}

fn default_request_id() -> String {
    format!("cli-{}-{}", epoch_seconds(), std::process::id())
}

/// Parse, validate, execute, and report one CLI invocation.
///
/// Returns a plain process exit code rather than [`ExitCode`] (which has no
/// stable way to inspect the value it wraps) so this function is directly
/// assertable by tests against a fake-Podman backend. [`main`] wraps the
/// result and supplies the two pieces a test cannot substitute: the real
/// process argv and stdio.
fn run(args: impl Iterator<Item = String>) -> u8 {
    let policy = default_policy();
    let parsed = match parse_args(&policy, args) {
        Ok(parsed) => parsed,
        Err(message) => {
            eprintln!("error: {message}");
            print_usage();
            return 2;
        }
    };

    let request = CommandExecutionRequest {
        schema_version: SCHEMA_VERSION.to_owned(),
        request_id: parsed.request_id.unwrap_or_else(default_request_id),
        image_reference: parsed.image_reference,
        command: parsed.command,
        resources: ResourceRequest {
            memory_bytes: parsed.memory_bytes,
            cpu_millicores: parsed.cpu_millicores,
            maximum_processes: parsed.maximum_processes,
            lease_seconds: parsed.lease_seconds,
            tmpfs_bytes: parsed.tmpfs_bytes,
        },
    };

    let adapter = RootlessPodmanAdapter::new(parsed.podman_program);
    match execute_command(&adapter, &request, &policy, epoch_seconds()) {
        Ok(result) => {
            match serde_json::to_string_pretty(&result) {
                Ok(json) => println!("{json}"),
                Err(error) => eprintln!("warning: failed to render result as JSON: {error}"),
            }
            u8::try_from(result.exit_code().clamp(0, 255)).unwrap_or(255)
        }
        Err(error) => {
            eprintln!("error: {error}");
            2
        }
    }
}

fn main() -> ExitCode {
    ExitCode::from(run(env::args().skip(1)))
}

#[cfg(test)]
mod tests {
    use super::{
        default_policy, default_request_id, epoch_seconds, parse_args, parse_number, print_usage,
        run,
    };

    fn args(values: &[&str]) -> impl Iterator<Item = String> {
        values
            .iter()
            .map(|value| (*value).to_owned())
            .collect::<Vec<_>>()
            .into_iter()
    }

    #[test]
    fn print_usage_does_not_panic() {
        // Nothing to assert on stderr output itself; this just exercises the
        // line the real `--help`-shaped error path always prints through.
        print_usage();
    }

    #[test]
    fn run_returns_exit_code_two_on_invalid_arguments_without_touching_any_backend() {
        assert_eq!(run(args(&[])), 2);
    }

    #[test]
    fn parse_args_accepts_a_full_invocation_and_defaults_resources_to_the_policy_ceiling() {
        let policy = default_policy();
        let parsed = parse_args(
            &policy,
            args(&[
                "run",
                "--image",
                "repo@sha256:deadbeef",
                "--",
                "pytest",
                "-q",
            ]),
        )
        .expect("a minimal valid invocation must parse");

        assert_eq!(parsed.image_reference, "repo@sha256:deadbeef");
        assert_eq!(parsed.command, vec!["pytest".to_owned(), "-q".to_owned()]);
        assert_eq!(parsed.memory_bytes, policy.maximum_memory_bytes);
        assert_eq!(parsed.cpu_millicores, policy.maximum_cpu_millicores);
        assert_eq!(parsed.maximum_processes, policy.maximum_processes);
        assert_eq!(parsed.lease_seconds, policy.maximum_lease_seconds);
        assert_eq!(parsed.tmpfs_bytes, policy.maximum_tmpfs_bytes);
        assert_eq!(parsed.podman_program, "podman");
        assert!(parsed.request_id.is_none());
    }

    #[test]
    fn parse_args_accepts_every_resource_and_identity_override() {
        let policy = default_policy();
        let parsed = parse_args(
            &policy,
            args(&[
                "run",
                "--image",
                "repo@sha256:deadbeef",
                "--request-id",
                "custom-id",
                "--memory-bytes",
                "1024",
                "--cpu-millicores",
                "500",
                "--max-processes",
                "8",
                "--timeout-seconds",
                "30",
                "--tmpfs-bytes",
                "2048",
                "--podman",
                "/usr/local/bin/podman",
                "--",
                "sh",
                "-c",
                "echo hi",
            ]),
        )
        .expect("every documented flag must parse");

        assert_eq!(parsed.request_id.as_deref(), Some("custom-id"));
        assert_eq!(parsed.memory_bytes, 1024);
        assert_eq!(parsed.cpu_millicores, 500);
        assert_eq!(parsed.maximum_processes, 8);
        assert_eq!(parsed.lease_seconds, 30);
        assert_eq!(parsed.tmpfs_bytes, 2048);
        assert_eq!(parsed.podman_program, "/usr/local/bin/podman");
        assert_eq!(
            parsed.command,
            vec!["sh".to_owned(), "-c".to_owned(), "echo hi".to_owned()]
        );
    }

    #[test]
    fn parse_args_rejects_a_missing_subcommand() {
        let policy = default_policy();
        assert_eq!(
            parse_args(&policy, args(&[])).unwrap_err(),
            "missing subcommand: expected `run`"
        );
    }

    #[test]
    fn parse_args_rejects_an_unknown_subcommand() {
        let policy = default_policy();
        assert_eq!(
            parse_args(&policy, args(&["stop"])).unwrap_err(),
            "unknown subcommand: stop"
        );
    }

    #[test]
    fn parse_args_rejects_a_missing_image_flag() {
        let policy = default_policy();
        assert_eq!(
            parse_args(&policy, args(&["run", "--", "echo", "hi"])).unwrap_err(),
            "--image is required"
        );
    }

    #[test]
    fn parse_args_rejects_no_command_after_the_separator() {
        let policy = default_policy();
        assert_eq!(
            parse_args(&policy, args(&["run", "--image", "repo@sha256:deadbeef"])).unwrap_err(),
            "no command given after `--`"
        );
    }

    #[test]
    fn parse_args_rejects_an_unknown_flag() {
        let policy = default_policy();
        assert_eq!(
            parse_args(
                &policy,
                args(&[
                    "run",
                    "--image",
                    "repo@sha256:deadbeef",
                    "--bogus",
                    "--",
                    "echo"
                ]),
            )
            .unwrap_err(),
            "unknown flag: --bogus"
        );
    }

    #[test]
    fn parse_args_rejects_a_flag_missing_its_value() {
        let policy = default_policy();
        assert_eq!(
            parse_args(&policy, args(&["run", "--image"])).unwrap_err(),
            "--image requires a value"
        );
    }

    #[test]
    fn parse_args_rejects_a_non_numeric_resource_value() {
        let policy = default_policy();
        assert_eq!(
            parse_args(
                &policy,
                args(&[
                    "run",
                    "--image",
                    "repo@sha256:deadbeef",
                    "--memory-bytes",
                    "not-a-number",
                    "--",
                    "echo",
                ]),
            )
            .unwrap_err(),
            "--memory-bytes must be a non-negative integer, got \"not-a-number\""
        );
    }

    #[test]
    fn parse_number_parses_valid_input_and_rejects_invalid_input() {
        assert_eq!(parse_number::<u32>("42", "--flag"), Ok(42));
        assert_eq!(
            parse_number::<u32>("-1", "--flag"),
            Err("--flag must be a non-negative integer, got \"-1\"".to_owned())
        );
    }

    #[test]
    fn default_policy_is_internally_valid() {
        // `default_policy()` feeds every CLI invocation; if it were internally
        // inconsistent every invocation would fail identically and opaquely.
        assert!(default_policy().validate().is_ok());
    }

    #[test]
    fn epoch_seconds_and_default_request_id_produce_plausible_values() {
        let now = epoch_seconds();
        assert!(
            now > 1_700_000_000,
            "epoch_seconds must return a real Unix timestamp"
        );

        let id = default_request_id();
        assert!(id.starts_with("cli-"));
        assert!(id.contains(&std::process::id().to_string()));
    }

    /// `run()`'s success/backend-error paths need a real subprocess to shell
    /// out to, matching `tests/podman_command_execution.rs`'s established
    /// fake-Podman-process pattern; gated to Linux like every such test in
    /// this crate (the `/bin/sh` scripts assume a Linux CI runner).
    #[cfg(target_os = "linux")]
    mod fake_backend {
        use super::{args, run};
        use std::{
            fs,
            os::unix::fs::PermissionsExt,
            path::PathBuf,
            sync::atomic::{AtomicU64, Ordering},
            time::{SystemTime, UNIX_EPOCH},
        };

        static NEXT_PATH_ID: AtomicU64 = AtomicU64::new(0);

        fn write_executable(name: &str, script: &str) -> PathBuf {
            let nanos = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("system clock should be after the Unix epoch")
                .as_nanos();
            let unique_id = NEXT_PATH_ID.fetch_add(1, Ordering::Relaxed);
            let program = std::env::temp_dir().join(format!(
                "quarantine-sandbox-runtime-cli-{name}-{}-{nanos}-{unique_id}",
                std::process::id()
            ));
            fs::write(&program, script).expect("fake Podman should be writable");
            let mut permissions = fs::metadata(&program)
                .expect("fake Podman metadata should exist")
                .permissions();
            permissions.set_mode(0o700);
            fs::set_permissions(&program, permissions).expect("fake Podman should be executable");
            program
        }

        const SUCCESS_SCRIPT: &str = "#!/bin/sh\nset -eu\ncase \"${1:-}:${2:-}\" in\n  \
             info:--format) printf '%s\\n' '{\"host\":{\"security\":{\"rootless\":true,\"seccompEnabled\":true,\"seccompProfilePath\":\"/x\",\"apparmorEnabled\":true,\"selinuxEnabled\":false}},\"version\":{\"Version\":\"6.1.0\"}}' ;;\n  \
             create:--name) printf 'fake-id\\n' ;;\n  \
             start:*) : ;;\n  \
             container:inspect) printf '%s\\n' '[{\"Id\":\"fake-id\",\"AppArmorProfile\":\"containers-default\",\"ProcessLabel\":\"\",\"EffectiveCaps\":[],\"BoundingCaps\":[],\"Config\":{\"User\":\"65532:65532\"},\"HostConfig\":{\"ReadonlyRootfs\":true,\"Privileged\":false,\"SecurityOpt\":[\"no-new-privileges\"],\"UsernsMode\":\"auto\",\"PidMode\":\"private\",\"IpcMode\":\"none\",\"Memory\":268435456,\"NanoCpus\":1000000000,\"PidsLimit\":16}}]' ;;\n  \
             top:*) printf 'PID SECCOMP CAPEFF CAPBND CAPINH CAPPRM CAPAMB LABEL\\n1 filter - - - - - containers-default (enforce)\\n' ;;\n  \
             wait:*) printf '9\\n' ;;\n  \
             logs:*) printf 'cli stdout\\n' ;;\n  \
             rm:--force) : ;;\n  \
             *) exit 91 ;;\nesac\n";

        fn digest_pinned_image() -> String {
            format!("repo@sha256:{}", "e".repeat(64))
        }

        #[test]
        fn run_mirrors_the_sandboxed_exit_code_and_prints_json_on_a_successful_call() {
            let program = write_executable("success", SUCCESS_SCRIPT);
            let image = digest_pinned_image();
            let exit_code = run(args(&[
                "run",
                "--image",
                image.as_str(),
                "--podman",
                program.to_str().expect("temp path should be valid UTF-8"),
                "--",
                "pytest",
                "-q",
            ]));

            assert_eq!(
                exit_code, 9,
                "the CLI's own exit code must mirror the sandboxed command's exit code"
            );
            let _ = fs::remove_file(program);
        }

        #[test]
        fn run_returns_exit_code_two_when_the_backend_itself_fails() {
            let script = "#!/bin/sh\nset -eu\nexit 91\n";
            let program = write_executable("backend-error", script);
            let image = digest_pinned_image();
            let exit_code = run(args(&[
                "run",
                "--image",
                image.as_str(),
                "--podman",
                program.to_str().expect("temp path should be valid UTF-8"),
                "--",
                "pytest",
                "-q",
            ]));

            assert_eq!(
                exit_code, 2,
                "a backend failure must not be reported as a sandboxed exit code"
            );
            let _ = fs::remove_file(program);
        }
    }
}
