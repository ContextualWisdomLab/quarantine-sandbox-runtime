# Artifact Analyzer Worker Port Traceability

Issue #69 / Draft #70 defines the backend-neutral Analyzer Worker Execution port required by ADR-0009. This record distinguishes contract consistency from effective runtime confinement; a Rust type, matching receipt field, or worker self-report is never sufficient isolation evidence.

## Current authority

- Parent artifact-analysis authority: #18 `c0647152ec052d82969b2ae078891e25e6d4d69a`.
- Causal host-capability predecessor: issue #49 / `70b49a5020037165fae9f68edcf4762a2c5b1055`, native CI `34018611889`, coverage `101446928192`, where an externally supplied analyzer reached the controller-host loopback socket.
- First safety repair: #18 rejects externally supplied in-process analyzers with `IsolatedAnalyzerWorkerRequired`; this is fail-closed safety, not worker capability.
- Initial worker-port RED: `7b01e5ee349a263eb03df082f3eead2f6b8a2872`.
- Runtime worker/backend identity hardening: `0cd3db3142d32c1086f6cbb942d31394d5e9efc7`.
- Requested/applied CPU/RAM/PID/wall-time/scratch/output budget hardening: `5609fa1d13a1a83bbae91e22e11d45fa1ac66188`.
- Request-pinned isolation-policy SHA-256 hardening: `4118ce0ac93766d3ba7263009faefc7352e493f7`.
- Bounded identifier and exact lower-case SHA-256 hardening: `a0bd5d1ce729fd2d332c32ebdd05d9577a1d191a`.
- Per-item normalized worker outcome hardening: `fd2cf84d715380b87b4a1f97d8c5b76e0aa3ce35`; #50 remains owner of aggregate worker-result/output-byte admission before trusted-controller accumulation.
- Formatting prerequisites were repaired without production semantics through `53fea75fdfeb82c80e4a7a82b6a69ed7b3403f03`.
- Core-isolation reuse hardening: `368a7cafd3f7e92e4cce6ea598cfa9a9ca847dbf` requires worker evidence to compose the existing Core `VerifiedIsolationState` instead of duplicating its effective-control facts inside another Core worker type.

Exact `53fea75fdfeb82c80e4a7a82b6a69ed7b3403f03`, native CI `34068781915`, crossed the intended first port gate. Verify `101582191893` passed exact checkout, dependency lock, repository-policy validation, coverage-parser tests, and `cargo fmt --check`, then failed in `cargo test --locked --workspace --all-targets` with Rust E0432 because the public crate root lacked the `AnalyzerWorker*` port/value-object API. Hosted negative rootless/AppArmor `101582191903` passed. Coverage `101582191940` and branch coverage `101582191898` reached the same missing-port compilation boundary. Positive-LSM `101582191746` remained queued. This is causal RED authority for introducing the minimum port contract; its results do not transfer to later heads.

Candidate `4bc77fa89050f313f42ffccd0258bc107aeb0d17` added the first backend-neutral implementation. Exact-head review then found two valid blocking defects against ADR-0009:

1. `artifact_analysis` defined `AnalyzerWorkerBudget` and `AnalyzerWorkerIsolationEvidence`, thereby owning reusable resource, backend, denied-capability, and cleanup semantics that ADR-0009 assigns to Core `sandbox_execution`. The worker CPU-time field also used milliseconds while existing Core `ResourceRequest.cpu_millicores` represents a CPU-rate request; retaining both as Supporting-context isolation vocabulary would blur distinct units and ownership.
2. `cleanup_completed: bool` was not coupled to a runtime-observed terminal state for the exact `worker_id`. A cleanup assertion without a bound terminal result cannot establish that the same worker stopped before its resources were released.

These are repair findings. `4bc77fa...` is not a GREEN authority.

Repair RED `f76299c27cbeef57397d166c9b9e476c0933c749` adds `tests/artifact_analysis_worker_core_boundary_red.rs`. It requires reusable worker resource/isolation/lifecycle values to be Core-owned as `SandboxWorkerBudget`, `SandboxWorkerIsolationEvidence`, `SandboxWorkerTerminationEvidence`, and `SandboxWorkerTerminationState`; rejects duplicate `AnalyzerWorkerBudget` / `AnalyzerWorkerIsolationEvidence` definitions in the Supporting context; and requires receipt validation to fail when termination is non-terminal or belongs to a different worker identity.

Review of that RED found a second DDD duplication risk before production repair: merely moving `AnalyzerWorkerIsolationEvidence` into Core as a new worker-specific structure could duplicate Core's already-canonical `VerifiedIsolationState` facts for credentials, egress, resource verification, rootless execution, capability drop, seccomp, and LSM enforcement. Test hardening `368a7cafd3f7e92e4cce6ea598cfa9a9ca847dbf` therefore requires `SandboxWorkerIsolationEvidence` to compose `VerifiedIsolationState`, treats service-only loopback publication as not applicable for the worker fixture, and adds fail-closed cases for unavailable Core egress verification and ambient credentials. Worker-specific evidence may add exact worker identity, host-loopback denial, host-filesystem/runtime-socket/subprocess controls, termination, and cleanup facts that are not yet represented by the shared state; it must not copy shared facts into a parallel model.

