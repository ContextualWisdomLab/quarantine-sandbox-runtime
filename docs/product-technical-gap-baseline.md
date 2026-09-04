# Product and Technical Gap Baseline

Last reviewed on 2026-09-04 KST against dependency-root PR #1 source head `5dcabdbd5a61f9e42e474896e07ed9480dafc491`, protected `develop@60a85c7633e03b425b67159ec6822c8178cf87ea`, and application-service runtime-identity RED test-bearing head `f6688cad3ada971afa00231b807a93f0f89513f5`. This ledger distinguishes protected truth, active-PR implementation, checked-in RED evidence, backend-applied configuration evidence, live effective-runtime proof, and queued/cancelled checks. Predecessor evidence never transfers to a moved head.

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
| Rootless backend | Podman security info is parsed and execution fails closed unless rootless mode plus required seccomp/LSM host capability are present. A causal predecessor ran Ubuntu 24.04 / Podman 4.9.3. | Active repair; current-head verification pending | Re-prove unchanged current head on real Podman; host capability is not per-sandbox proof. |
| Read-only / writable surfaces | Launch uses read-only rootfs, `--read-only-tmpfs=false`, bounded `/tmp` with `rw,noexec,nosuid,nodev`, image volumes ignored, no generated hosts file, no proxy inheritance, and no systemd/sdnotify integration. Unsupported Podman 4.9.3 `--no-hostname` has been removed and the current fixture asserts it stays absent. | Configured intent plus partial runtime inspection | Bind exact applied tmpfs mount set/options/size, then require live mount proof. |
| Privilege / namespace isolation | Current repair checks empty container Effective/Bounding caps and process Effective/Bounding/Inheritable/Permitted/Ambient sets, no-new-privileges, seccomp, user/PID/IPC namespaces, numeric non-root identity, and enforcing SELinux/AppArmor evidence. | Implemented on active root | Preserve negative/positive real-backend tests; empty/unconfined/malformed/contradictory LSM evidence never counts as Verified. |
| Network isolation | Per-sandbox internal DNS-disabled network is inspected and publication must resolve only to loopback. | Implemented on active root | Add final real reachability proof; controlled egress is a separate reviewed profile. |
| Credential isolation | P0 request has no provider/user credentials, arbitrary environment map, host device/runtime socket, or broad host mount. | Active contract | Any future secret flow requires an explicit purpose-bound broker. |
| CPU/RAM/PID bounds | `HostConfig.Memory`, `NanoCpus`, and `PidsLimit` are inspected against the request. | Backend-applied binding only; live proof incomplete | Verify the exact sandbox's authoritative cgroup-v2 values before release claim. |
| tmpfs / wall time | Launch applies bounded `/tmp` and `--timeout`, but current root does not deserialize/bind `HostConfig.Tmpfs` or `Config.Timeout`; inspect state alone would still not prove live enforcement. Draft #19 carries REDs for missing/wrong hardening, contradictory/duplicate tmpfs options, widened writable mounts, timeout mismatch, cleanup/non-publication, and exact inspect state without live proof. | P0 RED staged on Draft #19 | After root integration, non-force adopt #19, execute RED, add minimal inspect binding, then require live cgroup/mount/wall-time proof. |
| Process lifecycle | Adapter invokes Podman without a shell, creates network/container, starts, attests isolation before port/readiness, and attempts complete cleanup after partial launch/attestation failure. | Active repair | Current exact head must execute full tests; durable crash/restart orphan recovery belongs to Recovery context. |
| Ownership/idempotency | Draft #6 implements caller-scoped lease ownership/idempotency but is process-local and stale behind current root. | Implemented on stale descendant | After root integration, adopt protected parent non-force and reacquire exact-head evidence. |
| Runtime resource identity | Root `sandbox_identity()` derives `qsr-app-*` / `qsr-net-*` from consumer `request_id`, image, policy, and whole-second start time. Independent runtime instances can therefore derive the same cleanup-owning container/network identity. Issue #20 and Draft #21 carry a concurrent RED at test-bearing `f6688cad3ada971afa00231b807a93f0f89513f5`; production remains unchanged. | P0 RED staged on Draft #21 | Execute the checked-in RED first. After reproduced collision, introduce a runtime-generated invocation/lease identity below consumer correlation and use it consistently for names, labels, receipts, and cleanup while preserving #6 idempotency semantics separately. |
| gVisor/containerd/Kubernetes | Architecture targets only. | Missing | Add independent adapters after P0 contract stabilizes; public contracts remain backend-neutral. |

## Attestation evidence model

Configured launch intent, backend-applied inspection, and live effective enforcement are different evidence levels. A security control may advance only to the level actually proved for the exact sandbox identity. In particular:

- Podman inspect can bind requested configuration but cannot by itself prove kernel cgroup or mount enforcement or that a wall-time termination actually occurred.
- CPU/RAM/PID effective claims require authoritative cgroup-v2 evidence where the backend exposes it.
- `/tmp` effective claims require live mount evidence showing tmpfs, exact mount point, required restrictions, and bounded size.
- Wall-time effective claims require behavioral/runtime-owned termination and cleanup evidence or an equivalent reviewed watchdog.
- Host AppArmor/SELinux availability is not per-sandbox confinement; positive LSM evidence remains a separate release gate.
- Consumer correlation/idempotency identity is not runtime resource ownership. Cleanup-owning container/network identity must remain unique for actually independent invocations across runtime processes, restarts, and stale resources.

