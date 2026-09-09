# Product and Technical Gap Baseline

Last reviewed on 2026-09-09 KST against live protected/default `develop@60a85c7633e03b425b67159ec6822c8178cf87ea`, dependency-root Draft #1 exact `5c6a44bb2b35eb17d0315d72db242f4488c3c426`, command-runtime Draft #14 exact `bc9450998848dbb690a74aec7b11878f80b4da62`, process-boundary issue #71 / Draft #72 exact `900d0273625115f7bcc602d888baadded0caeb4f`, artifact-analysis parent #18 exact `c0647152ec052d82969b2ae078891e25e6d4d69a`, Analyzer Worker Execution parent #70 exact `34de52819549e0362ebcd0a110a361146b3556d1`, and active worker-contract descendants #78/#80/#82. Historical detail through 2026-09-07 is preserved verbatim at `docs/doctoring/PRODUCT_TECHNICAL_GAP_BASELINE_2026-09-07_HISTORY.md`; this file is the code-current commercial/technical authority. Predecessor CI evidence never transfers to a moved head.

## Product responsibility and DDD

Quarantine Sandbox Runtime is the canonical owner of reusable hostile-workload isolation, application-service isolation, artifact-analysis execution isolation, resource ceilings, lifecycle/cleanup evidence, and backend-neutral sandbox contracts. `sandbox_execution` is Core; `application_service` and `artifact_analysis` are Supporting bounded contexts; Podman/gVisor/containerd/Kubernetes/VM remain infrastructure adapters. Wardnet owns verdict/incident/SOC truth, AppGuardrail owns SAST/SARIF truth, Noema owns Agent/runtime capability and task/tool authorization, Contextual-Orchestrator owns LLM routing/capability truth, EgressWeave owns outbound-policy authority. Consumers use released versioned contracts/ACLs only; sibling source copies, mutable PR-head dependencies and cross-service SQL remain forbidden.

ADR-0009 remains Proposed while the isolated analyzer worker is not yet proven on a real capability-denying backend. A worker receipt or controller-side equality check proves consistency, not kernel enforcement. Runtime/controller facts and analyzer facts therefore remain separate authorities.

## Application-service and command-runtime isolation

The active design keeps immutable digest-pinned images, rootless execution, read-only rootfs, bounded writable storage, dropped capabilities, no-new-privileges, private user/PID/IPC/UTS/cgroup namespaces, non-root identity, CPU/RAM/PID/wall-time ceilings, deny-by-default networking, loopback-only publication, readiness, termination and cleanup as fail-closed invariants. Static Podman inspection is applied-configuration evidence only; effective LSM, cgroup, mount, namespace and egress claims still require runtime-observed evidence.

Draft #14 remains the command-runtime owner. Its focused repairs cover exact result schema, cidfile-backed ownership, post-create exact container identity, direct argv/ENTRYPOINT semantics, immutable image identity, applied namespace/tmpfs/time bounds, log/storage limits, output encoding, timeout termination, result chronology, and pre-payload attestation. Child PRs must adopt #14 fixes non-force rather than duplicate them.

Issue #71 / Draft #72 is the current prerequisite for recurring process-boundary failures. Draft #72 exact `900d0273625115f7bcc602d888baadded0caeb4f` has hosted verify, complete production coverage, branch coverage and hosted negative confinement GREEN, but dedicated positive-LSM remains queued and no qualifying approval is present. Downstream hosted jobs still rotate generic `BackendInvocationFailed` failures across unrelated tests because older ancestry collapses Spawn/Wait/Capture details. Retry, sleep, mutex, leaf-semantic weakening, or errno inference from that collapsed class are not authorized. The next causal step is dependency-safe adoption of #72 followed by a fresh specimen with the richer failure taxonomy.

Draft #106 owns issue #105's malformed-successful-create receipt case: if Podman stdout is malformed but the runtime-owned cidfile contains the acquired long ID, destructive cleanup must bind to that ID rather than generated name correlation. Its candidate remains blocked by the inherited process-boundary prerequisite and positive-LSM/review gates; it must not be merged around #71/#72.

## Artifact analysis

The active artifact-analysis foundation preserves immutable SHA-256 artifact identity, bounded ingestion, deterministic format classification/evidence ordering, analyzer failure attribution, and fail-closed refusal to execute externally supplied analyzer code in the controller process. A real external analyzer must cross an `AnalyzerWorkerExecutionPort` backed by capability-denying infrastructure and return controller-verifiable worker/analyzer/artifact/policy/lifecycle evidence.

Draft #70 provides the backend-neutral worker port while keeping reusable isolation/resource/lifecycle values in Core `sandbox_execution`. Worker-specific result semantics remain in Supporting `artifact_analysis`. Worker-dependent descendants stay Draft until #14→#18→#70 ancestry is dependency-safe and exact-head evidence is reacquired.

### #77 / #78 — analyzer evidence authority

Causal RED exact `ab919126a1ada3a039fb1f8979bdb66300469d91`, CI `34306038425`, verify `102322814270`, proved that an untrusted analyzer could label an otherwise-valid finding as controller/runtime-owned `ArtifactIdentity`. Minimum production repair `31dc17771b08e2535e33c8e0861c15eeb5113d4f` rejects `ArtifactIdentity | PolicyBoundary` in `AnalyzerWorkerFinding::validate` while retaining analyzer-owned static evidence.

