# Product and Technical Gap Baseline

Last reviewed against the active implementation branch on 2026-09-01. This file distinguishes code already present on the active PR from work that still requires implementation and exact-head verification. Protected `develop` remains the shipped authority until merge.

## Product responsibility

Quarantine Sandbox Runtime is evolving from an artifact-analysis-only leaf into a reusable **sandbox execution and evidence runtime** with two consumer verticals:

1. `artifact_analysis`: hostile artifact ingestion, analysis execution, evidence, and completeness for Wardnet and other security consumers;
2. `application_service`: short-lived isolated application services for Chat/Agent control planes such as `contextual-orchestrator`.

The Core bounded context is `sandbox_execution`. Wardnet retains maliciousness verdict/incident/quarantine authority. Chat/Agent consumers retain conversation, task, tool, authorization, application-selection, secret, and user-action authority.

## DDD and repository structure

| Gap | Current evidence | Status | Required action |
| --- | --- | --- | --- |
| Artifact analysis was flattened under root `contracts.rs`, `ingestion.rs`, and `runtime.rs`. | Active PR moves them under `src/artifact_analysis/` while preserving public crate exports. | Active PR | Keep architectural fitness checks so future domain logic cannot return to generic root modules. |
| Sandbox lifecycle and application intent were previously conflated with future malware detonation. | Active PR separates `sandbox_execution` from `application_service` and documents the consumer Context Map. | Active PR | Add explicit backend port trait once a second implementation is introduced; avoid premature generic abstraction before then. |
| Repository name `quarantine-sandbox-runtime` is security-biased for the broader responsibility. | No organization collision was found for an isolation-runtime name, but the connected GitHub action set does not expose repository rename. | Known gap | Re-evaluate a repository rename such as `isolation-runtime` through an authorized repository-settings path before GA; preserve redirects/consumer migration if renamed. |
| Existing product-authority documentation is split across PR #3 and PR #1. | PR #3 contains unique authority/consumer ADR content but reflects the earlier artifact-only scope. | Convergence required | Preserve applicable authority decisions in #1 under the broader Context Map, then close #3 only after exact semantic comparison proves no unique current-scope work is lost. |

## Application-service isolation

| Capability | Current evidence | Status | Required action |
| --- | --- | --- | --- |
| Immutable application identity | `ApplicationServiceRequest` accepts only lower-case `@sha256:<64 hex>` image references; Podman uses `--pull=never`. | Active PR | Add registry/import admission as a separate trusted control-plane operation if required; never pull implicitly during task launch. |
| Rootless backend | Adapter probes Podman `Host.Security.Rootless` and fails closed unless it returns `true`. | Active PR | Add real rootless Podman E2E in CI/operational acceptance. |
| Read-only/writable surface bounds | `--read-only`, `--read-only-tmpfs=false`, and one bounded `/tmp` tmpfs with `noexec,nosuid,nodev`. | Active PR | Add explicit read-only input mounts through a typed mount contract; no arbitrary host path request. |
| Privilege/namespace isolation | `--cap-drop=all`, `no-new-privileges`, `--userns=auto`, private PID/IPC/UTS/cgroup namespaces, numeric non-root UID/GID. | Active PR | Verify the effective runtime state in real-container E2E instead of relying only on argv construction. |
| Network isolation | Per-sandbox `--internal --disable-dns` bridge and loopback-only random host port publication. | Active PR | Add packet-level/real-container proof of no external route. Controlled egress must be a different profile. |
| Credential isolation | No consumer/provider credentials, environment map, credential mounts, runtime sockets, or host devices exist in the P0 request contract. | Active PR | Add explicit secret-broker design only if a buyer workflow cannot operate without a task-scoped capability; default remains credential-free. |
| Resource limits | Memory, CPU, PID, lease, tmpfs, readiness timeout/polling, and shutdown grace are policy-bounded. Podman receives memory/CPU/PID/timeout controls. | Active PR | Test rootless cgroup behavior on representative Linux hosts and fail closed when a requested limit cannot be enforced. |
| Process-boundary lifecycle | Active PR invokes Podman directly without a shell, checks rootless mode, creates network/container, starts, resolves loopback port, gates readiness, and removes resources. Fake-Podman integration tests exercise the process boundary. | Active PR | Add real Podman E2E; add durable crash-recovery reaper for stale container/network resources. |
| Runtime attestation | Lease records schema, request, immutable image, backend, sandbox/network, policy, endpoint, timestamps, and isolation facts. Cleanup receipt records removal. | Active PR | Bind attestation to build/source identity and later sign durable receipts. |
| gVisor/containerd | OCI/gVisor are architecture targets only. | Missing | Implement separate backend adapter after Podman contract stabilizes; verify compatibility and isolation deltas. |
| Kubernetes | No RuntimeClass/Job/Service adapter exists. | Missing | Add managed deployment profile without exposing backend implementation in consumer domain models. |

