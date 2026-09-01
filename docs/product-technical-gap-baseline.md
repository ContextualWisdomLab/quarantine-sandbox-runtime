# Product and Technical Gap Baseline

Last reviewed against active PR #1 head `c1a88edba21aa4afc6a2c3b8b4555bc8fbb2dffc` on 2026-09-01. This file distinguishes code already present on the active PR from work that still requires implementation and exact-head verification. Protected `develop` remains the shipped authority until merge. Any later branch movement makes the exact-head observations below stale until revalidated.

## Product responsibility

Quarantine Sandbox Runtime is evolving from an artifact-analysis-only leaf into a reusable **sandbox execution and evidence runtime** with two consumer verticals:

1. `artifact_analysis`: hostile artifact ingestion, analysis execution, evidence, and completeness for Wardnet and other security consumers;
2. `application_service`: short-lived isolated application services for Chat/Agent control planes such as `contextual-orchestrator`.

The Core bounded context is `sandbox_execution`. `artifact_analysis` and `application_service` are Supporting bounded contexts. Wardnet retains maliciousness verdict/incident/quarantine authority. Chat/Agent consumers retain conversation, task, tool, authorization, application-selection, secret, and user-action authority. Podman/gVisor/containerd/Kubernetes/VM implementations are infrastructure adapters rather than domain entities.

## DDD and repository structure

| Gap | Current evidence | Status | Required action |
| --- | --- | --- | --- |
| Artifact analysis was flattened under root `contracts.rs`, `ingestion.rs`, and `runtime.rs`. | Active PR keeps the implementation under `src/artifact_analysis/` while preserving public crate exports. Repository policy forbids the obsolete root paths. | Corrected on active PR | Preserve the architectural fitness gate and exact-head tests. |
| Podman infrastructure was located under Core `src/sandbox_execution/` and depended on Supporting `ApplicationService*` types. | Head `c1a88ed…` moves Podman to `src/infrastructure/podman.rs`, gives Core validation `SandboxExecutionError`, translates it to `ApplicationServiceError` at the crate composition boundary, and adds dependency/path fitness checks. | Source corrected; test alignment pending | Update direct `IsolationPolicy::validate()` tests to expect `SandboxExecutionError`; request-level tests keep `ApplicationServiceError`; regenerate exact-head evidence. Tracked by issue #4. |
| Pre-publication ADR files reused identifiers `0001`–`0004`, while `docs/adr/README.md` indexed a different canonical line. | Head `c1a88ed…` removes the duplicate legacy files and adds ADR-number uniqueness enforcement; canonical ADRs are `0001`–`0006`. | Corrected on active PR | Verify no unique decision/research evidence was lost and keep the uniqueness gate green. |
| Product-authority documentation was split across PR #3 and PR #1. | PR #3 was semantically compared against the broader current product line and closed as superseded; its applicable authority/consumer-contract content is preserved or strengthened in #1. | Converged | Keep one canonical documentation line in #1/protected `develop`. |
| Repository name `quarantine-sandbox-runtime` is security-biased for the broader responsibility. | Runtime responsibility now includes isolated application services in addition to artifact analysis. Repository-settings rename support is outside this writer path. | Known gap | Re-evaluate a rename such as `isolation-runtime` before GA through an authorized repository-settings path; preserve redirects and consumer migration if renamed. |

## Application-service isolation

| Capability | Current evidence | Status | Required action |
| --- | --- | --- | --- |
| Immutable application identity | `ApplicationServiceRequest` accepts only lower-case `@sha256:<64 hex>` image references; Podman uses `--pull=never`. | Active PR | Add registry/import admission as a separate trusted control-plane operation if required; never pull implicitly during task launch. |
| Rootless backend | Adapter probes Podman `Host.Security.Rootless` and fails closed unless it returns `true`. | Active PR | Add real rootless Podman E2E in CI/operational acceptance. |
| Read-only/writable surface bounds | `--read-only`, `--read-only-tmpfs=false`, bounded `/tmp` with `noexec,nosuid,nodev`, `--image-volume=ignore`, `--no-hosts`, `--no-hostname`, `--systemd=false`, `--sdnotify=ignore`, and `--http-proxy=false`. | Active PR | Add explicit read-only input mounts through a typed mount contract only when required; no arbitrary host path request. |
| Privilege/namespace isolation | `--cap-drop=all`, `no-new-privileges`, `--userns=auto`, private PID/IPC/UTS/cgroup namespaces, numeric non-root UID/GID. | Active PR | Verify the effective runtime state in real-container E2E instead of relying only on argv construction. |
| Network isolation | Per-sandbox `--internal --disable-dns` bridge and loopback-only random host port publication. | Active PR | Add packet-level/real-container proof of no external route. Controlled egress must be a separate reviewed profile. |
| Credential isolation | No consumer/provider credentials, environment map, credential mounts, runtime sockets, or host devices exist in the P0 request contract; proxy environment inheritance is disabled. | Active PR | Add an explicit secret-broker design only if a buyer workflow cannot operate without a task-scoped capability; default remains credential-free. |
| Resource limits | Memory, CPU, PID, lease, tmpfs, readiness timeout/polling, and shutdown grace are policy-bounded. Podman receives memory/CPU/PID/runtime controls. | Active PR | Test rootless cgroup behavior on representative Linux hosts and fail closed when a requested limit cannot be enforced. |
| Process-boundary lifecycle | Adapter invokes Podman directly without a shell, checks rootless mode, creates network/container, starts, resolves loopback port, gates readiness, and removes resources. Fake-Podman integration tests exercise lifecycle and failure cleanup. | Active PR | Add real Podman E2E; add durable crash-recovery reaper for stale container/network resources. |
| Runtime attestation | Lease records schema, request, immutable image, backend, sandbox/network, policy, endpoint, timestamps, and isolation facts. Cleanup receipt records removal. | Active PR | Bind attestation to build/source identity and later sign durable receipts. |
| gVisor/containerd | Architecture targets only. | Missing | Implement a separate backend adapter after Podman contract stabilizes; verify compatibility and isolation deltas. |
| Kubernetes | No RuntimeClass/Job/Service adapter exists. | Missing | Add managed deployment profile without exposing backend implementation in consumer domain models. |

