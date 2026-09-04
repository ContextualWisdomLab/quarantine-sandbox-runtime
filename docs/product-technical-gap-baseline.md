# Product and Technical Gap Baseline

Last reviewed on 2026-09-04 KST against dependency-root PR #1 latest test/workflow-bearing head `6ad2b1c9d8f616be68dc28b35d017206f26c0787` and protected `develop@60a85c7633e03b425b67159ec6822c8178cf87ea`. This ledger distinguishes protected truth, active-PR implementation, checked-in RED evidence, backend-applied configuration evidence, live effective-runtime proof, and queued/cancelled checks. Predecessor evidence never transfers to a moved head.

## Product responsibility

Quarantine Sandbox Runtime owns reusable sandbox execution, isolation-policy enforcement, resource bounds, lease/readiness/cleanup attestation, and artifact-analysis evidence. `sandbox_execution` is the Core bounded context; `artifact_analysis` and `application_service` are Supporting contexts. Podman/gVisor/containerd/Kubernetes/VM remain infrastructure adapters. Wardnet retains verdict/incident/quarantine authority. Agent/chat consumers retain task/tool/application authorization, identity, secrets, and user-action authority.

## DDD and repository structure

| Gap | Current evidence | Status | Required action |
| --- | --- | --- | --- |
| Artifact analysis had been flattened under root modules. | PR #1 keeps implementation under `src/artifact_analysis/` and repository fitness tests forbid obsolete root paths. | Corrected on active PR | Preserve through protected integration. |
| Podman infrastructure had leaked into Core and Supporting application-service types. | PR #1 keeps Podman under `src/infrastructure/`, backend-neutral Core errors/contracts, and composition-boundary application translation. | Corrected on active PR | Keep dependency-direction and unique-ADR fitness tests GREEN. |
| Pre-publication ADR identities conflicted. | Canonical ADR line is `0001`–`0006`; ADR-0006 remains Proposed while the runtime decision is unmerged. | Corrected on active PR | Promote only after protected integration and then-current runtime evidence. |
| Admission, session lifecycle, recovery, and network/egress still meet inside process-local application-service coordination. | Issue #8 defines the intended bounded-context extraction and forbids durable/distributed responsibilities from accumulating in generic application-service internals. | Known structural gap | Extract only with executable contracts and compatibility boundaries; no cosmetic folder churn. |
| Repository name remains security-biased relative to application-service isolation responsibility. | Product scope spans hostile artifact analysis and reusable application isolation. | Known naming gap | Re-evaluate before GA through repository-settings owner path with redirects/consumer migration. |

## Application-service isolation

