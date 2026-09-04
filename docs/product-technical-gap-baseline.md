# Product and Technical Gap Baseline

Last reviewed on 2026-09-05 KST against dependency-root PR #1 current docs-only head `28f355337390089190067e5f8a7f0eeba35f81e3`, latest root test-bearing head `c43e5ca27acd96d085a4719fe5cb69de270aa723`, reconciled command-stack head `2aed65cf8479c85517e47dad709d0f44f7bfb36a`, and protected/default `develop@60a85c7633e03b425b67159ec6822c8178cf87ea`. This ledger distinguishes protected truth, active-PR implementation, checked-in RED evidence, backend-applied configuration evidence, live effective-runtime proof, queued/cancelled checks, and post-integration protected-head evidence. Predecessor evidence never transfers to a moved head.

## Product responsibility

Quarantine Sandbox Runtime owns reusable sandbox execution, isolation-policy enforcement, resource bounds, lease/readiness/cleanup attestation, command/artifact-analysis isolation, and artifact-analysis evidence. `sandbox_execution` is the Core bounded context; `artifact_analysis` and `application_service` are Supporting contexts. Podman/gVisor/containerd/Kubernetes/VM remain infrastructure adapters. Wardnet retains verdict/incident/quarantine authority. Agent/chat consumers retain task/tool/application authorization, identity, secrets, and user-action authority.

## DDD and repository structure

| Gap | Current evidence | Status | Required action |
| --- | --- | --- | --- |
| Artifact analysis had been flattened under root modules. | PR #1 keeps implementation under `src/artifact_analysis/` and repository fitness tests forbid obsolete root paths. | Corrected on active PR | Preserve through protected integration. |
| Podman infrastructure had leaked into Core and Supporting application-service types. | Root/current stack keeps Podman under `src/infrastructure/`, backend-neutral Core/application ports, and composition-boundary translation. | Corrected on active stack | Keep dependency direction and unique-ADR fitness tests GREEN. |
| Pre-publication ADR identities conflicted. | Canonical root line is `0001`–`0006`; command descendants add Proposed ADR-0007/0008. | Corrected on active stack | Promote only after protected integration and then-current runtime evidence. |
| Admission, session lifecycle, recovery, and network/egress still meet inside process-local application-service coordination. | Issue #8 defines the intended bounded-context extraction and forbids durable/distributed responsibilities from accumulating in generic application-service internals. | Known structural gap | Extract only with executable contracts and compatibility boundaries; no cosmetic folder churn. |
| Repository name remains security-biased relative to application-service isolation responsibility. | Product scope spans hostile artifact analysis and reusable application/command isolation. | Known naming gap | Re-evaluate before GA through repository-settings owner path with redirects/consumer migration. |

## Application-service isolation

