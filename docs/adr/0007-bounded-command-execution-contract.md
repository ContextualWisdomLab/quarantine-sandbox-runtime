# ADR 0007: Bounded command-execution contract

- **Status:** Proposed
- **Date:** 2026-09-02

This ADR remains Proposed while PR #13 is Draft and its dependency chain has not integrated into the protected branch. The contract may be reviewed and tested on the candidate stack, but it becomes an Accepted architectural decision only after protected integration and fresh exact-head contract/security evidence under live governance.

## Context

`ApplicationServiceRequest`/`ApplicationServiceLease` (ADR-0006) model a long-lived, readiness-gated network service. A CI/security consumer also needs a narrower shape: run one bounded command to completion and return structured exit status plus bounded output without creating another isolation authority.

## Decision

Add `CommandExecutionRequest`, `CommandExecutionResult`, `CommandExecutionBackend`, and `execute_command` inside the existing `application_service` Supporting bounded context. The contract reuses digest-pinned image validation, `IsolationPolicy`, and `ResourceRequest`. A nonzero workload exit status is a valid result rather than a runtime malfunction.

PR #13 contains the contract/coordinator and fake-backend tests only. A real Podman backend and transport remain downstream and must reuse the same effective-isolation policy rather than duplicate it.

## Alternatives

- Reuse the service-lease contract for one-shot commands: rejected because readiness/endpoint/lease semantics do not describe completion and exit status.
- Treat nonzero workload exit as backend error: rejected because consumers must distinguish workload failure from sandbox/runtime failure.
- Add backend and transport in the same contract slice: rejected to keep the boundary reviewable and dependency-ordered.

## Verification rule

Fake-backend tests prove contract validation/delegation/serde only. A release claim about real command isolation requires the same effective rootless-runtime evidence bar as ADR-0006 and an immutable released consumer contract.
