# Product and Technical Gap Baseline

Last reviewed on 2026-09-08 KST against canonical root Draft PR #1 exact `5c6a44bb2b35eb17d0315d72db242f4488c3c426`, typed-spawn/UL repair Draft #72 production candidate `d5cf70820295f77dac5803e3eb919d3d5ef9bb35`, protected/default `develop@60a85c7633e03b425b67159ec6822c8178cf87ea`, and live PR/Issue/review/security state. This ledger distinguishes protected truth, active-PR implementation, checked-in RED, causal RED, minimum candidate, exact-head GREEN, real backend evidence, central required-workflow evidence, and release authority. Evidence from a predecessor SHA never transfers after source or dependency movement.

## Product responsibility and DDD authority

Quarantine Sandbox Runtime owns reusable hostile-workload/application-service isolation, resource/lifecycle bounds, lease/readiness/cleanup attestation, and artifact-analysis execution evidence.

- `sandbox_execution` is the Core bounded context and owns reusable sandbox/worker isolation, resource, lifecycle, termination, and cleanup vocabulary.
- `artifact_analysis` is a Supporting context and owns artifact identity/admission, analyzer orchestration, evidence normalization/completeness, and analyzer-result ACL semantics.
- `application_service` is a Supporting context and owns consumer-neutral service intent plus translation to/from Core sandbox execution.
- `infrastructure` owns concrete Podman/process/backend translation and observation. Podman/gVisor/containerd/Kubernetes/VM types do not become consumer domain contracts.
- Wardnet retains verdict/incident/response authority. contextual-orchestrator retains LLM/Agent/tool authorization. Noema retains Agent/runtime capability authority. Keyverse retains identity authority. EgressWeave retains outbound-policy authority.
- Consumer integration is through a future immutable released contract/artifact. Sibling source imports, mutable PR-head dependencies, direct foreign runtime calls, and cross-service SQL are not integration mechanisms.

The runtime currently owns no durable database. Any future durable job/evidence/reaper store requires an explicit persistence ADR, 3NF schema, descriptive multiword `snake_case` objects, retention/recovery semantics, and migration/rollback evidence.

## Current root, coverage, CI, and repository-policy authority

