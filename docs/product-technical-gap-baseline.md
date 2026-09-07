# Product and Technical Gap Baseline

Last reviewed on 2026-09-08 KST against root Draft PR #1 exact repair lineage through `47948f5efd31031a4d54f397d45de3c65f23f44c`, protected/default `develop@60a85c7633e03b425b67159ec6822c8178cf87ea`, and the live open PR/Issue/security state. This ledger separates protected truth, active-PR implementation, checked-in RED, causal RED, candidate GREEN, real backend evidence, central required-workflow evidence, and release authority. Evidence from a predecessor SHA never transfers after head movement.

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
| Root owned-production coverage and process-fixture stability | Exact root `c0839dc1a4eac69bf12dfd82777c2182e0b8769d`, native CI `34157422907`, produced current coverage artifacts before enforcement. Branch artifact `10031469433` reports functions `189/189`, lines `1973/1974`, regions `2635/2637`, branches `454/460`; line artifact `10031465485` independently reports the same statement/function/region surface. Misses are limited to one `application_service` registry-authority branch, five `podman` branches plus its one uncovered line, and one `bounded_command` region. Exact source review shows `parse_process_security_evidence` uses `split_whitespace()`, requires at least eight fields, and therefore cannot construct empty LSM/capability tokens; empty-token checks downstream are unreachable rather than acceptance cases to synthesize. Commit `47948f5efd31031a4d54f397d45de3c65f23f44c` adds public-behavior coverage for the reachable empty-registry-authority rejection and SELinux `unconfined` fail-closed path without weakening isolation or substituting fake Podman for release evidence. | Exact causal coverage RED + focused reachable-edge test candidate; moved head requires fresh evidence | Re-run exact current head. If the focused tests pass, remove only parser-dominated unreachable empty-token branches while retaining `unconfined`, AppArmor enforce-mode, and nonzero capability semantics. Identify the remaining `bounded_command` region from fresh JSON rather than excluding it or lowering thresholds. |
| Repeated backend spawn failure is under-classified | Exact root verify `101852033066` failed a process-backed application-service test at `rootless_probe` before the intended `container_create` failure. Dependency-safe #92 coverage independently hit the same generic `rootless_probe` invocation class in another integration-test binary while #92 verify was GREEN. Current root still maps `BoundedCommandError::{Spawn, Wait, Capture}` to one `BackendInvocationFailed`, so these observations do not distinguish ENOENT/EACCES/resource pressure/other causes. Issue #71 / Draft #72 already has an executed missing-executable RED and a bounded `std::io::ErrorKind` classification candidate, but exact #72 `68d8141f9115b0cc2064d77686af7d54e21c0f38` diverges from the root at merge base `7482108...` and is ten root commits behind. | Real repeated RCA evidence + canonical typed-evidence owner exists; current-root integration pending | Preserve #72's `Spawn(io::ErrorKind)` and bounded public class by ordinary non-force integration after current root coverage repair stabilizes. Do not add retry, extra mutexes, or isolation weakening before the concrete OS class is observable on current ancestry. |
| Repository workflow SHA-pin validation | Issue #86 / Draft #92 original hardened RED executed at `54299dd50748cb5237530299ed6e1fa6e3845781` / CI `34150580702`; the same-repository `$/...` positive controls later executed causally on `8f33668cc7d3481070474220522b67858c694aca` and failed only because the blanket SHA rule rejected GitHub's commit-bound self-reference syntax. Minimum production `d906165bf081b3e4d8ca9d71e48bdb6888cab9e8` admits only `$/` self-repository targets before retaining the exact 40-character lowercase SHA rule for external dependencies. Non-force restack `549e5eb7c41f7b27f71566dd007f0d9e0b99bb3e` adopted root `c0839dc...`; root has moved again. | Two causal REDs + minimum candidate; dependency restack required after current root movement | Non-force adopt the final current root while preserving #92's child-owned files, then reacquire exact-head policy/CI evidence. Add no tag/branch exception. |
| Central CodeQL terminal verdict | Exact root `c0839dc...` CodeQL `34157422811` reached current-head dispatch, then completed FAILURE because actions/python shards failed `Release runner or enforce current-head CodeQL verdict`. | Central required-workflow failure on exact predecessor root | `.github#1929` remains canonical owner. Do not reinterpret dispatch success as terminal CodeQL acceptance or patch leaf status. |
| Dependency Review availability | Exact root Security Scan `34157422895` verified exact checkout/head, then dependency-review failed `Check dependency review support`; actual Dependency Review was skipped while OSV/Scorecard/Trivy succeeded independently. | Central required-workflow failure on exact predecessor root | `.github#810` owns availability. Do not substitute other scanners for Dependency Review. |
| SAST | Exact root `c0839dc...` SAST Semgrep `34157422844` completed SUCCESS. | Exact predecessor security evidence only after root movement | Reacquire on every moved exact root and do not treat one security gate as full release authority. |
| Native runner acquisition | Active branches materialize native CI jobs; dedicated positive-LSM jobs may remain queued on `[self-hosted, linux, cwl-hostile-workload, selinux]`. | Capacity/admission may block one lane without authorizing weaker evidence | `.github#712` owns hosted queue specimens; `.github#1590` owns dedicated positive-LSM runner capacity/evidence. Continue safe independent work. |

