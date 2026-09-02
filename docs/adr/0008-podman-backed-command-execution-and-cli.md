# ADR 0008: Podman-backed command execution and a CLI transport

- **Status:** Accepted
- **Date:** 2026-09-02

## Context

ADR-0007 accepted `CommandExecutionRequest`/`CommandExecutionResult`/`CommandExecutionBackend`
(`src/application_service/command_execution.rs`) as the "run one bounded command to completion" wire
contract, deliberately shipping only the validated contract, the coordinator, and a `#[cfg(test)]`-only
fake backend. It named two follow-on gaps: a real Podman-backed backend, and a CLI or HTTP entrypoint a
caller outside this crate can actually invoke. Both were tracked in
`docs/product-technical-gap-baseline.md` ("Bounded command execution") as `Missing`.

`ContextualWisdomLab/.github`'s `scripts/ci/sandboxed_verify.py`/`scripts/ci/sandboxed_web_e2e.py` --
named in that repository's own `CLAUDE.md` as the "actually-executed PoC" evidence mechanism the
OpenCode review gate requires -- still isolate locally on the CI runner itself rather than through this
runtime. Landing a real backend and a first transport is what makes wiring them (a follow-on, owner-path
change in `.github`, not this repository) possible at all.

## Decision

### Backend: `RootlessPodmanAdapter::run_command_at`

Add `run_command_at` to the existing `RootlessPodmanAdapter` (`src/infrastructure/podman.rs`), reusing
its P0 isolation profile (rootless, no-pull, read-only rootfs, all capabilities dropped, no-new-privileges,
isolated user/pid/ipc/uts/cgroup namespaces, non-root UID/GID, resource limits) and its `Send + Sync`
`BoundedCommandRunner` process supervisor (`src/infrastructure/bounded_command.rs`), rather than adding a
second adapter or a second process-supervision primitive. `CommandExecutionBackend` is implemented for
`RootlessPodmanAdapter` the same way `ApplicationServiceBackend` already is
(`src/infrastructure/application_service_backend.rs`): one thin trait method forwarding to the inherent
`run_command_at`.

Command execution differs from the service lease in three ways that shaped the implementation, each
confirmed by exercising a real rootless Podman installation directly (this adapter's existing tests all
either use a `#[cfg(test)]` fake process or are `#[ignore]`d pending a dedicated CI runner; none of them
previously ran the *new* code path against a real container runtime before this change did):

1. **No per-sandbox network.** The service lease creates an internal, DNS-disabled network so it can
   publish a loopback port. A one-shot command has no port to publish, so it runs with `--network none`
   instead -- strictly stronger egress denial (no network namespace attachment at all) that needs no
   separate network object to create or clean up.
2. **Isolation verification must tolerate the workload already having exited.** The service lease
   inspects the *live* process (`podman top`, which requires a running PID 1) before trusting the
   sandbox enough to return an endpoint the caller will connect to. A bounded command can legitimately
   finish in microseconds -- faster than this adapter can issue the `podman top` call -- and that race is
   not an isolation failure; it is the sandbox doing exactly what it was asked. `run_command_at` attempts
   the live check and falls back to Podman's own static container-configuration record (`podman
   container inspect`, which remains valid regardless of run state) when the process has already exited.
   Both paths verify every P0 control; only the source of capability/seccomp/LSM evidence differs.
3. **Completion is observed with `podman wait` plus `podman logs`, not by attaching to the workload's
   live pipes.** `podman start --attach` on an *already-started* container (necessary here because
   verification needs the container running first) was observed against Podman 6.1.0 to not reliably
   mirror the container's own exit code through its own process exit status. `podman wait`'s stdout is
   the authoritative exit code either way -- for the ordinary path and, after an explicit `podman kill`,
   for a workload forcibly terminated on timeout -- and `podman logs` against a `k8s-file` log driver
   (the service lease keeps `--log-driver=none`, since it never needs to retrieve logs) returns the
   workload's stdout and stderr on its own already-separated stdout/stderr streams.

`BoundedCommandRunner` gained a `run_to_completion` mode (`src/infrastructure/bounded_command.rs`)
alongside its existing `run`: a wall-clock timeout or retained-output overflow on the *administrative*
CLI calls `run` supervises (a well-behaved Podman CLI never legitimately produces more than a few
kilobytes of JSON) is correctly a hard error, but the same conditions on the *workload's own* output are
expected, reportable facts -- `CommandExecutionResult::timed_out`/`stdout_truncated`/`stderr_truncated`
exist precisely to carry them, not to be permanently `false` because a real backend discarded that
evidence to keep reusing the hard-fail path.

Three Podman-version-specific deserialization/comparison gaps were found and fixed by testing against a
real installation (Podman 6.1.0 locally; CI is pinned to 5.8.4) rather than assumed from the API
documentation alone:

- `ContainerInspection.{effective_caps,bounding_caps}` can be JSON `null` (not merely absent) once every
  capability is dropped; `#[serde(default)]` alone only covers a missing key, not an explicit `null` for
  a non-`Option` field, so both fields now use a `deserialize_with` helper that treats either the same.
- `HostConfig.UsernsMode` was empty for a container created with `--userns=auto`; the same fact is
  recorded only as the `io.podman.annotations.userns` annotation. `isolated_user_namespace_verified`
  checks both representations and is shared by the service-lease and command-execution verification
  paths.
- `podman top`'s label column can carry a trailing NUL byte through from the underlying
  NUL-terminated `/proc/<pid>/attr/current` kernel interface; `parse_process_security_top` now strips it,
  since a real LSM label never legitimately contains one.

