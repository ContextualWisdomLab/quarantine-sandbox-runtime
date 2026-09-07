# Isolated Application Service Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make Quarantine Sandbox Runtime provide a reusable, fail-closed isolated application-service profile for Chat/Agent consumers while preserving the separate artifact-analysis boundary.

**Architecture:** Add a Core `sandbox_execution` bounded context and a Supporting `application_service` context. The first infrastructure adapter renders and executes rootless Podman commands using immutable image digests, a per-sandbox internal network, loopback-only port publication, read-only rootfs, dropped capabilities, no-new-privileges, user-namespace isolation, and explicit resource limits. Wardnet and contextual-orchestrator remain external consumer authorities.

**Tech Stack:** Rust 1.97.1 / Edition 2024, stdlib process/network/time primitives, serde, sha2, thiserror, rootless Podman as the first OCI adapter.

**Spec:** `docs/superpowers/specs/2026-09-01-isolated-application-service-design.md`

## Global Constraints

- Production implementation remains Rust-first and `#![forbid(unsafe_code)]`.
- Application images must be immutable `@sha256:` references and launch with `--pull=never`.
- Standard application service profile is rootless, non-privileged, read-only-root, capability-free, no-new-privileges, isolated-user-namespace, internal-network, loopback-only ingress, default-deny external egress, credential-free, and resource-bounded.
- Consumer Chat/Agent policy and Wardnet maliciousness/incident policy stay outside this repository.
- Public APIs require beginner-readable rustdoc.
- Production statement/function/region/branch coverage remains 100% where tooling exposes it.

---

### Task 1: Application-service domain contracts and RED tests

**Files:**
- Create: `tests/application_service_contracts.rs`
- Create: `docs/superpowers/specs/2026-09-01-isolated-application-service-design.md`
- Create: `docs/superpowers/plans/2026-09-01-isolated-application-service.md`

**Interfaces:**
- Consumes: existing crate root.
- Produces: failing compile-time expectations for `ApplicationServiceRequest`, `IsolationPolicy`, `ResourceRequest`, `ServiceProtocol`, `ApplicationServiceError`, and `RootlessPodmanAdapter::plan_at`.

- [ ] Add tests proving tag-only images fail, resource requests cannot exceed policy, and the Podman launch plan contains the required isolation controls.
- [ ] Push the test-only commit to `feat/runtime-foundation-tdd`.
- [ ] Run exact-head CI and verify the new test fails because the application-service API does not yet exist.

### Task 2: Core sandbox-execution values and application-service validation

**Files:**
- Create: `src/sandbox_execution/mod.rs`
- Create: `src/application_service/mod.rs`
- Modify: `src/lib.rs`

**Interfaces:**
- Produces: `IsolationPolicy::validate`, `ResourceRequest`, `ServiceProtocol`, `ApplicationServiceRequest::validate`, `ApplicationServiceError`.
- Consumes: no consumer repository internals.

- [ ] Implement explicit bounded identifier/text validation without shell parsing.
- [ ] Implement lowercase `@sha256:<64 hex>` image-reference validation.
- [ ] Implement nonzero port/resources and resource <= policy invariants.
- [ ] Export the new public domain types at the crate root.
- [ ] Run focused tests and keep changes minimal until GREEN.

### Task 3: Deterministic rootless Podman launch plan

**Files:**
- Create: `src/sandbox_execution/podman.rs`
- Modify: `src/sandbox_execution/mod.rs`
- Modify: `src/lib.rs`

**Interfaces:**
- Produces: `RootlessPodmanAdapter::plan_at(request, policy, started_at_epoch_seconds) -> Result<PodmanLaunchPlan, ApplicationServiceError>`.
- Produces audit accessors for rootless probe, network-create args, container-create args, and expiry.

- [ ] Derive deterministic sandbox/network names from request/image/policy with SHA-256 rather than raw caller text.
- [ ] Render `podman info --format {{.Host.Security.Rootless}}`.
- [ ] Render `podman network create --internal --disable-dns --ignore <network>`.
- [ ] Render `podman create` with `--pull=never`, `--read-only`, `--cap-drop=all`, `--security-opt=no-new-privileges`, `--userns=auto`, numeric non-root user/group, CPU/RAM/PID limits, bounded tmpfs, the unique internal network, and `--publish 127.0.0.1::<port>/tcp`.
- [ ] Append image reference and bounded command args without a shell.
- [ ] Run focused tests and full Rust formatting/clippy/tests.

