# ADR 0007: Bounded command-execution contract

- **Status:** Proposed
- **Date:** 2026-09-02

This ADR remains Proposed while the command/runtime owner stack is Draft and its dependency chain has not integrated into the protected branch. The contract may be reviewed and tested on the candidate stack, but it becomes an Accepted architectural decision only after protected integration and fresh exact-head contract/security evidence under live governance.

## Context

`ApplicationServiceRequest`/`ApplicationServiceLease` (ADR-0006) model a long-lived, readiness-gated network service: the runtime returns a loopback endpoint once the workload is reachable, and the consumer polls or connects to it. That shape does not fit every consumer.

`ContextualWisdomLab/.github` central review (Noema and OpenCode Review) evaluates every PR across the organization by reading diffs and reasoning about them; it never executes the code under review. Its own `scripts/ci/sandboxed_verify.py` and `scripts/ci/sandboxed_web_e2e.py` are named as the "actually-executed PoC" evidence mechanism its review gate requires, but both currently isolate locally on the CI runner itself -- `sandboxed_verify.py` copies the repository into a scrubbed `tempfile` workspace and calls `subprocess.run` directly on the runner (see its module docstring and `copy_workspace`/`run_command`); `sandboxed_web_e2e.py` additionally wraps commands in `bwrap` (bubblewrap) when available. Neither calls out to this runtime. Wiring them to a shared, dedicated isolation runtime -- so a review verdict is backed by execution evidence from an actual quarantine boundary rather than a runner-local sandbox -- was the concrete integration gap identified when this change started (see `docs/product-technical-gap-baseline.md`, "External consumer: CI review PoC execution").

That consumer does not want a network service. It wants to run one bounded command (a test suite, a proof-of-concept script) inside an isolated sandbox *to completion*, and to receive a structured pass/fail plus bounded output. A nonzero exit status is an expected, valid outcome the consumer decides how to treat -- not a runtime failure the way a readiness-gated service failure is. The two lifecycle models therefore require different domain contracts even when they share the same isolation policy and infrastructure backend.

The repository's current Context Map assigns **Command Execution** to the `sandbox_execution` Core subdomain and the readiness-gated application-service lifecycle to the `application_service` Supporting subdomain. The original ADR implementation placed the bounded-command contract under `src/application_service/command_execution.rs`; issue #116 identified that physical ownership as inconsistent with the live architecture and DDD dependency rule.

## Decision

`CommandExecutionRequest`, `CommandExecutionResult`, `CommandExecutionBackend`, `CommandExecutionError`, and `execute_command` are owned by `sandbox_execution` Core at `src/sandbox_execution/bounded_command_execution.rs`. The `application_service` Supporting context does not keep a second command source of truth. The crate facade preserves the existing root-level public command names while the implementation owner moves to Core.

The Core command request reuses Core `IsolationPolicy`/`ResourceRequest`, digest-pinned image validation, identifier limits, and direct-argv validation rather than depending on Supporting application-service types or copying those invariants. Infrastructure adapters implement the Core backend port. `execute_command` validates a request against an operator policy and delegates to that port; a nonzero `CommandExecutionResult::exit_code` is a successful call, never a `CommandExecutionError`.

The existing public `ApplicationServiceError` name is retained as a compatibility alias for the shared sandbox-runtime failure taxonomy while the owner stack remains unreleased. This is a compatibility surface, not authority for the Supporting context to own command domain truth. Any later split of service-only failure variants must preserve the released contract or be versioned explicitly rather than reintroducing a Core-to-Supporting dependency.

The concrete Podman command backend remains infrastructure work in `src/infrastructure/podman.rs` and the runtime-gate adapter. Backend implementation and Core contracts must remain separate: Podman CLI response shapes and lifecycle mechanics do not enter the Core request/result model.

## Alternatives

- **Keep bounded command execution inside `application_service`:** rejected. A run-to-completion command has no readiness endpoint or service lease and is a Core sandbox-execution lifecycle already named in the repository Context Map. Keeping its request/result/backend port under the Supporting context reverses the intended dependency direction.
- **Reuse `ApplicationServiceRequest`/`ApplicationServiceLease` directly, treating "the command exited" as a readiness signal:** rejected. Readiness means "reachable as a network service," not "finished running with an exit code." Overloading it would make readiness-timeout and `ServiceEndpoint` semantics meaningless for a command that never exposes a port, and would force command callers to reason about service lease semantics that do not apply.
- **Model a nonzero exit status as an error:** rejected. A failing test suite or PoC that legitimately returns nonzero is the exact signal the consumer needs back, not a sandbox malfunction; conflating the two would make `CommandExecutionError` ambiguous between "the sandbox could not run your command" and "your command ran and failed."
- **Copy image/resource validation into both contexts:** rejected. The same isolation and immutable-image invariants would drift and create two authorities. Supporting contexts consume Core invariants instead.
- **Move the Podman adapter into Core together with the contract:** rejected. Podman is an infrastructure adapter. Core defines the port and lifecycle semantics; infrastructure implements them.

## Verification rule

Architecture fitness tests must fail if the bounded-command source of truth returns to `src/application_service`, if `sandbox_execution` imports the Supporting context, or if a second command module is introduced there. Contract/unit tests prove validation, delegation, result semantics, and serde behavior. Real command-isolation release claims additionally require exact-head Podman E2E evidence, positive effective LSM evidence, resource/lifecycle enforcement, 100% owned-production coverage, independent review/security approval, and immutable release provenance.

The ownership repair is not considered complete merely because files moved. The exact integrated head must keep public schemas and runtime semantics compatible, pass rustfmt/full tests/Clippy/rustdoc and architecture-fitness checks, and keep `docs/ARCHITECTURE.md`, TRD/traceability, and the product-technical gap baseline code-current before this ADR can become Accepted.
