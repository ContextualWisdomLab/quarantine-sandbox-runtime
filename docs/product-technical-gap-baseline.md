# Product and Technical Gap Baseline

Last reviewed on 2026-09-04 KST against dependency-root PR #1 production/source head `06988737a70e3cb1c7dd49a515e59079f81bbf73`, protected `develop@60a85c7633e03b425b67159ec6822c8178cf87ea`, and latest test-bearing PR #19 commit `209cc9cb603485334d676924e9319ec844fc5e4f`. Commits after that test-bearing identity in PR #19 update research/traceability/gap documentation only. This ledger distinguishes protected truth, active production implementation, checked-in RED evidence, backend-applied configuration evidence, live effective-runtime proof, and queued/non-executed checks; predecessor evidence never transfers to a moved head.

## Product responsibility

Quarantine Sandbox Runtime owns reusable sandbox execution, isolation-policy enforcement, resource bounds, lease/readiness/cleanup attestation, and artifact-analysis evidence. `sandbox_execution` is the Core bounded context; `artifact_analysis` and `application_service` are Supporting contexts. Podman/gVisor/containerd/Kubernetes/VM remain infrastructure adapters. Wardnet retains verdict/incident/quarantine authority. Agent/chat consumers retain task/tool/application authorization, identity, secrets, and user-action authority.

## DDD and repository structure

| Gap | Current evidence | Status | Required action |
| --- | --- | --- | --- |
| Artifact analysis had been flattened under root modules. | PR #1 keeps implementation under `src/artifact_analysis/` and fitness tests forbid obsolete root paths. | Corrected on active PR | Preserve fitness tests through protected integration. |
| Podman infrastructure had leaked into Core and depended on Supporting application-service types. | PR #1 keeps Podman under `src/infrastructure/`, with backend-neutral Core contracts and composition-boundary error translation. | Corrected on active PR | Keep dependency-direction tests and exact-head evidence green. |
| Pre-publication ADR identities conflicted. | Canonical ADR line is `0001`–`0006`; ADR-0006 remains Proposed while the runtime decision is unmerged. | Corrected on active PR | Promote only after protected integration and then-current runtime evidence. |
| Admission, session lifecycle, recovery, and network/egress responsibilities still meet inside process-local application-service coordination. | Issue #8 records the intended bounded-context split and forbids durable/distributed recovery or admission from accumulating in generic application-service internals. | Known structural gap | Extract only when executable contracts and compatibility boundaries are ready; no cosmetic folder churn. |
| Repository name remains security-biased relative to application-service isolation responsibility. | Product scope spans hostile artifact analysis and reusable application isolation. | Known naming gap | Re-evaluate before GA through an authorized repository-settings path, preserving redirects and consumer migration. |

## Attestation evidence model

Security controls have two evidence levels that must not be collapsed into one `Verified` claim.

1. **Backend-applied configuration binding** proves that the backend reports the requested immutable configuration on the exact sandbox. For Podman this includes inspect-visible identity, namespace/security state, resource values, `/tmp` tmpfs configuration, and timeout configuration. This is stronger than argv intent but remains configuration/state evidence.
2. **Live effective-runtime proof** demonstrates that the kernel/runtime is actually enforcing the control. For cgroup-v2 resource controls this requires runtime-owned evidence from the exact sandbox cgroup. For `/tmp`, positive proof must establish that the running sandbox sees a tmpfs at the intended mount point with required restrictions and bounded size. Wall time requires behavioral/runtime-owned proof that the sandbox is terminated at the bound and cleanup completes; `Config.Timeout` alone cannot prove that the kill occurred.

Inspect evidence may be a fail-closed prerequisite, but missing, contradictory, malformed, host-only, or merely requested intent never becomes effective proof. A lease/release profile that claims effective resource isolation must bind live proof to the same sandbox identity.

## Application-service isolation

