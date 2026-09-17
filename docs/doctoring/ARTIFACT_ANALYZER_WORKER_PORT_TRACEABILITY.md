# Artifact Analyzer Worker Port Traceability

Issue #69 / Draft #70 defines the backend-neutral Analyzer Worker Execution port required by ADR-0009. A Rust type, matching receipt field, or worker self-report is contract evidence only; it is never sufficient proof that hostile analyzer code was actually confined.

## Current authority

- Parent artifact-analysis authority: #18 `c0647152ec052d82969b2ae078891e25e6d4d69a`.
- Causal host-capability predecessor: issue #49 / `70b49a5020037165fae9f68edcf4762a2c5b1055`, native CI `34018611889`, where externally supplied analyzer code reached controller-host loopback.
- Initial worker-port causal RED: exact `53fea75fdfeb82c80e4a7a82b6a69ed7b3403f03`, native CI `34068781915`, verify `101582191893`, failed after formatting with E0432 for the absent public `AnalyzerWorker*` contract.
- First implementation `4bc77fa89050f313f42ffccd0258bc107aeb0d17` was reviewed invalid because Supporting `artifact_analysis` duplicated reusable isolation/resource/lifecycle vocabulary and represented cleanup without exact-worker terminal evidence.
- Core-boundary RED `f76299c27cbeef57397d166c9b9e476c0933c749`, hardened by `368a7cafd3f7e92e4cce6ea598cfa9a9ca847dbf`, requires Core-owned worker budget/isolation/termination values and composition of canonical `VerifiedIsolationState`.
- Exact `0ecc123e900d8b6fb56d3a1dec0a5084c4b665cc`, native CI `34086376848`, verify `101631105900`, executed that hardened RED for its intended cause: formatting passed and `cargo test --locked --workspace --all-targets` failed with E0432 for missing `SandboxWorkerBudget`, `SandboxWorkerIsolationEvidence`, `SandboxWorkerTerminationEvidence`, and `SandboxWorkerTerminationState`.
- Minimum production repair `c7a05a3870164c8811790af5f4d81d46dc7dc43b` moved reusable worker budget/termination/isolation semantics to Core, composed `VerifiedIsolationState`, bound cleanup to terminal evidence for the same worker, and removed duplicate Supporting-context isolation values without adding a concrete backend.
- Review `5130709433` found that repair incomplete: seven P0 controls already represented by `VerifiedIsolationState` were not required to be positively verified before a worker receipt could be accepted.
- Test-only `32dbd922a431f8e8ced830c8c40e54259cbb1cdf` added `tests/artifact_analysis_worker_required_isolation_controls_red.rs`. Formatter prerequisites were repaired without semantic changes through `56534eaa767a1472057347640bf5711b24306b0e`, `e93a08e3c65b2026e8b75c728dfb4d27ee6d101c`, and `6c07f238d8f95d3927a0901b8488757adae1b16d`.
- Exact `6c07f238d8f95d3927a0901b8488757adae1b16d`, native CI `34121419565`, verify `101740094874`, executed the isolation-completeness RED for its intended semantic cause. Exact checkout, dependency lock, repository policy, coverage-parser tests, and `cargo fmt --check` all passed; `cargo test --locked --workspace --all-targets` then failed in `receipt_rejects_unverified_required_worker_isolation_controls` when `rootless` was `Unavailable`, with the assertion that the worker receipt must fail closed. Hosted negative rootless/AppArmor `101740094718` passed. Coverage/branch-coverage reached test execution and failed on the same RED family; dedicated positive-LSM `101740094545` remained unassigned.
- Minimum production repair `18e113eeda491280ed99cf7b2e72ba8ba1d3280c` extends Core `SandboxWorkerIsolationEvidence::boundary_violation()` only: `rootless`, `read_only_root_filesystem`, `all_capabilities_dropped`, `no_new_privileges`, `isolated_user_namespace`, `seccomp_enforced`, and `lsm_enforced` must each be `Verified`. Existing egress/resource/credential/worker-specific capability checks remain unchanged, and `loopback_only_publication=NotApplicable` remains valid for the analyzer-worker profile.

Transient queued/running workflow identifiers are kept in PR/Issue operational metadata. Historical workflow identifiers appear here only after an unchanged semantic head actually executes the relevant gate. The production repair above is a candidate, not GREEN, until its exact descendant including this doctoring record passes the full gate.

## DDD and contract boundary

`artifact_analysis` owns analyzer identity/version/digest, immutable artifact binding, policy reference, normalized finding/failure semantics, evidence ordering, and analysis completeness/disposition. It composes Core isolation values but must not create a second sandbox policy/resource/lifecycle model.

