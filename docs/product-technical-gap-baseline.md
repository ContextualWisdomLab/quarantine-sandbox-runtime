# Product and Technical Gap Baseline

Last reviewed on 2026-09-05 KST against dependency-root PR #1 exact docs-only head `83115eda7c20c520d039192d253e4b4de0fe3b9d`, latest root test-bearing head `c43e5ca27acd96d085a4719fe5cb69de270aa723`, command-runtime RED head `585f3d955bddfb95f28e5918cfcadcac632589df`, and protected/default `develop@60a85c7633e03b425b67159ec6822c8178cf87ea`. This ledger distinguishes protected truth, active-PR implementation, checked-in RED evidence, backend-applied configuration evidence, live effective-runtime proof, queued/cancelled checks, and post-integration protected-head evidence. Predecessor evidence never transfers to a moved head.

## Product responsibility

Quarantine Sandbox Runtime owns reusable sandbox execution, isolation-policy enforcement, resource bounds, lease/readiness/cleanup attestation, command/artifact-analysis isolation, and artifact-analysis evidence. `sandbox_execution` is the Core bounded context; `artifact_analysis` and `application_service` are Supporting contexts. Podman/gVisor/containerd/Kubernetes/VM remain infrastructure adapters. Wardnet retains verdict/incident/quarantine authority. Agent/chat consumers retain task/tool/application authorization, identity, secrets, and user-action authority.

## DDD and repository structure

| Gap | Current evidence | Status | Required action |
| --- | --- | --- | --- |
| Artifact analysis had been flattened under root modules. | PR #1 keeps implementation under `src/artifact_analysis/` and repository fitness tests forbid obsolete root paths. | Corrected on active PR | Preserve through protected integration. |
| Podman infrastructure had leaked into Core and Supporting application-service types. | Current stack keeps Podman under `src/infrastructure/`, backend-neutral Core/application ports, and composition-boundary translation. | Corrected on active stack | Keep dependency direction and unique-ADR fitness tests GREEN. |
| Pre-publication ADR identities conflicted. | Canonical root line is `0001`–`0006`; command descendants add Proposed ADR-0007/0008. | Corrected on active stack | Promote only after protected integration and then-current runtime evidence. |
| Admission, session lifecycle, recovery, and network/egress still meet inside process-local application-service coordination. | Issue #8 defines the intended bounded-context extraction and forbids durable/distributed responsibilities from accumulating in generic application-service internals. | Known structural gap | Extract only with executable contracts and compatibility boundaries; no cosmetic folder churn. |
| Repository name remains security-biased relative to application-service isolation responsibility. | Product scope spans hostile artifact analysis and reusable application/command isolation. | Known naming gap | Re-evaluate before GA through repository-settings owner path with redirects/consumer migration. |

## Application-service isolation

