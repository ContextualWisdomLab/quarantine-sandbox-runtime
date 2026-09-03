# ADR 0007: Bounded command-execution contract

- **Status:** Proposed
- **Date:** 2026-09-02

This ADR remains Proposed while PR #13 is Draft and its dependency chain has not integrated into the protected branch. The contract may be reviewed and tested on the candidate stack, but it becomes an Accepted architectural decision only after protected integration and fresh exact-head contract/security evidence under live governance.

## Context

`ApplicationServiceRequest`/`ApplicationServiceLease` (ADR-0006) model a long-lived, readiness-gated
network service: the runtime returns a loopback endpoint once the workload is reachable, and the
consumer polls or connects to it. That shape does not fit every consumer.

`ContextualWisdomLab/.github` central review (Noema and OpenCode Review) evaluates every PR across the
organization by reading diffs and reasoning about them; it never executes the code under review. Its
own `scripts/ci/sandboxed_verify.py` and `scripts/ci/sandboxed_web_e2e.py` are named as the
"actually-executed PoC" evidence mechanism its review gate requires, but both currently isolate
locally on the CI runner itself -- `sandboxed_verify.py` copies the repository into a scrubbed
`tempfile` workspace and calls `subprocess.run` directly on the runner (see its module docstring and
`copy_workspace`/`run_command`); `sandboxed_web_e2e.py` additionally wraps commands in `bwrap`
(bubblewrap) when available. Neither calls out to this runtime. Wiring them to a shared, dedicated
isolation runtime -- so a review verdict is backed by execution evidence from an actual quarantine
boundary rather than a runner-local sandbox -- was the concrete integration gap identified when this
change started (see `docs/product-technical-gap-baseline.md`, "External consumer: CI review PoC
execution").

That consumer does not want a network service. It wants to run one bounded command (a test suite, a
proof-of-concept script) inside an isolated sandbox *to completion*, and to receive a structured
pass/fail plus bounded output. A nonzero exit status is an expected, valid outcome the consumer decides
how to treat -- not a runtime failure the way an `ApplicationServiceError` is. No existing type in this
runtime models "run to completion and report exit status"; only "become ready as a network service"
exists.

## Decision

Add `CommandExecutionRequest`/`CommandExecutionResult` and a `CommandExecutionBackend` port
(`src/application_service/command_execution.rs`) as a narrower sibling contract inside the existing
`application_service` Supporting bounded context, alongside (not replacing) the service-lease contract.
`CommandExecutionRequest` reuses the existing digest-pinned image reference, identifier, and
command-argument validation, and the `IsolationPolicy`/`ResourceRequest` budget contracts, rather than
duplicating them. `execute_command` validates a request against an operator policy and delegates to a
backend; a nonzero `CommandExecutionResult::exit_code` is a successful call, never a
`CommandExecutionError`.

This PR ships the validated contract, the coordinator function, and a `#[cfg(test)]`-only
`FakeCommandExecutionBackend` proving the contract's request/validation/result/serde behavior. It does
**not** ship a real Podman-backed `CommandExecutionBackend`, and it does not add a CLI or HTTP
entrypoint. Per ADR-0004 (truthful capability claims), this capability is
`implemented_on_active_pr` for the contract and coordinator, and `planned` for a real backend and any
transport:

- A real backend requires running the requested command to completion inside the same P0 Podman
  isolation profile as `ApplicationServiceBackend` (rootless, no-pull, read-only rootfs, dropped
  capabilities, internal network, resource limits) and capturing bounded stdout/stderr plus exit
  status -- infrastructure work in `src/infrastructure/podman.rs`, not this contract.
- A CLI or HTTP entrypoint that a CI reviewer script can shell out to or call over the network is
  separate follow-on work; it has no backend to wrap until the item above lands.
- Both remain blocked, like the rest of `application_service`'s real-isolation claims, on
  `ContextualWisdomLab/.github#1590` (the dedicated LSM-capable runner) for any *release* evidence,
  though the contract and a real backend's unit/fake-process tests do not themselves require that
  runner.

## Alternatives

- **Reuse `ApplicationServiceRequest`/`ApplicationServiceLease` directly, treating "the command exited"
  as a readiness signal:** rejected. Readiness means "reachable as a network service," not "finished
  running with an exit code." Overloading it would make `ApplicationServiceError::ReadinessTimeout` and
  `ServiceEndpoint` meaningless for a command that was never meant to expose a port, and would force
  every command-execution caller to also reason about lease expiry semantics that do not apply.
- **Model a nonzero exit status as an error:** rejected. A failing test suite or a PoC that legitimately
  returns nonzero is the exact signal the consumer needs back, not a sandbox malfunction; conflating
  the two would make `CommandExecutionError` ambiguous between "the sandbox could not run your command"
  and "your command ran and failed," which a CI reviewer must be able to tell apart.
- **Ship a real Podman-backed backend and a CLI entrypoint in this same PR:** rejected for this
  iteration. The existing `application_service` stack (PRs #1/#6/#9/#10) is active, Draft, and already
  blocked on `.github#1590` for real-isolation release evidence; landing an unrelated new backend and a
  first CLI/HTTP surface in the same change as a new domain contract would both overreach one bounded
  slice and risk conflicting with that in-flight stack. The contract lands first so the next slice has
  a reviewed target to implement against.

## Verification rule

Fake-backend tests in this PR are necessary but not sufficient, same as ADR-0006: they prove the
contract's validation, delegation, and serde behavior, not real command isolation. A release claim about
real command-execution isolation requires the same rootless-Podman E2E evidence bar as ADR-0006, once a
real backend exists.
