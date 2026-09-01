# Product and Technical Gap Baseline

Last reviewed against stacked PR #6 source head `03c2dc67e3ad04fd407f9d203e912e8f6cd1c14e` and its parent PR #1 head `4b0bb12e9bd10e5c9bf65ca970ab3b8332c5e972` on 2026-09-01. Documentation-only commits may advance PR #6 without changing that reviewed source tree. This file distinguishes active-PR evidence from protected `develop`, which remains shipped authority until merge. Any later source change makes these observations stale until revalidated.

## Product responsibility

Quarantine Sandbox Runtime is evolving from an artifact-analysis-only leaf into a reusable **sandbox execution and evidence runtime** with two consumer verticals:

1. `artifact_analysis`: hostile artifact ingestion, analysis execution, evidence, and completeness for Wardnet and other security consumers;
2. `application_service`: short-lived isolated application services for Chat/Agent control planes such as `contextual-orchestrator`.

The Core bounded context is `sandbox_execution`. `artifact_analysis` and `application_service` are Supporting bounded contexts. Wardnet retains maliciousness verdict/incident/quarantine authority. Chat/Agent consumers retain conversation, task, tool, authorization, application-selection, secret, and user-action authority. Podman/gVisor/containerd/Kubernetes/VM implementations are infrastructure adapters rather than domain entities.

## DDD and repository structure

| Gap | Current evidence | Status | Required action |
| --- | --- | --- | --- |
| Artifact analysis was flattened under root `contracts.rs`, `ingestion.rs`, and `runtime.rs`. | Parent PR #1 keeps the implementation under `src/artifact_analysis/` while preserving public crate exports. Repository policy forbids the obsolete root paths. | Corrected on parent PR | Preserve the architectural fitness gate and exact-head tests. |
| Podman infrastructure was located under Core `src/sandbox_execution/` and depended on Supporting `ApplicationService*` types. | Parent source head keeps Podman in `src/infrastructure/podman.rs`, uses `SandboxExecutionError` for Core validation, and translates it at the crate composition boundary. | Corrected on parent PR | Keep the DDD fitness tests and both error boundaries green. Issue #4 remains open until protected evidence is complete. |
| Application-service lifecycle ownership initially had no backend-neutral coordination boundary. | PR #6 adds `ApplicationServiceBackend` in the Supporting `application_service` context and implements the Podman adapter in `src/infrastructure/application_service_backend.rs`; the coordinator does not depend on concrete Podman types. | Corrected on stacked PR #6; exact-head verification pending | Keep the port implementation in infrastructure and add future containerd/gVisor implementations without leaking backend DTOs into the domain. |
| Caller identity could have been added to the untrusted application payload. | PR #6 uses bounded `LeaseOwnerId` as authenticated command context, separate from `ApplicationServiceRequest`. | Corrected by design on stacked PR #6 | The future transport must derive this value from verified caller identity; never trust a payload-supplied owner string as authentication. |
| Pre-publication ADR files reused identifiers `0001`–`0004`, while `docs/adr/README.md` indexed a different canonical line. | Parent PR contains only canonical ADRs `0001`–`0006` and enforces identifier uniqueness. | Corrected on parent PR | Keep the uniqueness gate green. |
| Product-authority documentation was split across PR #3 and PR #1. | PR #3 was semantically compared against the broader current product line and closed as superseded; its applicable authority/consumer-contract content is preserved or strengthened in #1. | Converged | Keep one canonical documentation line in #1/protected `develop`. |
| Repository name `quarantine-sandbox-runtime` is security-biased for the broader responsibility. | Runtime responsibility now includes isolated application services in addition to artifact analysis. Repository-settings rename support is outside this writer path. | Known gap | Re-evaluate a rename such as `isolation-runtime` before GA through an authorized repository-settings path; preserve redirects and consumer migration if renamed. |

## Application-service isolation