## Artifact analysis

| Capability | Current evidence | Status | Required action |
| --- | --- | --- | --- |
| Static foundation | SHA-256 identity, bounded ingestion, format detection, analyzer interface, deterministic evidence, and failure attribution exist on active foundation. | Active PR | Preserve under `artifact_analysis`; complete exact-head coverage/review. |
| Claude plugin package quarantine profile | Issue #17 defines immutable source/artifact/AppGuardrail identity, fixed probes, credential-free execution, bounded resources, deny-by-default network, cleanup/recovery, and evidence-only receipts. Draft #18 contains contract-first RED and remains stale behind its moving command parent. | RED staged, not production truth | Do not overtake root/resource/identity/command P0 blockers; reconcile ancestry non-force before execution/implementation. |
| YARA-X / capa / Ghidra / LIEF | No production adapters yet. | Missing | Add one evidence producer per bounded increment with exact version/digest provenance. |
| Dynamic Linux/Windows detonation | No release-grade detonation worker exists. | Missing | Consume sandbox Core/stronger backend; never execute hostile bytes in control process. |
| Durable evidence/signing | In-memory evidence only. | Missing | Add persistence only after owner/retention/recovery/3NF/idempotency ADR. |

## Verification and release state

- Protected `develop` remains `60a85c7633e03b425b67159ec6822c8178cf87ea`; PR #1 remains Draft, with no qualifying approval at the latest read.
- The causal effective-attestation RED on predecessor `3fa5c5493fcbfbfb1c28b075e3bad30c03ea29b3` executed and failed because the old runtime could return a lease without effective sandbox inspection. The same real lane exposed unsupported Podman 4.9.3 `--no-hostname`; both defects have causal repairs on the active root lineage.
- Root source/test lineage advanced to `5dcabdbd5a61f9e42e474896e07ed9480dafc491`; current PR #1 head `24526eb55cf5db48ea07079b314f7d1b676eb48d` is a documentation-only gap-ledger reconciliation.
- Exact root CI `33867568229` has `podman-e2e=101005801190`, `branch-coverage=101005801271`, `coverage=101005801385`, and `verify=101005801874`; all remained queued with no execution steps at the latest read. Security `33867568217` and SAST `33867568350` were also queued. Queued evidence is non-passing.
- Application-service runtime-identity Draft #21 is stacked directly on exact root `24526eb55cf5db48ea07079b314f7d1b676eb48d`. Its latest test-bearing head `f6688cad3ada971afa00231b807a93f0f89513f5` changes only the concurrent RED. Initial CI `33868867225` materialized `coverage=101009874705`, `podman-e2e=101009874973`, `verify=101009875048`, and `branch-coverage=101009875059`; all were queued before execution. The RED has therefore not yet reproduced the collision and production GREEN is forbidden.
- Central hosted-runner admission remains `.github#712`. Positive effective-LSM runner capability remains `.github#1590`; generic hosted recovery cannot substitute for per-sandbox positive confinement proof.
- Current descendants are deliberately not destructively restacked while the root is non-GREEN. #6/#9/#10/#13/#14 retain valid implementation/test deltas; #18 and #19 retain RED-only deltas on older parent snapshots; #21 retains the new application-service identity RED directly on the current root. They must adopt protected ancestry non-force and reacquire exact-head evidence after root integration as applicable.
- GitHub Releases was freshly read as empty earlier on 2026-09-04; no immutable runtime release exists for Wardnet, contextual-orchestrator, or Noema to pin. Re-read release authority before any publication decision.

## Consumer and release contract

Wardnet remains SOC/gateway/verdict authority; contextual-orchestrator remains LLM/Agent orchestration authority; Noema remains capability/admission authority. Consumers may use only a future immutable released runtime artifact and versioned contract/ACL. Direct Podman/containerd calls, sibling source imports, mutable PR heads, and cross-service SQL are not integration mechanisms.

The first release remains blocked until one exact integrated protected candidate carries complete owned statement/branch/edge coverage and public rustdoc, realistic rootless isolation E2E, positive effective LSM/seccomp/capability/resource/network/cleanup evidence, unique cleanup-owned runtime resource identity, required review/security/SAST gates, package smoke, SPDX SBOM, provenance, checksum/signature as supported, reproducibility, upgrade/rollback evidence, and an immutable artifact identity.

## Next bounded slices

1. Execute the current root exact-head CI/security reruns; repair only an actually reproduced current-head failure and merge normally only after approval/rules/effective-runtime gates are satisfied.
2. Execute Draft #21's application-service runtime-identity RED when admitted. After reproduced collision, apply only the smallest runtime-generated invocation-identity repair and prove cleanup ownership cannot cross independent invocations.
3. After root integration, adopt Draft #19 non-force and execute its resource-attestation RED; implement only the smallest causal inspect-binding GREEN, then add live cgroup/mount/wall-time proof.
4. Reconcile #6/#9/#10/#13/#14 dependency-first after root integration; execute and repair issue #16 command identity and Podman-4.9 compatibility REDs before command backend integration.
5. Obtain dedicated positive LSM real-backend acceptance without weakening the P0 profile.
6. Reconcile #18 only after higher-stack P0 blockers and parent ancestry are current; keep artifact-analysis receipts as risk evidence, not admission/verdict authority.
7. Publish the first immutable runtime release only from one exact integrated protected head, then hand off released version/digest pinning to consumer owner paths.