| Capability | Current evidence | Status | Required action |
| --- | --- | --- | --- |
| Immutable application identity | Registry-style image references require SHA-256 digest identity; explicit host-backed/alternate-store `containers/image` transports are rejected and launch uses `--pull=never`. | Corrected on active PR | Keep import/admission separate from launch; no mutable or host-path consumer resolution. |
| Rootless backend | Exact root `24526eb...` executed on GitHub-hosted Ubuntu 24.04 / Podman 4.9.3 and passed the explicit rootless/backend-version contract. | Real hosted evidence obtained | Re-prove every moved exact head; rootless capability does not prove per-sandbox LSM confinement. |
| Read-only / writable surfaces | Launch uses read-only rootfs, `--read-only-tmpfs=false`, bounded `/tmp` with `rw,noexec,nosuid,nodev`, image volumes ignored, no generated hosts file, no proxy inheritance, and no systemd/sdnotify integration. Unsupported Podman 4.9.3 `--no-hostname` has been removed and current contracts keep it absent. | Configured intent plus partial runtime inspection | Bind exact applied tmpfs mount set/options/size, then require live mount proof. |
| Privilege / namespace isolation | Current repair checks empty container Effective/Bounding caps and process Effective/Bounding/Inheritable/Permitted/Ambient sets, no-new-privileges, seccomp, user/PID/IPC namespaces, numeric non-root identity, and enforcing SELinux/AppArmor evidence. Exact hosted E2E `24526eb...` failed closed at `lsm`, proving that the ordinary hosted image cannot supply positive effective confinement. | Negative effective-LSM evidence obtained; positive proof pending | Hosted Ubuntu is now an explicit negative LSM lane. Positive release evidence remains on the dedicated SELinux-capable runner; never reinterpret unavailable/unconfined evidence as Verified. |
| Network isolation | Per-sandbox internal DNS-disabled network is inspected and publication must resolve only to loopback. Draft #23 carries a stronger RED requiring positive exact container attachment and rejection of missing/additional attachments. | Object-level evidence implemented; attachment proof pending | Execute #23 on current ancestry, then add exact attachment and real negative-egress proof. |
| Credential isolation | P0 request has no provider/user credentials, arbitrary environment map, host device/runtime socket, or broad host mount. | Active contract | Any future secret flow requires an explicit purpose-bound broker. |
| CPU/RAM/PID bounds | `HostConfig.Memory`, `NanoCpus`, and `PidsLimit` are inspected against the request. | Backend-applied binding only; live proof incomplete | Verify the exact sandbox's authoritative cgroup-v2 values before release claim. |
| tmpfs / wall time | Launch applies bounded `/tmp` and `--timeout`, but current root does not deserialize/bind `HostConfig.Tmpfs` or `Config.Timeout`; inspect state alone would still not prove live enforcement. Draft #19 preserves hostile REDs for missing/wrong hardening, contradictory/duplicate tmpfs options, widened writable mounts, timeout mismatch, cleanup/non-publication, and exact inspect state without live proof. | P0 RED preserved on a non-force root-adopted descendant; new root movement must be adopted before final evidence | Execute #19 after current root stabilizes; after the intended `resource_limits` RED, add only the smallest inspect-binding GREEN, then live cgroup/mount/wall-time proof. |
| Process lifecycle | Adapter invokes Podman without a shell, creates network/container, starts, attests isolation before port/readiness, and attempts complete cleanup after partial launch/attestation failure. | Active repair | Current exact head must execute full tests; durable crash/restart orphan recovery belongs to Recovery context. |
| Subprocess spawn pressure | Exact root `24526eb...` verify reached `cargo test` and one fake-Podman case returned `BackendInvocationFailed { operation: "rootless_probe" }` instead of the intended `InvalidPortMapping`. The runner currently collapses spawn/capture failures into one class, so the observed evidence does not prove that the underlying OS error was `WouldBlock`. | Reproduced current-head failure; errno-level cause still unproven | An unchanged-head verify rerun was requested. Do not adopt #6's bounded `WouldBlock` retry unless repeated evidence or focused injection proves that exact transient cause; avoid generic retry workarounds. |
| Ownership/idempotency | Draft #6 implements caller-scoped lease ownership/idempotency and carries additional subprocess changes. | Implemented on descendant | Preserve caller-scoped ownership separately from backend invocation identity; only adopt subprocess retry after causal proof. |
| gVisor/containerd/Kubernetes | Architecture targets only. | Missing | Add independent adapters after P0 contract stabilizes; public contracts remain backend-neutral. |

## Attestation evidence model

Configured launch intent, backend-applied inspection, and live effective enforcement are different evidence levels. A security control may advance only to the level actually proved for the exact sandbox identity. In particular:

- Podman inspect can bind requested configuration but cannot by itself prove kernel cgroup or mount enforcement or that a wall-time termination actually occurred.
- A separately inspectable internal network does not prove that the running container is attached only to that network.
- CPU/RAM/PID effective claims require authoritative cgroup-v2 evidence where the backend exposes it.
- `/tmp` effective claims require live mount evidence showing tmpfs, exact mount point, required restrictions, and bounded size.
- Wall-time effective claims require behavioral/runtime-owned termination and cleanup evidence or an equivalent reviewed watchdog.
- Host AppArmor/SELinux availability is not per-sandbox confinement. Ordinary hosted CI is a negative fail-closed lane; positive LSM evidence remains a separate release gate.

## Artifact analysis

| Capability | Current evidence | Status | Required action |
| --- | --- | --- | --- |
| Static foundation | SHA-256 identity, bounded ingestion, format detection, analyzer interface, deterministic evidence, and failure attribution exist on active foundation. | Active PR | Preserve under `artifact_analysis`; complete exact-head coverage/review. |
| Claude plugin package quarantine profile | Issue #17 defines immutable source/artifact/AppGuardrail identity, fixed probes, credential-free execution, bounded resources, deny-by-default network, cleanup/recovery, and evidence-only receipts. Draft #18 contains contract-first RED and remains stale behind its moving command parent. | RED staged, not production truth | Do not overtake root/resource/command P0 blockers; reconcile ancestry non-force before execution/implementation. |
| YARA-X / capa / Ghidra / LIEF | No production adapters yet. | Missing | Add one evidence producer per bounded increment with exact version/digest provenance. |
| Dynamic Linux/Windows detonation | No release-grade detonation worker exists. | Missing | Consume sandbox Core/stronger backend; never execute hostile bytes in control process. |
| Durable evidence/signing | In-memory evidence only. | Missing | Add persistence only after owner/retention/recovery/3NF/idempotency ADR. |

## Verification and release state