| Capability | Current evidence | Status | Required action |
| --- | --- | --- | --- |
| Immutable application identity | Registry-style image references require SHA-256 digest identity; explicit host-backed/alternate-store transports are rejected and launch uses `--pull=never`. | Corrected on active PR | Keep import/admission separate from launch; no mutable or host-path consumer resolution. |
| Rootless backend | Exact root `24526eb55cf5db48ea07079b314f7d1b676eb48d` executed on GitHub-hosted Ubuntu 24.04 / Podman 4.9.3 and passed the explicit rootless/backend-version contract. | Real hosted evidence obtained | Re-prove every moved exact head; rootless capability does not prove per-sandbox LSM confinement. |
| Read-only / writable surfaces | Launch uses read-only rootfs, bounded `/tmp`, disabled host-proxy inheritance and restricted namespaces/surfaces. Unsupported Podman 4.9.3 `--no-hostname` is absent on the root lineage and reconciled #14 command backend. | Configured intent plus partial runtime inspection | Bind exact applied tmpfs mount set/options/size, then require live mount proof. |
| Privilege / namespace isolation | Current repair checks empty effective/bounding/process capability sets, no-new-privileges, seccomp, user/PID/IPC namespaces, numeric non-root identity, and enforcing SELinux/AppArmor evidence. Exact hosted E2E `24526eb...` failed closed at `lsm`, proving ordinary hosted Ubuntu cannot supply positive effective confinement. | Negative effective-LSM evidence obtained; positive proof pending | Hosted Ubuntu stays an explicit negative LSM lane. Positive release evidence remains on the dedicated SELinux-capable runner. |
| Network isolation | Per-sandbox internal DNS-disabled network is inspected and publication must resolve only to loopback. Draft #23 exact/test-bearing `dd6f4efcc06e9cc988e1ed640c95274547e6cb48` carries a stronger RED requiring positive exact container attachment and rejection of missing/additional attachments, with cleanup bound to exact launch identities. | Object-level evidence implemented; attachment proof pending | Execute #23 on stabilized current ancestry, then add exact attachment and real negative-egress proof. |
| Credential isolation | P0 request exposes no provider/user credentials, arbitrary environment map, host device/runtime socket, or broad host mount. | Active contract | Any future secret flow requires an explicit purpose-bound broker. |
| CPU/RAM/PID bounds | `HostConfig.Memory`, `NanoCpus`, and `PidsLimit` are inspected against the request. | Backend-applied binding only; live proof incomplete | Verify authoritative cgroup-v2 values for the exact sandbox before release claim. |
| tmpfs / wall time | Current root does not yet bind live `/tmp` mount enforcement or wall-time termination as effective proof. Draft #19 exact/test-bearing `a7a05159a2087243956d615a606472fc6eadd91b` preserves hostile REDs for missing/wrong hardening, contradictory/duplicate tmpfs options, widened writable mounts, timeout mismatch, exact cleanup/non-publication, and exact inspect state without live proof. | P0 RED preserved on older root ancestry; no production GREEN | After root becomes stable/protected, adopt ancestry non-force, execute the intended RED, add only the smallest applied-config GREEN, then live cgroup/mount/wall-time proof. |
| Process lifecycle | Adapter invokes Podman without a shell, creates/starts/attests before publication and attempts complete cleanup after partial launch/attestation failure. | Active repair | Current exact heads must execute full tests; durable crash/restart orphan recovery belongs to Recovery context. |
| Subprocess spawn pressure | Exact root `24526eb...` observed `BackendInvocationFailed { operation: "rootless_probe" }`, but `BoundedCommandRunner` collapses spawn/capture failures so the observation did not prove `WouldBlock`. The generic retry previously introduced downstream was removed because it lacked focused RED/errno evidence. | Unsupported workaround removed; errno-level observability gap remains | If spawn failure recurs, first add focused error-kind preservation/injection RED; only then consider a causal retry policy. |
| Ownership/idempotency | Draft #6 is now non-force reconciled at `7e9b76251f274bbbec7b2226bdc6a2babae108cb` on current root `28f355...`; caller-scoped lease ownership/idempotency is preserved without the unsupported spawn retry. | Implemented on reconciled descendant; exact checks queued | Preserve caller-scoped ownership separately from backend invocation identity and reacquire exact-child evidence. |
| Runtime invocation identity | Draft #21 exact `3babf075d3291faa38e0ae7ba33514e2cea058e6`, latest test-bearing `f66a295c21934b462342c2cc892a646d252b1638`, requires independent same-request/same-second invocations not to collide in names, labels, lease receipts, or cleanup targets. | P0 RED staged; production unchanged | Execute the RED on stabilized ancestry, then introduce one testable collision-resistant invocation identity below consumer correlation/idempotency semantics. |
| gVisor/containerd/Kubernetes | Architecture targets only. | Missing | Add independent adapters after P0 contract stabilizes; public contracts remain backend-neutral. |

## Command-execution isolation

| Capability | Current evidence | Status | Required action |
| --- | --- | --- | --- |
| Provider-neutral command contract | Draft #13 is reconciled at `592408c4a9a3e1340a58503b9d6e0a8eebf02247` on #10 and preserves ADR-0007, schemas, validation and typed workload-vs-runtime outcomes. | Implemented on Draft; exact CI queued | Keep ADR-0007 Proposed until protected truth and current evidence. |
| Podman command backend / CLI | Draft #14 exact head `2aed65cf8479c85517e47dad709d0f44f7bfb36a`, source/restack head `3e511d8585809bd4fb6bff5508e2d32b1b69e0f3`, preserves `RootlessPodmanAdapter::run_command_at`, CLI, ADR-0008, exact-revision staging and command regressions on current parent authority. | Non-force stack repair complete; exact CI queued | Execute the reconciled exact head; do not treat historical local/predecessor evidence as release proof. |
| Output / timeout supervision | #14 carries `BoundedCommandRunner::run_to_completion`, per-stream truncation facts and the repaired deterministic overflow fixture. Historical production increments lacked a separate executed RED-before-GREEN sequence. | TDD debt retained explicitly | Require preserved regressions and complete exact-head GREEN before any protected merge. |
| Command sandbox identity | #14 carries invocation-unique `qsr-cmd-*` identity and Podman-4.9-compatible create argv without `--no-hostname`; the earlier dedicated RED attempt was cancelled before checkout. | Implementation exists; causal RED debt remains | Do not retroactively claim completed TDD; current exact full regression evidence is mandatory. |
| PR-source staging | #14 stages an exact revision into a runtime-owned read-only `noexec,nosuid,nodev` tree before sandbox execution. | Draft implementation | Revalidate on exact head and retain source provenance in command result/evidence. |

