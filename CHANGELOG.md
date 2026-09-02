# Changelog

All notable changes to this project are documented in this file.

The format follows Keep a Changelog, and this project uses Semantic Versioning.

## [Unreleased]

### Added

- Bounded command-execution contract (`CommandExecutionRequest`/`CommandExecutionResult`/
  `CommandExecutionBackend`, `execute_command`) alongside the existing service-lease contract, for a
  consumer that needs to run one command to completion and receive a structured exit status plus
  bounded output rather than a readiness-gated network endpoint. Ships as a validated contract, a
  coordinator function, and a `#[cfg(test)]`-only fake backend; no production backend or CLI/HTTP
  entrypoint yet (see ADR-0007).
- Source-agnostic analysis request and evidence contracts.
- Bounded immutable artifact ingestion and SHA-256 identity.
- Deterministic executable, archive, document, script, text, and unknown-format classification.
- Pluggable static analyzer interface and attributable analyzer-failure evidence.
- Explicit runtime boundary declaration for no credentials, no network access, and no artifact execution in the static foundation profile.
- Draft 2020-12 JSON Schemas for artifact-analysis requests/evidence bundles and isolated application-service request/policy/lease/cleanup contracts.
- Security, threat-model, operability, testing, architecture, ADR, product-gap, and research-traceability baselines.
- Exact-head Rust quality and coverage workflows.
- Property tests for deterministic immutable ingestion.
- Optional typed `BoundedSourceContext` for source channel, original leaf file name, declared media type, opaque host artifact reference, and UTC submission time.
- Core `sandbox_execution` bounded context for isolation policy, resource budget, sandbox lifecycle, service lease, readiness, cleanup, and attestation.
- Supporting `application_service` bounded context for Agent/Chat consumers that need to launch one approved immutable application as a short-lived service.
- Rootless Podman P0 adapter with immutable image/no-pull policy, internal DNS-disabled network, loopback-only publication, read-only rootfs, bounded tmpfs, host-proxy inheritance disabled, capability drop, no-new-privileges, isolated namespaces, numeric non-root identity, CPU/RAM/PID/TTL bounds, readiness gating, and cleanup.
- Versioned `ApplicationServiceLease`, `IsolationAttestation`, and `CleanupReceipt` evidence contracts.
- Canonical effective-policy SHA-256 bound to both Podman ownership labels and application-service leases.
- Application-service lease schema `1.2.0`, including inspected backend version and effective seccomp/LSM/resource-control status; request and cleanup contracts remain `1.0.0`.
- Process-boundary fake-Podman integration tests covering launch/readiness/termination and fail-closed readiness cleanup.
- Real rootless-Podman acceptance covering the pinned backend, immutable fixture pre-pull, effective isolation, bounded HTTP readiness, explicit cleanup, and final container/network leak rejection on the reviewed source head.
- Caller-scoped `LeaseOwnerId`, `ApplicationServiceBackend` port, and process-local `ApplicationServiceCoordinator` for active-lease ownership, idempotent replay, bounded expiry cleanup, and backend-neutral lifecycle coordination.
- Regression coverage for duplicate retry suppression, changed-request conflicts, effective-policy conflicts, wrong-owner termination, concurrent duplicate launch, failed-launch reservation release, expired-lease attribution, cleanup-failure fairness across more than one bounded cleanup batch, and residual bounding capabilities.
- Consumer owner-path integration issue for `contextual-orchestrator` so Chat/Agent domain code consumes the published lease contract rather than directly invoking Podman/containerd.
- Architectural fitness validation for unique ADR identifiers, bounded-context dependency direction, and infrastructure-adapter placement.
- Fail-closed `RELEASE.md` runbook, tag-driven release workflow, and versioned release-evidence schema binding exact protected source, full coverage, an LSM-capable real runtime acceptance, byte-reproducible Cargo package, SPDX 3 SBOM, SHA-256 checksums, GitHub attestations, and GitHub Release assets.

### Changed

