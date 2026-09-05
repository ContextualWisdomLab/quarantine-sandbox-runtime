# Artifact Analyzer Isolation Traceability

Status: Proposed security boundary; issue #49; production behavior unchanged.

## Problem

`AnalysisEngine::analyze_bytes` accepts public `StaticAnalyzer` trait objects and invokes `analyzer.analyze(&artifact)` directly in the quarantine runtime process. After those calls it emits `PolicyBoundary` and `RuntimeManifest` evidence with `network_access_performed=false`, `credentials_available=false`, and `dynamic_execution_performed=false`.

The current implementation has no OS capability boundary around analyzer code. The trait contract therefore cannot prove the emitted no-network/no-credential/no-execution statements: an analyzer can use the host process's sockets, environment, filesystem, child-process API, or runtime sockets before the evidence bundle is assembled. This is especially material because `docs/THREAT_MODEL.md` already models a compromised analyzer as an adversary.

## Constraints

- Artifact bytes remain immutable and SHA-256 identified.
- `artifact_analysis` remains a Supporting bounded context; quarantine-sandbox-runtime owns isolation/evidence, not Wardnet verdicts or AppGuardrail SARIF/static-scan authority.
- Static-profile analyzer execution must not gain Internet/loopback egress, consumer/provider credentials, arbitrary environment, broad host filesystem, runtime sockets, devices, or uncontrolled child-process authority.
- Analyzer CPU, RAM, PID, wall time, output and temporary storage must be bounded.
- Analyzer failure must remain attributable and fail closed/inconclusive; it must not become `Completed` with fabricated boundary evidence.
- A launch flag or a Rust type restriction is not live isolation evidence. Release claims require exact-worker runtime proof.

## Alternatives

### Keep in-process trait objects and correct the booleans

Rejected. Reporting `network_access_performed=true` after an unconstrained analyzer does not make hostile parsing safe and still exposes the runtime host. The product boundary is isolation, not merely more candid telemetry.

### Catch panic or validate analyzer output

Rejected as insufficient. Panic handling and result validation address failure representation, not ambient socket/environment/filesystem/process capability.

### Require analyzer authors to promise not to use host capabilities

Rejected. The threat model explicitly includes compromised/buggy analyzers; a convention is not an enforceable security boundary.

### Execute analyzer workers behind a capability-denying sandbox

Selected direction. The concrete backend may be rootless Podman, gVisor, a VM boundary, or an equivalent profile-specific worker, but the runtime must bind exact artifact identity, analyzer identity, policy, worker identity and observed isolation evidence before publishing no-network/no-credential/no-execution assertions.

## RED

Test-bearing authority: `tests/artifact_analysis_host_capability_red.rs` (issue #49).

The regression constructs an otherwise-valid `StaticOnly` engine with an analyzer whose `analyze()` attempts a TCP connection to a loopback listener owned by the test. The acceptance boundary is intentionally implementation-neutral:

1. the analyzer must not establish the runtime-host network connection;
2. the runtime may reject an unsafe in-process analyzer before invocation, or execute it in a worker where the host listener is unreachable;
3. if an evidence bundle is returned, its no-network statement must correspond to the enforced worker boundary rather than an assumption about trait behavior.

Current `AnalysisEngine` calls the analyzer directly, so the checked-in test is expected to RED when it executes.

## Smallest causal GREEN after executed RED

Introduce a production analyzer-execution port whose implementation creates an isolated worker with deny-by-default capabilities and bounded resources. Move arbitrary analyzer code behind that port rather than adding socket-specific guards to `AnalysisEngine`. Bind worker evidence to the exact artifact SHA-256 and configured analyzer identity. Preserve the existing deterministic evidence assembly only after the worker result and isolation attestation are verified.

A compatibility-only in-process adapter, if retained for tests or trusted development tooling, must not be eligible to emit production `PolicyBoundary` evidence claiming denied capabilities.

## Release evidence

Completion requires real sandbox E2E demonstrating denied network and ambient credential/environment access, bounded resource enforcement, worker cleanup/recovery, exact artifact/analyzer provenance, current-head coverage/security/SBOM/provenance, and positive confinement evidence. Fake or in-process tests are contract evidence only.

## References

Rust Project Developers. (2026). *std::process—Module process (Rust 1.98.1).* https://doc.rust-lang.org/std/process/

Souppaya, M., Morello, J., & Scarfone, K. (2017). *Application container security guide* (NIST Special Publication 800-190). National Institute of Standards and Technology. https://doi.org/10.6028/NIST.SP.800-190