## Attestation evidence model

Configured launch intent, backend-applied inspection, and live effective enforcement are different evidence levels. A security control may advance only to the level actually proved for the exact sandbox identity.

- Podman inspect can bind requested configuration but cannot by itself prove kernel cgroup or mount enforcement or that wall-time termination actually occurred.
- A separately inspectable internal network does not prove the running container is attached only to that network.
- CPU/RAM/PID effective claims require authoritative cgroup-v2 evidence where the backend exposes it.
- `/tmp` effective claims require live mount evidence showing tmpfs, exact mount point, required restrictions, and bounded size.
- Wall-time effective claims require behavioral/runtime-owned termination and cleanup evidence or an equivalent reviewed watchdog.
- Host AppArmor/SELinux availability is not per-sandbox confinement. Ordinary hosted CI is a negative fail-closed lane; positive LSM evidence remains a separate release gate.

## Artifact analysis

| Capability | Current evidence | Status | Required action |
| --- | --- | --- | --- |
| Static foundation | SHA-256 identity, bounded ingestion, format detection, analyzer interface, deterministic evidence, and failure attribution exist on active foundation. | Active PR | Preserve under `artifact_analysis`; complete exact-head coverage/review. |
| Claude plugin package quarantine profile | Issue #17 defines immutable source/artifact/AppGuardrail identity, fixed probes, credential-free execution, bounded resources, deny-by-default network, cleanup/recovery, and evidence-only receipts. Draft #18 latest test-bearing `ccc19dc1826f21236d63652594442a0f17e43313` non-force adopts current `#14@2aed65cf8479c85517e47dad709d0f44f7bfb36a`, closes the prior merge-conflict/base drift, and strengthens the RED with raw duplicate-JSON-member rejection, marketplace-blob identity validation, non-empty required metadata, and canonical Windows/UNC/relative-path rejection. Production remains unchanged. | RED reconciled; exact execution still queued | Execute this test-bearing head and require the positive profile case to fail for the missing public contract before any production GREEN. RFC 7493/RFC 8259 wire constraints are recorded in doctoring traceability. |
| YARA-X / capa / Ghidra / LIEF | No production adapters yet. | Missing | Add one evidence producer per bounded increment with exact version/digest provenance. |
| Dynamic Linux/Windows detonation | No release-grade detonation worker exists. | Missing | Consume sandbox Core/stronger backend; never execute hostile bytes in control process. |
| Durable evidence/signing | In-memory evidence only. | Missing | Add persistence only after owner/retention/recovery/3NF/idempotency ADR. |

## Verification and release state

