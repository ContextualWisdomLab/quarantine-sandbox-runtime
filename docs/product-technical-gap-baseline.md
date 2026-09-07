# Product and Technical Gap Baseline

Last reviewed on 2026-09-08 KST against root Draft PR #1 exact `78281e244c530dcafb3368b9f1d9896e846206a9`, protected/default `develop@60a85c7633e03b425b67159ec6822c8178cf87ea`, and the live open PR/Issue/security state. This ledger separates protected truth, active-PR implementation, checked-in RED, causal RED, candidate GREEN, real backend evidence, central required-workflow evidence, and release authority. Evidence from a predecessor SHA never transfers after head movement.

## Product responsibility and DDD authority

Quarantine Sandbox Runtime owns reusable hostile-workload/application-service isolation, resource/lifecycle bounds, lease/readiness/cleanup attestation, and artifact-analysis execution evidence.

- `sandbox_execution` is the Core bounded context and owns reusable sandbox/worker isolation, resource, lifecycle, termination, and cleanup vocabulary.
- `artifact_analysis` is a Supporting context and owns artifact identity/admission, analyzer orchestration, evidence normalization/completeness, and analyzer-result ACL semantics.
- `application_service` is a Supporting context and owns consumer-neutral service intent plus translation to/from Core sandbox execution.
- `infrastructure` owns concrete Podman/process/backend translation and observation. Podman/gVisor/containerd/Kubernetes/VM types do not become consumer domain contracts.
- Wardnet retains verdict/incident/response authority. contextual-orchestrator retains LLM/Agent/tool authorization. Noema retains Agent/runtime capability authority. Keyverse retains identity authority. EgressWeave retains outbound-policy authority.
- Consumer integration is through a future immutable released contract/artifact. Sibling source imports, mutable PR-head dependencies, direct foreign runtime calls, and cross-service SQL are not integration mechanisms.

The runtime currently owns no durable database. Any future durable job/evidence/reaper store requires an explicit persistence ADR, 3NF schema, descriptive multiword `snake_case` objects, retention/recovery semantics, and migration/rollback evidence.

## Current root and repository-policy authority

| Gap / gate | Current evidence | Status | Required action |
| --- | --- | --- | --- |
| Checkout credential persistence | Predecessor root `7482108c0b74f58f447722a98330f9ad44215eec`, native CI `34089522598`, verify `101640016166` proved every checkout uses `persist-credentials: false`. | Causal GREEN preserved in root ancestry | Do not regress; predecessor GREEN does not replace current-root gates. |
| Root owned-production coverage | The same predecessor exposed production totals below 100%. Current root `78281e244c530dcafb3368b9f1d9896e846206a9` adds only `tests/root_coverage_edges.rs` for reachable image/admission, capability parsing, non-UTF-8 process evidence, and port parsing edges; production Rust and thresholds are unchanged. Issue #83 owns the repair. | Candidate coverage repair; current native CI `34138612571` queued | Obtain exact current-head statement/function/region/branch evidence; fake Podman tests remain contract coverage, never confinement evidence. |
| Repository workflow SHA-pin validation | `scripts/validate_repository.py` on current root reads only `.github/workflows/ci.yml`; additional `.yml`/`.yaml` workflows are outside its `uses:` scan, and missing `ci.yml` raises rather than producing a deterministic policy result. Issue #86 / Draft #92 stages behavioral RED fixtures for second `.yml`, `.yaml`, missing-workflow, and all-pinned controls. | RED-only child; production/policy implementation unchanged | Execute #92 focused RED, then deterministically enumerate all direct workflow `.yml`/`.yaml` files and retain the exact 40-character lower-case SHA rule without exceptions. |
| Central CodeQL terminal verdict | Root current CodeQL `34138612577`: language detection `101795272391` and dispatch `101810826339` succeeded; python `101805461420` and actions `101805461451` failed at `Release runner or enforce current-head CodeQL verdict`. | Current exact-head central required-workflow failure | `.github#1929` remains canonical owner. Do not reinterpret dispatch success as terminal CodeQL acceptance or patch leaf status. |
| Dependency Review availability | Root current Security Scan `34138612547`: dependency-review `101804393112` verified exact checkout then failed `Check dependency review support`; actual Dependency Review was skipped. OSV `101804393118` and Scorecard `101804393153` succeeded independently. | Current exact-head central required-workflow failure | `.github#810` owns availability. Do not substitute OSV/Scorecard/Trivy for Dependency Review. |
| SAST | Root current SAST Semgrep `34138612530` completed SUCCESS. | Current exact-head partial security evidence | Preserve, but do not treat one security gate as full release authority. |
| Native runner acquisition | Root and active focused children continue to materialize five native CI jobs while many remain `steps=[]`, `runner_id=null` pre-checkout queued. | Capacity/admission blocker per lane, not product GREEN/RED | `.github#712` owns hosted queue specimens; `.github#1590` owns dedicated positive-LSM runner capacity/evidence. Continue safe work on independent lanes. |

