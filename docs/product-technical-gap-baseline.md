# Product and Technical Gap Baseline

Last reviewed on 2026-09-09 KST against root Draft PR #1 exact `5c6a44bb2b35eb17d0315d72db242f4488c3c426`, application-readiness Draft #104 lineage through protocol/coverage repair `236d1a67eb90f6d7c10c4714dd2d1d55faee72c6`, protected/default `develop@60a85c7633e03b425b67159ec6822c8178cf87ea`, and the live open PR/Issue/security state. This ledger separates protected truth, active-PR implementation, checked-in RED, causal RED, candidate GREEN, real backend evidence, central required-workflow evidence, and release authority. Evidence from a predecessor SHA never transfers after implementation or dependency movement.

## Product responsibility and DDD authority

Quarantine Sandbox Runtime owns reusable hostile-workload/application-service isolation, resource/lifecycle bounds, lease/readiness/cleanup attestation, and artifact-analysis execution evidence.

- `sandbox_execution` is the Core bounded context and owns reusable sandbox/worker isolation, resource, lifecycle, termination, and cleanup vocabulary.
- `artifact_analysis` is a Supporting context and owns artifact identity/admission, analyzer orchestration, evidence normalization/completeness, and analyzer-result ACL semantics.
- `application_service` is a Supporting context and owns consumer-neutral service intent plus translation to/from Core sandbox execution.
- `infrastructure` owns concrete Podman/process/backend translation and observation. Podman/gVisor/containerd/Kubernetes/VM types do not become consumer domain contracts.
- Wardnet retains verdict/incident/response authority. contextual-orchestrator retains LLM/Agent/tool authorization. Noema retains Agent/runtime capability authority. Keyverse retains identity authority. EgressWeave retains outbound-policy authority.
- Consumer integration is through an immutable released contract/artifact. Sibling source imports, mutable PR-head dependencies, direct foreign runtime calls, and cross-service SQL are not integration mechanisms.

The runtime currently owns no durable database. Any future durable job/evidence/reaper store requires an explicit persistence ADR, 3NF schema, descriptive multiword `snake_case` objects, retention/recovery semantics, and migration/rollback evidence.

## Current root and repository-policy authority

| Gap / gate | Current evidence | Status | Required action |
| --- | --- | --- | --- |
| Checkout credential persistence | Root ancestry retains the causal GREEN proving every checkout uses `persist-credentials: false`; current root exact is `5c6a44bb2b35eb17d0315d72db242f4488c3c426`. | Integrated into current Draft root | Do not regress; predecessor GREEN does not replace current-head gates. |
| Root owned-production coverage / canonical source-region admission | Issue #93 / PR #94 was normally integrated into root. Current root native CI `34176680115` has verify `101907363084`, coverage `101907363224`, branch coverage `101907363185`, and hosted negative rootless/AppArmor `101907363162` GREEN. The checker preserves raw LLVM diagnostics while gating canonical owned-production source coordinates. | Exact current-root hosted GREEN | Preserve 100% line/function/canonical-source-region/branch admission and raw LLVM traceability. Positive-LSM and central security gates remain independent. |
| Dedicated positive effective-LSM evidence | Current root positive-LSM job `101907363137` remains queued on `[self-hosted, linux, cwl-hostile-workload, selinux]`. Hosted negative rootless/AppArmor is GREEN but is not positive confinement evidence. | Independent release/security lane pending dedicated runner | `.github#1590` remains canonical owner. Do not substitute hosted negative evidence or weaken the LSM gate. |
| Repeated backend spawn/invocation failure | Earlier root/process lanes and #92 reproduced generic `BackendInvocationFailed { operation: "rootless_probe" }` before their intended backend failures. Issue #71 / Draft #72 causally executed missing-executable/provider-neutral REDs and now carries exact `900d0273625115f7bcc602d888baadded0caeb4f`; native CI `34231220456` has verify `102077568988`, coverage `102077568604`, branch `102077568933`, hosted negative `102077569052` GREEN, while positive-LSM `102077569145` remains queued and no qualifying approval exists. #104 ordinary coverage at `59cc738...` independently reproduced the same `rootless_probe` class in `root_coverage_edges`. | Canonical typed-error repair hosted GREEN; independent positive-LSM/review gates still open | Preserve #72 as owner. Do not add retry, mutex, environment workaround, or weakened expectations in consumer children. Integrate non-force only after its remaining gates. |
| Repository workflow SHA-pin validation | Issue #86 / Draft #92 owns external full-SHA versus same-repository `$/...` policy. Current exact `30c5de5c73d729a1695609325d867a456eaf2dda`; policy validator tests, coverage-parser tests, formatting, complete production coverage, branch coverage and hosted negative are GREEN. Broader verify fails only in inherited `rootless_probe` process/backend invocation before the intended cleanup failure. | Policy-specific candidate GREEN; inherited #71/#72 runtime prerequisite blocks full verify | Do not mutate the validator or use retry. Adopt the canonical runtime prerequisite non-force, then rerun exact full gates. |
| Central CodeQL terminal verdict | Current root CodeQL `34176680113` dispatched exact-head analysis but python `101907470730` and actions `101907470755` failed `Release runner or enforce current-head CodeQL verdict`. | Central required-workflow failure | `.github#1929` remains canonical owner. Dispatch is not a terminal CodeQL verdict. |
| Dependency Review availability | Current root Security Scan `34176680132` verified exact head; OSV, Scorecard and Trivy succeeded, while dependency-review `101907475098` failed `Check dependency review support` and actual Dependency Review was skipped. | Central required-workflow failure | `.github#810` owns availability. Do not substitute sibling scanners. |