## Artifact analysis

| Capability | Current evidence | Status | Required action |
| --- | --- | --- | --- |
| Static foundation | SHA-256 identity, bounded ingestion, format detection, analyzer interface, deterministic evidence and failure attribution. | Active PR | Preserve under `artifact_analysis`; complete exact-head coverage/review. |
| YARA-X / capa / Ghidra / LIEF | No production adapters yet. | Missing | Add one analyzer per bounded increment with version/digest provenance and hostile fixtures. |
| Linux detonation | No dynamic worker integrated. | Missing | Consume `sandbox_execution` or stronger gVisor/microVM profile; never execute hostile bytes in the control process. |
| Windows detonation | No Windows VM worker. | Missing | Separate Windows execution pool; preserve the same evidence contract. |
| Network sinkhole/telemetry | Not implemented. | Missing | Add explicit isolated telemetry profile with no production credentials and bounded capture. |
| Durable evidence/signing | In-memory evidence only. | Missing | Add durable queue/object store/signing only after persistence owner and retention boundaries are accepted. |

## Consumer integrations

| Consumer | Authority retained by consumer | Runtime integration state |
| --- | --- | --- |
| Wardnet | verdict, incident, quarantine, block/allow/review, notification, retention | Existing Wardnet product-gap direction; versioned runtime ACL still requires implementation after runtime publication. |
| contextual-orchestrator | chat, agent/task/tool policy, authorization, immutable application selection, secrets, user-visible action | Owner-path issue #991 created. It requires an ACL over a published runtime artifact, lease cleanup on every task terminal state, and no direct Podman/containerd calls from Agent domain code. |

## Verification and release gaps

- Exact-current-head Rust formatting, tests, Clippy, rustdoc, statement/function/region/branch coverage, SAST, dependency/security, SBOM/provenance, and review evidence must all be regenerated after this architecture change.
- The execution environment used for this session does not provide local Rust tooling; GitHub-hosted exact-head jobs are therefore authoritative and pending/queued states are non-passing.
- Real Podman isolation has not yet been proven by CI. Fake-Podman tests prove command/lifecycle integration only.
- The existing dependency-review evidence path has previously returned HTTP 403; it must be revalidated on the final exact head rather than bypassed.
- PR #1 remains Draft until current-head implementation, coverage, real-container evidence appropriate to the claims, security review, documentation convergence, and repository policy all pass.
- A release/version bump is premature until one integrated protected head satisfies all required gates and produces reproducible package/SBOM/provenance evidence.

## Next bounded slices

1. Finish exact-head application-service contracts, schemas, DDD docs, and 100% coverage.
2. Add a real rootless Podman E2E job that validates effective isolation rather than argv alone.
3. Add crash/restart lease reclamation and orphan cleanup evidence.
4. Publish an immutable runtime artifact and generated/typed consumer contract.
5. Integrate `contextual-orchestrator` through issue #991 and Wardnet through its own ACL, without moving consumer policy into this repository.
6. Add gVisor/containerd and Kubernetes RuntimeClass adapters only after the P0 contract is stable.
7. Resume artifact-analysis adapters and dynamic detonation on top of the shared sandbox execution Core.