## Artifact analysis

| Capability | Current evidence | Status | Required action |
| --- | --- | --- | --- |
| Static foundation | SHA-256 identity, bounded ingestion, format detection, analyzer interface, deterministic evidence and failure attribution. | Active PR | Preserve under `artifact_analysis`; complete exact-head coverage/review. |
| YARA-X / capa / Ghidra / LIEF | No production adapters yet. | Missing | Add one analyzer per bounded increment with version/digest provenance and hostile fixtures. |
| Linux detonation | No dynamic worker integrated. | Missing | Consume `sandbox_execution` or a stronger gVisor/microVM profile; never execute hostile bytes in the control process. |
| Windows detonation | No Windows VM worker. | Missing | Separate Windows execution pool; preserve the same evidence contract. |
| Network sinkhole/telemetry | Not implemented. | Missing | Add explicit isolated telemetry profile with no production credentials and bounded capture. |
| Durable evidence/signing | In-memory evidence only. | Missing | Add durable queue/object store/signing only after persistence owner and retention boundaries are accepted. |

## Consumer integrations

| Consumer | Authority retained by consumer | Runtime integration state |
| --- | --- | --- |
| Wardnet | verdict, incident, quarantine, block/allow/review, notification, retention | Runtime contract is consumer-neutral; Wardnet ACL still requires owner-path implementation after runtime publication. |
| contextual-orchestrator | chat, agent/task/tool policy, authorization, immutable application selection, secrets, user-visible action | Owner-path issue #991 exists. It requires an ACL over a published immutable runtime artifact, lease cleanup on every task terminal state, and no direct Podman/containerd calls from Agent domain code. |

## Verification and release gaps

- Current exact head for this review is `c1a88edba21aa4afc6a2c3b8b4555bc8fbb2dffc`; revalidate if the branch moves.
- One direct-Core validation test still expects the former Supporting-context error type after the DDD extraction. Correct it to `SandboxExecutionError` while retaining request-level `ApplicationServiceError` semantics; issue #4 tracks this exact repair.
- Exact-head CI run `33472977145` is pending with verify/coverage/branch-coverage jobs queued. Security Scan `33472977118` and SAST `33472977125` are queued. Pending/queued is non-passing.
- Organization ruleset `CWL Central required workflows` is active on the default branch and requires one approving review, resolved review threads, and the central required workflows; bypass capability is not merge evidence and must not be used by this loop.
- Real Podman isolation has not yet been proven by CI. Fake-Podman tests prove command/lifecycle integration only.
- The dependency-review evidence path has previously returned HTTP 403; it must be revalidated on the final exact head rather than bypassed.
- PR #1 remains Draft until current-head implementation, coverage, real-container evidence appropriate to the claims, security review, documentation convergence, and repository policy all pass.
- A release/version bump is premature until one integrated protected head satisfies all required gates and produces reproducible package/SBOM/provenance evidence.

## Next bounded slices

1. Align the direct Core policy-validation test with `SandboxExecutionError`, then obtain exact-head full GREEN evidence for the DDD extraction.
2. Add a real rootless Podman E2E job that validates effective isolation rather than argv alone.
3. Add crash/restart lease reclamation and orphan cleanup evidence.
4. Publish an immutable runtime artifact and generated/typed consumer contract.
5. Integrate `contextual-orchestrator` through issue #991 and Wardnet through its own ACL, without moving consumer policy into this repository.
6. Add gVisor/containerd and Kubernetes RuntimeClass adapters only after the P0 contract is stable.
7. Resume artifact-analysis adapters and dynamic detonation on top of the shared sandbox execution Core.