GitHub's immutable-action guidance requires full-length commit SHA pinning for external actions, while its `$/` self-repository syntax is already bound to the exact running workflow commit and must not carry an `@ref` suffix. Organization/reusable CI/review/security/release policy remains canonical in `ContextualWisdomLab/.github`; repository validation is defense in depth and must preserve that distinction without duplicating central workflow implementation.

## Application-service isolation and lifecycle

| Capability | Current evidence | Status | Required action |
| --- | --- | --- | --- |
| Immutable application identity | Public request requires lower-case SHA-256 digest identity and launch uses `--pull=never`; mutable tags/alternate host-backed transports are rejected on root lineage. | Implemented contract; exact protected integration still required | Keep registry/import/admission outside launch and bind effective image identity before release. |
| Rootless / read-only / capability / namespace / seccomp / LSM boundary | Root lineage requests restrictive Podman controls and performs effective checks. Current root hosted lanes are GREEN; positive acceptance remains separate on the dedicated SELinux runner. | Hosted negative and unit/process evidence GREEN; positive confinement pending | Never convert unavailable/unconfined evidence to Verified. Preserve all required controls and obtain real positive effective-LSM evidence. |
| Protocol-aware HTTP readiness / security-consumer bootstrap | Consumer `contextual-orchestrator#1094` run `34238690594` / job `102107934422` failed before Strix model review because Caido never became usable at `127.0.0.1:48080`. Issue #103 / Draft #104 exact RED `65663052ec30bc178adbe5ff4514f5409d10971f` proved that TCP acceptance alone could issue an HTTP lease. Candidate `c05d378cfc736e4257594d69bb06871893ef2d0f` added a bounded loopback HTTP/1.1 `/` probe requiring 2xx while retaining TCP connect semantics. Exact `59cc738f1428d78eaf7a7999e65cc247307b990d` corrected inherited plain-TCP process fixtures and made verify + hosted negative GREEN; its branch coverage then isolated one direct owned helper gap (`1989/1990` lines, `2664/2665` canonical regions, `462/466` branches). Commit `236d1a67eb90f6d7c10c4714dd2d1d55faee72c6` composes bounded socket setup/write/read errors through one fail-closed `io::Result` chain without changing the wire contract or accepting caller URLs. | Causal RED → behavioral candidate GREEN in full verify → exact coverage repair awaiting fresh final-head evidence | Reacquire final-head verify + 100% coverage/branch + hosted negative; keep application login/auth consumer-owned. Positive-LSM, qualifying review and central release gates remain independent. Release immutable runtime before consumer version/digest bump. |
| Missing Podman security fields | Issue #73 / Draft #89 causally proved that missing `EffectiveCaps`, `BoundingCaps`, or `dns_enabled` was defaulted to apparently secure values. Minimum production `80e6fd19ab5ef11eab3d18b3e80b43f880ce6058` removes only those defaults; the child remains separate. | Causal RED + minimum candidate, not protected-root integrated | Restack/adopt non-force when prerequisites permit; explicit empty/false values remain observed values, while field absence is malformed evidence. |
| Podman option/data boundary | Issue #90 / Draft #91 exact `1b1026a92b7d9ef23a709564c1e22aa0421179ff` causally proved missing option termination; production delta is one literal `--` before `request.image_reference`. Exact verify, complete production coverage, branch coverage and hosted negative are GREEN; positive-LSM and qualifying approval remain open. | Hosted exact candidate GREEN; release gates open | Preserve exact argv data and do not shell-join/sanitize. Merge only after positive confinement/review/security gates. |
| Public lease deserialization | Issue #84 / Draft #85 exact `e0040d748a8076e17f5103a8a238d2fbd878efbf` preserves private closed wire DTO + validated reconstruction. Verify, complete coverage, branch coverage and hosted negative are GREEN; positive-LSM and qualifying approval remain open. | Hosted exact candidate GREEN; release gates open | Keep strict admission and validated domain construction; do not promote mutable head to consumer authority. |
| Public cleanup-receipt deserialization | Issue #87 / Draft #88 exact `08fed96e0193d80f380aed07f6bcf4b2c29ca397` has verify, complete coverage, branch coverage and hosted negative GREEN. Stale review findings based on obsolete root ancestry were answered and resolved; positive-LSM and qualifying approval remain open. | Hosted exact candidate GREEN; release gates open | Preserve serialization/admission boundary; deserialized evidence never becomes destructive authority. |
| Effective network binding | Draft #23 retains exact network-attachment, acquired-network-identity, foreign-safe cleanup, and negative-egress REDs. | P0 REDs staged; production truth incomplete | Require exact acquired network identity, exclusive deny-by-default attachment, non-force foreign-safe cleanup, and real negative-egress evidence. |
| CPU/RAM/PID/tmpfs/wall time | Applied Podman configuration exists, but inspect/configuration is not equivalent to authoritative cgroup/mount/termination evidence. Draft #19 owns resource/namespace/image/argv applied-state REDs. | Backend-applied intent; live proof incomplete | Bind inspect state only after causal RED, then verify live cgroup-v2, tmpfs mount restrictions/size, and runtime-owned wall-time termination. |
| Runtime/lifecycle ownership | Draft #21 retains independent invocation collision, exact acquired container ID, malformed create-ID, and destructive-authority REDs. | REDs staged | Preserve consumer `request_id` as correlation; use collision-resistant invocation identity and exact acquired backend IDs for lifecycle authority. |
| Pre-attestation command execution | Command path can start hostile payload before positive effective isolation is established; issue #25 remains the hold/attest/release boundary owner. | Security gap; RED staged downstream | Require a trusted hold/attest/release primitive or equivalent. Cleanup after execution does not undo pre-attestation code execution. |
| Crash/restart orphan recovery | Current leases/receipts are process-local and GA requires durable reclamation after runtime crash. | Missing GA capability | Add Recovery context/reaper only with explicit persistence/retention/idempotency/recovery ADR and tests. |