Exact `fea4594468ad273599d97040d71e07b2d4e67faf`, CI `34314242415`, verify `102347001047`, executed both producer-authority regressions GREEN, repository/fmt prerequisites GREEN, and all five CI credential-contract tests GREEN including `persist-credentials: false`. The verify job later failed in an inherited command-runtime cleanup test with `BackendInvocationFailed { operation: "backend_security_info" }`; this is #71/#72 evidence, not a failure of the worker evidence ACL.

CodeRabbit generated no actionable code finding on `fea459...` but reported 33.33% docstring coverage across 15 touched functions. Repository policy requires 100% owned-production rustdoc, so documentation-only `c6b9a93762d62bad3f89286285932bf594d6f3c5` documents the previously undocumented analyzer identity/request/finding/outcome validators and bounded-text/SHA-256 predicates without changing semantics. Doctoring `1ac2bbef4d773ad10fc992359897d824d2c630b9` records the full chain. The current #78 head will move again with this baseline synchronization; fresh exact-head CI/review is required and predecessor broad GREEN is not claimed.

### #79 / #80 — runtime exit versus semantic outcome

Current exact `af1bf102ee9cee4c0ced6c41785940e3c357999b`, CI `34314399678`, preserves production `f570f5fd...` and edge hardening `de4f89ca...`: `AnalyzerWorkerOutcome::Completed` requires observed zero runtime exit, non-zero runtime exit cannot be presented as Completed, and a zero runtime exit does not erase an explicit semantic `Failed` outcome. Branch coverage executed all three focused exit/outcome regressions GREEN and the CI credential-disposal contract GREEN. A later inherited command-runtime ENTRYPOINT test fails on stale ancestry, while another hosted lane rotates `backend_security_info` into application-service ownership. #80 must adopt canonical #14/#18/#70 prerequisites rather than reimplement command semantics.

CodeRabbit review capacity was exhausted for the current #80 head; absence of a fresh external review is not approval.

### #81 / #82 — exact worker cleanup identity

Current exact `ffaf3b646371895f6304e8b72b1b03e1c0f88ef0`, CI `34313855324`, preserves production `1719eeca0c634b0fa0b8fecec52215612813dfcc`. Branch coverage executed all three cleanup-identity regressions GREEN: completed cleanup for the exact worker is admitted, cleanup for a different worker is rejected, and incomplete cleanup for the exact worker is rejected. Adjacent Core worker boundary/identifier/outcome/port/isolation suites and all five CI credential-contract tests also pass. The same job later rotates `BackendInvocationFailed { operation: "backend_security_info" }` into a command-runtime wall-time/kill test. Positive-LSM remains queued. Draft review has been manually requested but no qualifying review submission exists.

## CI, coverage and release authority

Canonical CI uses explicit supported hosted runners for ordinary checks and `persist-credentials: false` for every checkout. The dedicated positive confinement lane requires `[self-hosted, linux, cwl-hostile-workload, selinux]`; hosted rootless/AppArmor negative evidence cannot substitute for positive effective isolation. Owned production rustdoc, tests and edge-case coverage are required at 100%; statement/function/region/branch coverage must be exact for the unchanged candidate where tooling exposes those dimensions.

No active worker child is merge-ready. #78/#80/#82 have focused semantic GREEN evidence, but their whole heads remain blocked by stale command/runtime ancestry, the #71/#72 process-boundary prerequisite, dedicated positive-LSM, qualifying review/security gates and protected integration. Do not force-rebase, force-push, retry-mask or transfer predecessor checks. Restack descendants by ordinary merge/adoption and preserve every child-owned test/fixture/contract/evidence delta.

Protected release authority still requires one unchanged integrated `develop` head with exact native CI, full tests/Clippy/rustdoc, complete coverage, real rootless effective isolation, positive LSM/seccomp/capability/resource/network/cleanup evidence, central CodeQL/dependency/security gates, qualifying approval and resolved threads. Only then may version/CHANGELOG/tag/package/GitHub Release, SPDX SBOM, provenance, checksums/signatures where supported, reproducibility, recovery and rollback evidence be published. Mutable PR heads are never consumer authority.

## Buyer-visible gaps and next bounded actions

1. Integrate #72's bounded process-invocation taxonomy into #14 by ordinary non-force adoption, reacquire a failing specimen, and repair only the concrete Spawn/Wait/Capture cause that remains. Preserve positive-LSM as a separate release gate.
2. Once #14 is admissible, repair #18's divergence through ordinary integration, then restack #70 and worker children #78/#80/#82 without dropping their focused semantic tests, CI credential contract, doctoring or DDD ownership decisions.
3. Reacquire exact-head full workspace GREEN for #78/#80/#82 after the prerequisite restack. #78 additionally must prove the rustdoc repair under the repository's 100% requirement and fresh review; #80/#82 need fresh qualifying review rather than rate-limit/draft-skip metadata.
4. Implement a concrete capability-denying analyzer worker adapter only after the port/foundation is stable. Real acceptance must deny host loopback/external network, ambient credentials/environment, broad host filesystem/runtime sockets and uncontrolled subprocess authority while proving CPU/RAM/PID/time/storage/output ceilings plus exact termination/cleanup.
5. Continue YARA-X/capa/Ghidra/LIEF and dynamic Linux/Windows artifact-analysis slices only behind that worker boundary; static evidence must never be promoted to observed runtime behavior.
6. Merge only after one unchanged integrated protected candidate satisfies all review/security/coverage/runtime gates, then publish the first immutable runtime release and hand only its released version/digest to consumers.