GitHub's immutable-action guidance and the local policy agree on full-length commit SHA pinning. Organization/reusable CI/review/security/release policy remains canonical in `ContextualWisdomLab/.github`; repository validation is defense in depth and must not duplicate central workflow implementation.

## Application-service isolation and lifecycle

| Capability | Current evidence | Status | Required action |
| --- | --- | --- | --- |
| Immutable application identity | Public request requires lower-case SHA-256 digest identity and launch uses `--pull=never`; mutable tags/alternate host-backed transports are rejected on root lineage. | Implemented contract; exact current runtime evidence still required | Keep registry/import/admission outside launch and bind effective image identity before release. |
| Rootless / read-only / capability / namespace / seccomp / LSM boundary | Root lineage requests restrictive Podman controls and performs effective checks. Hosted Ubuntu is a negative LSM lane; positive acceptance is separate `[self-hosted, linux, cwl-hostile-workload, selinux]`. | Configured + partial negative evidence; positive exact-head proof pending | Never convert unavailable/unconfined evidence to Verified. Re-prove all required controls on exact candidate. |
| Missing Podman security fields | Issue #73 / Draft #89 causally proved that missing `EffectiveCaps`, `BoundingCaps`, or `dns_enabled` was defaulted to apparently secure values. Minimum production commit `80e6fd19ab5ef11eab3d18b3e80b43f880ce6058` removes only those defaults; current non-force-restacked head `4e05824864489d24ee786ee4ffc40c3c96842de6`, CI `34139220581`, remains queued. | Causal RED + minimum candidate, not current-head GREEN | Execute exact restacked head; retain explicit empty/false values as observed secure values and reject field absence as malformed evidence. |
| Podman option/data boundary | Issue #90 / Draft #91 exact `1decc8056939877c5ce3bce8a1036bb0eb9e3dda` requires literal `--` immediately before consumer-controlled digest-pinned image while preserving command argv. Production is unchanged. | RED-only; CI `34140177269` queued | After causal RED, add only the option terminator before the image operand. Do not shell-join or sanitize argv data. |
| Public lease deserialization | Issue #84 / Draft #85 exact `012008ffc9170046d2a177b16930cfb190e5f058` proves public Serde construction can bypass runtime-owned lease/endpoint/attestation invariants and closed schema constraints. | RED-only; CI `34139552353` queued | After causal RED, remove public deserialization if evidence is serialization-only or reconstruct through strict wire DTO + validated domain constructors. |
| Public cleanup-receipt deserialization | Issue #87 / Draft #88 exact `ede681afc9e11a901f6a8e79a78bff23da600dfd` proves cleanup evidence can be caller-constructed outside its invariant-establishing path. | RED-only; CI `34139581789` queued | Enforce strict wire admission or serialization-only evidence; deserialized evidence never becomes destructive authority. |
| Effective network binding | Draft #23 retains exact network-attachment, acquired-network-identity, foreign-safe cleanup, and negative-egress REDs. | P0 REDs staged; production truth incomplete | Execute on current root ancestry; require exact acquired network identity, exclusive deny-by-default attachment, non-force foreign-safe cleanup, and real negative-egress evidence. |
| CPU/RAM/PID/tmpfs/wall time | Applied Podman configuration exists, but inspect/configuration is not equivalent to authoritative cgroup/mount/termination evidence. Draft #19 owns resource/namespace/image/argv applied-state REDs. | Backend-applied intent; live proof incomplete | Bind inspect state only after causal RED, then verify live cgroup-v2, tmpfs mount restrictions/size, and runtime-owned wall-time termination. |
| Runtime/lifecycle ownership | Draft #21 retains independent invocation collision, exact acquired container ID, malformed create-ID, and destructive-authority REDs. | REDs staged | Preserve consumer `request_id` as correlation; use collision-resistant invocation identity and exact acquired backend IDs for lifecycle authority. |
| Pre-attestation command execution | Command path can start hostile payload before positive effective isolation is established; issue #25 remains the hold/attest/release boundary owner. | Security gap; RED staged downstream | Require a trusted hold/attest/release primitive or equivalent. Cleanup after execution does not undo pre-attestation code execution. |
| Crash/restart orphan recovery | Current leases/receipts are process-local and GA requires durable reclamation after runtime crash. | Missing GA capability | Add Recovery context/reaper only with explicit persistence/retention/idempotency/recovery ADR and tests. |

## Artifact-analysis execution and evidence