## Artifact-analysis execution and evidence

| Capability | Current evidence | Status | Required action |
| --- | --- | --- | --- |
| Static foundation | Bounded ingestion, SHA-256 artifact identity, non-executing format detection, deterministic evidence ordering, analyzer interface, and failure attribution exist on active foundation #18. | Active foundation, unmerged | Preserve `artifact_analysis` ownership and exact-head verification. |
| Analyzer worker Core ownership | Issue #69 / Draft #70 executed a Core-boundary E0432 RED, then a seven-required-control semantic RED. Minimum production requires rootless/read-only/cap-drop/NNP/userns/seccomp/LSM `Verified`. | Causal RED + minimum Core candidate | Obtain exact candidate GREEN with applicable isolation evidence before parent integration. |
| Bounded source-context semantic presence | Issue #95 / Draft #96 executed the all-null schema/domain mismatch RED. Minimum schema candidate adds only semantic-presence `anyOf` branches while retaining optional top-level/per-field nullability. | Executed RED + minimum schema candidate | Reacquire exact candidate gates and preserve published/Rust acceptance-set parity. |
| Artifact ingestion name invariant | Issue #97 / Draft #98 owns policy/name admission parity with `ArtifactDescriptor`; formatter prerequisite was repaired without production change. | RED lane executing | Require configured maximum within canonical 255-byte descriptor bound and reject non-leaf names without basename coercion/truncation. |
| Analyzer producer namespace authority | Issue #99 / Draft #100 executed reserved `runtime_core` producer collision RED; minimum candidate introduces one private runtime-core producer identifier and rejects analyzer collision through the existing error class. | Causal RED + minimum candidate | Reacquire exact gates; keep evidence taxonomy ownership in `artifact_analysis`, not Core. |
| UTF-8 byte-bound publication | Issue #101 / Draft #102 owns Rust-byte versus JSON-Schema-character acceptance mismatch; formatter prerequisite repaired. | RED lane executing | Make byte semantics executable/fail-closed without relaxing Rust byte limits or pretending an unsupported extension keyword is a standard assertion. |
| Analyzer evidence producer authority | Issue #77 / Draft #78 rejects untrusted worker claims to controller/runtime-owned evidence kinds while retaining analyzer-owned static evidence. | RED-only child of #70 | After causal RED, add the smallest Supporting-context producer-authority predicate; do not move taxonomy ownership into Core. |
| Worker process exit vs completed outcome | Issue #79 / Draft #80 requires `Completed` only with exact worker `Exited { exit_code: 0 }`. | RED-only child of #70 | Add one cross-field ACL only after causal execution; semantic analyzer failure remains distinct from process failure. |
| Worker cleanup identity | Issue #81 / Draft #82 requires cleanup evidence to identify the exact worker rather than expose an unscoped completion boolean. | RED-only child of #70 | Introduce backend-neutral Core cleanup evidence with exact worker identity after causal RED, then consume it from `artifact_analysis`. |
| Evidence-bundle unknown fields | Issue #75 / Draft #76 tests Rust deserialization against the published Draft 2020-12 `additionalProperties: false` contract. Current focused strict-deserialization controls pass, while broader workspace execution is blocked by the inherited #71/#72 backend invocation class. | Focused candidate GREEN; inherited prerequisite open | Adopt canonical runtime prerequisite non-force and rerun full exact gates; do not mutate evidence ACL for the unrelated backend failure. |
| Dynamic analysis truthfulness | #49 containment and #50/#52/#54/#56/#58/#60/#62/#64/#66 evidence/resource/provenance/cardinality/identity families remain independent focused owners above #18. | Multiple staged REDs; no release-grade detonation worker | Do not execute hostile bytes in the Rust controller. Worker execution must consume Core isolation and preserve attributable provenance/completeness. |
| YARA-X / capa / Ghidra / LIEF adapters | No release-grade production adapters yet. | Missing | Add one bounded evidence producer at a time with immutable tool/version/digest/config provenance and hostile fixtures. |