| Gap / gate | Current evidence | Status | Required action |
| --- | --- | --- | --- |
| Checkout credential persistence | Predecessor root `7482108c0b74f58f447722a98330f9ad44215eec`, native CI `34089522598`, verify `101640016166` proved every checkout uses `persist-credentials: false`. | Causal GREEN preserved in root ancestry | Do not regress; predecessor GREEN does not replace current-head gates. |
| Canonical source-region coverage | Issue #93 / PR #94 audited LLVM monomorphized region evidence, separated raw compiler-instance summaries from production source-region truth, added denominator reconciliation, and was normally merged into root. Current root exact `5c6a44bb...`, native CI `34176680115`, has verify `101907363084`, coverage `101907363224`, branch coverage `101907363185`, and hosted negative rootless/AppArmor `101907363162` GREEN. | Integrated exact-root hosted GREEN for verify/coverage/branch/negative lanes | Preserve raw LLVM diagnostics, 100% source/function/line/branch gates, and fail-closed denominator reconciliation. Do not reintroduce parser-dominated impossible predicates merely to alter metrics. |
| Dedicated positive effective-LSM evidence | Root exact `5c6a44bb...` positive-LSM job `101907363137` remains queued. #72 predecessor exact `01563100108c158dc29922b6fd008edc31d42839` also had positive-LSM `101914208208` queued, and current production candidate `d5cf708...` had `101925919616` queued before this ledger movement. Every lane targets `[self-hosted, linux, cwl-hostile-workload, selinux]`. Hosted negative evidence is not positive confinement evidence. | Independent release/security lane blocked on dedicated runner | `.github#1590` remains canonical owner. Do not substitute hosted negative evidence or weaken the LSM gate. |
| Typed backend-spawn evidence and provider-neutral UL | Issue #71 / Draft #72 executed the missing-executable RED and provider-neutral UL RED, then non-force adopted current root. Exact predecessor `01563100108c158dc29922b6fd008edc31d42839`, native CI `34179057453`, subsequently reached full hosted execution: verify `101914208131` GREEN and hosted negative rootless/AppArmor `101914208181` GREEN. Coverage `101914208138` and branch coverage `101914207986` generated/uploaded evidence and failed only complete-production admission. Exact artifacts `10038444108` / `10038373778` show functions `192/192`, branches `452/452`, lines `1997/1998`, with the only zero-count production source line at the Podman ACL `BoundedCommandError::Wait | Capture -> BackendInvocationFailed`; raw region discrepancy remains governed by merged #93/#94 source-region reconciliation. Review `5136944224` rejected deleting/collapsing legitimate Wait/Capture classes. Minimum candidate `d5cf70820295f77dac5803e3eb919d3d5ef9bb35` extracts the unchanged bounded-command error translation into a pure infrastructure ACL helper and directly covers Timeout/OutputLimit/Spawn/Wait/Capture. | Causal REDs + semantics-preserving coverage candidate; ledger movement requires fresh exact-head execution | Reacquire exact-head hosted gates after this documentation commit. No retry, extra mutex, timeout change, provider coupling, or isolation weakening follows from failure classification. Keep timeout/output-limit/wait/capture/nonzero-exit distinct. |
| Repository workflow SHA-pin validation | Issue #86 / Draft #92 executed both the multi-workflow SHA-pin RED and same-repository `$/...` false-positive RED. Minimum production `d906165bf081b3e4d8ca9d71e48bdb6888cab9e8` admits only `$/` self-repository targets before retaining the exact 40-character lowercase SHA rule for external dependencies. #92 is on predecessor root ancestry. | Two causal REDs + minimum candidate; dependency restack required | Non-force adopt the stabilized root while preserving #92 child-owned files, then reacquire exact-head repository-policy/full-gate evidence. Add no tag/branch exception. |
| Central CodeQL terminal verdict | Root CodeQL `34176680113`: language detection and current-head dispatch succeeded, but python `101907470730` and actions `101907470755` failed `Release runner or enforce current-head CodeQL verdict`. | Central required-workflow failure | `.github#1929` remains canonical owner. Dispatch is not a terminal CodeQL verdict; do not patch leaf status. |
| Dependency Review availability | Root Security Scan `34176680132`: OSV, Scorecard, and Trivy succeeded independently; dependency-review `101907475098` verified exact checkout/head then failed `Check dependency review support`, so actual Dependency Review was skipped. | Central required-workflow failure | `.github#810` owns availability. Do not substitute sibling scanners for Dependency Review. |
| SAST | Root SAST has an independent exact-head lane and must be judged separately from CodeQL/Dependency Review/native CI. | Independent gate | One scanner never substitutes for another required security verdict. |

GitHub's immutable-action guidance requires full-length commit SHA pinning for external actions, while its `$/` same-repository syntax is bound to the exact workflow commit and cannot use an `@ref` suffix. Organization/reusable CI/review/security/release policy remains canonical in `ContextualWisdomLab/.github`; repository validation is defense in depth rather than a copy of central workflow ownership.

## Application-service isolation and lifecycle