| Capability | Current evidence | Status | Required action |
| --- | --- | --- | --- |
| Immutable application identity | Registry-style image references must carry SHA-256 digest identity; explicit host-backed/alternate-store transports are rejected and launch uses `--pull=never`. | Corrected on active PR | Keep import/admission separate from launch; no mutable or host-path consumer resolution. |
| Rootless backend | PR #1 parses Podman security info and fails closed unless rootless mode and required seccomp/LSM host capabilities are present. A causal predecessor real lane ran Ubuntu 24.04 with Podman 4.9.3. | Active repair; exact-head checks pending | Re-prove unchanged current head on real Podman; host capability is not per-sandbox effective proof. |
| Read-only and writable surfaces | Launch uses read-only rootfs, `--read-only-tmpfs=false`, bounded `/tmp` with `noexec,nosuid,nodev`, no image volumes, hosts file, proxy inheritance, systemd or sdnotify. | Configured intent plus partial runtime inspection | Bind `/tmp` inspect configuration before publication, then require live mount proof. |
| Privilege and namespace isolation | Container Effective/Bounding caps plus process Effective/Bounding/Inheritable/Permitted/Ambient sets must be empty; no-new-privileges, seccomp, user/PID/IPC namespace state, numeric non-root identity and LSM confinement are checked. | Implemented on active root | Keep negative/positive real-backend tests; host-only LSM capability, empty/unconfined label, or malformed/mismatched profile never counts as verified. |
| Network isolation | Per-sandbox internal DNS-disabled network is inspected and publication must resolve only to loopback. | Implemented on active root | Add real reachability proof on final release profile; controlled egress is a separate reviewed profile. |
| Credential isolation | P0 request has no provider/user credentials, arbitrary environment map, host device/runtime socket or broad host mount. | Active contract | Future secret capability requires a separate purpose-bound broker design. |
| CPU/RAM/PID bounds | `HostConfig.Memory`, `NanoCpus`, and `PidsLimit` are inspected and request-bounded. | Backend-applied binding only; live proof missing | Preserve inspect binding, then verify the exact running sandbox's cgroup-v2 limits before lease/release claim. |
| tmpfs and wall-time bounds | PR #1 launch argv carries bounded `--tmpfs` and `--timeout` but does not deserialize `HostConfig.Tmpfs` or `Config.Timeout`. PR #19 RED now rejects missing/wrong tmpfs configuration, missing `noexec`, timeout `0`, wrong positive timeout, and—critically—exact inspect configuration when no live runtime proof is supplied. Every failure requires stop/container/network cleanup and forbids port publication. | P0 RED on Draft PR #19 | Execute RED `209cc9cb603485334d676924e9319ec844fc5e4f`. First GREEN must add strict request-bound inspect parsing without treating it as effective proof. A second positive fixture must supply authoritative live cgroup/mount/time evidence before any lease can succeed. |
| Process lifecycle | Adapter invokes Podman without a shell, creates network/container, starts, verifies isolation before port/readiness, and attempts cleanup after partial launch/attestation failure. | Active repair | Current #1 exact-head CI/security must reach terminal GREEN; durable crash/restart orphan recovery belongs to Recovery context. |
| Ownership/idempotency | Draft #6 implements caller-scoped lease ownership/idempotency but is stale behind #1. | Implemented on stale descendant | After root integration, adopt current parent non-force and reacquire exact-head evidence. |
| gVisor/containerd/Kubernetes | Architecture targets only. | Missing | Add independent adapters only after P0 contract stabilizes; consumer contracts stay backend-neutral. |

## Artifact analysis

| Capability | Current evidence | Status | Required action |
| --- | --- | --- | --- |
| Static foundation | SHA-256 identity, bounded ingestion, format detection, analyzer interface, deterministic evidence and failure attribution exist on active foundation. | Active PR | Preserve under `artifact_analysis`; complete exact-head coverage/review. |
| Claude plugin package quarantine profile | Issue #17 defines immutable source/artifact/AppGuardrail identity, fixed probes, credential-free execution, bounded resources, deny-by-default network, cleanup/recovery, and evidence-only receipts. Draft #18 contains contract-first RED. | RED staged, not production truth | Do not overtake root/resource/command P0 blockers. |
| YARA-X / capa / Ghidra / LIEF | No production adapters yet. | Missing | Add one evidence producer per bounded increment with exact version/digest provenance. |
| Dynamic Linux/Windows detonation | No release-grade detonation worker exists. | Missing | Consume sandbox Core/stronger backend; never execute hostile bytes in control process. |
| Durable evidence/signing | In-memory evidence only. | Missing | Add persistence only after owner/retention/recovery/3NF/idempotency ADR. |

## Verification and release state

