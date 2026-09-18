# Artifact Evidence Identity Binding Traceability

## Authority and problem

Issue #58 owns the P0 subject-identity integrity contract in the `artifact_analysis` bounded context. PR #59 is based on current artifact-analysis parent #18 exact `703b4b1a047321bb08d1d6cb1de78d01cb350696`. Ordinary two-parent commit `52a129a5955df07a1e1a7638ed03a7b382655432` adopted that parent from the prior #59 lineage without force push or destructive rebase. Repository-wide `docs/product-technical-gap-baseline.md` remains #121 single-writer authority and is not a current #59 leaf delta.

`AnalysisEngine::analyze_bytes()` emits two representations of one analyzed subject: the top-level `ArtifactDescriptor` and one `ArtifactIdentity` evidence record. Current runtime assembly duplicates these machine facts into the nested record: `artifact_name`, optional `original_file_name`, `artifact_sha256`, `artifact_size_bytes`, and `artifact_kind`. `EvidenceBundle::validate()` validates the top-level descriptor and each evidence record independently; it does not bind those duplicated subject facts to each other.

A reconstructed or deserialized receipt can therefore present mutually contradictory subject claims while every individual value remains syntactically valid. This is not an analyzer verdict problem. One receipt must not truthfully name two artifacts depending on which representation a consumer reads.

## Causal-evidence history

Historical exact `9b4029f2dbd3a052854954654604c841049405b3` ran CI `34057653119`. Hosted negative rootless/AppArmor was GREEN and verify reached the workspace test step, but retained GitHub evidence does not expose assertion-level output for the subject-binding test and the ancestry also contained broader failures. That run is not promoted to assertion-level causal RED for issue #58.

Test-only `5ccc078d52719f401e590b3c2a0555355c206ea7` removed the original all-zero-digest false-GREEN path. It built two independently valid `StaticOnly` bundles from different bytes and transplanted the alternate engine-produced SHA-256 into the primary bundle. Both controls were valid before mutation, so a future sentinel/all-zero rejection could not satisfy the RED accidentally.

Formal review `5243543629` on current-parent exact `52a129a5955df07a1e1a7638ed03a7b382655432` found a broader false-completion path. The hardened witness still covered only `artifact_sha256`, while issue #58 and current runtime duplicate multiple machine fields. A SHA-only production repair could make that test GREEN while allowing contradictory artifact name, original file name, byte size, or kind in the same receipt.

Test-only `20f5a040a8bdcf0fde2b1918e3176a9727970e45` therefore expands the RED into five independent hostile cases. Every case starts from two independently valid engine-produced bundles. The controls deliberately differ in leaf name, original leaf name, bytes/digest, byte length, and detected kind (`Text` versus `PdfDocument`). Each test changes exactly one nested `ArtifactIdentity` subject attribute to the alternate bundle's already-valid value while leaving the primary top-level `ArtifactDescriptor` unchanged. The five required contradictions are:

- `artifact_name`;
- `original_file_name`;
- `artifact_sha256`;
- `artifact_size_bytes`;
- `artifact_kind`.

The helper asserts that the alternate value is materially different before mutation. Because the hostile value is accepted in another valid engine-produced receipt, a standalone syntax, sentinel, or format restriction cannot legitimately satisfy the witness. The RED still does not reference a future `ContractError` variant, so the intended first failure remains semantic validator acceptance rather than compilation failure.

Production Rust, public JSON Schema, runtime assembly, and wire version are unchanged in this RED hardening.

## DDD and contract boundary

`artifact_analysis` owns artifact subject identity, normalized evidence semantics, and receipt validation. `sandbox_execution` owns isolation/resource/lifecycle evidence and cannot redefine the analyzed artifact. Infrastructure adapters may report observations but cannot choose between contradictory subject representations. AppGuardrail remains static-scan/SARIF authority, Noema admission/activation authority, and Wardnet verdict/incident authority.

The canonical top-level `ArtifactDescriptor` is the subject descriptor. `ArtifactIdentity` is normalized evidence about that same subject, not a second independent source of truth. Therefore every duplicated subject attribute emitted under the versioned contract must agree exactly with the descriptor field from which it is derived. Validation must reject contradiction rather than normalize, rewrite, or silently prefer one representation.