GitHub's immutable-action guidance requires full-length commit SHA pinning for external actions, while its `$/` self-repository syntax is already bound to the exact running workflow commit and must not carry an `@ref` suffix. Organization/reusable CI/review/security/release policy remains canonical in `ContextualWisdomLab/.github`; repository validation is defense in depth and must preserve that distinction without duplicating central workflow implementation.

## Application-service isolation and lifecycle

| Capability | Current evidence | Status | Required action |
| --- | --- | --- | --- |
| Immutable application identity | Public request requires lower-case SHA-256 digest identity and launch uses `--pull=never`; mutable tags/alternate host-backed transports are rejected on root lineage. | Implemented contract; exact current runtime evidence still required | Keep registry/import/admission outside launch and bind effective image identity before release. |
| Rootless / read-only / capability / namespace / seccomp / LSM boundary | Root lineage requests restrictive Podman controls and performs effective checks. Hosted Ubuntu is a negative LSM lane; positive acceptance is separate `[self-hosted, linux, cwl-hostile-workload, selinux]`. Current coverage work preserves SELinux priority and AppArmor enforcement semantics and now explicitly exercises the reachable SELinux `unconfined` rejection. | Configured + partial negative evidence; positive exact-head proof pending | Never convert unavailable/unconfined evidence to Verified. Re-prove all required controls on exact candidate. |
| Missing Podman security fields | Issue #73 / Draft #89 causally proved that missing `EffectiveCaps`, `BoundingCaps`, or `dns_enabled` was defaulted to apparently secure values. Minimum production commit `80e6fd19ab5ef11eab3d18b3e80b43f880ce6058` removes only those defaults; predecessor restacked head `4e05824864489d24ee786ee4ffc40c3c96842de6` must adopt the moved root again. | Causal RED + minimum candidate, not current-root-integrated GREEN | Restack non-force after root stabilizes; retain explicit empty/false values as observed secure values and reject field absence as malformed evidence. |
| Podman option/data boundary | Issue #90 / Draft #91 requires literal `--` immediately before consumer-controlled digest-pinned image while preserving command argv. Production remains separate from root coverage repair. | RED-only child | After causal RED, add only the option terminator before the image operand. Do not shell-join or sanitize argv data. |
| Public lease deserialization | Issue #84 / Draft #85 tests that public Serde construction can bypass runtime-owned lease/endpoint/attestation invariants and closed schema constraints. | RED-only child | After causal RED, remove public deserialization if evidence is serialization-only or reconstruct through strict wire DTO + validated domain constructors. |
| Public cleanup-receipt deserialization | Issue #87 / Draft #88 tests that cleanup evidence can be caller-constructed outside its invariant-establishing path. | RED-only child | Enforce strict wire admission or serialization-only evidence; deserialized evidence never becomes destructive authority. |
| Effective network binding | Draft #23 retains exact network-attachment, acquired-network-identity, foreign-safe cleanup, and negative-egress REDs. | P0 REDs staged; production truth incomplete | Execute on current root ancestry; require exact acquired network identity, exclusive deny-by-default attachment, non-force foreign-safe cleanup, and real negative-egress evidence. |
| CPU/RAM/PID/tmpfs/wall time | Applied Podman configuration exists, but inspect/configuration is not equivalent to authoritative cgroup/mount/termination evidence. Draft #19 owns resource/namespace/image/argv applied-state REDs. | Backend-applied intent; live proof incomplete | Bind inspect state only after causal RED, then verify live cgroup-v2, tmpfs mount restrictions/size, and runtime-owned wall-time termination. |
| Runtime/lifecycle ownership | Draft #21 retains independent invocation collision, exact acquired container ID, malformed create-ID, and destructive-authority REDs. | REDs staged | Preserve consumer `request_id` as correlation; use collision-resistant invocation identity and exact acquired backend IDs for lifecycle authority. |
| Pre-attestation command execution | Command path can start hostile payload before positive effective isolation is established; issue #25 remains the hold/attest/release boundary owner. | Security gap; RED staged downstream | Require a trusted hold/attest/release primitive or equivalent. Cleanup after execution does not undo pre-attestation code execution. |
| Crash/restart orphan recovery | Current leases/receipts are process-local and GA requires durable reclamation after runtime crash. | Missing GA capability | Add Recovery context/reaper only with explicit persistence/retention/idempotency/recovery ADR and tests. |

