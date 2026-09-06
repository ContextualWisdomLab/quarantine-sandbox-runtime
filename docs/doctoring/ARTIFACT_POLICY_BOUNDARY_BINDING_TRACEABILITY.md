# Artifact Policy-Boundary Binding Traceability

Status: issue #62 checked-in RED pending causal execution. No production GREEN is authorized yet.

## Problem

`artifact_analysis` serializes three runtime-boundary facts twice in one `EvidenceBundle`: the top-level `RuntimeManifest` booleans and the runtime-owned `PolicyBoundary` record attributes `dynamic_execution_performed`, `network_access_performed`, and `credentials_available`.

On parent Draft #18 exact `994ba5fa5ceda20c622861efbff241294867dc39`, `AnalysisEngine::analyze_bytes()` emits the two representations consistently. `RuntimeManifest::validate()` checks the top-level booleans, while `EvidenceRecord::validate()` checks only generic attribute bounds and text. `EvidenceBundle::validate()` never compares the overlapping representations.

A reconstructed or deserialized receipt can therefore keep the top-level runtime manifest at `false/false/false`, change only one runtime-owned `PolicyBoundary` attribute to `true`, and remain syntactically valid under the current Rust `1.0.0` contract. That is an internal evidence-integrity defect: one receipt can present incompatible security-boundary facts depending on which representation a consumer reads.

## DDD ownership

`artifact_analysis` owns evidence normalization, runtime-attestation semantics, and receipt integrity. `sandbox_execution` owns the actual isolation, resource, lifecycle, termination, and cleanup facts that a future analyzer worker must prove. Infrastructure adapters translate those backend-neutral controls. AppGuardrail remains SAST/SARIF authority, Noema admission/activation authority, and Wardnet verdict/incident authority.

Issue #52 remains owner of dynamic-profile execution/completeness/`RuntimeBehavior` semantics. Issue #62 owns only agreement between duplicated runtime-boundary representations once those facts are present in a receipt. It does not authorize dynamic execution or weaken #49 worker isolation.

## Current implementation evidence

`AnalysisEngine::analyze_bytes()` emits one runtime-owned `PolicyBoundary` record after analyzer processing with these attributes:

- `dynamic_execution_performed=false`;
- `network_access_performed=false`;
- `credentials_available=false`;
- `policy_id=<engine policy>`.

The same bundle separately sets `RuntimeManifest.dynamic_execution_performed=false`, `RuntimeManifest.network_access_performed=false`, and `RuntimeManifest.credentials_available=false`. Production construction is internally consistent; the public validation boundary is weaker than production assembly.

## RED

Test-bearing commit: `9d9c994dbb18e2c40d142456662fd9b6350a3acf`.

`tests/artifact_analysis_policy_boundary_binding_red.rs` first creates and validates an untouched `StaticOnly` control bundle through `AnalysisEngine::default()`. It locates the runtime-owned `PolicyBoundary` record and, in three independent hostile copies, changes only one overlapping attribute at a time from `false` to `true` while leaving the enclosing `RuntimeManifest` unchanged.

Each contradictory receipt must fail closed. Current production is expected to RED because bundle validation does not compare `PolicyBoundary` attributes with the runtime manifest. The test changes no production Rust or JSON Schema. A checked-in RED is not causal execution evidence until an exact-head runner executes it and fails for this intended validation gap.

## Smallest causal GREEN after executed RED

After causal execution, make overlapping runtime-boundary facts a versioned `artifact_analysis` invariant:

- reserved `PolicyBoundary` keys for execution, network, and credentials must be canonical booleans and equal the enclosing `RuntimeManifest` values;
- missing or malformed reserved values must fail closed when a runtime-owned PolicyBoundary record claims those facts;
- contradictions are rejected rather than rewritten;
- #52 remains authoritative for whether a requested profile may truthfully set dynamic execution and how that composes with `RuntimeBehavior` and completeness;
- documentation and executable validation must identify which representation is authoritative if a future wire version removes duplicated truth.

A repair that merely tells consumers to ignore one representation, deletes evidence after deserialization, or accepts arbitrary diagnostic contradictions is rejected.

## Risk and effect

Without this binding, a signed or stored receipt may be internally well-formed while claiming both that the runtime had no network/credential/execution activity and that it did. That weakens audit reconstruction, policy verification, and any downstream evidence reference that assumes one receipt has one security-boundary meaning. The repair narrows admissible evidence; it does not make a sandbox enforcement claim by itself.

## Related release gates

Issue #62 remains independent of #49 analyzer capability isolation, #50 bounded worker-result ingestion, #52 truthful dynamic execution/completeness, #54 stable analyzer provenance, #56 ToolFailure/completeness consistency, #58 artifact-subject binding, and #60 evidence-record/job identity. Passing this contract test cannot promote ADR-0009 or artifact-analysis release readiness by itself.

## References

National Institute of Standards and Technology. (2020, updated 2025). *Security and privacy controls for information systems and organizations (NIST SP 800-53 Rev. 5, Release 5.2.0).* https://csrc.nist.gov/pubs/sp/800/53/r5/upd1/final

SLSA Community. (2025). *SLSA specification v1.2: Provenance*. https://slsa.dev/spec/v1.2/provenance

JSON Schema. (2022). *JSON Schema: A media type for describing JSON documents, Draft 2020-12*. https://json-schema.org/draft/2020-12/json-schema-core