- Product responsibility is broadened from artifact-analysis-only to reusable hostile-workload isolation plus artifact-analysis evidence while preserving consumer business authority.
- Artifact-analysis implementation moved from generic crate-root files into `src/artifact_analysis/` to match the accepted DDD bounded context while preserving the public crate facade.
- Rootless Podman implementation moved from the Core `sandbox_execution` path into `src/infrastructure/`; the Core no longer depends on `application_service` error types.
- Podman now implements the application-service lifecycle port from `src/infrastructure/`; the Supporting `application_service` coordinator does not depend on the concrete Podman adapter.
- Failed expired-lease cleanup now increments a bounded retry-attempt counter; later cleanup passes prioritize expired leases with fewer attempts before repeatedly failing entries, preventing the first 64 failures from starving later expired workloads.
- Pre-publication duplicate ADR identifiers were consolidated into the canonical ADR 0001–0006 sequence before protected-branch integration.
- Evidence identity now includes policy, source revision, and ordered analyzer identifiers.
- Static analyzer findings are restricted to file-format and static-capability evidence.
- JSON Schemas expose UTF-8 byte limits and reject control text consistently with Rust validation.
- Runtime analysis no longer requires a caller-supplied artifact name; optional source-context file names remain untrusted classification metadata.
- Required free-form source metadata was replaced by a closed typed context capped at 1,024 serialized UTF-8 bytes.
- Source timestamp and media-type validation trace to RFC 3339 and BCP 13 (RFC 6838 as updated by RFC 9694).
- Architecture now separates Core `sandbox_execution` from Supporting `artifact_analysis` and `application_service` and treats Podman/gVisor/containerd/Kubernetes mechanisms as infrastructure adapters.
- Repository CI covers both live `develop` integration and stable `main`; publication remains restricted to a version tag on the exact protected `main` tip.
- Production consumers are required to pin a released package checksum and provenance/SBOM evidence rather than a transient pull-request head or branch artifact.

### Security

- Duplicate or malformed analyzer identifiers fail engine construction.
- Universal Mach-O headers are structurally bounded so Java class magic is not treated as Mach-O by signature alone.
- Unsupported dynamic profiles and analyzer failures remain explicitly inconclusive.
- Path-like file names, URL/credential-shaped host references, unknown source-context fields, malformed media types, invalid UTC timestamps, and oversized context payloads fail closed before artifact analysis.
- Mutable/tag-only OCI image references fail closed.
- Standard application-service contract exposes no privileged mode, host namespace, device, runtime socket, arbitrary mount, arbitrary environment, credential, wildcard bind, or arbitrary Internet-egress capability.
- Podman backend must report rootless mode; service publication is validated as IPv4 loopback before a lease is returned.
- Effective isolation is inspected after container start; read-only rootfs, no-new-privileges, private namespaces, seccomp, an LSM, resource limits, internal networking, and loopback publication must be positively verified for the P0 lease path.
- Capability-drop attestation requires both Podman effective and bounding capability sets to be empty; residual bounding capability such as `CAP_SYS_ADMIN` fails closed and triggers cleanup.
- Podman host proxy environment inheritance is explicitly disabled so `HTTP_PROXY`/`HTTPS_PROXY`/`NO_PROXY` values cannot become ambient application inputs.
- Partial-launch/readiness/attestation failures attempt cleanup and cleanup uncertainty becomes `CleanupFailed` rather than being hidden.
- Lease ownership is scoped by authenticated command context rather than an untrusted request field; wrong-owner cleanup fails before the backend is invoked.
- Application-service replay is bound to both immutable request content and the full effective isolation policy, so a changed policy cannot silently reuse a lease created under older limits.
- Repeated cleanup failures cannot monopolize the bounded expiry-cleanup window and indefinitely hide other expired application-service leases.
- Release publication requires a dedicated no-production-secret, SELinux-capable rootless Podman runner or a separately reviewed stronger isolation backend; an LSM-unavailable hosted runner cannot be promoted to passing release evidence.

### Not yet release evidence

- No protected product release exists yet. `0.1.0` remains the package version under development until the complete stacked runtime integrates and a dated changelog section is reviewed on the exact protected stable candidate.
- The GitHub-hosted Ubuntu 24.04 rootless Podman lane correctly fails closed because it cannot prove the required effective LSM on the current candidate. `ContextualWisdomLab/.github#1590` owns provisioning of the dedicated LSM-capable release/security runner; that external infrastructure prerequisite is not bypassed here.
- Caller-scoped lease ownership is currently process-local; authenticated transport binding, durable restart/orphan reclamation, distributed admission/resource reservation, stable wire errors, and signed durable receipts remain follow-on work.
- gVisor/containerd/Kubernetes adapters, controlled egress, secret broker, and stronger dynamic-detonation profiles remain follow-on work.