- Protected/default `develop` remains protected at `60a85c7633e03b425b67159ec6822c8178cf87ea` in the latest fresh read.
- Root PR #1 current docs-only head is `28f355337390089190067e5f8a7f0eeba35f81e3`; latest test-bearing `c43e5ca27acd96d085a4719fe5cb69de270aa723` carries issue #24's RED requiring native CI push coverage for actual protected/default `develop` while `.github/workflows/ci.yml` intentionally remains `push.branches: [main]` until the RED executes.
- Root exact `28f355...` currently has CI `33899175609`, Security Scan `33899175719`, SAST `33899175661`, and CodeQL `33899175676`, all queued at the last fresh read. Queue state is incomplete evidence, not RED/GREEN proof.
- The main dependency chain is non-force reconciled and mergeable at each edge: #6 `7e9b76251f274bbbec7b2226bdc6a2babae108cb` -> #9 `64af991ef0f0d9c2616b59c647161fa3971ba68c` -> #10 `e23522c44fef4b42bb96ee7eb8e32e7c5ed2d09d` -> #13 `592408c4a9a3e1340a58503b9d6e0a8eebf02247` -> #14 `2aed65cf8479c85517e47dad709d0f44f7bfb36a`.
- #18 test-bearing `ccc19dc...` now adopts #14 with a two-parent non-force merge and preserves only the artifact-analysis RED/doctoring delta above current command-stack authority. CI `33904501126` materialized five jobs; all are queued before checkout with no assigned runner/steps. Hosted jobs use `ubuntu-24.04`; positive LSM job `101126118599` requires `[self-hosted, linux, cwl-hostile-workload, selinux]`. This is non-passing exact-head evidence.
- Issue #24 remains distinct from runner acquisition: `.github#712` owns runs/jobs that materialize but cannot obtain execution capacity, while a stale `main` push filter can prevent a protected-`develop` integration run from being created at all.
- RED-only/security descendants #19, #21, and #23 preserve valid deltas on intentionally older non-GREEN ancestry and must adopt stabilized/protected ancestry non-force before final execution/merge evidence. #18 no longer has the old #14 base conflict, but its RED is still unexecuted and no predecessor evidence transfers.
- Organization ruleset `18156473` remains active on the default branch: one approval, stale-review dismissal, review-thread resolution, seven central required workflows, deletion prohibition and non-fast-forward prohibition. Administrator bypass exists but is not merge evidence.
- GitHub Releases remains empty. No immutable runtime release exists for Wardnet, contextual-orchestrator, Noema, or other consumers to pin.

## Protected integration and release authority

PR-head GREEN is necessary but not sufficient for release authority. A normal merge can produce a different protected integration SHA, so the first release requires native CI/security/runtime evidence on the exact protected `develop` head created by integration. Issue #24 treats a missing post-integration CI run as a release-evidence failure, not as equivalent to a successful PR check.

Release-evidence Draft #10 separately retains the protected-source RED: release preflight must resolve repository authority and prove the release source is the protected/default branch rather than hard-code stale `main` assumptions. Native CI trigger repair and release-source validation are related but distinct gates.

## Consumer and release contract

Wardnet remains SOC/gateway/verdict authority; contextual-orchestrator remains LLM/Agent orchestration authority; Noema remains capability/admission authority. Consumers may use only a future immutable released runtime artifact and versioned contract/ACL. Direct Podman/containerd calls, sibling source imports, mutable PR heads, and cross-service SQL are not integration mechanisms.

The first release remains blocked until one exact integrated protected candidate carries complete owned statement/branch/edge coverage and public rustdoc, realistic rootless isolation E2E, positive effective LSM/seccomp/capability/resource/network/cleanup evidence, required review/security/SAST gates, package smoke, SPDX SBOM, provenance, checksum/signature as supported, reproducibility, upgrade/rollback evidence, and an immutable artifact identity.

## Next bounded slices

1. Execute root current exact head. Issue #24 must first fail for stale `push.branches: [main]`; only then change native CI push coverage to `develop` and reacquire exact-head GREEN.
2. Reacquire exact-head evidence for the non-force-reconciled #6 -> #9 -> #10 -> #13 -> #14 chain; no predecessor evidence transfers.
3. If `rootless_probe` spawn failure recurs, preserve/inject concrete `io::ErrorKind` in a focused RED before any retry behavior.
4. After root stabilizes, non-force adopt it into Draft #19 and execute the preserved resource-attestation RED; follow with the smallest inspect-binding GREEN, then live cgroup/mount/wall-time proof.
5. Execute Draft #23's effective network-binding RED on stabilized ancestry; after reproduced failure, require positive exact attachment to the runtime-owned deny-by-default network and real negative-egress proof.
6. Execute Draft #21's runtime-identity collision RED on stabilized ancestry, then introduce one testable collision-resistant invocation identity below consumer correlation/idempotency semantics.
7. Execute #18's reconciled artifact-analysis RED when runner capacity is available; do not implement the profile contract until the positive case actually fails for the current missing contract, and keep receipt output as risk evidence rather than admission/verdict authority.
8. Publish the first immutable runtime release only from one exact integrated protected head, then hand off released version/digest pinning to consumer owner paths.
