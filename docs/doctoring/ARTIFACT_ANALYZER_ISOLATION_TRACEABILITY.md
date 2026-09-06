# Artifact Analyzer Isolation Traceability

Status: issue #49 causal RED executed; first fail-closed production repair under exact-head verification; isolated worker capability remains Proposed.

## Problem

The predecessor `AnalysisEngine::analyze_bytes` accepted public `StaticAnalyzer` trait objects and invoked `analyzer.analyze(&artifact)` directly in the quarantine runtime process. After those calls it emitted `PolicyBoundary` and `RuntimeManifest` evidence with `network_access_performed=false`, `credentials_available=false`, and `dynamic_execution_performed=false`.

The implementation had no OS capability boundary around analyzer code. The trait contract therefore could not prove the emitted no-network/no-credential/no-execution statements: an analyzer could use the host process's sockets, environment, filesystem, child-process API, or runtime sockets before the evidence bundle was assembled. This is especially material because `docs/THREAT_MODEL.md` already models a compromised analyzer as an adversary.

## Causal RED evidence

Test-bearing authority: `tests/artifact_analysis_host_capability_red.rs` (issue #49), introduced at `d67e16a241335b3d3ecd4e37cc80d9efc5200342` and preserved through the current #18 stack.

On descendant exact head `70b49a5020037165fae9f68edcf4762a2c5b1055`, native CI run `34018611889`, coverage job `101446928192` executed the regression on GitHub-hosted Ubuntu 24.04. The fixture's externally supplied analyzer successfully connected to a loopback listener in the runtime host process. The test failed with the exact causal assertion:

`static analyzer reached a runtime-host network socket; the host-process trait call is not an isolation boundary`

The same job had exact checkout proof for `70b49a...`; this is executed causal evidence rather than a queued/predecessor inference. The job stopped on the #49 RED before later artifact-analysis sibling REDs could execute.

## Constraints

- Artifact bytes remain immutable and SHA-256 identified.
- `artifact_analysis` remains a Supporting bounded context; quarantine-sandbox-runtime owns isolation/evidence, not Wardnet verdicts or AppGuardrail SARIF/static-scan authority.
- Static-profile external analyzer execution must not gain Internet/loopback egress, consumer/provider credentials, arbitrary environment, broad host filesystem, runtime sockets, devices, or uncontrolled child-process authority.
- Analyzer CPU, RAM, PID, wall time, output and temporary storage must be bounded once worker execution is enabled.
- Analyzer failure must remain attributable and fail closed/inconclusive; it must not become `Completed` with fabricated boundary evidence.
- A launch flag or a Rust type restriction is not live isolation evidence. Release claims require exact-worker runtime proof.

## Alternatives

### Keep in-process trait objects and correct the booleans

Rejected. Reporting `network_access_performed=true` after an unconstrained analyzer does not make hostile parsing safe and still exposes the runtime host. The product boundary is isolation, not merely more candid telemetry.

### Catch panic or validate analyzer output

Rejected as insufficient. Panic handling and result validation address failure representation, not ambient socket/environment/filesystem/process capability.

### Require analyzer authors to promise not to use host capabilities

Rejected. The threat model explicitly includes compromised/buggy analyzers; a convention is not an enforceable security boundary.

### Reject external analyzers before invocation

Selected as the first causal repair. The public compatibility constructor may still validate an external analyzer configuration, but `AnalysisEngine::analyze_bytes` must not call that implementation in the controller process. Until a worker exists it returns an attributable typed `IsolatedAnalyzerWorkerRequired` error instead. This removes the demonstrated ambient-host exposure without claiming worker isolation that does not yet exist.

### Execute analyzer workers behind a capability-denying sandbox

Selected commercial direction after the fail-closed repair. The concrete backend may be rootless Podman, gVisor, a VM boundary, or an equivalent profile-specific worker, but the runtime must bind exact artifact identity, analyzer identity, policy, worker identity and observed isolation evidence before publishing no-network/no-credential/no-execution assertions.

## Production repair candidate

The first production increment introduces an internal execution-path distinction:

- runtime-owned `with_bundled_static_analyzers` executes only the crate's bundled non-executing foundation analyzer;
- externally supplied analyzers configured through `AnalysisEngine::new` are marked `IsolatedWorkerRequired`;
- `analyze_bytes` checks that state before ingestion-side analyzer invocation and returns `AnalysisError::IsolatedAnalyzerWorkerRequired` without calling the external analyzer.

The host-capability regression intentionally constructs successfully and then attempts analysis, so a GREEN result demonstrates that the unsafe analyzer was not invoked. Existing integration tests are adjusted to use the explicit bundled path where they require foundation format evidence and to assert fail-closed external-analyzer admission where they previously exercised arbitrary in-process fixtures.

This is not a socket-specific guard and does not inspect analyzer IDs. It removes the whole externally supplied in-process execution class. It also does not silently promote external analyzers to `Completed`, `ToolFailure`, or no-network evidence.

## Remaining worker implementation

The next implementation must introduce the backend-neutral Analyzer Worker Execution port described by ADR-0009, then a concrete capability-denying adapter. A production analyzer worker receives only the exact immutable artifact/input and bounded scratch/output channels. Worker evidence must be runtime-generated and bound to exact artifact SHA-256, analyzer package/version or immutable digest, policy, worker identity, termination, and cleanup.

Result-channel limits (#50/#51), truthful dynamic execution/completeness (#52/#53), analyzer provenance (#54/#55), ToolFailure/disposition consistency (#56/#57), and artifact subject binding (#58/#59) remain separate downstream controls. The fail-closed #49 repair must not erase or falsely satisfy those contracts.

## Release evidence

Completion requires real sandbox E2E demonstrating denied host-loopback/external network and ambient credential/environment access, bounded resource enforcement, worker cleanup/recovery, exact artifact/analyzer provenance, current-head 100% owned production statement/function/region/branch coverage, security/SBOM/provenance, and qualifying review. Fake, in-process, queued, or predecessor evidence is contract evidence only.

## References

Open Container Initiative. (2025). *Open Container Initiative Runtime Specification version 1.3.0*. https://specs.opencontainers.org/runtime-spec/?v=v1.3.0

Souppaya, M., Morello, J., & Scarfone, K. (2017). *Application container security guide* (NIST Special Publication 800-190). National Institute of Standards and Technology. https://doi.org/10.6028/NIST.SP.800-190