| Capability | Current evidence | Status | Required action |
| --- | --- | --- | --- |
| Immutable application identity | Public request requires lower-case SHA-256 digest identity and launch uses `--pull=never`; mutable tags/alternate host-backed transports are rejected on root lineage. | Implemented contract; protected exact runtime evidence still required | Keep registry/import/admission outside launch and bind effective image identity before release. |
| Rootless / read-only / capability / namespace / seccomp / LSM boundary | Root requests restrictive Podman controls and performs effective checks. Exact root hosted-negative and coverage lanes prove reachable fail-closed behavior including SELinux `unconfined` rejection. Positive acceptance remains separate on the dedicated SELinux runner. | Hosted negative/process evidence GREEN; positive confinement pending | Never convert unavailable/unconfined evidence to Verified. Preserve required controls and obtain real positive effective-LSM evidence. |
| Spawn failure RCA | `BoundedCommandError::Spawn(io::ErrorKind)` preserves the OS class in #72; Podman ACL bounds it to `BackendInvocationFailureKind::{NotFound, PermissionDenied, ResourceExhausted, Other}`. Missing output pipe remains Capture; timeout/output-limit/wait/nonzero exit remain distinct. Exact `0156310...` verify and hosted-negative lanes are GREEN. Its coverage artifact identified only the otherwise legitimate Wait/Capture ACL source line as uncovered, leading to pure mapper extraction/test candidate `d5cf708...` without semantic collapse. | Typed evidence behavior proven on predecessor; current mapper-coverage candidate requires fresh exact-head evidence | Use future real recurrence to identify the concrete OS class. Classification is evidence, not retry policy; preserve every bounded command failure class. |
| Consumer-neutral invocation failure text | #72 exact `8850158a.../101910682603` causally proved provider leakage in `BackendInvocationFailed` while the typed missing-executable test passed. Minimum `4f3da8b...` changed only public wording/doc comment; exact `0156310...` verify later passed the full test/lint/rustdoc suite. | Causal RED -> predecessor hosted GREEN; current ledger head requires fresh gate | Do not rename variants, restore provider text, or broaden scope. |
| Missing Podman security fields | Issue #73 / Draft #89 causally proved missing `EffectiveCaps`, `BoundingCaps`, or `dns_enabled` could default to secure-looking values. Minimum production `80e6fd19ab5ef11eab3d18b3e80b43f880ce6058` removes only those defaults; child remains on predecessor root ancestry. | Causal RED + minimum candidate, not current-root-integrated GREEN | Restack non-force after current root/#72 stabilization; explicit empty/false remain observed values while field absence is malformed evidence. |
| Podman option/data boundary | Issue #90 / Draft #91 requires literal `--` immediately before consumer-controlled digest-pinned image while preserving direct command argv. | RED-only child | After causal RED, add only the option terminator before image operand. Do not shell-join or mutate argv data. |
| Public lease deserialization | Issue #84 / Draft #85 tests that public Serde construction can bypass runtime-owned lease/endpoint/attestation invariants and closed schema constraints. | RED-only child | Remove public deserialization for evidence-only types or introduce strict validated wire DTO admission; unsupported values must fail, not coerce. |
| Public cleanup-receipt deserialization | Issue #87 / Draft #88 tests that cleanup evidence can be caller-constructed outside its invariant-establishing path. | RED-only child | Enforce strict wire admission or serialization-only evidence; deserialized evidence never becomes destructive authority. |
| Effective network binding | Draft #23 retains exact network-attachment, acquired-network-identity, foreign-safe cleanup, and negative-egress REDs. | P0 REDs staged; production truth incomplete | Execute on current ancestry; require exact acquired network identity, exclusive deny-by-default attachment, non-force foreign-safe cleanup, and real negative-egress evidence. |
| CPU/RAM/PID/tmpfs/wall time | Applied Podman configuration exists, but inspect/configuration is not equivalent to authoritative cgroup/mount/termination evidence. Draft #19 owns resource/namespace/image/argv applied-state REDs. | Backend-applied intent; live proof incomplete | Bind inspect state only after causal RED, then verify live cgroup-v2, tmpfs mount restrictions/size, and runtime-owned wall-time termination. |
| Runtime/lifecycle ownership | Draft #21 retains independent invocation collision, exact acquired container ID, malformed create-ID, and destructive-authority REDs. Root review thread for ADR-0006 remains unresolved and correctly points to this owner. | REDs staged | Preserve consumer `request_id` as correlation; use collision-resistant runtime invocation identity and exact acquired backend IDs for lifecycle authority. |
| Stream-worker/process-group deadline | Root review identified that descendant-held pipes can outlive the direct-child deadline; Issue #74 owns process-group ownership and deadline-bounded capture/reap. | P0 stability gap; owner path exists | RED must itself be non-hanging and prove no descendant/pipe leak. Do not mix this into #72's spawn classification. |
| Pre-attestation command execution | Command path can start hostile payload before positive effective isolation is established; issue #25 remains hold/attest/release boundary owner. | Security gap; RED staged downstream | Require trusted hold/attest/release or equivalent. Cleanup after execution does not undo pre-attestation code execution. |
| Crash/restart orphan recovery | Current leases/receipts are process-local and GA requires durable reclamation after runtime crash. | Missing GA capability | Add Recovery context/reaper only with explicit persistence/retention/idempotency/recovery ADR and tests. |

