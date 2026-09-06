# Artifact Evidence Identity Binding Traceability

## Authority and problem

Issue #58 owns a P0 evidence-integrity defect in the `artifact_analysis` bounded context. The current parent is PR #18 exact `63260881e36bdb8b1efaf4c3dc7a3a82dd0dd2e1`; the focused test-bearing RED is `52354e070830802e4a387f68e6faf9ed01244751`.

`AnalysisEngine::analyze_bytes()` derives both the top-level `ArtifactDescriptor` and the `ArtifactIdentity` evidence attributes from one ingested artifact. That production path is internally consistent when it creates a bundle. The public `EvidenceBundle::validate()` contract, however, validates those two representations independently. A reconstructed or deserialized receipt can therefore retain a valid top-level artifact SHA-256 while replacing only `ArtifactIdentity.attributes["artifact_sha256"]` with a different well-formed digest and still satisfy the current validator.

This is not an analyzer-verdict problem. It is a subject-binding invariant owned by artifact-analysis evidence semantics: one receipt cannot truthfully attest two different artifact identities.

## Reproduction authority

`tests/artifact_analysis_identity_binding_red.rs` first creates an untouched `StaticOnly` receipt using `AnalysisEngine::default()` and proves that receipt validates. It then changes only the ArtifactIdentity SHA-256 attribute to another syntactically valid 64-character lower-case digest while preserving the top-level descriptor. The required behavior is fail-closed rejection.

The test intentionally does not assert a particular future `ContractError` variant. It fixes the invariant, not the implementation shape. Production Rust and the checked-in JSON Schema remain unchanged until the RED executes causally on its exact head.

## DDD and contract boundary

- `artifact_analysis` owns artifact subject identity, normalized evidence semantics, and receipt validation.
- `sandbox_execution` owns isolation, resource, and lifecycle evidence; it does not decide which artifact an analysis receipt describes.
- infrastructure adapters may report runtime observations but cannot redefine artifact identity.
- AppGuardrail owns static scan/SARIF semantics, Noema owns admission/activation, and Wardnet owns verdict/incident authority.
- consumer code must not be required to choose between contradictory top-level and nested subject claims.

The current `1.0.0` wire schema provides syntactic shape validation. JSON Schema Draft 2020-12 does not provide a portable general-purpose mechanism for asserting equality between arbitrary values at two instance locations. If semantic subject equality cannot be represented without a non-standard extension, the canonical executable Rust validator and public contract documentation must say so explicitly. A schema that merely validates both SHA-256 strings independently is not evidence of subject binding.

## Minimal causal repair

After exact-head execution proves the RED for the intended cause, the smallest acceptable GREEN must:

1. define one canonical ArtifactIdentity-to-`ArtifactDescriptor` mapping under the versioned artifact-analysis contract;
2. reject a contradictory subject digest before a receipt becomes consumer-authoritative;
3. preserve untouched engine-produced receipts as valid;
4. compare stable machine fields rather than summary/display text;
5. avoid normalizing, overwriting, or silently repairing forged receipt content during validation;
6. preserve #49 host-capability isolation, #50 bounded result ingestion, #52 dynamic completeness, #54 analyzer provenance, and #56 ToolFailure/completeness as independent release gates.

Follow-up checks should test `evidence_id` ownership by `analysis_job_id`, `PolicyBoundary` evidence against `RuntimeManifest`, and required/unique foundation identity evidence only after determining whether they share the same causal contract repair. They must not be bundled into this RED merely to inflate scope.

## Security and commercial effect

A receipt with two different subject identities is not reliable audit, SOC 2/CSAP evidence, admission input, or provenance. Downstream services may parse the top-level descriptor while human/operator tooling reads normalized evidence attributes, producing divergent conclusions from the same signed or stored payload. Signing a contradictory bundle would protect the contradiction rather than repair it.

Release authority therefore requires semantic subject consistency before signing, provenance publication, or consumer handoff.

## Evidence basis

SLSA v1.2 Provenance is currently Approved and defines provenance as verifiable information describing where, when, and how an artifact was produced. Its build requirements treat artifact names and cryptographic digests as provenance subject data and require protected provenance generation. NIST SP 800-53 Release 5.2.0 was finalized on August 27, 2025 and continues the catalog's software/information-integrity assurance requirements. The peer-reviewed in-toto work shows why supply-chain evidence must remain cryptographically and semantically attributable to the artifacts and steps it represents.

### References

National Institute of Standards and Technology. (2020, updated 2025). *Security and privacy controls for information systems and organizations (NIST Special Publication 800-53 Rev. 5, Release 5.2.0).* https://csrc.nist.gov/pubs/sp/800/53/r5/upd1/final

OpenSSF. (n.d.). *SLSA specification v1.2: Provenance.* Retrieved September 6, 2026, from https://slsa.dev/spec/v1.2/provenance

Torres-Arias, S., Afzali, H., Kuppusamy, T. K., Curtmola, R., & Cappos, J. (2019). in-toto: Providing farm-to-table guarantees for bits and bytes. In *28th USENIX Security Symposium (USENIX Security 19)* (pp. 1393–1410). USENIX Association. https://www.usenix.org/conference/usenixsecurity19/presentation/torres-arias