| Capability | Current evidence | Status | Required action |
| --- | --- | --- | --- |
| Immutable application identity | Registry-style image references require SHA-256 digest identity; explicit host-backed/alternate-store transports are rejected and launch uses `--pull=never`. | Corrected on active PR | Keep import/admission separate from launch; no mutable or host-path consumer resolution. |
| Rootless backend | Exact root `24526eb55cf5db48ea07079b314f7d1b676eb48d` executed on GitHub-hosted Ubuntu 24.04 / Podman 4.9.3 and passed the explicit rootless/backend-version contract. | Real hosted evidence obtained | Re-prove every moved exact head; rootless capability does not prove per-sandbox LSM confinement. |
| Read-only / writable surfaces | Launch uses read-only rootfs, bounded `/tmp`, disabled host-proxy inheritance and restricted namespaces/surfaces. Unsupported Podman 4.9.3 `--no-hostname` is absent on the root lineage and current #14 command backend. | Configured intent plus partial runtime inspection | Bind exact applied tmpfs mount set/options/size, then require live mount proof. |
| Privilege / namespace isolation | Current repair checks empty effective/bounding/process capability sets, no-new-privileges, seccomp, user/PID/IPC namespaces, numeric non-root identity, and enforcing SELinux/AppArmor evidence. Exact hosted E2E `24526eb...` failed closed at `lsm`, proving ordinary hosted Ubuntu cannot supply positive effective confinement. | Negative effective-LSM evidence obtained; positive proof pending | Hosted Ubuntu stays an explicit negative LSM lane. Positive release evidence remains on the dedicated SELinux-capable runner. |
| Network isolation | Per-sandbox internal DNS-disabled network is inspected and publication must resolve only to loopback. Draft #23 exact `9b6ea9fe2e160c445ae7d9c8b3d7f89ef2870ddb` carries the stronger RED requiring positive exact container attachment and rejection of missing/additional attachments, with cleanup bound to exact launch identities. | Object-level evidence implemented; attachment proof pending | Execute #23 on stabilized current ancestry, then add exact attachment and real negative-egress proof. |
| Credential isolation | P0 request exposes no provider/user credentials, arbitrary environment map, host device/runtime socket, or broad host mount. | Active contract | Any future secret flow requires an explicit purpose-bound broker. |
| CPU/RAM/PID bounds | `HostConfig.Memory`, `NanoCpus`, and `PidsLimit` are inspected against the request. | Backend-applied binding only; live proof incomplete | Verify authoritative cgroup-v2 values for the exact sandbox before release claim. |
| tmpfs / wall time | Current root does not yet bind live `/tmp` mount enforcement or wall-time termination as effective proof. Draft #19 exact `03c76208c97f302551c9e40b71fcc8e559c04cc3` preserves hostile REDs for missing/wrong hardening, contradictory/duplicate tmpfs options, widened writable mounts, timeout mismatch, exact cleanup/non-publication, and exact inspect state without live proof. | P0 RED staged; production unchanged | After root becomes stable/protected, adopt ancestry non-force, execute the intended RED, add only the smallest applied-config GREEN, then live cgroup/mount/wall-time proof. |
| Process lifecycle | Application-service launch creates/starts/attests before readiness/publication and attempts complete cleanup after partial launch/attestation failure. | Active repair | Current exact heads must execute full tests; durable crash/restart orphan recovery belongs to Recovery context. |
| Subprocess spawn pressure | Exact root `24526eb...` observed `BackendInvocationFailed { operation: "rootless_probe" }`, but `BoundedCommandRunner` collapses spawn/capture failures so the observation did not prove `WouldBlock`. The generic retry previously introduced downstream was removed because it lacked focused RED/errno evidence. | Unsupported workaround removed; errno-level observability gap remains | If spawn failure recurs, first add focused error-kind preservation/injection RED; only then consider a causal retry policy. |
| Ownership/idempotency | Draft #6 exact `63ce9b1074bc3921c39eca26df53f7714e9cf392` preserves caller-scoped lease ownership/idempotency on current root ancestry without the unsupported spawn retry. | Implemented on descendant; exact checks pending | Preserve caller-scoped ownership separately from backend invocation identity and reacquire exact-child evidence. |
| Runtime invocation identity | Draft #21 exact `787d3b5e23d81886cb3d6794856b0f1763c2fe93` requires independent same-request/same-second invocations not to collide in names, labels, lease receipts, or cleanup targets. | P0 RED staged; production unchanged | Execute the RED on stabilized ancestry, then introduce one testable collision-resistant invocation identity below consumer correlation/idempotency semantics. |
| gVisor/containerd/Kubernetes | Architecture targets only. | Missing | Add independent adapters after P0 contract stabilizes; public contracts remain backend-neutral. |

## Command-execution isolation

| Capability | Current evidence | Status | Required action |
| --- | --- | --- | --- |
| Provider-neutral command contract | Draft #13 exact `cbfe4b2eb5f90120782348fb8e5000c2ba9c15a3` on #10 preserves ADR-0007, schemas, validation and typed workload-vs-runtime outcomes. | Implemented on Draft; exact CI queued | Keep ADR-0007 Proposed until protected truth and current evidence. |
| Podman command backend / CLI | Draft #14 current test-bearing head `585f3d955bddfb95f28e5918cfcadcac632589df` preserves `RootlessPodmanAdapter::run_command_at`, CLI, ADR-0008, exact-revision staging and command regressions on #13 current authority. | Draft implementation; exact CI queued | Execute the current RED-bearing exact head; do not transfer predecessor evidence. |
| Pre-attestation payload execution | Issue #25 and `tests/podman_command_execution_pre_attestation_red.rs` prove the critical ordering boundary: current production binds the consumer command as the OCI process, calls `podman start`, and only afterwards runs live `podman top`/effective-isolation verification. A fail-closed error after `start` cannot undo hostile payload execution that already occurred. | P0 RED checked in at `585f3d955bddfb95f28e5918cfcadcac632589df`; production deliberately unchanged | Execute the RED and require the payload-side-effect assertion to fail for the intended reason. Then add only a trusted hold/attest/release phase or an equivalent backend primitive that prevents consumer payload execution until positive effective isolation is established. Static-inspect reordering is not GREEN. |
| Live process attestation race | Existing regression rejects static-only security evidence when a short-lived command exits before `podman top` can sample it. That fail-closed behavior is necessary but does not close the earlier pre-attestation execution window. | Fail-closed regression exists; distinct P0 ordering gap remains #25 | Preserve the static-only rejection while implementing the stronger pre-execution boundary. |
| Output / timeout supervision | #14 carries `BoundedCommandRunner::run_to_completion`, per-stream truncation facts and repaired deterministic overflow fixtures. Historical production increments lacked a separate executed RED-before-GREEN sequence. | TDD debt retained explicitly | Require preserved regressions and complete exact-head GREEN before any protected merge. |
| Command sandbox identity | #14 carries invocation-unique `qsr-cmd-*` identity and Podman-4.9-compatible create argv without `--no-hostname`; the earlier dedicated RED attempt was cancelled before checkout. | Implementation exists; causal RED debt remains | Do not retroactively claim completed TDD; current exact full regression evidence is mandatory. |
| PR-source staging | #14 stages an exact revision into a runtime-owned read-only `noexec,nosuid,nodev` tree before sandbox execution. | Draft implementation | Revalidate on exact head and retain source provenance in command result/evidence. |