## Artifact-analysis execution and evidence

| Capability | Current evidence | Status | Required action |
| --- | --- | --- | --- |
| Static foundation | Bounded ingestion, SHA-256 artifact identity, non-executing format detection, deterministic evidence ordering, analyzer interface, and failure attribution exist on active foundation #18. | Active foundation, unmerged | Preserve `artifact_analysis` ownership and exact-head verification. |
| Analyzer worker Core ownership | Issue #69 / Draft #70 executed a Core-boundary E0432 RED, then a seven-required-control semantic RED. Minimum production now requires rootless/read-only/cap-drop/NNP/userns/seccomp/LSM `Verified`; current parent candidate remains separate from root repair. | Causal RED + minimum Core candidate | Obtain exact candidate GREEN with full coverage and real applicable isolation evidence before parent integration. |
| Analyzer evidence producer authority | Issue #77 / Draft #78 rejects untrusted worker claims to controller/runtime-owned evidence kinds while retaining analyzer-owned static evidence. | RED-only child of #70 | After causal RED, add the smallest Supporting-context producer-authority predicate; do not move taxonomy ownership into Core. |
| Worker process exit vs completed outcome | Issue #79 / Draft #80 requires `Completed` only with exact worker `Exited { exit_code: 0 }`. | RED-only child of #70 | Add one cross-field ACL only after causal execution; semantic analyzer failure remains distinct from process failure. |
| Worker cleanup identity | Issue #81 / Draft #82 requires cleanup evidence to identify the exact worker rather than expose an unscoped completion boolean. | RED-only child of #70 | Introduce backend-neutral Core cleanup evidence with exact worker identity after causal RED, then consume it from `artifact_analysis`. |
| Evidence-bundle unknown fields | Issue #75 / Draft #76 tests that Rust deserialization must match the published Draft 2020-12 `additionalProperties: false` contract. | RED-only | After causal RED, add strict unknown-field rejection to published evidence wire structures without changing schema version/shape. |
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

Protected/default `develop` is still `60a85c7633e03b425b67159ec6822c8178cf87ea` and protected. Organization ruleset `18156473` remains the default-branch release authority and requires review/current required workflows/non-fast-forward safety; administrator bypass is not acceptance evidence and is not used.

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

1. Execute exact-head CI for the current root after `47948f5...` reachable-edge coverage repair plus this ledger movement. The empty registry authority must reject and SELinux `unconfined` must fail closed without changing positive release semantics.
2. Use that exact coverage JSON to remove only parser-dominated unreachable empty-token predicates in `podman.rs`, retaining every reachable `unconfined`/mode/capability check. Identify and repair the remaining `bounded_command.rs` region from evidence; no exclusions, sample reduction, or threshold changes.
3. Once the root coverage/process fixture is stable, non-force integrate #71/#72's typed spawn-failure evidence onto the current root while preserving both sides' `application_service`, `podman`, and gap-ledger deltas. Re-run a real recurrence to classify the `rootless_probe` OS failure before considering any retry policy.
4. Non-force restack issue #86 / Draft #92 onto the final current root, preserving its executed `$/` self-reference RED and minimum external-SHA/self-reference policy repair. Re-run focused repository policy and full gates on the restacked exact head.
5. Keep CodeQL terminal-verdict failures on `.github#1929`, Dependency Review availability on `.github#810`, hosted queue capacity on `.github#712`, and positive-LSM capacity/evidence on `.github#1590`; no leaf bypass or substitute gate.
6. Execute #70 current minimum Core worker candidate; only after exact GREEN may #78/#80/#82 proceed independently with their own causal RED→minimum GREEN loops.
7. Restack current root children #89/#85/#88/#91 independently after root stabilizes; preserve every child delta by ordinary non-force integration and do not fold unrelated repairs together.
8. After root exact GREEN, reconcile the ordered command/release path `#1 -> #6 -> #9 -> #10 -> #13 -> #14 -> #18` dependency-first without force, then execute resource/network/runtime-identity/pre-attestation RED owners on current ancestry.
9. Publish no release until the normal merge produces an exact protected `develop` head and that exact integrated SHA reacquires every release gate listed above; then publish immutable version/artifacts and hand released version/digest pinning to consumer owner paths.
