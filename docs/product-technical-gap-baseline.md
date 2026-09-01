# Product and Technical Gap Baseline

Last reviewed against active PR #1 source head `984d3a6ea2c267c8dd647fabf698465eb4ac0980` on 2026-09-01. This documentation follow-up may advance the PR commit without changing that reviewed source tree. This file distinguishes code already present on the active PR from work that still requires implementation and exact-head verification. Protected `develop` remains the shipped authority until merge. Any later source change makes the observations below stale until revalidated.

## Product responsibility

Quarantine Sandbox Runtime is evolving from an artifact-analysis-only leaf into a reusable **sandbox execution and evidence runtime** with two consumer verticals:

1. `artifact_analysis`: hostile artifact ingestion, analysis execution, evidence, and completeness for Wardnet and other security consumers;
2. `application_service`: short-lived isolated application services for Chat/Agent control planes such as `contextual-orchestrator`.

The Core bounded context is `sandbox_execution`. `artifact_analysis` and `application_service` are Supporting bounded contexts. Wardnet retains maliciousness verdict/incident/quarantine authority. Chat/Agent consumers retain conversation, task, tool, authorization, application-selection, secret, and user-action authority. Podman/gVisor/containerd/Kubernetes/VM implementations are infrastructure adapters rather than domain entities.

## DDD and repository structure

| Gap | Current evidence | Status | Required action |
| --- | --- | --- | --- |
| Artifact analysis was flattened under root `contracts.rs`, `ingestion.rs`, and `runtime.rs`. | Active PR keeps the implementation under `src/artifact_analysis/` while preserving public crate exports. Repository policy forbids the obsolete root paths. | Corrected on active PR | Preserve the architectural fitness gate and exact-head tests. |
| Podman infrastructure was located under Core `src/sandbox_execution/` and depended on Supporting `ApplicationService*` types. | Source head `984d3a6…` keeps Podman in `src/infrastructure/podman.rs`, uses `SandboxExecutionError` for Core validation, translates it to `ApplicationServiceError` at the crate composition boundary, fixes the moved artifact-ingestion import, and retains dependency/path fitness checks. | Corrected and locally verified on active PR | Keep the DDD fitness tests and both error boundaries green. Issue #4's reported mismatch is corrected; protected-branch and hosted exact-head evidence remain required. |
| Pre-publication ADR files reused identifiers `0001`–`0004`, while `docs/adr/README.md` indexed a different canonical line. | Reviewed source head `984d3a6…` contains only canonical ADRs `0001`–`0006` and enforces identifier uniqueness. | Corrected and locally verified on active PR | Keep the uniqueness gate green. |
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
| Process-boundary lifecycle | Adapter invokes Podman directly without a shell, checks rootless mode, creates network/container, starts, resolves loopback port, gates readiness, and removes resources. Fake-Podman tests exercise failure paths. Exact source head `984d3a6…` also passed hosted real rootless-Podman E2E, including effective isolation, HTTP readiness, cleanup, and final no-leak checks. | Active PR, real P0 lane verified on reviewed source head | Preserve the real lane; add a durable crash-recovery reaper for stale container/network resources. |
| Runtime attestation | Lease records schema, request, immutable image, backend, sandbox/network, policy ID plus canonical full-policy SHA-256, endpoint, timestamps, and isolation facts. The same policy digest is attached to the Podman resource. Cleanup receipt records removal. | Active PR | Bind attestation to verified build/artifact identity and later sign durable receipts. |
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

The runtime still exposes only an embeddable Rust library. There is no supported authenticated
process boundary or generated Python consumer yet, and `127.0.0.1` lease reachability is defined
only for a co-located host-network consumer. Before issue #991 can integrate, this repository must
choose and publish one versioned transport or binding, bind leases to authenticated caller identity
and idempotency scope, define the supported network topology, and provide stable bounded wire errors
and semantic lease/cleanup validation. A sibling checkout, ad-hoc subprocess protocol, or direct
Podman call is not an acceptable substitute.

## Verification and release gaps

- Reviewed source head `984d3a6ea2c267c8dd647fabf698465eb4ac0980` passes `cargo test --locked --workspace --all-targets` locally (the dedicated real-Podman acceptance remains ignored off its Linux lane), `cargo clippy --locked --workspace --all-targets -- -D warnings`, warning-denied rustdoc, repository policy validation, rustfmt, and diff-check with the pinned Rust 1.97.1 toolchain.
- The DDD extraction's two build breaks are corrected: artifact runtime uses its sibling ingestion module, and application-service request validation translates Core resource errors through the public boundary. The direct-Core test expects `SandboxExecutionError`; request-level tests retain `ApplicationServiceError`.
- Hosted job `99750285437` passed the real rootless-Podman acceptance at exact source head `984d3a6…`: pinned Podman 5.8.4/rootless verification, immutable image pre-pull, effective isolation/HTTP service checks, explicit cleanup, and the final container/network leak rejection all succeeded. The prior run exposed a connection-reset panic before cleanup; `984d3a6…` makes the live HTTP probe bounded and guarantees termination after every post-lease assertion result.
- Verify, coverage, branch-coverage, security, SAST, OpenCode, and Noema checks remain queued or pending. Pending/queued is non-passing, and the documentation-only tip still requires its own exact-head required workflows.
- Organization ruleset `CWL Central required workflows` is active on the default branch and requires one approving review, resolved review threads, and the central required workflows; bypass capability is not merge evidence and must not be used by this loop.
- Real Podman isolation is proven for the pinned hosted runner and policy above; broader host profiles and a published release remain unproven.
- The dependency-review evidence path has previously returned HTTP 403; it must be revalidated on the final exact head rather than bypassed.
- PR #1 remains Draft until current-head implementation, coverage, real-container evidence appropriate to the claims, security review, documentation convergence, and repository policy all pass.
- A release/version bump is premature until one integrated protected head satisfies all required gates and produces reproducible package/SBOM/provenance evidence.

## Next bounded slices

1. Obtain terminal exact-head hosted verify, complete coverage, security, SAST, and independent-review evidence; keep the now-passing real rootless-Podman lane green.
2. Choose and implement the authenticated, versioned consumer transport or supported language binding, including caller-scoped idempotency, stable wire errors, strict response validation, and an explicit co-location/network-topology contract.
3. Add crash/restart lease reclamation and orphan cleanup evidence.
4. Publish an immutable runtime artifact, generated/typed consumer contract, SBOM, and provenance bound into lease attestation.
5. Integrate `contextual-orchestrator` through issue #991 and Wardnet through its own ACL, without moving consumer policy into this repository.
6. Add gVisor/containerd and Kubernetes RuntimeClass adapters only after the P0 contract is stable.
7. Resume artifact-analysis adapters and dynamic detonation on top of the shared sandbox execution Core.
