# Product and Technical Gap Baseline

Last reviewed on 2026-09-04 KST against dependency-root PR #1 production/source head `06988737a70e3cb1c7dd49a515e59079f81bbf73`, protected `develop@60a85c7633e03b425b67159ec6822c8178cf87ea`, and latest test-bearing PR #19 commit `62b195f73d9399ceee1666bdd3d048843d530139`. Commits after that test-bearing identity in PR #19 update traceability/gap documentation only. This ledger distinguishes protected truth, active production implementation, checked-in RED evidence, backend-applied configuration evidence, live effective-runtime proof, and queued/non-executed checks; predecessor evidence never transfers to a moved head.

## Product responsibility

Quarantine Sandbox Runtime owns reusable sandbox execution, isolation-policy enforcement, resource bounds, lease/readiness/cleanup attestation, and artifact-analysis evidence. `sandbox_execution` is the Core bounded context; `artifact_analysis` and `application_service` are Supporting contexts. Podman/gVisor/containerd/Kubernetes/VM remain infrastructure adapters. Wardnet retains verdict/incident/quarantine authority. Agent/chat consumers retain task/tool/application authorization, identity, secrets, and user-action authority.

## DDD and repository structure

| Gap | Current evidence | Status | Required action |
| --- | --- | --- | --- |
| Artifact analysis was flattened under root `contracts.rs`, `ingestion.rs`, and `runtime.rs`. | PR #1 keeps implementation under `src/artifact_analysis/` and repository fitness tests forbid obsolete root paths. | Corrected on active PR | Preserve fitness tests through protected integration. |
| Podman infrastructure had leaked into Core and depended on Supporting application-service types. | PR #1 keeps Podman under `src/infrastructure/`, with backend-neutral Core contracts and composition-boundary error translation. | Corrected on active PR | Keep dependency-direction tests and exact-head evidence green. |
| Pre-publication ADR identities conflicted. | Canonical ADR line is `0001`–`0006`; ADR-0006 remains Proposed while the runtime decision is unmerged. | Corrected on active PR | Promote only after protected integration and then-current runtime evidence. |
| Admission, session lifecycle, recovery, and network/egress responsibilities still meet inside process-local application-service coordination. | Issue #8 records the intended bounded-context split and forbids durable/distributed recovery or admission from accumulating in generic application-service internals. | Known structural gap | Extract only when executable contracts and compatibility boundaries are ready; do not perform cosmetic folder churn. |
| Repository name remains security-biased relative to application-service isolation responsibility. | Product scope spans hostile artifact analysis and reusable application isolation. | Known naming gap | Re-evaluate before GA through an authorized repository-settings path, preserving redirects and consumer migration. |

## Attestation evidence model

Security controls use two distinct evidence levels and must not collapse them into one `Verified` claim.

1. **Backend-applied configuration binding** proves that the backend reports the requested immutable configuration on the exact sandbox: for Podman this includes inspect-visible identity, namespace/security state, resource values, `/tmp` tmpfs configuration, and timeout configuration. This is stronger than argv intent, but it remains configuration/state evidence.
2. **Live effective-runtime proof** demonstrates that the kernel/runtime is actually enforcing the control. For cgroup-v2 resource controls this requires runtime-owned evidence from the sandbox cgroup (for example `memory.max`, `pids.max`, and `cpu.max` where the backend exposes an authoritative cgroup path). For `/tmp`, positive proof must establish that the running sandbox sees a tmpfs at the intended mount point with the required restrictions and bounded size. Wall-time requires behavioral/runtime-owned proof that the sandbox is terminated at the bound and cleaned up; `Config.Timeout` alone is not sufficient to claim that kill enforcement happened.

A release profile may use inspect evidence as an admission prerequisite, but it may not promote missing, contradictory, malformed, host-only, or merely requested intent into effective proof. Positive release acceptance must bind both evidence levels to the same sandbox/artifact identity where the control is security-relevant.

## Application-service isolation

