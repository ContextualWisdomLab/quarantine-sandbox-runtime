# Artifact Evidence Identity Binding Traceability

## Authority and problem

Issue #58 owns a P0 evidence-integrity defect in the `artifact_analysis` bounded context. PR #59 is currently based on PR #18 exact `c0647152ec052d82969b2ae078891e25e6d4d69a`; the original focused test-bearing RED was `52354e070830802e4a387f68e6faf9ed01244751`. PR #18 itself remains open and intentionally stale behind its moving command/runtime prerequisite, so #59 does not treat that ancestry as release authority.

`AnalysisEngine::analyze_bytes()` derives both the top-level `ArtifactDescriptor` and the `ArtifactIdentity` evidence attributes from one ingested artifact. That production path is internally consistent when it creates a bundle. The public `EvidenceBundle::validate()` contract, however, validates those two representations independently. A reconstructed or deserialized receipt can therefore retain a valid top-level artifact SHA-256 while replacing only `ArtifactIdentity.attributes["artifact_sha256"]` with another valid artifact digest and still satisfy the current validator.

This is not an analyzer-verdict problem. It is a subject-binding invariant owned by artifact-analysis evidence semantics: one receipt cannot truthfully attest two different artifact identities.

## Reproduction authority

Historical exact `9b4029f2dbd3a052854954654604c841049405b3` ran CI `34057653119`. Hosted negative rootless/AppArmor job `101552375182` was GREEN. Verify job `101552375139` passed exact checkout, dependency lock, repository policy, coverage-parser tests, and rustfmt before failing in the workspace `Test` step; coverage job `101552375176` likewise failed during production-coverage generation. The available GitHub evidence no longer exposes assertion-level output for that historical failure, and the parent ancestry also had broad test failures. Therefore this run is not promoted to an assertion-level causal RED for issue #58.

Review found a second causality weakness in the original witness: it forged the nested digest with `"0".repeat(64)`. That value satisfies the current lower-case SHA-256 syntax check, but a future general sentinel/all-zero digest rule could reject it and make the subject-binding RED pass without implementing subject equality.

Test-only commit `5ccc078d52719f401e590b3c2a0555355c206ea7` removes that false-GREEN path. `tests/artifact_analysis_identity_binding_red.rs` now creates two independently valid `StaticOnly` bundles from different artifact bytes, proves both controls validate, and transplants the second bundle's engine-produced artifact SHA-256 into only the first bundle's `ArtifactIdentity` record. The first bundle's top-level `ArtifactDescriptor` is left unchanged. The hostile digest is therefore not merely syntactically plausible; it is a real digest already accepted in another valid receipt, and the intended remaining contradiction is subject binding.

The test still does not reference a future `ContractError` variant before production defines one. Doing so would convert the intended semantic validator-acceptance RED into a compile-time failure. Production Rust and the checked-in JSON Schema remain unchanged until the hardened exact head executes and fails for the intended subject-binding cause.

## DDD and contract boundary

- `artifact_analysis` owns artifact subject identity, normalized evidence semantics, and receipt validation.
- `sandbox_execution` owns isolation, resource, and lifecycle evidence; it does not decide which artifact an analysis receipt describes.
- infrastructure adapters may report runtime observations but cannot redefine artifact identity.
- AppGuardrail owns static scan/SARIF semantics, Noema owns admission/activation, and Wardnet owns verdict/incident authority.
- consumer code must not be required to choose between contradictory top-level and nested subject claims.

The current `1.0.0` wire schema provides syntactic shape validation. JSON Schema Draft 2020-12 does not provide a portable general-purpose mechanism for asserting equality between arbitrary values at two instance locations. If semantic subject equality cannot be represented without a non-standard extension, the canonical executable Rust validator and public contract documentation must say so explicitly. A schema that merely validates both SHA-256 strings independently is not evidence of subject binding.

## Minimal causal repair

After exact-head execution proves the hardened RED for the intended cause, the smallest acceptable GREEN must:

1. define one canonical `ArtifactIdentity`-to-`ArtifactDescriptor` mapping under the versioned artifact-analysis contract;
2. reject a contradictory subject digest before a receipt becomes consumer-authoritative;
3. preserve both untouched engine-produced control receipts as valid;
4. return a dedicated typed subject-identity mismatch rather than relying on an unrelated validation error;
5. compare stable machine fields rather than summary/display text;
6. avoid normalizing, overwriting, or silently repairing forged receipt content during validation;
7. preserve #49 host-capability isolation, #50 bounded result ingestion, #52 dynamic completeness, #54 analyzer provenance, #56 ToolFailure/completeness, #60 record/job identity, #62 runtime-boundary binding, and #64 foundation cardinality as independent gates.

The dedicated typed error and its expected/actual subject fields belong in the GREEN step after the semantic RED executes. They must not be referenced prematurely in the test.

## Security and commercial effect

A receipt with two different subject identities is not reliable audit, SOC 2/CSAP evidence, admission input, or provenance. Downstream services may parse the top-level descriptor while human/operator tooling reads normalized evidence attributes, producing divergent conclusions from the same signed or stored payload. Signing a contradictory bundle would protect the contradiction rather than repair it.

Release authority therefore requires semantic subject consistency before signing, provenance publication, or consumer handoff.

## Evidence basis

SLSA v1.2 Provenance is an Approved specification and defines provenance as verifiable information about where, when, and how an artifact was produced. NIST finalized SP 800-53 Release 5.2.0 on August 27, 2025 and explicitly strengthened software integrity, validation, developer testing, and update reliability controls. The peer-reviewed in-toto work supplies the supply-chain integrity rationale for binding attestations to the artifacts and steps they describe. These sources support the integrity objective; this repository's versioned artifact-analysis contract remains normative for the exact subject-equality rule.

### References

National Institute of Standards and Technology. (2025). *Security and privacy controls for information systems and organizations (NIST Special Publication 800-53 Rev. 5, Release 5.2.0).* https://csrc.nist.gov/pubs/sp/800/53/r5/upd1/final

SLSA Community. (2025). *SLSA specification v1.2: Provenance.* https://slsa.dev/spec/v1.2/provenance

Torres-Arias, S., Afzali, H., Kuppusamy, T. K., Curtmola, R., & Cappos, J. (2019). in-toto: Providing farm-to-table guarantees for bits and bytes. In *28th USENIX Security Symposium (USENIX Security 19)* (pp. 1393–1410). USENIX Association. https://www.usenix.org/conference/usenixsecurity19/presentation/torres-arias

## Current gate

Current #59 must independently execute after `5ccc078d...` and this doctoring update. Historical broad failures do not transfer. No production validator change, merge, release, provenance publication, or consumer handoff is authorized until the hardened witness fails for the intended subject-binding cause and the resulting minimal GREEN passes on one unchanged exact head together with the repository's required review, coverage, security, real-runtime, and positive-LSM gates.
