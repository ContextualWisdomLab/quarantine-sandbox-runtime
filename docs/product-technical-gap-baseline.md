# Product and Technical Gap Baseline

Last reviewed against active PR #1 production/source head `e90f1789da2bf4a4d511ae4666433cf86c0237e5` and protected `develop@60a85c7633e03b425b67159ec6822c8178cf87ea` on 2026-09-04 KST. The immediately following ledger commit changes documentation only; any later production/source change invalidates these observations until revalidated. This file distinguishes active-PR implementation from protected-branch truth and never transfers predecessor check results.

## Product responsibility

Quarantine Sandbox Runtime is evolving from an artifact-analysis-only leaf into a reusable **sandbox execution and evidence runtime** with two consumer verticals:

1. `artifact_analysis`: hostile artifact ingestion, analysis execution, evidence, and completeness for Wardnet and other security consumers;
2. `application_service`: short-lived isolated application services for Chat/Agent control planes such as `contextual-orchestrator`.

The Core bounded context is `sandbox_execution`. `artifact_analysis` and `application_service` are Supporting bounded contexts. Wardnet retains maliciousness verdict/incident/quarantine authority. Chat/Agent consumers retain conversation, task, tool, authorization, application-selection, secret, and user-action authority. Podman/gVisor/containerd/Kubernetes/VM implementations are infrastructure adapters rather than domain entities.

## DDD and repository structure

| Gap | Current evidence | Status | Required action |
| --- | --- | --- | --- |
| Artifact analysis was flattened under root `contracts.rs`, `ingestion.rs`, and `runtime.rs`. | Active PR keeps the implementation under `src/artifact_analysis/` while preserving public crate exports. Repository policy forbids the obsolete root paths. | Corrected on active PR | Preserve the architectural fitness gate and exact-head tests. |
| Podman infrastructure was located under Core `src/sandbox_execution/` and depended on Supporting `ApplicationService*` types. | Active PR keeps Podman in `src/infrastructure/podman.rs`, uses `SandboxExecutionError` for Core validation, translates it to `ApplicationServiceError` at the crate composition boundary, fixes the moved artifact-ingestion import, and retains dependency/path fitness checks. | Corrected on active PR | Keep the DDD fitness tests and both error boundaries green; protected-branch and current exact-head hosted evidence remain required. |
| Pre-publication ADR files reused identifiers `0001`–`0004`, while `docs/adr/README.md` indexed a different canonical line. | Active PR contains only canonical ADRs `0001`–`0006` and enforces identifier uniqueness. ADR-0006 remains Proposed while its runtime decision is unmerged. | Corrected on active PR | Keep identifier uniqueness green and promote ADR-0006 only after protected integration plus current-head runtime evidence. |
| Product-authority documentation was split across PR #3 and PR #1. | PR #3 was semantically compared against the broader current product line and closed as superseded; its applicable authority/consumer-contract content is preserved or strengthened in #1. | Converged | Keep one canonical documentation line in #1/protected `develop`. |
| Repository name `quarantine-sandbox-runtime` is security-biased for the broader responsibility. | Runtime responsibility now includes isolated application services in addition to artifact analysis. Repository-settings rename support is outside this writer path. | Known gap | Re-evaluate a rename such as `isolation-runtime` before GA through an authorized repository-settings path; preserve redirects and consumer migration if renamed. |

## Application-service isolation

| Capability | Current evidence | Status | Required action |
| --- | --- | --- | --- |
| Immutable application identity | A fresh security review found that a digest suffix alone did not constrain Podman image resolution: the prior validator could accept explicit `containers/image` transports such as `dir:`, `docker-archive:`, `oci-archive:`, `docker-daemon:`, or `containers-storage:` and pass them directly to `podman create --pull=never`, allowing consumer-controlled host-backed paths or alternate stores. RED `02ee218a…` adds hostile transport cases; production repair `6df890ca…` restricts the consumer contract to normal registry/storage names with an optional numeric registry port; schema repair `63d7e1d5…` and consumer docs `f63f284b…` align the wire boundary; `e90f1789…` adds primary transport documentation traceability. | Corrected on active PR; exact-head CI pending | Keep explicit image-import/admission as a separate trusted control-plane operation if needed. Never permit launch-time host path/alternate-store transports or implicit pulls. |
| Rootless backend | Adapter probes Podman `Host.Security.Rootless` and fails closed unless it returns `true`. Hosted acceptance explicitly targets the Ubuntu 24.04 distribution Podman 4.9.3 after the runner image reverted from the prior 5.8.4 bundle. | Active PR; current-head rerun pending | Re-prove the unchanged effective-isolation contract on the current exact source head; backend version identity is evidence, not a substitute for effective controls. |
| Read-only/writable surface bounds | `--read-only`, `--read-only-tmpfs=false`, bounded `/tmp` with `noexec,nosuid,nodev`, `--image-volume=ignore`, `--no-hosts`, `--no-hostname`, `--systemd=false`, `--sdnotify=ignore`, and `--http-proxy=false`. Consumer-controlled image transports that could select host directories/archives/alternate stores are now rejected before Podman invocation. | Active PR | Add explicit read-only input mounts through a typed mount contract only when required; no arbitrary host path request. |
| Privilege/namespace isolation | `--cap-drop=all`, no-new-privileges, `--userns=auto`, private PID/IPC/UTS/cgroup namespaces, numeric non-root UID/GID. | Active PR | Verify effective capability sets, namespaces, LSM profile/label where required, and runtime state in real-container evidence instead of relying on argv intent. |
| Network isolation | Per-sandbox `--internal --disable-dns` bridge and loopback-only random host port publication. | Active PR | Preserve real-container proof of no external route. Controlled egress must be a separate reviewed profile. |
| Credential isolation | No consumer/provider credentials, environment map, credential mounts, runtime sockets, or host devices exist in the P0 request contract; proxy environment inheritance is disabled. | Active PR | Add an explicit secret-broker design only if a buyer workflow cannot operate without a task-scoped capability; default remains credential-free. |
| Resource limits | Memory, CPU, PID, lease, tmpfs, readiness timeout/polling, and shutdown grace are policy-bounded. Podman receives memory/CPU/PID/runtime controls. | Active PR | Test rootless cgroup behavior on representative Linux hosts and fail closed when a requested limit cannot be enforced. |
| Process-boundary lifecycle | Adapter invokes Podman directly without a shell, checks rootless mode, creates network/container, starts, resolves loopback port, gates readiness, and removes resources. Fake-Podman tests cover failure paths. Earlier exact source `984d3a6…` passed hosted real rootless-Podman E2E on 5.8.4. The current stack also contains concrete bounded-subprocess tests for actual `Child` spawn/capture/timeout/output-overflow cleanup and must re-run the real Podman lane on 4.9.3. | Active PR; predecessor real P0 evidence only | Obtain terminal current-head real-container and complete coverage evidence; then add durable crash-recovery reaping for stale resources. |
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