| Capability | Current evidence | Status | Required action |
| --- | --- | --- | --- |
| Immutable application identity | Registry-style image references must carry SHA-256 digest identity; explicit host-backed/alternate-store transports are rejected and Podman launch uses `--pull=never`. | Corrected on active PR | Keep image import/admission separate from launch; never permit mutable or host-path resolution through consumer input. |
| Rootless backend | PR #1 parses Podman security info and fails closed unless rootless mode and required seccomp/LSM host capabilities are present. The causal predecessor real lane ran Ubuntu 24.04 with distribution Podman 4.9.3. | Active repair; exact-head checks pending | Re-prove unchanged current head on real Podman; backend capability is not effective sandbox proof. |
| Read-only and writable-surface bounds | Launch plan uses read-only rootfs, `--read-only-tmpfs=false`, bounded `/tmp` with `noexec,nosuid,nodev`, no image volumes, no hosts file, no proxy inheritance, no systemd/sdnotify. Unsupported Podman 4.9.3 `--no-hostname` was removed by the dependency-root repair rather than weakening isolation. | Configured intent plus partial runtime inspection | Bind exact `/tmp` tmpfs options/size through inspect before publication, then add live mount proof on the release profile. |
| Privilege and namespace isolation | Effective container Effective/Bounding caps plus process Effective/Bounding/Inheritable/Permitted/Ambient sets are required empty; no-new-privileges, runtime seccomp, user/PID/IPC namespace state, numeric non-root identity and LSM confinement are checked before readiness. | Implemented on active root | Keep negative/positive real-backend tests; host-only LSM capability, empty/unconfined labels or malformed/mismatched profiles never count as verified. |
| Network isolation | Per-sandbox internal DNS-disabled network is inspected and publication must resolve only to loopback. | Implemented on active root | Add real reachability proof on the final release profile; controlled egress remains a separate reviewed profile. |
| Credential isolation | P0 request has no provider/user credentials, arbitrary environment map, host device/runtime socket or broad host mount. | Active contract | Default remains credential-free; future secret capability requires a separate purpose-bound broker design. |
| CPU/RAM/PID bounds | `HostConfig.Memory`, `NanoCpus`, and `PidsLimit` are inspected and must be positive and no greater than the request. | Backend-applied configuration binding implemented; live enforcement proof pending | Preserve fail-closed inspect binding, then verify the exact running sandbox's cgroup-v2 limits on a supported real backend before release. Do not label inspect fields alone as effective kernel enforcement. |
| tmpfs and wall-time bounds | Request/policy bound `tmpfs_bytes` and `lease_seconds`; launch argv carries `--tmpfs ...size=<bytes>` and `--timeout <seconds>`. Current PR #1 does not deserialize `HostConfig.Tmpfs` or `Config.Timeout`, so `resource_limits_match` can mark `resource_limits` verified from CPU/RAM/PID alone. PR #19 RED now covers missing tmpfs, wrong size, missing `noexec`, timeout `0`, wrong positive timeout, cleanup/non-publication, and an order-independent positive-control fixture. | P0 RED on Draft PR #19 | First execute the RED. Then minimally deserialize and strictly bind inspect configuration to the request without order-sensitive string comparison. Treat that as backend-applied configuration evidence only; add live mount and timeout-enforcement acceptance before release-level effective proof. |
| Process-boundary lifecycle | Adapter invokes Podman without a shell, creates network/container, starts, verifies isolation before port/readiness, and attempts container/network cleanup after partial launch/attestation failure. | Active repair | Current #1 exact-head CI/security must reach terminal GREEN; add durable crash/restart orphan recovery later under Recovery context. |
| Application-service ownership/idempotency | Draft #6 implements caller-scoped lease ownership/idempotency but is stale behind the advanced root #1 head. | Implemented on stale descendant | After root integration, adopt current parent non-force and reacquire exact-head evidence; do not copy predecessor checks. |
| gVisor/containerd/Kubernetes | Architecture targets only. | Missing | Add independent adapters only after the P0 contract stabilizes; keep consumer contracts backend-neutral. |

## Artifact analysis

| Capability | Current evidence | Status | Required action |
| --- | --- | --- | --- |
| Static foundation | SHA-256 identity, bounded ingestion, format detection, analyzer interface, deterministic evidence and failure attribution exist on active foundation. | Active PR | Preserve under `artifact_analysis`; complete exact-head coverage/review. |
| Claude plugin package quarantine profile | Issue #17 defines immutable source/artifact/AppGuardrail identity, fixed probes, credential-free execution, bounded CPU/RAM/PID/disk/output/wall-time, deny-by-default network, cleanup/recovery, and evidence-only receipt semantics. Draft #18 contains contract-first RED and is stacked below stale command descendants. | RED staged, not production truth | Do not overtake P0 root/resource/command blockers. Execute current RED after dependency order is repaired, then add the smallest strict profile contract and hostile fixtures. |
| YARA-X / capa / Ghidra / LIEF | No production adapters yet. | Missing | Add one evidence producer per bounded increment with exact version/digest provenance. |
| Dynamic Linux/Windows detonation | No release-grade detonation worker exists. | Missing | Consume sandbox Core/stronger backend; never execute hostile bytes in the control process. |
| Durable evidence/signing | In-memory evidence only. | Missing | Add persistence only after an ADR defines owner, retention, backup/restore and 3NF/idempotency boundaries. |