The earlier test-bearing runs `34072872833` and `34073054059` were cancelled after subsequent head movement and are not causal repair-RED evidence. Live queued/running job IDs are intentionally kept in PR/Issue operational metadata rather than versioned TRACEABILITY: committing a transient queue identifier changes the exact source head and thereby invalidates the identifier it was attempting to declare authoritative. This document records a repair-RED run only after an unchanged semantic test delta has actually executed for its intended cause. Production repair therefore remains unauthorized until an exact descendant preserving the Core-ownership, `VerifiedIsolationState` composition, and termination RED executes for those intended causes.

## DDD and contract boundary

`artifact_analysis` owns analyzer identity/version/digest, exact immutable artifact binding, policy selection reference, normalized analyzer finding/failure semantics, evidence ordering, and analysis completeness/disposition. It may compose Core isolation values but must not define a second sandbox policy/resource/lifecycle model.

`sandbox_execution` owns reusable backend-neutral isolation policy, worker resource bounds, runtime identity, effective-control evidence, terminal-state semantics, cleanup semantics, and the invariants that bind lifecycle evidence to one exact runtime-owned worker identity. `VerifiedIsolationState` remains the canonical shared representation of effective controls already modeled there; worker-specific Core evidence composes it rather than re-declaring those controls. A dedicated Core worker budget is acceptable where worker execution needs cumulative CPU time and bounded output that do not map one-to-one to the existing service-oriented `ResourceRequest`; it must be named and documented as a distinct unit/model rather than silently treating CPU milliseconds as CPU millicores.

`infrastructure` owns concrete Podman/gVisor/containerd/VM process and configuration translation, runtime observation, and evidence collection. AppGuardrail remains static-scan/SARIF authority; Wardnet remains verdict/incident authority; Noema remains admission/activation authority.

The artifact-analysis request must still bind immutable analyzer implementation identity, exact artifact identity, policy identity, expected immutable isolation-policy SHA-256, and the Core-owned worker budget. A worker receipt must bind those request inputs to runtime-owned worker/backend/policy/budget/lifecycle evidence, the canonical Core effective-isolation state, and normalized analyzer output. Controller-side validation must reject contradictions, malformed identity, unavailable required Core controls, ambient credentials, denied-capability violations, a non-terminal worker, termination evidence for another worker, cleanup uncertainty, and malformed per-item outcomes before downstream evidence ingestion.

A matching applied budget or policy digest is anti-contradiction evidence only. The runtime/backend must independently observe effective isolation and lifecycle state. Analyzer self-report cannot authorize its own confinement, termination, or cleanup.

## RED and minimum repair

The first missing-port RED has executed. The current repair RED demands Core-owned `SandboxWorker*` resource/lifecycle values while reusing `VerifiedIsolationState` for shared effective-isolation facts. The smallest causal repair, after this hardened RED executes for its intended causes, is to move reusable worker budget/identity/termination/cleanup semantics into `sandbox_execution`, compose the existing canonical isolation state in the worker evidence, change the analyzer request/receipt to consume those Core values, remove duplicate Supporting-context isolation structs, and reject unavailable required controls, ambient credentials, non-terminal termination, or wrong-worker termination evidence.

That repair must preserve the existing bounded/control-free identifier rules, exact 64-character lower-case hexadecimal SHA-256 rules, analyzer/artifact/policy/isolation-policy binding, requested/applied budget consistency, denied-capability checks, and bounded per-item finding/failure normalization. #50 remains the aggregate result-channel owner.

Port-level GREEN still does not complete #49 or ADR-0009. A concrete adapter must prove denied host-loopback/external network, ambient credential/environment, broad host filesystem/runtime sockets and uncontrolled subprocess authority; actual CPU/RAM/PID/time/storage/output ceilings; runtime-owned termination; cleanup and leak rejection. Those observations must be tied to one immutable integrated source identity and exact worker/analyzer/artifact/policy identities.

## Standards and research

NIST SP 800-190 remains NIST's final application-container security guide and treats container isolation/resource controls as runtime security mechanisms rather than interface conventions. The Open Container Initiative Runtime Specification current public release is v1.3.0, released 2025-11-04. These sources support a backend-neutral contract plus concrete enforcement evidence; neither makes a particular adapter sufficient without observation at the runtime boundary.

Peer-reviewed systems research supports the same separation. Van't Hof and Nieh (2022) show that conventional containers rely on operating-system enforcement and introduce a smaller security monitor to strengthen that trust boundary. The relevant implication here is not adoption of BlackBox; it is that API declarations and worker self-report do not substitute for enforcement at the actual isolation boundary.

### References

Open Container Initiative. (2025, November 4). *OCI runtime-spec v. 1.3.0 release notice*. https://opencontainers.org/release-notices/v1-3-0-runtime-spec/

Souppaya, M., Morello, J., & Scarfone, K. (2017). *Application container security guide* (NIST Special Publication 800-190). National Institute of Standards and Technology. https://doi.org/10.6028/NIST.SP.800-190

Van't Hof, A., & Nieh, J. (2022). BlackBox: A container security monitor for protecting containers on untrusted operating systems. In *16th USENIX Symposium on Operating Systems Design and Implementation (OSDI 22)* (pp. 683–700). USENIX Association. https://www.usenix.org/conference/osdi22/presentation/vant-hof