- Protected `develop` is `60a85c7633e03b425b67159ec6822c8178cf87ea`; dependency root Draft PR #1 is `06988737a70e3cb1c7dd49a515e59079f81bbf73`, mergeable but not merge-ready.
- The effective-attestation RED on predecessor `3fa5c5493fcbfbfb1c28b075e3bad30c03ea29b3` executed and failed because the old implementation could return a lease without the effective sandbox inspection then required. That evidence does not transfer to later heads.
- The same predecessor real lane acquired Ubuntu 24.04/Podman 4.9.3, passed rootless preflight, then exposed unsupported `--no-hostname`; #1 removed only that compatibility defect while retaining P0 controls.
- PR #1 CI run `33848759065` has `verify=100946526543`, `coverage=100946526492`, `branch-coverage=100946526294`, and `podman-e2e=100946526494`; they remain pre-checkout queued with no assigned runner/steps. Security Scan `33848759256` and SAST `33848759165` are also queued. Queued evidence is non-passing.
- PR #1 has no unresolved review threads but no qualifying approval. Rules/review/security gates remain authoritative; administrator capability is not evidence.
- Positive effective LSM acceptance still depends on the dedicated SELinux-capable runner capability tracked by `.github#1590`; generic hosted-runner recovery cannot substitute for per-sandbox positive confinement evidence.
- Central hosted-runner acquisition/queue health remains `.github#712`. Product controls are not weakened to obtain a runner.
- Draft PR #19 is a descendant RED, not release authority. Latest test-bearing commit is `209cc9cb603485334d676924e9319ec844fc5e4f`. CI run `33852512572` has `coverage=100958278561`, `podman-e2e=100958278760`, `verify=100958278820`, and `branch-coverage=100958278832`; all are queued on `ubuntu-24.04` with empty steps and no assigned runner at observation time. The RED has therefore not executed yet.
- Organization ruleset `18156473` (`CWL Central required workflows`) is active. GitHub Releases is empty. There is no immutable runtime release for Wardnet/contextual-orchestrator/Noema to pin.

## Primary-source traceability for resource bounds

- Podman documents `--timeout` as the maximum run time before conmon sends the kill signal. This specifies the backend mechanism, not proof that a particular sandbox was killed on schedule.
- Podman documents `--tmpfs` as a tmpfs mount and supports Linux mount options; the runtime requires `rw,noexec,nosuid,nodev` plus an exact size bound. Inspect parsing must be order-insensitive and fail closed on missing/contradictory values.
- Linux cgroup v2 is the authoritative kernel interface for hierarchical resource distribution. CPU/RAM/PID effective claims must bind the exact sandbox to authoritative live controller values where the backend exposes them.
- Linux `/proc/<pid>/mountinfo` exposes mount point, per-mount options, filesystem type and superblock options. Equivalent runtime-owned evidence can distinguish a live `/tmp` tmpfs from configuration intent.
- Detailed APA-style references and implementation/test mappings are maintained in `docs/doctoring/REFERENCES.md` and `docs/doctoring/STANDARD_TRACEABILITY.md`.

## Next bounded slices

1. Let unchanged #1 acquire terminal exact-head CI/security evidence; repair only a reproduced current-head failure and merge normally only after approval/rules/effective-runtime gates are satisfied.
2. Execute PR #19 RED `209cc9cb603485334d676924e9319ec844fc5e4f`. The intended current failure is that exact inspect resource configuration can still reach lease/publication without live resource-enforcement proof.
3. After causal RED, implement the smallest fail-closed inspect binding for `Tmpfs` and `Timeout`, then add a positive live-proof fixture using authoritative cgroup-v2, mount and wall-time evidence; only that full path may return a lease.
4. After #1 integrates, adopt/adapt descendants by non-force restack; #6/#9/#10/#13/#14/#18/#19 must reacquire exact-head evidence.
5. Execute and repair issue #16 invocation-unique one-shot command identity RED before command backend integration.
6. Obtain dedicated positive LSM real-backend acceptance, then bind the final release candidate to complete coverage/rustdoc, security/SAST/review, package, SPDX SBOM, provenance, reproducibility and rollback evidence.
7. Publish the first immutable runtime release only from one exact integrated protected head; then hand off released version/digest pinning to consumer owner paths.
8. Continue issue #17 artifact-analysis profile only after higher-stack P0 security/release blockers are resolved.
