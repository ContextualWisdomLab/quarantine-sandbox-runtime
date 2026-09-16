# Artifact Foundation Evidence Cardinality Traceability

## Status

Issue #64 owns structural cardinality for the public v1 `EvidenceBundle`: one `ArtifactIdentity`, one bundled `FileFormat`, and one `PolicyBoundary` record must survive deserialization and validation exactly once. Trusted producer/runtime origin remains #54/#55 authority and is not inferred from mutable `producer_id` text.

The narrowed cardinality RED was executed on exact `5b7d3d2b8d122c4f3828632c91f62f33abe80e57` in CI run `34057733664`. The hosted branch-coverage job `101552592394` reached `Generate branch coverage evidence` and failed after exact checkout/dependency setup; the ordinary coverage job `101552592599` likewise reached production-coverage generation and failed. At that exact source, `EvidenceBundle::validate()` only rejected an empty evidence vector and validated records independently, while `tests/artifact_analysis_foundation_evidence_cardinality_red.rs` removed or duplicated each foundation kind after restoring contiguous sequence numbers and canonical record identifiers. This establishes the intended missing/duplicate-cardinality RED. Hosted negative rootless/AppArmor `101552592535` was GREEN. Verify `101552592557` independently stopped earlier at rustfmt, so it is not evidence for the semantic failure.

Production commit `701097b97205d630869344d0d23df8d0505c6c50` adds the smallest v1 validator: after each record passes its own validation, the bundle counts `ArtifactIdentity`, `FileFormat`, and `PolicyBoundary` and rejects any count other than one through typed `InvalidFoundationEvidenceCardinality { evidence_kind, actual_count }`. It does not inspect `producer_id`, insert/rewrite evidence, change positions, or alter optional evidence kinds. Test commit `f27a98fdbb6a0ce7662bb8e744e1560c9a91befc` formats the focused regression and binds the missing/duplicate cases to counts `0` and `2` for each stable evidence-kind wire code.

Current exact head is `f27a98fdbb6a0ce7662bb8e744e1560c9a91befc`; exact-head CI run `35106908249` is the validation authority. Predecessor GREEN or failure states do not transfer to this moved head.

## Problem and contract authority

`AnalysisEngine::analyze_bytes()` emits the current v1 foundation evidence set around bundled static analysis: one `ArtifactIdentity`, one bundled `FileFormat`, and one `PolicyBoundary` record. The public `EvidenceBundle` is deserializable, so validation must reject a reconstructed receipt that removes one of those records or duplicates it while keeping sequence numbers and record identifiers internally consistent.

This invariant is separate from #58/#59 subject-content binding, #60/#61 record/job identity, #62/#63 `PolicyBoundary`/`RuntimeManifest` value binding, and #54/#55 analyzer provenance. Those controls cannot restore a foundation fact that is absent, and provenance must not be simulated with display metadata.

The regression rewrites both `sequence_number` and the canonical `<analysis_job_id>:evidence:<sequence>` identity after removal/duplication. This prevents #60/#61 from becoming the accidental failure reason.

## Review finding: cardinality is not provenance

The serialized `producer_id` field is attribution text. The current public receipt has no signature, trusted attestation context, or versioned origin discriminator that lets `EvidenceBundle::validate()` prove that a deserialized record genuinely came from the runtime composition boundary. A validator that hard-codes `runtime_core` or `format_analyzer` would satisfy a display-name test while still trusting attacker-rewritable metadata.

For that reason #64/#65 owns structural v1 cardinality only. Trusted analyzer/runtime origin stays with #54/#55 and any future signed/provenance contract. The current v1 count is explicit contract behavior; if a later contract needs both one authoritative bundled `FileFormat` record and additional same-kind analyzer evidence, that contract must introduce a versioned trusted-origin discriminator instead of overloading `producer_id`.

## DDD ownership

`artifact_analysis` owns normalized evidence composition, evidence identity, subject identity, runtime-boundary evidence semantics, and receipt integrity. `sandbox_execution` owns enforcement and lifecycle facts that may later support isolated worker evidence. AppGuardrail remains SAST/SARIF authority, Noema remains admission/activation authority, and Wardnet remains verdict/incident authority.

#64 remains intentionally narrower than #54/#55. Cardinality validation rejects structural omission or duplication at the receipt boundary. Provenance validation must bind evidence to a trusted runtime/analyzer identity using versioned evidence that cannot be established from mutable `producer_id` text alone.

## Executed RED and causal GREEN

The focused contract requires:

- an untouched `StaticOnly` control bundle to remain valid;
- each v1 `ArtifactIdentity`, `FileFormat`, and `PolicyBoundary` kind to occur exactly once;
- removal of a foundation kind, after sequence/record identity repair, to return `InvalidFoundationEvidenceCardinality` with `actual_count: 0`;
- duplication of a foundation kind, after sequence/record identity repair, to return the same typed error with `actual_count: 2`.

The production repair deliberately runs cardinality after individual record validation. Malformed records therefore still fail at their own causal boundary before aggregate cardinality is considered. It does not solve the problem with `evidence.len() >= 3`, fixed positions, silent normalization, or producer-name matching.

The JSON Schema continues to describe per-record structure and `minItems`; this commit does not claim JSON-Schema cardinality parity because no schema-parity RED has yet executed. Runtime `EvidenceBundle::validate()` is the executed #64 contract authority. A future schema-parity change must first add a causal schema validation witness rather than silently broadening this executed repair.

## Evidence basis

SLSA v1.2 defines provenance as verifiable information about where, when, and how an artifact was produced. NIST SP 800-53 Rev. 5 Release 5.2.0 includes software-integrity and validation controls. The in-toto supply-chain model provides peer-reviewed support for verifiable links between artifacts and the steps that describe them. These sources support the integrity rationale; this repository's versioned receipt semantics remain normative for cardinality/provenance separation.

## References

National Institute of Standards and Technology. (2025). *Security and privacy controls for information systems and organizations (NIST SP 800-53 Rev. 5, Release 5.2.0).* https://csrc.nist.gov/pubs/sp/800/53/r5/upd1/final

SLSA Community. (2025). *SLSA specification v1.2: Provenance.* https://slsa.dev/spec/v1.2/provenance

Torres-Arias, S., Afzali, H., Kuppusamy, T. K., Curtmola, R., & Cappos, J. (2019). in-toto: Providing farm-to-table guarantees for bits and bytes. In *28th USENIX Security Symposium (USENIX Security 19)* (pp. 1393–1410). USENIX Association. https://www.usenix.org/conference/usenixsecurity19/presentation/torres-arias

## Release effect

No artifact-analysis receipt is release-authoritative until current exact-head validation proves the cardinality repair together with required review/security/coverage gates and protected integration. A GREEN for #64 would not establish trusted producer origin and would not waive #49/#50/#52/#54/#56/#58/#60/#62, real positive isolation, protected integration, SBOM/provenance/reproducibility, rollback, or immutable release requirements.
