# Quarantine Sandbox Runtime Development Context

Read `AGENTS.md`, `docs/ARCHITECTURE.md`, `docs/PRD.md`, `docs/TRD.md`, accepted ADRs, and `docs/product-technical-gap-baseline.md` before changing behavior.

## Stable responsibility

This repository is a reusable Rust isolation/evidence runtime. `sandbox_execution` is the Core bounded context. `artifact_analysis` and `application_service` are Supporting contexts.

- Wardnet owns maliciousness verdicts, incidents, quarantine/blocking, notification, and retention.
- Chat/Agent control planes such as `contextual-orchestrator` own conversation/task/tool policy, application authorization/selection, secrets, and user-visible actions.
- This runtime owns sandbox lifecycle, isolation-policy enforcement, resource bounds, readiness, cleanup, attestation, and artifact-analysis evidence.

Do not move those foreign authorities into this repository and do not make consumers directly own Podman/gVisor/containerd internals.

## Security defaults

The P0 application-service profile is rootless, digest-pinned, no-pull, read-only-root, bounded-tmpfs, capability-free, no-new-privileges, isolated-namespace, non-root, resource-bounded, internal-network, loopback-only, credential-free, and cleanup-required.

Do not add ambient credentials, arbitrary env/maps, public binds, devices, runtime sockets, host namespaces, broad mounts, or Internet egress to make an application easier to run. Add a new reviewed profile/ADR if a genuine product requirement needs a new authority.

## Engineering

- Rust production implementation; unsafe forbidden unless a superseding ADR proves necessity.
- Test behavior first.
- 100% owned production statement/function/region/branch coverage where tooling exposes it.
- Exact-head CI/security/review evidence only.
- Fake backend tests are not real isolation proof; real Podman/gVisor E2E is required for those claims.
- Fix deprecations/toolchain failures at the cause.
- Keep DDD paths aligned; artifact analysis belongs under `src/artifact_analysis/` and backend adapters must not become consumer/domain entities.
- Preserve public contract compatibility explicitly; do not silently coerce unsupported input.
- Keep docs and JSON Schemas code-current.