| Capability | Current evidence | Status | Required action |
| --- | --- | --- | --- |
| Immutable application identity | `ApplicationServiceRequest` accepts only lower-case `@sha256:<64 hex>` image references; Podman uses `--pull=never`. | Parent PR | Add registry/import admission as a separate trusted control-plane operation if required; never pull implicitly during task launch. |
| Rootless backend | Adapter probes Podman `Host.Security.Rootless` and fails closed unless it returns `true`. | Parent PR | Preserve the real rootless Podman lane on every release head. |
| Read-only/writable surface bounds | `--read-only`, bounded `/tmp`, image volumes ignored, host proxy inheritance disabled, and no arbitrary host mount field in the request contract. | Parent PR | Add explicit read-only input mounts through a typed mount contract only when required; no arbitrary host path request. |
| Privilege/namespace isolation | All capabilities dropped, no-new-privileges, automatic user namespace, private process-related namespaces, numeric non-root UID/GID. | Parent PR | Preserve effective-runtime verification in the real-container lane. |
| Network isolation | Per-sandbox internal DNS-disabled bridge and loopback-only random host port publication. | Parent PR | Controlled egress must be a separate reviewed profile; preserve no-external-route proof in real-container acceptance. |
| Credential isolation | No consumer/provider credentials, environment map, credential mounts, runtime sockets, or host devices exist in the P0 request contract; proxy environment inheritance is disabled. | Parent PR | Add a secret-broker design only if a buyer workflow cannot operate without a task-scoped capability; default remains credential-free. |
| Resource limits | Memory, CPU, PID, lease, tmpfs, readiness timeout/polling, and shutdown grace are policy-bounded. Podman receives memory/CPU/PID/runtime controls. | Parent PR | Test rootless cgroup behavior on representative Linux hosts and fail closed when a requested limit cannot be enforced. |
| Process-boundary lifecycle | Adapter invokes Podman directly without a shell, creates and verifies the isolated service, returns a loopback endpoint, and removes runtime-owned resources. Parent source head passed hosted real rootless-Podman E2E. | Parent PR, real P0 lane verified on reviewed source head | Preserve the real lane; add durable crash-recovery reclamation for stale resources. |
| Caller-scoped lease ownership | PR #6 keys active state by authenticated-command `LeaseOwnerId` plus request ID; wrong-owner termination returns `UnknownLease` before backend cleanup. | Implemented on stacked PR #6; exact-head verification pending | Bind `LeaseOwnerId` to an authenticated versioned transport and add durable ownership state before multi-process deployment. |
| Idempotent launch replay | PR #6 returns the existing active lease for the same owner/request/effective-policy fingerprint, rejects changed request or policy as `IdempotencyConflict`, rejects concurrent duplicates as `LaunchInProgress`, and clears failed launch reservations for corrected retries. | Implemented on stacked PR #6; exact-head verification pending | Publish stable wire-level conflict/in-progress codes with the transport contract; add durable replay semantics before restart recovery claims. |
| Bounded expiry cleanup | PR #6 cleans at most 64 expired active leases per pass, retains failed cleanup for retry, and attributes outcomes to owner/request/lease without claiming restart recovery. | Implemented on stacked PR #6; exact-head verification pending | Add persistent lease journal plus orphan reconciliation before GA. |
| Runtime attestation | Lease records schema, request, immutable image, backend, sandbox/network, policy, endpoint, timestamps, and isolation facts. Cleanup receipt records removal. | Parent PR | Bind attestation to build/source identity, backend version, and effective policy digest; later sign durable receipts. |
| gVisor/containerd | Architecture targets only. | Missing | Implement a separate backend adapter after the Podman and lifecycle-port contracts stabilize; verify compatibility and isolation deltas. |
| Kubernetes | No RuntimeClass/Job/Service adapter exists. | Missing | Add managed deployment profile without exposing backend implementation in consumer domain models. |

## Artifact analysis