| Capability | Current evidence | Status | Required action |
| --- | --- | --- | --- |
| Static foundation | Bounded ingestion, SHA-256 artifact identity, non-executing format detection, deterministic evidence ordering, analyzer interface, and failure attribution exist on active foundation #18. | Active foundation, unmerged | Preserve `artifact_analysis` ownership and exact-head verification. |
| Analyzer worker Core ownership | Issue #69 / Draft #70 executed a Core-boundary E0432 RED, then a seven-required-control semantic RED on exact `6c07f238d8f95d3927a0901b8488757adae1b16d` / CI `34121419565`. Production `18e113eeda491280ed99cf7b2e72ba8ba1d3280c` now requires rootless/read-only/cap-drop/NNP/userns/seccomp/LSM `Verified`; current doctoring head is `24a183f1343470fd2a66d4a93c0267ced4900f07`. | Causal RED + minimum Core candidate; current CI `34140779748` queued | Obtain exact candidate GREEN with full coverage and real applicable isolation evidence before parent integration. |
| Analyzer evidence producer authority | Issue #77 / Draft #78 exact `be0ad22fcc2007b4781ab082381cb0293e30b09c` rejects untrusted worker claims to controller/runtime-owned evidence kinds while retaining analyzer-owned static evidence. | RED-only child of #70; CI `34140981661` queued | After causal RED, add the smallest Supporting-context producer-authority predicate; do not move taxonomy ownership into Core. |
| Worker process exit vs completed outcome | Issue #79 / Draft #80 exact `319bf5c2e12e00c0f6ec4391347f060fd1b17315` requires `Completed` only with exact worker `Exited { exit_code: 0 }`. | RED-only child of #70; CI `34141016864` queued | Add one cross-field ACL only after causal execution; semantic analyzer failure remains distinct from process failure. |
| Worker cleanup identity | Issue #81 / Draft #82 exact `f628c20eae0d69386f53260fb789afca9c55d1f4` requires cleanup evidence to identify the exact worker rather than expose an unscoped completion boolean. | RED-only child of #70; CI `34141046352` queued | Introduce backend-neutral Core cleanup evidence with exact worker identity after causal RED, then consume it from `artifact_analysis`. |
| Evidence-bundle unknown fields | Issue #75 / Draft #76 exact `fec3de0e7f6b37e87875143ed0031418b25d2459` tests that Rust deserialization must match the published Draft 2020-12 `additionalProperties: false` contract. | RED-only | After causal RED, add strict unknown-field rejection to the published evidence wire structures without changing schema version/shape. |
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
- Fake Podman/process tests may establish parser/ACL/cleanup behavior but are never release-grade positive isolation evidence.

## Protected integration and release authority

Protected/default `develop` is still `60a85c7633e03b425b67159ec6822c8178cf87ea` and protected. Organization ruleset `18156473` is active on the default branch and requires one approval, stale-review dismissal after push, review-thread resolution, central required workflows, and forbids deletion/non-fast-forward updates. Administrator bypass exists but is not acceptance evidence and is not used.

There are no GitHub Releases. No mutable PR head is consumer authority.

The first release remains blocked until one exact integrated protected head has, at minimum:

- exact 100% owned production statement/function/region/branch coverage and public rustdoc;
- native CI on the exact protected integration SHA;
- required review/thread resolution and current central CodeQL/Security/SAST gates;
- real rootless backend acceptance and positive effective LSM/seccomp/capability/resource/network/cleanup evidence;
- package/install/smoke evidence;
- SPDX SBOM, provenance/attestation, checksum/signature where supported;
- reproducibility plus upgrade/rollback evidence;
- immutable version/tag/package/GitHub Release or equivalent canonical publication.

## Next bounded slices

1. Let current root `78281e...` execute issue #83 coverage repair. Repair new causal failures only; do not lower coverage or replace real confinement evidence with fake tests.
2. In parallel, execute issue #86 / Draft #92 repository-workflow discovery RED. After the intended failures, make only the deterministic all-workflow SHA-pin discovery/admission repair and reacquire the moved exact head.
3. Keep root current CodeQL terminal-verdict failure on `.github#1929`, Dependency Review availability on `.github#810`, hosted queue capacity on `.github#712`, and positive-LSM capacity/evidence on `.github#1590`; no leaf bypass or substitute gate.
4. Execute #70 current minimum Core worker candidate; only after exact GREEN may #78/#80/#82 proceed independently with their own causal RED→minimum GREEN loops.
5. Execute current root children #89/#85/#88/#91 independently; preserve root ancestry by ordinary non-force integration and do not fold unrelated repairs together.
6. After root exact GREEN, reconcile the ordered command/release path `#1 -> #6 -> #9 -> #10 -> #13 -> #14 -> #18` dependency-first without force, then execute resource/network/runtime-identity/pre-attestation RED owners on current ancestry.
7. Publish no release until the normal merge produces an exact protected `develop` head and that exact integrated SHA reacquires every release gate listed above; then publish immutable version/artifacts and hand released version/digest pinning to consumer owner paths.