`sandbox_execution` owns backend-neutral isolation policy, worker resource bounds, runtime identity, effective-control evidence, terminal-state semantics, cleanup semantics, and invariants binding lifecycle evidence to one exact runtime-owned worker. `VerifiedIsolationState` is the canonical shared effective-control representation. Worker-specific Core evidence may add worker identity, host-loopback/filesystem/runtime-socket/subprocess observations, termination and cleanup facts that are not represented there; it must not copy shared controls into a parallel model.

`infrastructure` owns Podman/gVisor/containerd/VM translation, enforcement and runtime observation. AppGuardrail remains static-scan/SARIF authority, Wardnet verdict/incident authority, and Noema admission/activation authority.

A dedicated `SandboxWorkerBudget` is distinct from service-oriented `ResourceRequest`: cumulative CPU time in milliseconds is not CPU-rate millicores. The worker request binds immutable analyzer implementation identity, exact artifact identity, policy identity, immutable isolation-policy SHA-256 and Core worker budget. The receipt binds those inputs to runtime-owned worker/backend/policy/budget/lifecycle evidence, canonical effective-isolation state and bounded normalized output.

## Validation invariants

Controller-side receipt validation must reject:

- analyzer/artifact/policy/isolation-policy/budget contradictions;
- malformed or oversized worker/backend/version/digest/outcome data;
- non-terminal termination evidence or termination for another worker;
- cleanup uncertainty;
- ambient credentials;
- external-egress or resource-limit evidence that is not `Verified`;
- required P0 worker controls that are not `Verified`: rootless execution, read-only root filesystem, all capabilities dropped, no-new-privileges, isolated user namespace, seccomp and selected LSM;
- observed host-loopback, broad host-filesystem, runtime-socket or uncontrolled-subprocess authority.

The executed RED proved the previous candidate omitted the seven canonical checks. The current minimum production candidate performs those checks in Core rather than duplicating statuses in `artifact_analysis`; service-only loopback publication remains outside the worker requirement.

Request/receipt equality, applied budget equality and policy-digest equality remain anti-contradiction checks. They do not establish confinement. A concrete adapter must later prove actual network/environment/filesystem/runtime-socket/subprocess/resource/termination/cleanup behavior tied to the exact immutable source, worker, analyzer, artifact and policy identities.

## Dependent result authority

Issue #77 / Draft #78 is a separate child: an untrusted worker must not mint controller/runtime-owned `ArtifactIdentity` or `PolicyBoundary` merely by selecting a repository-wide `EvidenceKind`. `FileFormat` and `StaticCapability` remain deliberate static analyzer evidence. `RuntimeBehavior` / `NetworkAttempt` remains issue #52/#53 authority. Issue #79 / Draft #80 owns Completed/outcome binding to runtime-observed exit status; issue #81 / Draft #82 owns exact cleanup worker identity. None of those result/lifecycle refinements may be folded into this Core isolation-completeness gate.

## Release implications

Port-level GREEN does not complete issue #49 or ADR-0009. Release still requires a concrete capability-denying adapter, real rootless runtime E2E, effective CPU/RAM/PID/time/storage/output ceilings, exact-worker termination and leak-free cleanup, positive selected-LSM evidence, full owned production coverage, security/review gates, protected-head integration, SBOM/provenance/reproducibility/rollback and immutable publication.

## Standards and research

NIST SP 800-190 treats namespace/resource isolation as runtime security mechanisms rather than interface declarations. OCI Runtime Specification v1.3.0 defines the low-level runtime configuration/behavior contract but does not make requested configuration equivalent to effective enforcement. Van't Hof and Nieh (2022) likewise show why container isolation depends on enforcement at the operating-system/runtime boundary. These sources support keeping API consistency, runtime observation and enforcement evidence distinct.

### References

Open Container Initiative. (2025, November 4). *OCI runtime specification v1.3.0 release notice*. https://opencontainers.org/release-notices/v1-3-0-runtime-spec/

Souppaya, M., Morello, J., & Scarfone, K. (2017). *Application container security guide* (NIST Special Publication 800-190). National Institute of Standards and Technology. https://doi.org/10.6028/NIST.SP.800-190

Van't Hof, A., & Nieh, J. (2022). BlackBox: A container security monitor for protecting containers on untrusted operating systems. In *16th USENIX Symposium on Operating Systems Design and Implementation (OSDI 22)* (pp. 683–700). USENIX Association. https://www.usenix.org/conference/osdi22/presentation/vant-hof
