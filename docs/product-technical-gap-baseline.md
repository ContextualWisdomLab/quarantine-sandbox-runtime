# Product and Technical Gap Baseline

Last reviewed on 2026-09-01 against parent PR #1 exact head `e4222208f0d1a6d28f570c45dc7385fd75b16a4b` and stacked PR #6 pre-reconciliation head `806a0463224f99b8587e2973a8eec5d52caa7129`. The stack reconciliation combines the current parent security/wire contract with #6 caller-scoped lease ownership. Protected `develop` remains shipped authority until protected integration. Hosted evidence must be regenerated for the reconciled exact head; predecessor results do not transfer.

## Product responsibility

Quarantine Sandbox Runtime is a reusable hostile-workload isolation and evidence runtime with two consumer verticals:

1. `artifact_analysis` owns hostile artifact identity, bounded ingestion, analysis execution evidence, provenance, and analysis completeness;
2. `application_service` owns short-lived isolated application-service leases for authorized Chat/Agent consumers.

Core `sandbox_execution` owns isolation policy, resource bounds, runtime lease metadata, lifecycle primitives, and backend-neutral execution invariants. Podman/gVisor/containerd/Kubernetes/VM mechanisms are infrastructure adapters. Wardnet retains maliciousness verdict, incident, quarantine, blocking, notification, and retention authority. `contextual-orchestrator` retains chat, Agent/task/tool policy, authorization, application selection, secrets, and user-visible actions.

## DDD and repository structure

| Gap | Current evidence | Status | Required action |
| --- | --- | --- | --- |
| Artifact analysis was flattened under generic crate-root files. | PR #1 places implementation under `src/artifact_analysis/` and keeps only the public crate facade at root. | Corrected on parent PR | Keep architecture fitness checks green. |
| Podman implementation lived under Core and depended on Supporting application-service types. | PR #1 keeps Podman under `src/infrastructure/podman.rs`; Core owns `SandboxExecutionError`, while the application boundary translates errors. | Corrected on parent PR | Do not move container adapters back into Core. |
| Application-service lifecycle had no backend-neutral port or caller ownership. | PR #6 adds `ApplicationServiceBackend`, `ApplicationServiceCoordinator`, and `LeaseOwnerId`; Podman implements the port in infrastructure. | Implemented on stacked PR; reconciled exact-head proof required | Preserve port direction for future gVisor/containerd adapters. |
| Product-authority documentation was split across an older documentation PR. | PR #3 was closed as superseded after preserving applicable material in PR #1. | Converged | Keep one canonical documentation line. |
| Repository name is narrower than its present responsibility. | Product now serves application-service isolation as well as quarantine/artifact analysis. | Known product-name gap | Reassess rename before GA through repository-settings authority; preserve redirects and consumer migration if changed. |

## Application-service isolation

| Capability | Current evidence | Status | Required action |
| --- | --- | --- | --- |
| Immutable workload identity | `ApplicationServiceRequest` accepts digest-pinned lower-case SHA-256 OCI references; Podman uses no-pull launch semantics. | Parent PR | If image admission is added, make it a separate trusted operation and retain digest identity. |
| Rootless execution | Podman adapter verifies rootless backend state and fails closed otherwise. | Parent PR | Keep real rootless backend acceptance on every release head. |
| Filesystem isolation | Read-only root filesystem, bounded noexec/nosuid/nodev tmpfs, image volumes ignored, no arbitrary host mount request. | Parent PR | Add only typed reviewed read-only input mounts when a buyer flow requires them. |
| Privilege isolation | Capabilities dropped, no-new-privileges, isolated user/process-related namespaces, numeric non-root identity. | Parent PR | Preserve effective-runtime verification rather than relying only on argv construction. |
| Network isolation | Per-sandbox internal DNS-disabled network and loopback-only host publication. | Parent PR | Controlled egress must be a separate profile and must not silently enable Internet access. |
| Credential isolation | No consumer/provider credentials, arbitrary environment, runtime sockets, host devices, or ambient proxy variables enter the P0 workload contract. | Parent PR | Add a task-scoped secret broker only after an accepted ADR and explicit consumer authorization. |
| Resource limits | Memory, CPU, PID, lease duration, tmpfs, readiness timeout/polling, and shutdown grace are policy bounded. | Parent PR | Verify enforcement across supported host/cgroup profiles and fail closed where enforcement is unavailable. |
| Readiness and cleanup | Real rootless-Podman lane has proven bounded readiness, explicit termination, and final no-container/no-network leak checks on parent exact head. | Parent exact-head Podman lane GREEN; full release gates pending | Preserve the real E2E lane on the reconciled and final release head. |
| Runtime attestation | Lease schema `1.1.0` records backend/sandbox/network/policy, canonical full-policy SHA-256, loopback endpoint, timestamps, and P0 isolation facts. | Parent PR | Bind verified build/artifact/backend version identity and later sign durable receipts. |
| Caller-scoped ownership | `LeaseOwnerId` is authenticated command context, not an application payload field; wrong-owner termination fails before backend cleanup. | PR #6 | Bind the owner to an authenticated versioned transport before remote/multi-process use. |
| Idempotent launch | Coordinator keys process-local state by owner + request; identical active replay returns the lease, changed request/effective policy conflicts, and concurrent duplicate launch fails closed. | PR #6 | Publish stable wire error codes and durable replay semantics before restart claims. |
| Expiry cleanup | Coordinator selects at most 64 expired leases per pass and retains failed cleanup for retry. | PR #6 | Add persistent lease journal/orphan reconciliation and admission/resource reservation. |
| gVisor/containerd/Kubernetes | No production adapter. | Missing | Add separate infrastructure adapters only after P0/lifecycle contract stabilizes. |

