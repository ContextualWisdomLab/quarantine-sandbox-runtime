# Product and Technical Gap Baseline

Last reviewed on 2026-09-04 KST against dependency-root PR #1 production/source head `06988737a70e3cb1c7dd49a515e59079f81bbf73`, protected `develop@60a85c7633e03b425b67159ec6822c8178cf87ea`, and latest test-bearing PR #19 commit `2b9a1d9dced7b596a1b70bf80462e0cc862cf75b`. Commits after that test-bearing identity in PR #19 update traceability/gap documentation only. This ledger distinguishes protected truth, active production implementation, checked-in RED evidence, and queued/non-executed checks; predecessor evidence never transfers to a moved head.

## Product responsibility

Quarantine Sandbox Runtime owns reusable sandbox execution, isolation-policy enforcement, resource bounds, lease/readiness/cleanup attestation, and artifact-analysis evidence. `sandbox_execution` is the Core bounded context; `artifact_analysis` and `application_service` are Supporting contexts. Podman/gVisor/containerd/Kubernetes/VM remain infrastructure adapters. Wardnet retains verdict/incident/quarantine authority. Agent/chat consumers retain task/tool/application authorization, identity, secrets, and user-action authority.

## DDD and repository structure

| Gap | Current evidence | Status | Required action |
| --- | --- | --- | --- |
| Artifact analysis was flattened under root `contracts.rs`, `ingestion.rs`, and `runtime.rs`. | PR #1 keeps implementation under `src/artifact_analysis/` and repository fitness tests forbid obsolete root paths. | Corrected on active PR | Preserve fitness tests through protected integration. |
| Podman infrastructure had leaked into Core and depended on Supporting application-service types. | PR #1 keeps Podman under `src/infrastructure/`, with backend-neutral Core contracts and composition-boundary error translation. | Corrected on active PR | Keep dependency-direction tests and exact-head evidence green. |
| Pre-publication ADR identities conflicted. | Canonical ADR line is `0001`–`0006`; ADR-0006 remains Proposed while the runtime decision is unmerged. | Corrected on active PR | Promote only after protected integration and then-current runtime evidence. |
| Admission, session lifecycle, recovery, and network/egress responsibilities still meet inside process-local application-service coordination. | Issue #8 records the intended bounded-context split and forbids durable/distributed recovery or admission from accumulating in generic application-service internals. | Known structural gap | Extract only when executable contracts and compatibility boundaries are ready; do not perform cosmetic folder churn. |
| Repository name remains security-biased relative to application-service isolation responsibility. | Product scope now spans hostile artifact analysis and reusable application isolation. | Known naming gap | Re-evaluate before GA through an authorized repository-settings path, preserving redirects and consumer migration. |

## Application-service isolation

| Capability | Current evidence | Status | Required action |
| --- | --- | --- | --- |
| Immutable application identity | Registry-style image references must carry SHA-256 digest identity; explicit host-backed/alternate-store transports are rejected and Podman launch uses `--pull=never`. | Corrected on active PR | Keep image import/admission separate from launch; never permit mutable or host-path resolution through consumer input. |
| Rootless backend | PR #1 parses Podman security info and fails closed unless rootless mode and required seccomp/LSM host capabilities are present. The causal predecessor real lane ran Ubuntu 24.04 with distribution Podman 4.9.3. | Active repair; exact-head checks pending | Re-prove unchanged current head on real Podman; backend capability is not effective sandbox proof. |
| Read-only and writable-surface bounds | Launch plan uses read-only rootfs, `--read-only-tmpfs=false`, bounded `/tmp` with `noexec,nosuid,nodev`, no image volumes, no hosts file, no proxy inheritance, no systemd/sdnotify. Unsupported Podman 4.9.3 `--no-hostname` was removed by the dependency-root repair rather than weakening isolation. | Configured intent plus partial effective proof | Positive runtime evidence for the requested `/tmp` tmpfs remains missing; PR #19 owns the checked-in RED. |
| Privilege and namespace isolation | Effective container Effective/Bounding caps plus process Effective/Bounding/Inheritable/Permitted/Ambient sets are required empty; no-new-privileges, runtime seccomp, user/PID/IPC namespace state, numeric non-root identity and LSM confinement are checked before readiness. | Implemented on active root | Keep negative/positive real-backend tests; host-only LSM capability, empty/unconfined labels or malformed/mismatched profiles never count as verified. |
| Network isolation | Per-sandbox internal DNS-disabled network is inspected and publication must resolve only to loopback. | Implemented on active root | Add real reachability proof on the final release profile; controlled egress remains a separate reviewed profile. |
| Credential isolation | P0 request has no provider/user credentials, arbitrary environment map, host device/runtime socket or broad host mount. | Active contract | Default remains credential-free; future secret capability requires a separate purpose-bound broker design. |
| CPU/RAM/PID bounds | `HostConfig.Memory`, `NanoCpus`, and `PidsLimit` are inspected and must be positive and no greater than the request. | Effective inspection implemented | Verify real rootless cgroup behavior on supported hosts and preserve fail-closed semantics. |
| tmpfs and wall-time bounds | Request/policy already bound `tmpfs_bytes` and `lease_seconds`, and launch argv carries `--tmpfs ...size=<bytes>` plus `--timeout <seconds>`. Current PR #1 inspection model does **not** deserialize Podman `HostConfig.Tmpfs` or `Config.Timeout`; `resource_limits_match` therefore can mark resource limits verified from CPU/RAM/PID alone. Podman v4.9.3 source exposes both fields in inspect data. | P0 RED on Draft PR #19 | Execute `tests/podman_effective_resource_bounds_red.rs`; then minimally deserialize/validate effective `/tmp` options+size and timeout, failing closed on missing/malformed/contradictory/unbounded evidence before port/readiness. Re-run exact-head real Podman acceptance. |
| Process-boundary lifecycle | Adapter invokes Podman without a shell, creates network/container, starts, verifies effective isolation before port/readiness, and attempts container/network cleanup after partial launch/attestation failure. | Active repair | Current #1 exact-head CI/security must reach terminal GREEN; add durable crash/restart orphan recovery later under Recovery context. |
| Application-service ownership/idempotency | Draft #6 implements caller-scoped lease ownership/idempotency but is stale behind the advanced root #1 head. | Implemented on stale descendant | After root integration, adopt current parent non-force and reacquire exact-head evidence; do not copy predecessor checks. |
| gVisor/containerd/Kubernetes | Architecture targets only. | Missing | Add independent adapters only after the P0 contract stabilizes; keep consumer contracts backend-neutral. |

