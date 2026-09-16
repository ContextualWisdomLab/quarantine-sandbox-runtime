# Artifact Policy-Boundary Binding Traceability

Status: causal RED executed on `182a55637a42d274bf09d11e034afc6a2c8c81b4`; minimal production repair is checked in on the ordinary descendant lineage and remains a candidate GREEN until current exact-head CI, coverage, review, and protected integration succeed.

## Problem

`artifact_analysis` serializes three runtime-boundary facts twice in one `EvidenceBundle`: the top-level `RuntimeManifest` booleans and the runtime-owned `PolicyBoundary` record attributes `dynamic_execution_performed`, `network_access_performed`, and `credentials_available`.

On the RED parent, `AnalysisEngine::analyze_bytes()` emitted the two representations consistently. `RuntimeManifest::validate()` checked the top-level booleans, while `EvidenceRecord::validate()` checked only generic attribute bounds and text. `EvidenceBundle::validate()` did not compare the overlapping representations.

A reconstructed or deserialized receipt could therefore keep the top-level runtime manifest at `false/false/false`, change only one runtime-owned `PolicyBoundary` attribute to `true`, and remain syntactically valid under the Rust `1.0.0` contract. That is an internal evidence-integrity defect: one receipt could present incompatible security-boundary facts depending on which representation a consumer read.

## DDD ownership

`artifact_analysis` owns evidence normalization, runtime-attestation semantics, and receipt integrity. `sandbox_execution` owns the actual isolation, resource, lifecycle, termination, and cleanup facts that a future analyzer worker must prove. Infrastructure adapters translate those backend-neutral controls. AppGuardrail remains SAST/SARIF authority, Noema admission/activation authority, and Wardnet verdict/incident authority.

Issue #52 remains owner of dynamic-profile execution/completeness/`RuntimeBehavior` semantics. Issue #62 owns only agreement between duplicated runtime-boundary representations once those facts are present in a receipt. It does not authorize dynamic execution or weaken #49 worker isolation.

## Executed RED

The focused contract was carried through the non-force #63 lineage and executed at exact commit `182a55637a42d274bf09d11e034afc6a2c8c81b4` in CI run `34057708549`.

The branch-coverage job reached `tests/artifact_analysis_policy_boundary_binding_red.rs` after the existing unit and application-service suites had passed. The untouched `StaticOnly` control validated; the first hostile copy changed only `PolicyBoundary.dynamic_execution_performed` from `false` to `true` while leaving the enclosing `RuntimeManifest` unchanged. `contradictory.validate()` returned `Ok`, so the assertion failed with `PolicyBoundary attribute dynamic_execution_performed must not contradict the enclosing RuntimeManifest`. That is the intended causal RED.

The same run's `verify` job independently failed `cargo fmt --check` on formatting in the RED fixture before the normal test step. Commit `123385dfa1e3e22c6c5e0c975ad6193449068793` applied only the formatter-prescribed layout. The semantic RED evidence comes from the branch-coverage job and is not inferred from the formatter failure.

## Minimal causal repair

Production commit `e0c5ff8819f9068ab25372e88b7f97d6322cd611` changes only the artifact-analysis contract boundary. `EvidenceBundle::validate()` now treats every `EvidenceKind::PolicyBoundary` record as runtime-owned boundary evidence and validates its duplicated execution, network, and credential facts against the enclosing `RuntimeManifest`.

The binding uses the manifest boolean's canonical Rust string representation. A reserved value therefore validates only when it is present and exactly `true` or `false` as implied by the manifest. A contradictory, missing, or non-canonical value fails through the existing `RuntimeBoundaryViolated` error family rather than being rewritten or coerced. No wire field, JSON Schema shape, verdict authority, dynamic-execution policy, or sandbox enforcement behavior changes in this repair.

Ordinary descendant `2a3c387bf3e01ace7b89750f69a5edb7d2bfafea` extends the focused contract test to cover missing and malformed (`FALSE`) reserved attributes for each of the three duplicated facts. These tests are part of current-head verification; they are not claimed as executed until CI runs that exact head.

## Decision record

Problem: one validated receipt could carry contradictory runtime-boundary truth in two representations.

Constraints: preserve the `1.0.0` wire shape and public Rust contract surface where possible; do not absorb #52 dynamic-execution semantics; do not make a schema assertion pretend to express cross-object equality; do not trust producer text as a way to evade a runtime-reserved evidence kind.

Alternatives rejected:

- telling consumers to ignore one representation leaves contradictory signed/stored evidence admissible;
- silently rewriting `PolicyBoundary` attributes hides tampering or reconstruction errors;
- accepting arbitrary string booleans such as `FALSE` weakens deterministic evidence normalization;
- adding a new public `ContractError` variant is unnecessary for this causal repair and could expand the public Rust API surface; the existing runtime-boundary error already describes the fail-closed result;
- keying the invariant only on `producer_id == "runtime_core"` would let a reconstructed receipt evade the invariant by changing producer text while retaining a runtime-reserved `PolicyBoundary` kind.

Selected repair: make `PolicyBoundary` a runtime-owned evidence kind at the contract boundary and require its three duplicated reserved facts to equal the enclosing manifest exactly.

Risk/effect: admissible receipts become narrower. Engine-produced foundation receipts remain unchanged. This repair establishes receipt consistency only; it does not prove that a sandbox actually enforced the claimed boundary.

## Release gates

Current exact head after the test extension is `2a3c387bf3e01ace7b89750f69a5edb7d2bfafea`. It must obtain exact-head formatting, tests, clippy, rustdoc, 100% owned production statement/function/region/branch coverage, review/security gates, and protected integration before the repair can be called GREEN or release-authoritative.

Issue #62 remains independent of #49 analyzer capability isolation, #50 bounded worker-result ingestion, #52 truthful dynamic execution/completeness, #54 stable analyzer provenance, #56 ToolFailure/completeness consistency, #58 artifact-subject binding, and #60 evidence-record/job identity binding. Passing this contract does not promote ADR-0009 or artifact-analysis release readiness by itself.

## References

National Institute of Standards and Technology. (2020, updated 2025). *Security and privacy controls for information systems and organizations (NIST SP 800-53 Rev. 5, Release 5.2.0).* https://csrc.nist.gov/pubs/sp/800/53/r5/upd1/final

SLSA Community. (2025). *SLSA specification v1.2: Provenance*. https://slsa.dev/spec/v1.2/provenance

JSON Schema. (2022). *JSON Schema: A media type for describing JSON documents, Draft 2020-12*. https://json-schema.org/draft/2020-12/json-schema-core