## Attestation evidence model

Configured launch intent, backend-applied inspection, live effective enforcement, and pre-execution authorization are different evidence levels. A security control may advance only to the level actually proved for the exact sandbox identity.

- Podman inspect can bind requested configuration but cannot by itself prove kernel cgroup or mount enforcement or that wall-time termination actually occurred.
- A separately inspectable internal network does not prove the running container is attached only to that network.
- CPU/RAM/PID effective claims require authoritative cgroup-v2 evidence where the backend exposes it.
- `/tmp` effective claims require live mount evidence showing tmpfs, exact mount point, required restrictions, and bounded size.
- Wall-time effective claims require behavioral/runtime-owned termination and cleanup evidence or an equivalent reviewed watchdog.
- Host AppArmor/SELinux availability is not per-sandbox confinement. Ordinary hosted CI is a negative fail-closed lane; positive LSM evidence remains a separate release gate.
- For command execution, positive evidence gathered after the consumer payload has already become runnable is not proof of a pre-execution security boundary. The runtime needs a trusted hold/attest/release phase or equivalent backend semantics before it can claim that hostile payload execution starts only after effective isolation is established.

## Artifact analysis

| Capability | Current evidence | Status | Required action |
| --- | --- | --- | --- |
| Static foundation | SHA-256 identity, bounded ingestion, format detection, analyzer interface, deterministic evidence, and failure attribution exist on active foundation. | Active PR | Preserve under `artifact_analysis`; complete exact-head coverage/review. |
| Claude plugin package quarantine profile | Issue #17 defines immutable source/artifact/AppGuardrail identity, fixed probes, credential-free execution, bounded resources, deny-by-default network, cleanup/recovery, and evidence-only receipts. Draft #18 exact `a658a637720b63915078332ea52ef26021788e1c` remains RED-only on #14 ancestry. | RED staged, not production truth | Do not overtake higher-stack P0 blockers; execute RED only on current/stable ancestry before implementation. |
| YARA-X / capa / Ghidra / LIEF | No production adapters yet. | Missing | Add one evidence producer per bounded increment with exact version/digest provenance. |
| Dynamic Linux/Windows detonation | No release-grade detonation worker exists. | Missing | Consume sandbox Core/stronger backend; never execute hostile bytes in control process. |
| Durable evidence/signing | In-memory evidence only. | Missing | Add persistence only after owner/retention/recovery/3NF/idempotency ADR. |

## Standards and doctoring traceability

The command pre-attestation finding is grounded in primary runtime documentation and the existing container-security baseline: Podman `create` prepares a container for a specified command without starting it; Podman `start` starts the created container. NIST SP 800-190 treats the runtime as part of the security boundary responsible for container isolation/resource controls. Issue #25 records the causal mapping to `run_command_at` and the new RED. Formal APA 7th entry: Souppaya, M. P., Morello, J., & Scarfone, K. (2017). *Application container security guide* (NIST Special Publication 800-190). National Institute of Standards and Technology. https://doi.org/10.6028/NIST.SP.800-190

## Verification and release state