## Evidence hierarchy and release claims

Configured intent, backend-applied inspection, and live effective enforcement are different evidence levels. A claim may advance only to the strongest level actually proved for the exact sandbox/worker identity.

- Command argv or Podman inspect proves requested/applied configuration, not kernel enforcement.
- Host SELinux/AppArmor availability does not prove per-sandbox confinement.
- An internal network object does not prove exclusive attachment or absence of egress.
- CPU/RAM/PID claims require authoritative live cgroup evidence where available.
- `/tmp` claims require live mount evidence with exact mountpoint/type/restrictions/size.
- Wall-time claims require behavioral termination plus cleanup evidence.
- Request/receipt equality proves contradiction resistance, not confinement.
- Static analysis may not assert observed runtime behavior.
- Fake Podman/process tests may establish parser/ACL/cleanup/readiness behavior but are never release-grade positive isolation evidence.
- Canonical source-region coverage and compiler-instantiation coverage are distinct measurement targets; raw LLVM summaries remain traceable even when canonical source-region admission reconciles duplicate instances.

## Protected integration and release authority

Protected/default `develop` is still `60a85c7633e03b425b67159ec6822c8178cf87ea` and protected. Organization ruleset `18156473` remains the default-branch release authority and requires review/current required workflows/non-fast-forward safety; administrator bypass is not acceptance evidence and is not used.

There are no GitHub Releases. No mutable PR head is consumer authority.

The first release remains blocked until one exact integrated protected head has, at minimum:

- exact 100% owned production statement/function/source-region/branch coverage and public rustdoc;
- native CI on the exact protected integration SHA;
- required review/thread resolution and current central CodeQL/Security/SAST gates;
- real rootless backend acceptance and positive effective LSM/seccomp/capability/resource/network/cleanup evidence;
- package/install/smoke evidence;
- SPDX SBOM, provenance/attestation, checksum/signature where supported;
- reproducibility plus upgrade/rollback evidence;
- immutable version/tag/package/GitHub Release or equivalent canonical publication.

## Next bounded slices

1. Finish Issue #103 / Draft #104 on its final exact head: require verify, 100% owned production coverage/branch/source-region admission, hosted negative confinement, review, and dedicated positive-LSM evidence. Keep Caido login/consumer bootstrap outside the runtime and publish an immutable runtime artifact before contextual-orchestrator changes its consumed version/digest.
2. Preserve #71/#72 as the canonical repeated `rootless_probe` process-invocation RCA; integrate its typed error evidence only through ordinary non-force ancestry after remaining positive-LSM/review gates.
3. Re-run children blocked by that prerequisite (#92 and focused artifact-evidence lanes) after canonical adoption rather than adding leaf retries/workarounds.
4. Keep CodeQL terminal-verdict failures on `.github#1929`, Dependency Review availability on `.github#810`, and positive-LSM capacity/evidence on `.github#1590`; no leaf bypass or substitute gate.
5. Execute #70's current minimum Core worker candidate; only after exact GREEN may #78/#80/#82 proceed independently with their own causal RED→minimum GREEN loops.
6. Preserve #85/#88/#91 complete deltas and exact hosted evidence; do not merge while qualifying review/positive confinement remains open.
7. Reconcile the ordered command/release path `#1 -> #6 -> #9 -> #10 -> #13 -> #14 -> #18` dependency-first without force, then execute resource/network/runtime-identity/pre-attestation RED owners on current ancestry.
8. Publish no release until a normal merge produces an exact protected `develop` head and that exact integrated SHA reacquires every release gate above; only then publish immutable version/artifacts and hand released version/digest pinning to consumer owner paths.