## Artifact analysis

| Capability | Current evidence | Status | Required action |
| --- | --- | --- | --- |
| Static foundation | Immutable SHA-256 identity, bounded source context, format classification, ordered static analyzer port, deterministic evidence/failure attribution. | Parent PR | Keep under `artifact_analysis` and complete final exact-head gates. |
| YARA-X / capa / Ghidra / LIEF | No production adapters. | Missing | Add one adapter per bounded TDD increment with tool/version/digest provenance and hostile fixtures. |
| Linux dynamic detonation | Not integrated. | Missing | Consume `sandbox_execution` or a stronger gVisor/microVM profile; never execute hostile bytes in the host control process. |
| Windows detonation | Not integrated. | Missing | Use a separate Windows isolation pool while preserving evidence contracts. |
| Controlled network telemetry | Not integrated. | Missing | Add an explicit sinkhole/egress profile with no production credentials and bounded capture. |
| Durable evidence/signing | Evidence is process-local. | Missing | Define persistence/retention owner, immutable object storage, signing, replay, and chain-of-custody evidence before GA. |

## Context Fabric and Enterprise Architecture integration

`ContextualWisdomLab/context-graph-contracts` is the contract-only Shared Kernel for canonical object/authority references, truth status/origin, bitemporal time, provenance, Context Assertion, CloudEvents, schema conformance, and admission. `ContextualWisdomLab/enterprise-architecture-core` owns authoritative architecture and transformation decisions. While the dedicated Context Fabric writer is active, this repository treats both as read-only dependencies.

Current integration rules:

- do not copy malware verdicts or artifact risk scores into authoritative architecture facts;
- service/runtime/backend technology identity, lifecycle, ownership, risk/remediation, and transformation evidence must flow only through a released compatible `context-graph-contracts` profile with conformance evidence;
- Context Assertion events must preserve the CloudEvent envelope identity/provenance rather than emit bare assertion data;
- schema/profile/admission drift or missing conformance is an integration defect;
- no cross-service application-table SQL;
- Context Fabric owner-path PRs/issues are inventoried read-only and receive exact evidence/acceptance criteria rather than quarantine-writer source changes.

At this review, `context-graph-contracts` live metadata still reports `develop` as its default branch. An active stacked contract PR is repairing Context Assertion message composition with the shared CloudEvent envelope. These facts are live-state observations, not assumptions about the intended stable branch or release contract. Quarantine integration remains blocked from claiming a released Context Assertion projection until the owner path publishes a compatible release and conformance evidence.

## Consumer integrations

| Consumer | Authority retained by consumer | Runtime integration state |
| --- | --- | --- |
| Wardnet | gateway/SOC policy, maliciousness verdict, incident, quarantine/block/review, notification, retention | Runtime contract is consumer-neutral; Wardnet must consume it through its own ACL/owner path after publication. |
| contextual-orchestrator | LLM/model routing, chat, Agent/task/tool policy, caller authorization, application selection, secrets, user-visible actions | Owner-path issue exists for a published immutable runtime integration. Direct Podman/containerd calls and sibling source copies are not acceptable. |

The runtime currently exposes a Rust library and loopback lease topology. It does not yet publish an authenticated network process boundary or generated Python consumer. A future transport must derive `LeaseOwnerId` from verified caller context, keep idempotency scope stable, return bounded stable wire errors, validate lease/cleanup semantics, and define co-location/network topology explicitly.

## Verification and release state

- Parent PR #1 exact head `e4222208f0d1a6d28f570c45dc7385fd75b16a4b` has a successful real rootless-Podman job `99758127137` from workflow run `33476960369`.
- Parent exact-head verify, coverage, branch-coverage and several central security/review workflows were still queued or pending at the latest read; queued/pending is non-passing.
- PR #1 has no formal approving review at the latest read and remains Draft.
- PR #6 pre-reconciliation head `806a0463224f99b8587e2973a8eec5d52caa7129` had a successful Podman E2E job but verify/coverage/branch-coverage remained queued. Its old parent base omitted the current policy-digest/lease-1.1 contract, so the stack must be reconciled and all exact-head evidence regenerated.
- Active organization rules require qualifying review, resolved threads, and central required workflows. Admin/bypass capability is not merge evidence.
- No release exists. Version/release claims remain premature until one integrated protected head passes CI, security, SAST, complete coverage/docstrings, real isolation E2E, SBOM/provenance, review, rollback/recovery and protected-merge gates together.

## Next bounded slices

1. Reconcile PR #6 non-destructively onto the current PR #1 exact head and regenerate all exact-head gates.
2. Drain parent PR #1 gate/review findings and merge only through protected policy; then revalidate the stacked lease-ownership slice against the protected parent.
3. Implement one authenticated versioned consumer transport/binding with stable wire errors and caller identity mapping.
4. Add durable restart/orphan reclamation, lease journal, admission/resource reservation, and crash-recovery evidence.
5. Publish immutable runtime artifacts with SBOM/provenance and bind verified build/backend identity into attestation.
6. Integrate Wardnet and contextual-orchestrator only through their owner paths and published contracts.
7. Add stronger gVisor/containerd/Kubernetes isolation adapters and then resume dynamic artifact-analysis profiles.