| Capability | Current evidence | Status | Required action |
| --- | --- | --- | --- |
| Static foundation | SHA-256 identity, bounded ingestion, format detection, analyzer interface, deterministic evidence and failure attribution. | Parent PR | Preserve under `artifact_analysis`; complete exact-head coverage/review. |
| YARA-X / capa / Ghidra / LIEF | No production adapters yet. | Missing | Add one analyzer per bounded increment with version/digest provenance and hostile fixtures. |
| Linux detonation | No dynamic worker integrated. | Missing | Consume `sandbox_execution` or a stronger gVisor/microVM profile; never execute hostile bytes in the control process. |
| Windows detonation | No Windows VM worker. | Missing | Separate Windows execution pool; preserve the same evidence contract. |
| Network sinkhole/telemetry | Not implemented. | Missing | Add explicit isolated telemetry profile with no production credentials and bounded capture. |
| Durable evidence/signing | In-memory evidence only. | Missing | Add durable queue/object store/signing only after persistence owner and retention boundaries are accepted. |

## Consumer integrations

| Consumer | Authority retained by consumer | Runtime integration state |
| --- | --- | --- |
| Wardnet | verdict, incident, quarantine, block/allow/review, notification, retention | Runtime contract is consumer-neutral; Wardnet ACL still requires owner-path implementation after runtime publication. |
| contextual-orchestrator | chat, agent/task/tool policy, authorization, immutable application selection, secrets, user-visible action | Owner-path issue #991 exists. It requires an ACL over a published immutable runtime artifact, caller identity mapped into runtime command context, lease cleanup on every task terminal state, and no direct Podman/containerd calls from Agent domain code. |

PR #6 closes the **process-local** caller ownership and idempotency portion of the consumer gap if its exact-head gates pass. The runtime still exposes only an embeddable Rust library: there is no supported authenticated process boundary or generated Python consumer, and `127.0.0.1` lease reachability is defined only for a co-located host-network consumer. Before issue #991 can integrate, this repository must choose and publish one versioned transport or binding, define the supported network topology, and provide stable bounded wire errors plus semantic lease/cleanup validation. A sibling checkout, ad-hoc subprocess protocol, or direct Podman call is not an acceptable substitute.

## Verification and release gaps

- Parent reviewed source head `984d3a6ea2c267c8dd647fabf698465eb4ac0980` passed its local pinned Rust suite and hosted real rootless-Podman acceptance; parent documentation has since advanced without changing that source behavior.
- Parent hosted job `99750285437` passed pinned Podman 5.8.4/rootless verification, immutable image pre-pull, effective isolation/HTTP service checks, explicit cleanup, and final container/network leak rejection.
- PR #6 source head `03c2dc67e3ad04fd407f9d203e912e8f6cd1c14e` adds caller ownership/idempotency and an effective-policy replay regression, but this execution environment has not produced a local-green claim. Its exact-head CI/security/SAST/review evidence is authoritative and currently pending/queued.
- Organization ruleset `CWL Central required workflows` requires qualifying approval and central required workflows; bypass capability is not merge evidence.
- Real Podman isolation is proven for the reviewed parent hosted runner/profile only; broader host profiles and a published release remain unproven.
- The dependency-review evidence path has previously returned HTTP 403; it must be revalidated on the final exact head rather than bypassed.
- PR #1 and stacked PR #6 remain Draft until their current-head implementation, coverage, security, review, documentation, and applicable real-container evidence satisfy repository policy.
- A release/version bump is premature until one integrated protected head satisfies all required gates and produces reproducible package/SBOM/provenance evidence.

## Next bounded slices

1. Obtain terminal exact-head verify, complete coverage, security, SAST, and independent-review evidence for parent PR #1 while keeping the real rootless-Podman lane green.
2. Drive stacked PR #6 through exact-head compile/test/coverage/security/review, repair any valid findings, and merge it only after #1 is protected and #6 is revalidated on the resulting base.
3. Choose and implement the authenticated, versioned consumer transport or supported language binding with stable wire errors, strict response validation, and an explicit co-location/network-topology contract.
4. Add durable crash/restart lease reclamation, orphan cleanup, and admission/resource reservation evidence.
5. Publish an immutable runtime artifact, generated/typed consumer contract, SBOM, and provenance bound into lease attestation.
6. Integrate `contextual-orchestrator` through issue #991 and Wardnet through its own ACL, without moving consumer policy into this repository.
7. Add gVisor/containerd and Kubernetes RuntimeClass adapters only after the P0 and lifecycle-port contracts are stable.
8. Resume artifact-analysis adapters and dynamic detonation on top of the shared sandbox execution Core.