- Protected `develop` remains `60a85c7633e03b425b67159ec6822c8178cf87ea`; PR #1 remains Draft and had no qualifying approval at the latest review sweep.
- The earlier causal effective-attestation RED `3fa5c549...` executed and failed because the old runtime could return a lease without effective sandbox inspection. Its causal production repair remains on the root lineage.
- Exact root `24526eb55cf5db48ea07079b314f7d1b676eb48d` CI `33867568229` eventually acquired GitHub-hosted runners. Formatting/repository-policy setup passed. Verify failed in `malformed_port_mappings_fail_closed_after_cleanup` because `rootless_probe` invocation failed before the intended malformed-port boundary; the exact underlying OS spawn error is not exposed, so a `WouldBlock` retry is not yet causal evidence. An unchanged-head verify rerun was requested instead of changing production subprocess behavior blindly.
- The same exact root's real Podman E2E passed Podman 4.9.3/rootless checks and immutable fixture pre-pull, then failed closed at `IsolationVerificationFailed { control_name: "lsm" }`; leak rejection succeeded. This is valid negative effective-LSM evidence, not a reason to weaken the control.
- Test/workflow-bearing `6ad2b1c9d8f616be68dc28b35d017206f26c0787` therefore moves the hosted job to an explicit negative-rootless-AppArmor acceptance and adds the dedicated positive SELinux self-hosted job already proven necessary by `.github#1590`. The runtime verifier itself is unchanged.
- Draft #19 was non-force reconciled with root `24526eb...` after its predecessor CI proved stale ancestry masked the resource RED. Because root has now moved again, #19 must adopt the new root non-force after root evidence stabilizes; predecessor checks do not transfer.
- Draft #23 strengthened the network-binding RED with wrong-mode, missing-positive-attachment, and unexpected-extra-attachment cases; production remains unchanged until that RED executes for the intended cause.
- Generic hosted-runner capacity is intermittent rather than absent: several exact heads have eventually acquired runners after long queues. Queue-health ownership remains `.github#712`.
- Positive effective-LSM runner capability remains `.github#1590`; generic hosted recovery cannot substitute for per-sandbox positive confinement proof.
- Current descendants are not destructively rewritten while the root is non-GREEN. #6/#9/#10/#13/#14 retain valid implementation/test deltas; #18/#19/#21/#23 retain their valid RED/evidence deltas and must adopt protected/current ancestry non-force at the appropriate dependency point.
- GitHub Releases remains empty at the latest fresh read. No immutable runtime release exists for Wardnet, contextual-orchestrator, or Noema to pin.

## Consumer and release contract

Wardnet remains SOC/gateway/verdict authority; contextual-orchestrator remains LLM/Agent orchestration authority; Noema remains capability/admission authority. Consumers may use only a future immutable released runtime artifact and versioned contract/ACL. Direct Podman/containerd calls, sibling source imports, mutable PR heads, and cross-service SQL are not integration mechanisms.

The first release remains blocked until one exact integrated protected candidate carries complete owned statement/branch/edge coverage and public rustdoc, realistic rootless isolation E2E, positive effective LSM/seccomp/capability/resource/network/cleanup evidence, required review/security/SAST gates, package smoke, SPDX SBOM, provenance, checksum/signature as supported, reproducibility, upgrade/rollback evidence, and an immutable artifact identity.

## Next bounded slices

1. Execute the moved root exact-head CI/security runs. Treat hosted effective-LSM failure as the expected negative lane and keep the dedicated positive lane fail-closed pending `.github#1590` capacity.
2. Resolve the root verify `rootless_probe` invocation failure only from reproduced errno-level evidence. If the unchanged-head rerun passes, record the transient and add focused spawn-error observability/RED before any retry policy; if it repeats with proven `WouldBlock`, implement the smallest bounded causal behavior and test it directly.
3. After root stabilizes, non-force adopt it into Draft #19 and execute the preserved resource-attestation RED; then add only the smallest inspect-binding GREEN before live cgroup/mount/wall-time proof.
4. Execute Draft #23's effective network-binding RED on current ancestry; after reproduced failure, require positive exact attachment to the runtime-owned deny-by-default network and real negative-egress proof.
5. Reconcile #6/#9/#10/#13/#14 dependency-first without force. Since the root now owns the generic-negative/dedicated-positive LSM CI split, #9 must adapt that overlapping delta rather than reintroduce it as an independent mutable foundation.
6. Reconcile #18 only after higher-stack P0 blockers and parent ancestry are current; keep artifact-analysis receipts as risk evidence, not admission/verdict authority.
7. Publish the first immutable runtime release only from one exact integrated protected head, then hand off released version/digest pinning to consumer owner paths.