All three are genuine cross-version compatibility fixes to already-shared code, not scope unique to
command execution; they benefit the existing service-lease path's own real-Podman evidence equally.

### Transport: a synchronous CLI, not an HTTP service

Add `src/main.rs` and a `[[bin]]` target (`quarantine-sandbox-runtime`) exposing one subcommand:

```text
quarantine-sandbox-runtime run --image <repo@sha256:digest> [--request-id ID]
  [--memory-bytes N] [--cpu-millicores N] [--max-processes N] [--timeout-seconds N]
  [--tmpfs-bytes N] [--podman PATH] -- <command> [args...]
```

It prints the resulting `CommandExecutionResult` as JSON on stdout and exits with the sandboxed
command's own exit code (a validation or backend failure prints to stderr and exits `2`, with no JSON on
stdout -- a caller that needs to tell that apart from a sandboxed command that itself exited `2` checks
for JSON, not exit-code value alone).

A CLI, not an HTTP service, because `execute_command`/`CommandExecutionBackend` model "run one command to
completion and return its result," not "become reachable and stay up": there is no lease to renew, no
endpoint to poll, no shutdown to coordinate. `ApplicationServiceRequest`/`ApplicationServiceLease`
already model the shape that *does* need a long-lived, addressable service; `CommandExecutionRequest`
does not, and building an HTTP wrapper around a synchronous one-shot call would add a listener,
request/response framing, and a process-lifetime story this contract has no use for. A CI script (or
`scripts/ci/sandboxed_verify.py`'s eventual owner-path change in `.github`) can invoke the binary
directly and read its own exit code or parse its JSON stdout; nothing about that requires a network
transport. If a future consumer genuinely needs to run many commands concurrently behind one
long-lived listener, that is new evidence for an HTTP transport ADR at that time, not a reason to build
one speculatively now.

Argv, not a `--command "shell string"` flag, because `CommandExecutionRequest.command` is `Vec<String>`
executed directly with no shell (see ADR-0007's "no shell is used" invariant); a single shell-string flag
would either force reimplementing shell-word-splitting in this CLI (reintroducing exactly the ambiguity
the direct-argv contract exists to avoid) or only ever accept one argument. The Unix `--` convention
(`cargo run --`, `env`, `sudo`) both avoids that and needs no new parsing.

No config file and no `--network-policy` flag: the CLI validates every invocation against one
conservative, hardcoded `IsolationPolicy` ceiling (`default_policy()` in `src/main.rs`) because no
config-file loader exists yet, and there is no per-request network-policy flag because
`CommandExecutionRequest` has no network field at all (ADR-0007) -- the backend always denies all egress
unconditionally (`--network none`). Both are `docs/product-technical-gap-baseline.md` follow-on items,
not silently dropped scope.

## Alternatives

- **Ship an HTTP service as the first transport:** rejected for the reasons above -- the contract is a
  one-shot call, and the org's own convention (ADR-0007's own alternatives section) already rejected
  overreaching one bounded slice in a single change.
- **Attach to the workload's live pipes via a single `podman start --attach` after a separate
  `podman start`:** rejected after direct testing showed its own process exit status does not reliably
  mirror the container's exit code against Podman 6.1.0 once the container was already running (`podman
  start --attach` on a container this adapter itself just started via a separate detached `start` call
  returned exit status `125` -- an unrelated Podman CLI error code -- while the container's actual,
  `podman inspect`-confirmed exit code was `7`). `podman wait`'s stdout does not have this failure mode.
- **Drop pre-execution isolation verification for command execution, trusting create-time flags alone:**
  rejected. `CommandExecutionResult` has no attestation field to populate the way
  `ApplicationServiceLease::isolation_attestation` does, but the security invariants
  (`AGENTS.md`: "fail-closed contracts") apply to every workload this runtime executes, not only ones
  whose result type happens to carry attestation evidence back to the caller. A silently-downgraded
  sandbox would still report a plausible-looking pass/fail result, which is a worse outcome than the
  fail-closed error this ADR's verification step returns instead.
- **Require the sandboxed command's own wall-clock lease to bound the log-retrieval (`podman logs`) call
  too:** rejected. Log retrieval reads what the log driver already wrote after the workload's own
  completion (or forced termination) is already known; bounding it by the workload's potentially
  much-longer lease would let a genuinely hung log driver masquerade as the *workload* having hung, which
  `wait_for_command`'s already-captured `timed_out` fact would then misreport. `run_command_at` bounds log
  retrieval by the adapter's ordinary short administrative timeout instead, and treats a timeout there as
  a distinct `BackendCommandTimedOut { operation: "container_logs" }` backend error.

## Verification rule

`tests/podman_command_execution_e2e.rs` (mirroring `tests/podman_rootless_e2e.rs`'s existing
`#[ignore]`d acceptance shape) exercises `run_command_at` against a real rootless Podman installation:
exit status and bounded stdout/stderr are observed correctly; the sandbox cannot see the host's own
filesystem outside the pulled image and cannot reach an external network; and a command that exceeds its
lease is force-killed, reported `timed_out: true`, and completes near its bounded lease rather than
running to its own completion. These three properties were confirmed locally against Podman 6.1.0 before
this change merged (see the accompanying PR description for the exact commands and observed output); CI
runs the same tests as one `--exact` job step per test once `.github#1590`'s dedicated LSM-capable runner
exists, the same blocker ADR-0006/ADR-0007 already carry for the service lease's own release-time proof.
Fake-backend/process tests remain necessary but not sufficient, same as ADR-0006 and ADR-0007: a release
claim about real command-execution isolation requires this real-Podman evidence, not only the
`#[cfg(test)]`-only unit coverage this backend also carries.