## Artifact-analysis execution and evidence

| Capability | Current evidence | Status | Required action |
| --- | --- | --- | --- |
| Static foundation | Bounded ingestion, SHA-256 artifact identity, non-executing format detection, deterministic evidence ordering, analyzer interface, and failure attribution exist on active foundation #18. | Active foundation, unmerged | Preserve `artifact_analysis` ownership and exact-head verification. |
| Analyzer worker Core ownership | Issue #69 / Draft #70 executed a Core-boundary E0432 RED, then a seven-required-control semantic RED. Minimum production requires rootless/read-only/cap-drop/NNP/userns/seccomp/LSM `Verified`. Current exact #70 remains `24a183f1343470fd2a66d4a93c0267ced4900f07`; its native CI `34140779748` is still queued. | Causal RED + minimum Core candidate; exact candidate execution pending | Obtain exact candidate GREEN with applicable isolation evidence before parent integration. |
| Analyzer execution outside controller | Root review thread confirms arbitrary `StaticAnalyzer` code cannot truthfully execute directly in controller while self-attesting no network/dynamic execution. Issue #49 plus #69/#70 own hostile-capability RED and Core worker boundary. | P0 finding remains unresolved by design | Keep controller fail closed until sandboxed worker boundary is causally GREEN and adopted. |
| Analyzer evidence producer authority | Issue #77 / Draft #78 rejects untrusted worker claims to controller/runtime-owned evidence kinds while retaining analyzer-owned static evidence. | RED-only child of #70 | After causal RED, add smallest Supporting-context producer-authority predicate; do not move taxonomy ownership into Core. |
| Worker process exit vs completed outcome | Issue #79 / Draft #80 requires `Completed` only with exact worker `Exited { exit_code: 0 }`. | RED-only child of #70 | Add one cross-field ACL only after causal execution; semantic analyzer failure remains distinct from process failure. |
| Worker cleanup identity | Issue #81 / Draft #82 requires cleanup evidence to identify exact worker rather than expose an unscoped completion boolean. | RED-only child of #70 | Introduce backend-neutral Core cleanup evidence with exact worker identity after causal RED, then consume it from `artifact_analysis`. |
| Evidence-bundle unknown fields | Issue #75 / Draft #76 tests that Rust deserialization must match published Draft 2020-12 `additionalProperties: false` at bundle/artifact/runtime/evidence-record boundaries. Root review thread remains open until owner stack lands. | RED-only owner | After causal RED, add strict unknown-field rejection without changing schema version/shape. |
| Evidence kind truthfulness | Static-only schema still has review finding around `runtime_behavior` versus `dynamic_execution_performed: false`. | Open contract finding | Keep runtime behavior evidence out of static-only contract; handle through causal owner/versioned dynamic profile rather than silent semantic broadening. |
| Analyzer metadata normalization | Invalid analyzer finding/failure attributes can poison bundle validation instead of yielding bounded `ToolFailure` evidence. | Open Supporting-context finding | Add causal RED and bounded stable failure normalization; never copy invalid/control/overlong metadata into published evidence. |
| Analysis-job identity | Current review finding requires canonical artifact descriptor identity, including filename and kind, to participate in analysis job/evidence identity. | Open identity finding | Add focused collision RED before changing hash framing; preserve delimiter/version contract and provenance. |
| YARA-X / capa / Ghidra / LIEF adapters | No release-grade production adapters yet. | Missing | Add one bounded evidence producer at a time with immutable tool/version/digest/config provenance and hostile fixtures. |

