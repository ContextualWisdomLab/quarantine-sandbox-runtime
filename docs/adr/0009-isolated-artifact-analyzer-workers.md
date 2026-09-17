# ADR 0009: Isolated artifact-analyzer workers

- **Status:** Proposed
- **Date:** 2026-09-06

Issue #49's host-capability RED has now executed for its intended cause on descendant exact head `70b49a5020037165fae9f68edcf4762a2c5b1055`, native CI run `34018611889`, coverage job `101446928192`: an externally supplied `StaticAnalyzer` reached the runtime host's loopback listener before the controller emitted no-network evidence. The first production repair therefore fails closed before invoking externally supplied analyzer code. This ADR remains Proposed until an enforceable isolated-worker port lands and protected integration has exact-head real-worker isolation, cleanup, coverage, security, SBOM, provenance, and review evidence.

## Context

`artifact_analysis` is a Supporting bounded context that accepts hostile immutable bytes and assembles attributable evidence. The predecessor `AnalysisEngine::analyze_bytes` invoked public `StaticAnalyzer` trait objects directly inside the quarantine runtime process. An analyzer could therefore inherit the controller process's ambient socket, environment, filesystem, child-process, and runtime-socket authority before the engine emitted `network_access_performed=false`, `credentials_available=false`, and `dynamic_execution_performed=false`.

Native CI reproduced that defect on 2026-09-06. The host-capability regression's analyzer successfully connected to a loopback listener in the controller host process and panicked with `static analyzer reached a runtime-host network socket; the host-process trait call is not an isolation boundary`. This is causal execution evidence, not a static design inference.

That behavior is incompatible with the repository threat model, which treats a compromised analyzer as an adversary, and with the product invariant that static evidence must not be mislabeled as observed isolation behavior. A Rust trait boundary is a software composition boundary, not an OS capability boundary.

NIST SP 800-190 treats container isolation/resource controls as runtime security mechanisms. OCI Runtime Specification 1.3.0 provides a current standard execution model for isolated runtime bundles. These sources support an infrastructure boundary; they do not make any particular backend sufficient without observed evidence.

## Decision

Introduce a backend-neutral **Analyzer Worker Execution** port between `artifact_analysis` orchestration and executable analyzer code.

Responsibility remains split by bounded context:

- `artifact_analysis` owns analysis request/profile semantics, analyzer identity, immutable artifact identity, normalized findings/failures, deterministic evidence ordering, and completeness/disposition.
- `sandbox_execution` owns backend-neutral isolation policy, resource bounds, runtime identity, termination, cleanup, and isolation evidence requirements.
- `infrastructure` owns concrete Podman/gVisor/containerd/VM worker adapters and platform DTO/CLI translation.
- AppGuardrail remains static scan/SARIF authority; Wardnet remains verdict/incident authority; Noema remains admission/activation authority.

A production analyzer worker receives only the exact immutable artifact/input required for the selected analyzer and bounded scratch/output channels. The standard worker profile denies ambient credentials and arbitrary environment, network including host loopback, broad host filesystem, runtime sockets, devices, privilege escalation, and uncontrolled child-process authority. CPU, RAM, PID, wall time, temporary storage, and output are bounded.

Isolation evidence is produced by the runtime/backend boundary and bound to the exact worker identity, analyzer identity/version or immutable digest, artifact SHA-256, policy identity, termination result, and cleanup result. Analyzer self-report is never authority for its own confinement.

Analyzer execution failure or unverifiable isolation yields attributable failure/inconclusive analysis. It must never be rewritten as `Completed` with hard-coded no-network/no-credential/no-execution assertions.

As the first causal GREEN increment, `AnalysisEngine::new` keeps externally supplied analyzer configurations representable but marks their execution path as requiring an isolated worker. `AnalysisEngine::analyze_bytes` rejects that path before artifact-analyzer invocation with `IsolatedAnalyzerWorkerRequired`. Runtime-owned bundled non-executing analyzers use the separate `with_bundled_static_analyzers` path. This removes the demonstrated host-capability exposure without pretending that a worker already exists.

This fail-closed increment is not the commercial endpoint. Arbitrary external analyzers remain unavailable until the worker port and a concrete capability-denying adapter land. The public `StaticAnalyzer` trait is therefore a compatibility/configuration contract during migration, not production authority to execute code in the controller process.

## Backend direction

Rootless Podman is the first intended P0 worker adapter because this repository already owns that infrastructure boundary. The domain port must not depend on Podman types or CLI syntax. Stronger gVisor/containerd or VM-backed profiles may satisfy the same port when their evidence semantics meet or exceed the policy.

The first worker implementation must reuse Core isolation semantics rather than create a second artifact-analysis-specific sandbox policy. Shared Kernel is limited to stable backend-neutral isolation concepts; analyzer result contracts remain owned by `artifact_analysis`.

## Alternatives

### Keep arbitrary analyzers in-process and change evidence booleans

Rejected. More candid telemetry does not remove host authority from hostile or vulnerable analyzer code.

### Rely on analyzer author conventions, panic catching, or result validation

Rejected. These improve software hygiene or failure representation but do not deny sockets, environment, host files, runtime sockets, or subprocess authority.

### Fail closed before external analyzer invocation

Selected as the first causal production repair after the executed RED. It immediately removes the demonstrated controller-host capability exposure while preserving the future worker contract. It is intentionally incomplete: it restores truthful safety by making the capability unavailable rather than claiming worker isolation that has not yet been implemented.

### Make `artifact_analysis` call Podman directly

Rejected. It would couple a Supporting context to one infrastructure adapter, duplicate Core isolation semantics, and make backend replacement alter domain code.

### Use a subprocess without an enforceable OS capability boundary

Rejected as the P0 production baseline. Process separation alone does not prove denied network/filesystem/environment/runtime-socket authority.

## Verification and acceptance

Issue #49's `tests/artifact_analysis_host_capability_red.rs` is the first causal gate. The predecessor failed exactly because the analyzer reached the runtime-host loopback socket. The repair is GREEN only when an exact-head run proves that externally supplied analyzer code is not invoked and the test no longer observes that connection.

Acceptance of this ADR still requires, on one unchanged integrated protected source identity:

- an Analyzer Worker Execution port with exact artifact/analyzer/policy/worker provenance;
- real worker E2E proving denied host-loopback/external network and ambient credential/environment access;
- bounded CPU/RAM/PID/time/storage/output with hostile exhaustion fixtures;
- no broad host filesystem, runtime socket, device, or uncontrolled subprocess authority;
- attributable worker failure and leak-free termination/cleanup/recovery evidence;
- 100% owned production statement/function/region/branch and edge-case coverage where tooling exposes it, public rustdoc, SAST/security/dependency gates, SBOM/provenance/reproducibility, required review, and protected integration.

Fail-closed rejection of external analyzers is a valid interim safety repair but does not promote the worker capability, this ADR, or artifact-analysis release status to Accepted. Queued, skipped, predecessor, in-process-only, or analyzer-self-reported evidence cannot promote this ADR to Accepted.

## References

Open Container Initiative. (2025). *Open Container Initiative Runtime Specification version 1.3.0*. https://specs.opencontainers.org/runtime-spec/?v=v1.3.0

Souppaya, M., Morello, J., & Scarfone, K. (2017). *Application container security guide* (NIST Special Publication 800-190). National Institute of Standards and Technology. https://doi.org/10.6028/NIST.SP.800-190