### Task 4: Process-boundary launch, readiness, and cleanup

**Files:**
- Modify: `src/sandbox_execution/podman.rs`
- Create: `tests/podman_application_service.rs`

**Interfaces:**
- Produces: `RootlessPodmanAdapter::launch(&request, &policy) -> Result<ApplicationServiceLease, ApplicationServiceError>`.
- Produces: `RootlessPodmanAdapter::terminate(&lease) -> Result<CleanupReceipt, ApplicationServiceError>`.

- [ ] Write a controlled fake-Podman executable test that records argv, reports rootless mode, returns a deterministic loopback port, and uses a real local `TcpListener` to satisfy readiness.
- [ ] Verify RED because launch/terminate do not yet exist.
- [ ] Implement `std::process::Command` execution with bounded captured output and no shell.
- [ ] Require exact `true` rootless probe output.
- [ ] Create internal network, create/start container, query loopback port, poll bounded TCP readiness, and return a lease with isolation attestation and expiry.
- [ ] On readiness/start failures, attempt stop/remove/network cleanup before returning the failure.
- [ ] Implement explicit termination receipt.
- [ ] Verify the fake process-boundary suite and full crate suite GREEN.

### Task 5: Wire contracts, DDD authority docs, and product gap baseline

**Files:**
- Create: `schemas/application-service-request.schema.json`
- Create: `schemas/application-service-lease.schema.json`
- Create: `docs/adr/0005-sandbox-execution-context.md`
- Create: `docs/adr/0006-isolated-application-service.md`
- Create or update: `docs/product-technical-gap-baseline.md`
- Modify: `docs/ARCHITECTURE.md`
- Modify: `docs/PRD.md`
- Modify: `docs/TRD.md`
- Modify: `docs/SECURITY.md`
- Modify: `docs/THREAT_MODEL.md`
- Modify: `docs/TEST_STRATEGY.md`
- Modify: `docs/OPERABILITY.md`
- Modify: `docs/doctoring/REFERENCES.md`
- Modify: `docs/doctoring/STANDARD_TRACEABILITY.md`
- Modify: `AGENTS.md`
- Modify: `CLAUDE.md`
- Modify: `README.md`
- Modify: `CHANGELOG.md`
- Modify: `Cargo.toml`

**Interfaces:**
- Publishes consumer-neutral application-service request/lease contracts.
- Documents Wardnet and contextual-orchestrator as external consumers through ACLs, never as build-time dependencies.

- [ ] Update product description from artifact-only to reusable isolation + artifact analysis.
- [ ] Document the Context Map and Ubiquitous Language.
- [ ] Record Podman rootless/internal-network/loopback publication and gVisor/OCI sources in APA 7 form.
- [ ] Record remaining gaps: real rootless Podman CI E2E, durable TTL reaper after process crash, gVisor/containerd adapter, controlled egress, secret broker, Windows sandbox, and consumer integration.
- [ ] Update repository-policy validation if new required DDD/docs paths are introduced.
- [ ] Run docs/rustdoc/repository-policy checks.

### Task 6: Consumer owner-path integration

**Files:**
- No foreign source mutation from this repository writer.
- Create/update owner-path issue in `ContextualWisdomLab/contextual-orchestrator` only if live writer ownership permits issue routing.

**Interfaces:**
- Consumes the published application-service contract by immutable release/digest after this PR integrates.
- contextual-orchestrator keeps agent/task/tool authorization and treats a returned sandbox endpoint as a leased tool service, not ambient authority.

- [ ] Record an integration issue requiring a version/digest-pinned dependency, application allowlist/policy, request/session correlation, lease termination on task end/cancel, and no direct Podman/containerd calls from Chat/Agent domain code.
- [ ] Require exact consumer tests for unauthorized application rejection, lease cleanup, endpoint scoping, and no secret passthrough.

### Task 7: Exact-head verification and review

**Files:**
- No new behavior unless verification finds a defect.

- [ ] Run exact-head repository CI, branch coverage, SAST, Security, dependency review, SBOM/provenance, rustdoc, and package checks.
- [ ] Inspect all review threads and fix only valid current-head findings test-first.
- [ ] Keep PR Draft until the real container E2E claim and all live required gates are satisfied; do not represent command-plan/process-fake tests as real Podman isolation evidence.
- [ ] Enable auto-merge or merge only under unchanged-head live policy after qualifying independent review and every required gate succeeds.