## Artifact analysis

| Capability | Current evidence | Status | Required action |
| --- | --- | --- | --- |
| Static foundation | SHA-256 identity, bounded ingestion, format detection, analyzer interface, deterministic evidence and failure attribution exist on active foundation. | Active PR | Preserve under `artifact_analysis`; complete exact-head coverage/review. |
| Claude plugin package quarantine profile | Issue #17 defines immutable source/artifact/AppGuardrail identity, fixed probes, credential-free execution, bounded CPU/RAM/PID/disk/output/wall-time, deny-by-default network, cleanup/recovery, and evidence-only receipt semantics. Draft #18 contains contract-first RED and is stacked below stale command descendants. | RED staged, not production truth | Do not overtake P0 root/resource/command blockers. Execute current RED after dependency order is repaired, then add smallest strict profile contract and hostile fixtures. |
| YARA-X / capa / Ghidra / LIEF | No production adapters yet. | Missing | Add one evidence producer per bounded increment with exact version/digest provenance. |
| Dynamic Linux/Windows detonation | No release-grade detonation worker exists. | Missing | Consume sandbox Core/stronger backend; never execute hostile bytes in control process. |
| Durable evidence/signing | In-memory evidence only. | Missing | Add persistence only after an ADR defines owner, retention, backup/restore and 3NF/idempotency boundaries. |

## Verification and release state

- Protected `develop` is `60a85c7633e03b425b67159ec6822c8178cf87ea`; current dependency root is Draft PR #1 at `06988737a70e3cb1c7dd49a515e59079f81bbf73` and is mergeable but not merge-ready.
- The effective-attestation RED on predecessor `3fa5c5493fcbfbfb1c28b075e3bad30c03ea29b3` **executed**: coverage failed because the old implementation could return an application-service lease without effective sandbox inspection. That is causal RED evidence.
- The same predecessor real Podman run acquired Ubuntu 24.04/Podman 4.9.3, passed rootless preflight, and failed container creation because the plan used unsupported `--no-hostname`. Root #1 removed only that compatibility defect while retaining P0 controls.
- Current PR #1 CI run `33848759065` has `verify=100946526543`, `coverage=100946526492`, `branch-coverage=100946526294`, and `podman-e2e=100946526494`; all remain pre-checkout queued with `runner_id=0`, `steps=[]`. Security Scan `33848759256` and SAST `33848759165` are also queued. Queued evidence is non-passing.
- PR #1 has no unresolved review threads but also no qualifying approval; existing submissions are COMMENTED. Rules/review/security gates remain authoritative and administrator capability is not evidence.
- Positive effective LSM acceptance remains separately dependent on the reviewed dedicated-runner capability tracked by `.github#1590`; generic hosted failure or host LSM availability cannot substitute for per-sandbox effective confinement.
- Central hosted-runner acquisition/queue health remains `.github#712`. Product source is not churned merely to obtain a runner.
- Draft PR #19 is a descendant security RED, not a replacement for #1 and not release authority. Its latest test-bearing commit is `2b9a1d9dced7b596a1b70bf80462e0cc862cf75b`; later commits add only primary-source traceability and this gap ledger.
- GitHub Releases currently returns an empty collection. No immutable release exists, so Wardnet/contextual-orchestrator/Noema consumers must not pin active PR heads or sibling source.

## Next bounded slices

1. Let unchanged dependency-root #1 acquire terminal exact-head CI/security evidence; repair only a reproduced current-head failure and merge normally only after approval/rules/effective-runtime gates are satisfied.
2. Execute PR #19’s tmpfs/wall-time RED. Once it fails for the intended reason, apply the smallest Podman ACL repair and require positive effective tmpfs/timeout proof before publication/readiness.
3. After #1 integrates, adopt/adapt descendants in dependency order by non-force restack; #6/#9/#10/#13/#14/#18 must reacquire their own exact-head evidence.
4. Execute and repair issue #16’s invocation-unique one-shot command identity RED before command backend integration.
5. Obtain dedicated positive LSM real-backend acceptance without weakening the profile, then bind the final release candidate to complete coverage/rustdoc, security/SAST/review, package, SPDX SBOM, provenance, reproducibility and rollback evidence.
6. Publish the first immutable runtime release only from one exact integrated protected head; then hand off released version/digest pinning to consumer owner paths.
7. Continue issue #17 artifact-analysis profile only after the higher-stack P0 security/release blockers are resolved.