- Protected/default `develop` remains protected at `60a85c7633e03b425b67159ec6822c8178cf87ea` in the latest fresh read.
- Root PR #1 exact docs-only head is `83115eda7c20c520d039192d253e4b4de0fe3b9d`; latest test-bearing `c43e5ca27acd96d085a4719fe5cb69de270aa723` carries issue #24's RED requiring native CI push coverage for actual protected/default `develop` while `.github/workflows/ci.yml` intentionally remains `push.branches: [main]` until the RED executes.
- Root exact `83115eda...` has CI `33905792562`, Security Scan `33905792507`, SAST `33905792656`, and CodeQL `33905792587`, all still queued at the fresh read. CI hosted jobs `101130380106`, `101130380122`, `101130380124`, and `101130380157` are `ubuntu-24.04`, `runner_id=0`, `steps=[]`; positive-LSM `101130379895` is likewise queued on `[self-hosted, linux, cwl-hostile-workload, selinux]`. Queue state is incomplete evidence, not RED/GREEN proof.
- The live dependency chain is currently #6 `63ce9b1074bc3921c39eca26df53f7714e9cf392` -> #9 `bbb5efa95640a950dd9a53264034ea51fec00d41` -> #10 `65d02ec10a29c60499fb8fca6e1d0a5db63f145b` -> #13 `cbfe4b2eb5f90120782348fb8e5000c2ba9c15a3` -> #14 test-bearing `585f3d955bddfb95f28e5918cfcadcac632589df`. Each edge was live-read before this update; #14 was Draft/mergeable on current #13.
- #14 test-bearing exact `585f3d...` materialized CI `33909801395`; hosted `coverage=101143291758`, `branch-coverage=101143292017`, `verify=101143292131`, and `negative-rootless-AppArmor=101143292144` are queued before checkout with `runner_id=0`, `steps=[]`. Dedicated `positive-LSM=101143292085` is separately queued on `[self-hosted, linux, cwl-hostile-workload, selinux]`. Therefore the new #25 RED has not executed and no production GREEN is authorized.
- Issue #24 remains distinct from runner acquisition: `.github#712` owns runs/jobs that materialize but cannot obtain execution capacity, while a stale `main` push filter can prevent a protected-`develop` integration run from being created at all.
- Current RED-only/security descendants remain #18 `a658a637720b63915078332ea52ef26021788e1c`, #19 `03c76208c97f302551c9e40b71fcc8e559c04cc3`, #21 `787d3b5e23d81886cb3d6794856b0f1763c2fe93`, and #23 `9b6ea9fe2e160c445ae7d9c8b3d7f89ef2870ddb`. Preserve their valid deltas; do not destructive-restack non-GREEN ancestry merely to chase SHA movement.
- Root formal reviews are COMMENTED only at the latest fresh read; there is no qualifying approval.
- GitHub Releases remains empty at the latest verified state; no immutable runtime release exists for Wardnet, contextual-orchestrator, Noema, or other consumers to pin.

## Protected integration and release authority

PR-head GREEN is necessary but not sufficient for release authority. A normal merge can produce a different protected integration SHA, so the first release requires native CI/security/runtime evidence on the exact protected `develop` head created by integration. Issue #24 treats a missing post-integration CI run as a release-evidence failure, not as equivalent to a successful PR check.

Release-evidence Draft #10 separately retains the protected-source RED: release preflight must resolve repository authority and prove the release source is the protected/default branch rather than hard-code stale `main` assumptions. Native CI trigger repair and release-source validation are related but distinct gates.

## Consumer and release contract

Wardnet remains SOC/gateway/verdict authority; contextual-orchestrator remains LLM/Agent orchestration authority; Noema remains capability/admission authority. Consumers may use only a future immutable released runtime artifact and versioned contract/ACL. Direct Podman/containerd calls, sibling source imports, mutable PR heads, and cross-service SQL are not integration mechanisms.

The first release remains blocked until one exact integrated protected candidate carries complete owned statement/branch/edge coverage and public rustdoc, realistic rootless isolation E2E, positive effective LSM/seccomp/capability/resource/network/cleanup evidence, required review/security/SAST gates, package smoke, SPDX SBOM, provenance, checksum/signature as supported, reproducibility, upgrade/rollback evidence, and an immutable artifact identity. Command execution additionally needs pre-execution proof that consumer payload code cannot run before effective isolation attestation.

## Next bounded slices

1. Let the new #25 RED execute on exact test-bearing `585f3d...`; it must fail specifically because `podman start` makes the consumer sentinel runnable before live `top` attestation. Only then implement the smallest two-phase hold/attest/release or equivalent causal GREEN and reacquire exact-head evidence.
2. Execute root current exact head. Issue #24 must first fail for stale `push.branches: [main]`; only then change native CI push coverage to `develop` and reacquire exact-head GREEN.
3. Reacquire exact-head evidence for the current #6 -> #9 -> #10 -> #13 -> #14 chain; no predecessor evidence transfers.
4. If `rootless_probe` spawn failure recurs, preserve/inject concrete `io::ErrorKind` in a focused RED before any retry behavior.
5. After root stabilizes, non-force adopt it into Draft #19 and execute the resource-attestation RED; follow with the smallest inspect-binding GREEN, then live cgroup/mount/wall-time proof.
6. Execute Draft #23's effective network-binding RED on stabilized ancestry; after reproduced failure, require positive exact attachment to the runtime-owned deny-by-default network and real negative-egress proof.
7. Execute Draft #21's runtime-identity collision RED on stabilized ancestry, then introduce one testable collision-resistant invocation identity below consumer correlation/idempotency semantics.
8. Reconcile #18 only after higher-stack P0 blockers and command ancestry are stable; keep artifact-analysis receipts as risk evidence, not admission/verdict authority.
9. Publish the first immutable runtime release only from one exact integrated protected head, then hand off released version/digest pinning to consumer owner paths.