The runtime still exposes only an embeddable Rust library. There is no supported authenticated process boundary or generated Python consumer yet, and `127.0.0.1` lease reachability is defined only for a co-located host-network consumer. Before issue #991 can integrate, this repository must choose and publish one versioned transport or binding, bind leases to authenticated caller identity and idempotency scope, define the supported network topology, and provide stable bounded wire errors and semantic lease/cleanup validation. A sibling checkout, ad-hoc subprocess protocol, or direct Podman call is not an acceptable substitute.

## Verification and release gaps

- Protected `develop` remains `60a85c7633e03b425b67159ec6822c8178cf87ea`; the latest reviewed production/source head is `e90f1789da2bf4a4d511ae4666433cf86c0237e5`. PR #1 remains Draft/mergeable.
- Exact predecessor `d2893e39ef71d72d315f71f44ee04f498369b3e2` eventually received GitHub-hosted runners. Its `verify` job succeeded, proving that the earlier pre-checkout queue state was not a permanent leaf admission condition.
- On that same predecessor, strict coverage failed with all production lines/functions covered and all nightly branches covered, but one `src/infrastructure/bounded_command.rs` production region remained uncovered. Later source adds concrete real-process lifecycle tests; no coverage denominator, ignore rule, skip, or source rewriting is used.
- The predecessor Podman job failed before fixture launch because the workflow asserted Podman 5.8.4 while the current Ubuntu 24.04 hosted image supplies distribution Podman 4.9.3. The current stack explicitly verifies 4.9.3 and rootless mode while retaining the real effective-isolation/cleanup acceptance unchanged.
- For exact head `e90f1789…`, CI run `33797645958` plus SAST `33797645973`, Security Scan `33797645853`, Scorecard `33797646154`, and OSV `33797646539` are queued. Queued evidence is non-passing; predecessor successes or failures do not transfer.
- Central `.github#712` remains the canonical runner-admission/queue-health owner. Quarantine runner behavior is mixed: hosted assignment recovered for a predecessor while later exact heads can remain queued. Leaf source must not be churned merely to manufacture execution.
- Positive LSM evidence remains a separate release/security gate. Host LSM availability and effective per-sandbox profile/label/domain are distinct facts; an unavailable/empty/contradictory attestation is never promoted to verified.
- The active organization ruleset and required central workflows/review governance remain authoritative. Bypass capability is not merge evidence.
- No immutable release exists. Release/version publication is premature until one integrated protected head satisfies current CI/security/coverage/effective-runtime/LSM/review/package/SBOM/provenance/reproducibility/rollback evidence together.

## Next bounded slices

1. Obtain terminal exact-head CI for the current PR #1 head; if coverage, image-reference validation, or Podman E2E is red, use the first current causal boundary for another RED → smallest fix → GREEN cycle.
2. Finish positive effective-LSM/capability/resource/network attestation on the appropriate reviewed backend/runner without weakening profiles for generic hosted-runner convenience.
3. Choose and implement the authenticated, versioned consumer transport or supported language binding, including caller-scoped idempotency, stable wire errors, strict response validation, and an explicit co-location/network-topology contract.
4. Add crash/restart lease reclamation and orphan cleanup evidence.
5. Publish an immutable runtime artifact, generated/typed consumer contract, SBOM, and provenance bound into lease attestation.
6. Integrate `contextual-orchestrator` through issue #991 and Wardnet through its own ACL, without moving consumer policy into this repository.
7. Add gVisor/containerd and Kubernetes RuntimeClass adapters only after the P0 contract is stable.
8. Resume artifact-analysis adapters and dynamic detonation on top of the shared sandbox execution Core.