## Schema and interoperability findings

Root review still has unresolved contract findings for UTF-8 byte-limit parity between Rust and JSON Schema consumers, bounded-source-context all-null admission, Gregorian `submitted_at` validity, application-service request byte limits, artifact-name ingestion consistency, and static evidence semantics. These are real repair findings rather than reasons to close the root. They must be split into causal owner lanes where not already owned, preserve public contract compatibility explicitly, and use standard/schema authority rather than non-assertive custom keywords as silent acceptance evidence.

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
- Canonical source-region coverage and compiler-instantiation coverage are distinct measurement targets; raw LLVM summaries remain traceable when source-region admission reconciles duplicate instantiations.
- A typed OS spawn class is diagnostic evidence; it does not imply safe retryability.

## Protected integration and release authority

Protected/default `develop` is still `60a85c7633e03b425b67159ec6822c8178cf87ea` and protected. Organization ruleset `18156473` remains default-branch release authority and requires review/current required workflows/non-fast-forward safety; administrator bypass is not acceptance evidence and is not used.

No GitHub Release is currently consumer authority. No mutable PR head is consumer authority.

The first release remains blocked until one exact integrated protected head has, at minimum:

- exact 100% owned production statement/function/source-region/branch coverage and public rustdoc;
- native CI on the exact protected integration SHA;
- required approval and all review threads resolved or superseded by causally landed owner paths;
- current central CodeQL, Dependency Review/Security, and SAST gates;
- real rootless backend acceptance and positive effective LSM/seccomp/capability/resource/network/cleanup evidence;
- package/install/smoke evidence;
- SPDX SBOM, provenance/attestation, checksum/signature where supported;
- reproducibility plus upgrade/rollback evidence;
- immutable version/tag/package/GitHub Release or equivalent canonical publication.

## Next bounded slices

1. Execute the exact #72 ledger/current head after the `d5cf708...` mapper-coverage candidate. Require fmt/tests/clippy/rustdoc plus 100% owned production source/function/line/branch coverage and hosted negative semantics on the unchanged head; predecessor `0156310...` evidence remains causal history only after this documentation movement.
2. Keep #72 Draft while dedicated positive-LSM, current review/thread requirements, and central security gates remain incomplete. `.github#712` owns current hosted queue starvation and `.github#1590` owns positive-LSM runner capability; never cancel the sole current-head evidence or substitute hosted negative results.
3. Use typed spawn evidence on a future real `rootless_probe` recurrence to identify the concrete OS class before any retry policy. Do not add a generic retry, another fixture mutex, or isolation-predicate weakening.
4. Non-force restack issue #86 / Draft #92 and root children #89/#85/#88/#91 independently only after the root/#72 chain stabilizes, preserving every child-owned delta and reacquiring exact-head evidence.
5. Keep CodeQL terminal-verdict failures on `.github#1929`, Dependency Review availability on `.github#810`, positive-LSM capacity/evidence on `.github#1590`, and Node/action runtime migration on `.github#2011`; no leaf bypass or substitute gate.
6. Execute #70's current minimum Core worker candidate; only after exact GREEN may #78/#80/#82 proceed independently with their own causal RED -> minimum GREEN loops.
7. Repair unresolved root review findings through their existing canonical owner issues/PRs or create focused successors when no owner exists. A root review thread is not resolved merely because an issue was opened; it resolves only after valid delta is causally landed/adopted.
8. After the root is exact GREEN and review/security gates permit, reconcile ordered command/release dependencies without force and continue resource/network/runtime-identity/pre-attestation RED owners on current ancestry.
9. Publish no release until a normal merge produces an exact protected `develop` head and that exact integrated SHA reacquires every release gate above; only then publish immutable version/artifacts and hand released version/digest pinning to consumer owner paths.