## Verification and release state

- Protected `develop` is `60a85c7633e03b425b67159ec6822c8178cf87ea`; current dependency root is Draft PR #1 at `06988737a70e3cb1c7dd49a515e59079f81bbf73` and is mergeable but not merge-ready.
- The effective-attestation RED on predecessor `3fa5c5493fcbfbfb1c28b075e3bad30c03ea29b3` executed: coverage failed because the old implementation could return an application-service lease without effective sandbox inspection. That is causal RED evidence for the controls it exercised, not evidence for later heads.
- The same predecessor real Podman run acquired Ubuntu 24.04/Podman 4.9.3, passed rootless preflight, and failed container creation because the plan used unsupported `--no-hostname`. Root #1 removed only that compatibility defect while retaining P0 controls.
- Current PR #1 CI run `33848759065` has `verify=100946526543`, `coverage=100946526492`, `branch-coverage=100946526294`, and `podman-e2e=100946526494`; all remain pre-checkout queued with `runner_id=0`, `steps=[]`. Security Scan `33848759256` and SAST `33848759165` are also queued. Queued evidence is non-passing.
- PR #1 has no unresolved review threads but also no qualifying approval; rules/review/security gates remain authoritative and administrator capability is not evidence.
- Positive effective LSM acceptance remains separately dependent on the reviewed dedicated-runner capability tracked by `.github#1590`; generic hosted failure or host LSM availability cannot substitute for per-sandbox effective confinement.
- Central hosted-runner acquisition/queue health remains `.github#712`. Product source is not churned merely to obtain a runner.
- Draft PR #19 is a descendant security RED, not a replacement for #1 and not release authority. Its latest test-bearing commit is `62b195f73d9399ceee1666bdd3d048843d530139`. CI run `33851899138` has `podman-e2e=100956367359`, `coverage=100956367738`, `verify=100956367820`, and `branch-coverage=100956367924`; all are pre-checkout queued on `ubuntu-24.04` with `runner_id=0`, `steps=[]`, so the strengthened RED has not yet been observed failing for the intended causal reason.
- Organization ruleset `18156473` (`CWL Central required workflows`) remains active. GitHub Releases currently returns an empty collection. No immutable release exists, so Wardnet/contextual-orchestrator/Noema consumers must not pin active PR heads or sibling source.

## Primary-source traceability for resource bounds

- Podman documents `--timeout` as the maximum time a container may run before conmon sends the kill signal. That establishes the intended backend mechanism, not proof that a particular sandbox was actually killed on schedule.
- Podman documents `--tmpfs` as a tmpfs mount whose Linux mount options include `rw,noexec,nosuid,nodev` defaults when none are specified. The runtime explicitly requests these restrictions plus a size bound; inspect binding must be order-insensitive and fail closed on missing/contradictory options.
- Linux cgroup v2 is the authoritative kernel interface for hierarchical resource distribution. Release-grade CPU/RAM/PID claims should therefore bind the backend's sandbox identity to the relevant live cgroup values where the supported backend exposes them.
- Linux `/proc/<pid>/mountinfo` exposes mount point, mount options, filesystem type, and superblock options. A positive `/tmp` runtime check can use equivalent runtime-owned evidence to distinguish a live tmpfs mount from configuration intent.

## Next bounded slices

1. Let unchanged dependency-root #1 acquire terminal exact-head CI/security evidence; repair only a reproduced current-head failure and merge normally only after approval/rules/effective-runtime gates are satisfied.
2. Execute PR #19's strengthened tmpfs/wall-time RED. Once it fails for the intended reason, add the smallest request-bound inspect parser/validator as backend-applied configuration evidence; do not call that alone effective kernel/runtime enforcement.
3. Add a focused release-profile RED for live cgroup-v2 and `/tmp` mount proof, plus behavioral wall-time termination/cleanup proof or a runtime-owned watchdog design that can provide equivalent authoritative enforcement evidence.
4. After #1 integrates, adopt/adapt descendants in dependency order by non-force restack; #6/#9/#10/#13/#14/#18/#19 must reacquire their own exact-head evidence as applicable.
5. Execute and repair issue #16's invocation-unique one-shot command identity RED before command backend integration.
6. Obtain dedicated positive LSM real-backend acceptance without weakening the profile, then bind the final release candidate to complete coverage/rustdoc, security/SAST/review, package, SPDX SBOM, provenance, reproducibility and rollback evidence.
7. Publish the first immutable runtime release only from one exact integrated protected head; then hand off released version/digest pinning to consumer owner paths.
8. Continue issue #17 artifact-analysis profile only after the higher-stack P0 security/release blockers are resolved.
