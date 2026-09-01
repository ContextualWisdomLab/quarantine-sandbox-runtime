# Changelog

All notable changes to this project are documented in this file.

The format follows Keep a Changelog, and this project uses Semantic Versioning.

## [Unreleased]

### Added

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
- Process-boundary fake-Podman integration tests covering launch/readiness/termination and fail-closed readiness cleanup.
- Real rootless-Podman acceptance covering the pinned backend, immutable fixture pre-pull, effective isolation, bounded HTTP readiness, explicit cleanup, and final container/network leak rejection on the reviewed source head.
- Consumer owner-path integration issue for `contextual-orchestrator` so Chat/Agent domain code consumes the published lease contract rather than directly invoking Podman/containerd.
- Architectural fitness validation for unique ADR identifiers, bounded-context dependency direction, and infrastructure-adapter placement.

### Changed

- Product responsibility is broadened from artifact-analysis-only to reusable hostile-workload isolation plus artifact-analysis evidence while preserving consumer business authority.
- Artifact-analysis implementation moved from generic crate-root files into `src/artifact_analysis/` to match the accepted DDD bounded context while preserving the public crate facade.
- Rootless Podman implementation moved from the Core `sandbox_execution` path into `src/infrastructure/`; the Core no longer depends on `application_service` error types.
- Pre-publication duplicate ADR identifiers were consolidated into the canonical ADR 0001–0006 sequence before protected-branch integration.
- Evidence identity now includes policy, source revision, and ordered analyzer identifiers.
- Static analyzer findings are restricted to file-format and static-capability evidence.
- JSON Schemas expose UTF-8 byte limits and reject control text consistently with Rust validation.
- Runtime analysis no longer requires a caller-supplied artifact name; optional source-context file names remain untrusted classification metadata.
- Required free-form source metadata was replaced by a closed typed context capped at 1,024 serialized UTF-8 bytes.
- Source timestamp and media-type validation trace to RFC 3339 and BCP 13 (RFC 6838 as updated by RFC 9694).
- Architecture now separates Core `sandbox_execution` from Supporting `artifact_analysis` and `application_service` and treats Podman/gVisor/containerd/Kubernetes mechanisms as infrastructure adapters.

### Security

- Duplicate or malformed analyzer identifiers fail engine construction.
- Universal Mach-O headers are structurally bounded so Java class magic is not treated as Mach-O by signature alone.
- Unsupported dynamic profiles and analyzer failures remain explicitly inconclusive.
- Path-like file names, URL/credential-shaped host references, unknown source-context fields, malformed media types, invalid UTC timestamps, and oversized context payloads fail closed before artifact analysis.
- Mutable/tag-only OCI image references fail closed.
- Standard application-service contract exposes no privileged mode, host namespace, device, runtime socket, arbitrary mount, arbitrary environment, credential, wildcard bind, or arbitrary Internet-egress capability.
- Podman backend must report rootless mode; service publication is validated as IPv4 loopback before a lease is returned.
- Podman host proxy environment inheritance is explicitly disabled so `HTTP_PROXY`/`HTTPS_PROXY`/`NO_PROXY` values cannot become ambient application inputs.
- Partial-launch/readiness failures attempt cleanup and cleanup uncertainty becomes `CleanupFailed` rather than being hidden.

### Not yet release evidence

- The real rootless-Podman lane passed on the reviewed source head, but final release readiness still requires the same acceptance to remain green on the unchanged release head together with verify, complete coverage, security, SAST, review, SBOM, provenance, and protected-merge evidence. Fake-process tests alone remain insufficient isolation proof.
- Durable orphan/lease reclamation, gVisor/containerd/Kubernetes adapters, controlled egress, secret broker, and stronger dynamic-detonation profiles remain follow-on work.
