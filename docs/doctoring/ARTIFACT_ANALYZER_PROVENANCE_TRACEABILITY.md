# Artifact Analyzer Provenance Traceability

Status: RED-only design evidence for issue #54. Production behavior is unchanged.

## Problem

`artifact_analysis` currently identifies a static analyzer only by `StaticAnalyzer::analyzer_id()`. `AnalysisEngine::deterministic_job_id()` hashes that human-readable identifier together with request/profile/artifact/policy/runtime source revision, and normalized findings use the same identifier as `producer_id`.

A public trait permits two materially different analyzer implementations or configurations to return the same `analyzer_id`. Those analyzers can then emit different completed evidence under one identical deterministic `analysis_job_id` and the same producer identity. Runtime `source_revision` is not an analyzer implementation identity when analyzers are independently versioned, packaged, loaded, or configured.

This is an evidence-provenance defect, not a maliciousness-verdict concern. Wardnet retains verdict/incident authority. `artifact_analysis` owns analyzer/evidence semantics; `sandbox_execution` owns backend-neutral worker isolation/lifecycle evidence; Podman/gVisor/containerd/VM remain infrastructure adapters under Proposed ADR-0009.

## Current authority

Parent artifact-analysis Draft #18 is `178cd90dd298fd3c02c78e77c285d1893fa83617`.

On that source:

- `StaticAnalyzer` exposes `analyzer_id()` and `analyze()` only;
- `AnalysisEngine::new()` validates analyzer identifier syntax and rejects duplicate IDs only within one engine instance;
- `deterministic_job_id()` hashes request ID, profile, artifact SHA-256, policy ID, runtime source revision, and analyzer ID strings;
- `EvidenceRecord.producer_id` carries the analyzer ID but no immutable analyzer implementation/configuration identity;
- `RuntimeManifest.source_revision` identifies runtime source, not an independently versioned analyzer artifact.

The PRD requires attributable evidence and an auditor-verifiable execution identity. The TRD states evidence identifiers/order are deterministic for the same request/configuration/bytes. A configuration dimension that can change evidence semantics must therefore participate in provenance rather than be collapsed into a reusable display identifier.

## RED

Test-bearing commit `06ee0a1264960843a8ecd93d0e5ecff247866b17` adds `tests/artifact_analysis_analyzer_provenance_red.rs`.

The fixture constructs two otherwise-identical `StaticOnly` engines with the same request, artifact bytes, policy ID, runtime source revision, and analyzer ID. The two analyzer implementations deliberately emit different valid `StaticCapability` evidence. Current production should produce the same `analysis_job_id` because the analyzer implementation is not an input to deterministic identity.

The RED permits two future outcomes:

1. fail closed before execution when stable analyzer provenance is unavailable; or
2. admit both analyzers only when a stable versioned provenance identity makes their deterministic job identities distinct.

The RED rejects asymmetric admission and rejects different evidence under one identical job identity.

## Alternatives

### Keep `analyzer_id` as provenance

Rejected. A human-readable stable producer code is useful Ubiquitous Language but does not prove executable semantics. The public trait allows unrelated implementations to claim the same value.

### Reuse `RuntimeManifest.source_revision`

Rejected. Runtime source revision and analyzer source/package/configuration are different authorities. Overloading one field would become false when analyzers are independently released or externally packaged.

### Derive identity from Rust `TypeId`, pointer/address, process-local randomness, or source paths

Rejected. Those values are not stable auditable supply-chain provenance across builds/processes and do not establish an immutable analyzer artifact/configuration.

### Versioned analyzer provenance identity

Selected direction after causal RED. `artifact_analysis` should own a stable provenance value that can bind, as applicable, the immutable analyzer package/artifact digest, analyzer semantic/version identifier, approved configuration/profile identity, and exact worker/runtime provenance. The worker execution port defined by ADR-0009 consumes this identity; infrastructure adapters do not redefine it.

## Smallest causal GREEN

After `06ee0a...` executes for the intended collision:

- introduce an explicit analyzer provenance contract rather than inferring executable identity from a display ID;
- validate provenance before analyzer/worker invocation;
- bind provenance into deterministic job identity and attributable evidence;
- preserve analyzer provenance across serialization so the consumer can verify which analyzer semantics produced the evidence;
- keep request/artifact/policy/runtime identity and analyzer identity as separate fields even when all participate in a digest;
- version the JSON/Rust contract if the existing `1.0.0` wire semantics cannot be extended compatibly;
- keep #49 capability isolation, #50 bounded result transport, and #52 truthful dynamic execution as independent gates.

A provenance digest is not isolation evidence. A contained worker with unknown analyzer provenance is not auditable, and a fully identified analyzer running with host authority is not contained.

## Evidence chain

```text
request/profile
    + immutable artifact SHA-256
    + operator policy identity
    + runtime build/source identity
    + analyzer immutable artifact/package identity
    + analyzer semantic/version identity
    + approved analyzer configuration identity
    + exact worker/runtime provenance
        -> deterministic analysis-job identity
        -> attributable evidence records
        -> consumer-verifiable evidence bundle
```

## Release evidence

Before analyzer provenance can support an immutable release claim, one unchanged integrated protected candidate must prove:

- causal RED then focused/full GREEN for the provenance collision;
- exact analyzer package/artifact digest and approved configuration bound to the worker invocation;
- exact worker isolation evidence from #49 and bounded result ingestion from #50;
- truthful dynamic execution semantics from #52 where applicable;
- schema/compatibility tests for serialized provenance;
- 100% owned production rustdoc/test/edge/statement/function/region/branch coverage where tooling exposes it;
- exact-head security/SBOM/provenance/reproducibility/review gates and protected-branch integration.

## References

OpenSSF. (2025). *SLSA specification: Build provenance* (Version 1.2). https://slsa.dev/spec/v1.2/build-provenance

Souppaya, M., Scarfone, K., & Dodson, D. (2022). *Secure Software Development Framework (SSDF) Version 1.1: Recommendations for mitigating the risk of software vulnerabilities* (NIST Special Publication 800-218). National Institute of Standards and Technology. https://doi.org/10.6028/NIST.SP.800-218

SLSA provenance distinguishes the producer/build identity and resolved inputs needed to verify how an artifact was produced. NIST SSDF PS.3.2 requires provenance data for software components to be collected, safeguarded, maintained, and updated when components change. These references support the provenance model; repository-specific RED/GREEN evidence remains the acceptance authority.
