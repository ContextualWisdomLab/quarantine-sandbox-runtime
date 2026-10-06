# ADR 0011: Offline application-service launch-plan transport

- **Status:** Proposed
- **Date:** 2026-10-06

This ADR remains Proposed while its implementation exists only on a Draft PR stacked on root Draft #1. Promote it to Accepted only after protected integration and current-head verification.

## Context

`docs/PRD.md` requires that the application-service launch plan has no privileged, host-network or runtime-socket path and explicitly enforces the P0 isolation flags. The library already builds that plan without executing a process (`RootlessPodmanAdapter::plan_at`). No runnable transport exposes it (issue #149).

Non-Rust consumers also have no executable byte-accurate validation path. The runtime bounds `request_id` to 128 UTF-8 bytes and each argv entry to 1,024 UTF-8 bytes, while the published request schema still counts characters until the versioned byte vocabulary owned by #101/#102 is released (#138).

ADR numbers 0007–0010 are used by other unmerged lineages (0010 is the static artifact-analysis CLI proposed for #148), so this decision uses 0011.

## Decision

Add one offline, synchronous binary `qsr-service-plan`. It uses the Rust standard library plus the `libc` constants `O_NOFOLLOW`, `O_NONBLOCK` and `O_NOCTTY` on Unix targets; it adds no other dependency and contains no `unsafe` code:

```text
qsr-service-plan --request <request.json> --policy <policy.json> --started-at <epoch-seconds>
```

- The binary is a branch-free shim. Logic lives in `src/application_service/plan_cli.rs`, an inbound adapter of the `application_service` Supporting context. It adds no domain rule. It reads and validates the policy (`IsolationPolicy::validate`) before it opens the request, so an invalid policy always reports `invalid_policy` whatever the state of the request file. It then deserializes the request and calls `RootlessPodmanAdapter::plan_at`, which runs `ApplicationServiceRequest::validate`. Calling the pure infrastructure planner directly is accepted for this first adapter; an application-layer use case is added only when a second inbound adapter needs the same flow.
- Inputs are opened with `O_NOFOLLOW | O_NONBLOCK | O_NOCTTY`, checked as regular files through the opened handle, and read with a 64 KiB inclusive bound. The transport is Unix-only; other targets report the input as unavailable.
- `--started-at` is required and accepts ASCII decimal digits only, so the output is a deterministic function of request bytes, policy bytes and timestamp.
- Success writes one JSON object conforming to `schemas/application-service-launch-plan.schema.json` and exits `0`. `execution` is always `not_performed` and `isolation_evidence` is always `not_established`.
- Failure writes nothing to stdout and exactly one line `qsr-service-plan: error=<code>` to stderr. Domain failures use `ApplicationServiceError::code()`, an exhaustive match with no wildcard arm. Diagnostics never contain request values, policy values or host paths. If writing the plan to stdout fails part way (exit `70`), some plan bytes can already be on stdout; a consumer must discard stdout whenever the exit status is not `0`.
- Exit codes follow the sysexits-style convention also proposed for #148: 0 plan emitted, 64 usage, 65 invalid input, 66 input unavailable, 67 input too large, 70 internal.

## Consequences

- Operators and consumers can review the exact Podman argv and validate inputs with the runtime's real byte bounds before any container exists.
- The plan echoes consumer argv and image reference verbatim. Requests must therefore not carry secrets, which the P0 contract already forbids.
- The tool checks consistency between a caller-supplied policy and request. It does not enforce organizational policy and does not authorize an application.
- This is not schema GREEN for #138. Published schema byte semantics remain owned by #101/#102.
- A plan is not isolation evidence. Rootless execution, effective LSM/seccomp/capability/resource/network enforcement and cleanup still require the real rootless-Podman acceptance lanes.
