# Product Requirements Document

## Product statement

Quarantine Sandbox Runtime is a reusable hostile-workload isolation and evidence runtime. It exposes bounded, versioned contracts for two product verticals:

1. **Artifact analysis** — accept hostile bytes, execute approved analysis profiles, and return deterministic evidence/completeness to a security consumer.
2. **Isolated application service** — start one consumer-approved immutable application image as a short-lived loopback service under a restrictive isolation policy and return an attested lease.

The runtime is deliberately not the authority for the consumer's business decision. Wardnet owns maliciousness verdict/incident/response. Chat/Agent control planes own application authorization, conversation/task/tool state, secrets, and user-visible actions.

## Target users

- Security platform engineers integrating Wardnet or other SOC/admission flows.
- Chat/Agent platform engineers who need a short-lived application without ambient host authority.
- Runtime operators maintaining rootless/strong-isolation backends.
- Auditors verifying that execution occurred under a declared image, policy, backend, resource budget, and cleanup state.

## Core user stories

### Artifact analysis

- As a security consumer, I can submit bytes without source credentials and receive immutable artifact identity plus attributable evidence.
- As a verdict authority, I can distinguish analysis completeness from maliciousness and refuse decisions when evidence is incomplete.
- As an operator, I can add stronger detonation/analyzer adapters without moving incident policy into the runtime.

### Isolated application service

- As an Agent control plane, after I authorize a specific application digest, I can request a bounded service and receive only a loopback endpoint plus lease/attestation.
- As a security operator, I can prove the standard application profile did not grant privileged mode, host namespaces/devices, runtime sockets, broad host mounts, arbitrary Internet egress, or ambient credentials.
- As an orchestrator, I can terminate a lease on every task terminal state and receive cleanup evidence.
- As an operator, I can fail closed when rootless execution, resource enforcement, readiness, or cleanup cannot be proven.

## Functional requirements

### Shared sandbox execution

- Validate operator isolation policy before starting work.
- Apply consumer resource requests only within operator maxima.
- Give each sandbox a deterministic/auditable identity without exposing raw caller text as infrastructure names.
- Record backend/runtime identity, immutable workload identity, policy, resource bounds, timestamps, endpoint where applicable, and isolation facts.
- Support explicit termination and bounded failure cleanup.
- Keep backend-specific implementation behind the sandbox boundary.

### Application-service P0

- Require OCI image reference pinned by lower-case SHA-256 digest.
- Do not pull an image during task launch.
- Require a rootless backend.
- Use read-only rootfs, a single bounded tmpfs, capability drop, no-new-privileges, isolated user/PID/IPC/UTS/cgroup namespaces, numeric non-root UID/GID, no restart, and no ambient container logs in the standard profile.
- Enforce CPU, RAM, PID, tmpfs, lease, readiness, and shutdown bounds.
- Create a per-sandbox internal DNS-disabled network and publish exactly one service to host IPv4 loopback on a random port.
- Invoke the application without a shell.
- Return an endpoint only after bounded readiness.
- Return a versioned lease and cleanup receipt.
- Provide no request fields for credentials, arbitrary environment variables, broad host mounts, devices, privileged mode, host namespaces, runtime sockets, or arbitrary Internet egress.

### Artifact-analysis foundation

- Preserve source bytes and SHA-256 identity.
- Bound artifact bytes and optional source metadata before analysis.
- Classify supported file/container families without executing content.
- Invoke ordered static analyzer adapters.
- Preserve analyzer failures as attributable evidence.
- Return `inconclusive` when a requested dynamic profile is unavailable.
- Keep consumer maliciousness verdict outside the runtime.

## Non-functional requirements

### Security

- `#![forbid(unsafe_code)]` unless superseded by a reviewed ADR.
- Fail closed on unverifiable isolation, malformed backend output, unavailable required resource controls, readiness timeout, and cleanup uncertainty.
- Secrets never enter standard sandbox profiles.
- Backend sockets and host devices are never exposed by the P0 application contract.
- No service is published on wildcard/external host addresses in P0.

### Reliability and recovery

- Consumer-visible leases are time-bounded.
- Partial launch failures attempt cleanup before returning.
- GA requires durable orphan/lease reclamation after runtime process crash.
- Backend/process diagnostics must not include consumer secrets or untrusted payload bytes.

### Quality

- Rust production logic.
- Complete public rustdoc.
- 100% owned production statement/function/region/branch coverage where tooling exposes it.
- Property/fuzz/security/resource/concurrency tests appropriate to each boundary.
- Real container E2E before making real isolation claims.
- Exact-head CI/SAST/security/dependency/SBOM/provenance/review evidence before merge/release.

## Out of scope for the current P0 application-service increment

- mutable image-tag resolution or registry pull;
- arbitrary outbound Internet access;
- secret injection/broker;
- arbitrary host filesystem mounts;
- GPU/device passthrough;
- Kubernetes or remote cluster scheduling;
- gVisor/containerd adapter implementation;
- application discovery/authorization (consumer-owned);
- billing/identity/chat/agent/task persistence;
- maliciousness verdict or incident response.

## Acceptance criteria

- Public request, policy, lease, cleanup, analysis, and evidence contracts are versioned and represented by Draft 2020-12 JSON Schemas where serialized.
- Existing artifact-analysis public Rust API remains available after DDD directory migration.
- Tag-only images and over-budget resource requests fail closed.
- Launch plan has no privileged/host-network/runtime-socket path and explicitly enforces P0 isolation flags.
- Process-boundary tests prove direct argv invocation, readiness gating, error cleanup, and explicit termination behavior.
- Real rootless Podman E2E proves the effective security boundary before the capability is called release-ready.
- `contextual-orchestrator` integration occurs through its owner issue/ACL and a published runtime artifact; no direct consumer Podman calls.
- Wardnet verdict policy remains outside this repository.
