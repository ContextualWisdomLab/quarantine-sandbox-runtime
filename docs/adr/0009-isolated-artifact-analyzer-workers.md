# ADR 0009: Isolated artifact-analyzer workers

- **Status:** Proposed
- **Date:** 2026-09-06

This ADR remains Proposed while issue #49 and Draft PR #18 are unmerged and the checked-in host-capability RED has not executed for its intended cause. Acceptance requires protected integration plus exact-head real-worker isolation, cleanup, coverage, security, SBOM, provenance, and review evidence.

## Context

`artifact_analysis` is a Supporting bounded context that accepts hostile immutable bytes and assembles attributable evidence. The current `AnalysisEngine::analyze_bytes` invokes public `StaticAnalyzer` trait objects directly inside the quarantine runtime process. An analyzer can therefore inherit the controller process's ambient socket, environment, filesystem, child-process, and runtime-socket authority before the engine emits `network_access_performed=false`, `credentials_available=false`, and `dynamic_execution_performed=false`.

That is incompatible with the repository threat model, which treats a compromised analyzer as an adversary, and with the product invariant that static evidence must not be mislabeled as observed isolation behavior. A Rust trait boundary is a software composition boundary, not an OS capability boundary.

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

The existing public `StaticAnalyzer` trait may remain temporarily for foundation tests and trusted development-only analyzers, but arbitrary in-process implementations are not eligible to produce production isolation claims. Any wire/API compatibility change required to distinguish trusted in-process fixtures from isolated production workers must be versioned explicitly.

## Backend direction

Rootless Podman is the first intended P0 worker adapter because this repository already owns that infrastructure boundary. The domain port must not depend on Podman types or CLI syntax. Stronger gVisor/containerd or VM-backed profiles may satisfy the same port when their evidence semantics meet or exceed the policy.

The first implementation must reuse Core isolation semantics rather than create a second artifact-analysis-specific sandbox policy. Shared Kernel is limited to stable backend-neutral isolation concepts; analyzer result contracts remain owned by `artifact_analysis`.

## Alternatives

### Keep arbitrary analyzers in-process and change evidence booleans

Rejected. More candid telemetry does not remove host authority from hostile or vulnerable analyzer code.

### Rely on analyzer author conventions, panic catching, or result validation

Rejected. These improve software hygiene or failure representation but do not deny sockets, environment, host files, runtime sockets, or subprocess authority.

### Make `artifact_analysis` call Podman directly

Rejected. It would couple a Supporting context to one infrastructure adapter, duplicate Core isolation semantics, and make backend replacement alter domain code.

### Use a subprocess without an enforceable OS capability boundary

Rejected as the P0 production baseline. Process separation alone does not prove denied network/filesystem/environment/runtime-socket authority.

## Verification and acceptance

Issue #49's `tests/artifact_analysis_host_capability_red.rs` is the first causal gate: an analyzer attempting to reach a runtime-host loopback listener must not succeed. Production changes follow only after that RED executes for the current in-process cause.

Acceptance of this ADR requires, on one unchanged integrated protected source identity:

- exact artifact/analyzer/policy/worker provenance;
- real worker E2E proving denied host-loopback/external network and ambient credential/environment access;
- bounded CPU/RAM/PID/time/storage/output with hostile exhaustion fixtures;
- no broad host filesystem, runtime socket, device, or uncontrolled subprocess authority;
- attributable worker failure and leak-free termination/cleanup/recovery evidence;
- 100% owned production statement/function/region/branch and edge-case coverage where tooling exposes it, public rustdoc, SAST/security/dependency gates, SBOM/provenance/reproducibility, required review, and protected integration.

Queued, skipped, predecessor, in-process-only, or analyzer-self-reported evidence cannot promote this ADR to Accepted.

## References

Open Container Initiative. (2025). *Open Container Initiative Runtime Specification version 1.3.0*. https://specs.opencontainers.org/runtime-spec/?v=v1.3.0

Souppaya, M., Morello, J., & Scarfone, K. (2017). *Application container security guide* (NIST Special Publication 800-190). National Institute of Standards and Technology. https://doi.org/10.6028/NIST.SP.800-190