Portable JSON Schema Draft 2020-12 does not provide a general cross-instance value-equality operator tying arbitrary sibling/nested instance locations together. The checked-in schema can continue to prove shape and local syntax, but it must not be described as proving semantic subject equality unless a deliberately versioned executable dialect is introduced. The canonical executable contract validator and public documentation must state the semantic binding explicitly.

## Minimum causal repair after executed RED

No production GREEN is authorized until exact current-head CI executes the five hostile cases behind repository/format prerequisites and demonstrates that the existing validator accepts the contradictions.

After that causal RED, the smallest compatible repair is one canonical ArtifactIdentity-to-ArtifactDescriptor validator that:

1. locates the required foundation `ArtifactIdentity` record under the existing cardinality contract;
2. requires each duplicated machine field to equal the authoritative top-level descriptor value exactly;
3. requires `original_file_name` parity when that field is present under the versioned contract;
4. returns a dedicated typed subject-identity mismatch with stable field identity and expected/actual machine values, rather than an unrelated validation error;
5. preserves untouched engine-produced receipts as valid;
6. rejects hostile input without rewriting or normalization;
7. leaves #49 isolation, #50 bounded result ingestion, #52 dynamic completeness, #54 analyzer provenance, #56 ToolFailure/completeness, #60 record/job identity, #62 runtime-boundary consistency, and #64 foundation cardinality as independent gates.

If the exact foundation-cardinality prerequisite is not yet integrated when the GREEN is implemented, the repair must preserve that owner boundary rather than duplicating or weakening it locally.

## Security and commercial effect

Subject identity is provenance authority. A signed or stored receipt whose top-level descriptor and normalized identity evidence disagree remains contradictory even if its signature is valid: the signature protects the contradiction. Downstream services can otherwise make different admission, audit, or incident decisions depending on which representation they consume. For SOC 2/CSAP evidence readiness and immutable publication, the receipt must identify one subject consistently before signing or consumer handoff.

## Evidence basis

SLSA v1.2 Provenance defines provenance as verifiable information about where, when, and how an artifact was produced and treats artifact subjects/digests as provenance identity. NIST SP 800-53 Rev. 5 Release 5.2.0 retains software/information-integrity, validation, and developer-testing controls. The peer-reviewed in-toto work provides the supply-chain integrity rationale for binding attestations to the artifacts and steps they describe. These sources establish the integrity objective; this repository's versioned artifact-analysis contract remains normative for the exact field-equality rule.

### References

National Institute of Standards and Technology. (2025). *Security and privacy controls for information systems and organizations (NIST Special Publication 800-53 Rev. 5, Release 5.2.0).* https://csrc.nist.gov/pubs/sp/800/53/r5/upd1/final

SLSA Community. (2025). *SLSA specification v1.2: Provenance.* https://slsa.dev/spec/v1.2/provenance

Torres-Arias, S., Afzali, H., Kuppusamy, T. K., Curtmola, R., & Cappos, J. (2019). in-toto: Providing farm-to-table guarantees for bits and bytes. In *28th USENIX Security Symposium (USENIX Security 19)* (pp. 1393–1410). USENIX Association. https://www.usenix.org/conference/usenixsecurity19/presentation/torres-arias

## Current gate

Test-bearing exact `20f5a040a8bdcf0fde2b1918e3176a9727970e45` must execute independently on current #18 ancestry. Any documentation descendant likewise invalidates predecessor whole-head conclusions and must reacquire exact-head evidence. Keep #59 Draft/open until an unchanged dependency-safe head passes repository validation, rustfmt, full locked workspace/all-target tests, Clippy, public/private rustdoc with warnings denied, complete applicable owned-production statement/function/region/branch/edge coverage, qualifying review/security/thread gates, applicable positive effective isolation, protected integration, and immutable publication with SBOM/provenance/reproducibility/rollback. No predecessor GREEN, force update, simple close, or consumer publication is authorized.
