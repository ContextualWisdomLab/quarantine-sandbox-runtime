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

Initial test-bearing commit `06ee0a1264960843a8ecd93d0e5ecff247866b17` adds `tests/artifact_analysis_analyzer_provenance_red.rs`. Review-hardening commit `87f579311e992a5815e7887e83e050598d23902d` closes a false-GREEN by requiring identical analyzer configuration/input to retain identical job identity and semantic evidence across independent engine instances. Commit `c32c2c489356213e70360087851b6c08df855c61` closes two additional false-GREEN paths: changing only the job digest while leaving serialized analyzer `producer_id` ambiguous, and deriving purported provenance circularly from the analyzer findings themselves.

The hardened fixture now has three analyzer implementations sharing one display `analyzer_id`:

1. version one emits semantic result `alpha`;
2. version two emits semantic result `beta`;
3. version three is a distinct implementation but intentionally emits the same semantic result as version one.

Current production collapses all three onto the same deterministic job input and the same serialized analyzer `producer_id`. The third fixture matters because output hashing is not provenance: two different executable/configuration identities can legitimately emit identical findings and still must remain attributable to what actually ran.

The RED permits two future outcomes:

1. fail closed before execution when stable analyzer provenance is unavailable; or
2. admit analyzers only when a stable versioned provenance identity makes materially different implementations/configurations distinct while repeated identical provenance/input remains stable.

The RED rejects asymmetric admission, random/non-repeatable identity, different evidence under one identical job identity, job-only provenance that leaves serialized producer identity ambiguous, and provenance inferred from emitted findings.

## Alternatives

### Keep `analyzer_id` as provenance

Rejected. A human-readable stable producer code is useful Ubiquitous Language but does not prove executable semantics. The public trait allows unrelated implementations to claim the same value.

### Reuse `RuntimeManifest.source_revision`

Rejected. Runtime source revision and analyzer source/package/configuration are different authorities. Overloading one field would become false when analyzers are independently released or externally packaged.

### Derive identity from findings or result digests

Rejected. Evidence is an output of analyzer execution, not prior proof of which analyzer artifact/configuration was authorized and invoked. Distinct analyzers can emit identical results, and a compromised analyzer can choose outputs adversarially. Result-derived identity is therefore circular and cannot support pre-invocation admission or supply-chain attribution.

### Derive identity from Rust `TypeId`, pointer/address, process-local randomness, or source paths

Rejected. Those values are not stable auditable supply-chain provenance across builds/processes and do not establish an immutable analyzer artifact/configuration. The hardened RED requires repeated identical configuration/input to retain identical identity, so random salting cannot satisfy the contract.

### Versioned analyzer provenance identity

Selected direction after causal RED. `artifact_analysis` should own a stable provenance value that can bind, as applicable, the immutable analyzer package/artifact digest, analyzer semantic/version identifier, approved configuration/profile identity, and exact worker/runtime provenance. The worker execution port defined by ADR-0009 consumes this identity; infrastructure adapters do not redefine it.

## Smallest causal GREEN

After the hardened RED executes for the intended collision:

- introduce an explicit analyzer provenance contract rather than inferring executable identity from a display ID or findings;
- validate provenance before analyzer/worker invocation;
- bind provenance into deterministic job identity and attributable serialized producer/worker evidence;
- preserve analyzer provenance across serialization so the consumer can verify which analyzer semantics produced the evidence;
- keep repeated identical provenance/request/artifact/policy/runtime inputs deterministic across process/engine instances;
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
        -> attributable serialized producer/worker evidence
        -> consumer-verifiable evidence bundle
```

## Release evidence

Before analyzer provenance can support an immutable release claim, one unchanged integrated protected candidate must prove:

- causal RED then focused/full GREEN for provenance collision, repeat determinism, producer attribution, and same-output/different-producer guards;
- exact analyzer package/artifact digest and approved configuration bound to the worker invocation before findings are accepted;
- exact worker isolation evidence from #49 and bounded result ingestion from #50;
- truthful dynamic execution semantics from #52 where applicable;
- schema/compatibility tests for serialized provenance;
- 100% owned production rustdoc/test/edge/statement/function/region/branch coverage where tooling exposes it;
- exact-head security/SBOM/provenance/reproducibility/review gates and protected-branch integration.

## References

SLSA Community. (2025). *Build: Provenance* (SLSA Version 1.2). https://slsa.dev/spec/v1.2/build-provenance

Scarfone, K., Souppaya, M., & Dodson, D. (2022). *Secure Software Development Framework (SSDF) Version 1.1: Recommendations for mitigating the risk of software vulnerabilities* (NIST Special Publication 800-218). National Institute of Standards and Technology. https://doi.org/10.6028/NIST.SP.800-218

SLSA provenance distinguishes producer/build identity and resolved inputs needed to verify how an artifact was produced. NIST SSDF PS.3.2 requires provenance data for software components to be collected, safeguarded, maintained, and updated when components change. These references support the provenance model; repository-specific RED/GREEN evidence remains the acceptance authority.
