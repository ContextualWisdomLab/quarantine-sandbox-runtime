# Artifact Foundation Evidence Cardinality Traceability

## Status

Issue #64 is a RED-only evidence-integrity finding on artifact-analysis parent Draft #18 exact `994ba5fa5ceda20c622861efbff241294867dc39`. Initial test-bearing commit `02f07f44e686debe044d3b91d536c818e1173d02` establishes missing/duplicate foundation-cardinality REDs. Review hardening `dd5364f557bfd0e49870910b2f574ec51027e8d4` attempted to extend that RED to trusted runtime origin by mutating `producer_id`; review on the subsequent exact head found that this overclaimed what the public receipt can prove. Commit `ffde7a7c34fb2ae5df7cc96d9a9294ded0fe04fa` narrows the executable RED back to structural cardinality so a hard-coded producer-name check cannot masquerade as provenance. Production Rust and JSON Schema remain unchanged until the exact cardinality RED executes for its intended cause.

## Problem and contract authority

`AnalysisEngine::analyze_bytes()` emits the current v1 foundation evidence set around bundled static analysis: one `ArtifactIdentity`, one bundled `FileFormat`, and one `PolicyBoundary` record. The public `EvidenceBundle` is deserializable, but `EvidenceBundle::validate()` currently requires only a non-empty evidence vector and then validates records independently.

Consequently, a reconstructed receipt can remove one canonical foundation record or duplicate the canonical record while keeping sequence numbers and record identifiers internally consistent. Content-binding controls such as #58/#59 and #62/#63 cannot protect a foundation fact that is missing, and they do not remove ambiguity when the same canonical foundation record appears more than once.

The regression rewrites both `sequence_number` and the already-emitted canonical `<analysis_job_id>:evidence:<sequence>` identity after removal/duplication. This prevents #60/#61's separate record/job identity invariant from becoming the accidental failure reason.

## Review finding: cardinality is not provenance

The serialized `producer_id` field is attribution text. The current public receipt has no signature, trusted attestation context, or versioned origin discriminator that lets `EvidenceBundle::validate()` prove that a deserialized record genuinely came from the runtime composition boundary. A RED that merely changes `producer_id` and expects rejection can be satisfied by hard-coding `runtime_core` or `format_analyzer`; that would pass the test while still trusting attacker-rewritable display metadata.

For that reason #64/#65 owns structural cardinality only. Trusted analyzer/runtime origin stays with #54/#55 and the future signed/provenance contract. If a later contract needs both one authoritative bundled `FileFormat` foundation record and additional worker/analyzer `FileFormat` evidence, it must introduce a versioned trusted-origin discriminator instead of treating `EvidenceKind::FileFormat` as globally singleton by kind or by display producer name.

## DDD ownership

`artifact_analysis` owns normalized evidence composition, evidence identity, subject identity, runtime-boundary evidence semantics, and receipt integrity. `sandbox_execution` owns the enforcement and lifecycle facts that may later support isolated worker evidence. AppGuardrail remains SAST/SARIF authority, Noema remains admission/activation authority, and Wardnet remains verdict/incident authority.

#64 is intentionally narrower than #54/#55. Cardinality validation can reject structural omission or duplication at the receipt boundary. Provenance validation must bind a record to a trusted runtime/analyzer identity using versioned evidence that cannot be established from mutable `producer_id` text alone.

## RED acceptance

`tests/artifact_analysis_foundation_evidence_cardinality_red.rs` requires:

- an untouched `StaticOnly` control bundle to remain valid;
- the current v1 bundled-runtime control to contain one `ArtifactIdentity`, one bundled `FileFormat`, and one `PolicyBoundary` foundation record;
- removing any one canonical foundation record, while restoring contiguous sequence/record identities, to fail closed;
- duplicating any one canonical foundation record, while restoring contiguous sequence/record identities, to fail closed.

The current validator has no foundation-cardinality rule, so these are checked-in RED candidates. They are not causal execution evidence until an exact-head runner executes the narrowed test and fails for these reasons.

## Smallest causal GREEN after executed RED

The repair should make the v1 bundled foundation set structurally unambiguous without inventing provenance. It must compose with #58/#59 subject binding, #60/#61 record/job identity binding, #62/#63 runtime-manifest/PolicyBoundary value binding, and #54/#55 analyzer provenance.

A false GREEN would merely require `evidence.len() >= 3`, assume fixed positions, or infer trusted origin from `producer_id`. If authoritative versus non-authoritative same-kind evidence must coexist, add a versioned origin/provenance discriminator generated and validated by the trusted composition boundary; do not silently overload `producer_id` or prohibit future analyzer evidence by an undocumented global kind count.

## Evidence basis

SLSA v1.2 is the current Approved specification, and its Provenance section defines provenance as verifiable information about where, when, and how an artifact was produced. NIST finalized SP 800-53 Release 5.2.0 on 2025-08-27, including software-integrity and validation updates. The in-toto supply-chain model provides peer-reviewed support for verifiable links between artifacts and the steps that describe them. These sources support the integrity rationale; this repository's versioned receipt semantics remain the normative authority for cardinality and provenance separation.

## References

National Institute of Standards and Technology. (2025). *Security and privacy controls for information systems and organizations (NIST SP 800-53 Rev. 5, Release 5.2.0).* https://csrc.nist.gov/pubs/sp/800/53/r5/upd1/final

SLSA Community. (2025). *SLSA specification v1.2: Provenance.* https://slsa.dev/spec/v1.2/provenance

Torres-Arias, S., Afzali, H., Kuppusamy, T. K., Curtmola, R., & Cappos, J. (2019). in-toto: Providing farm-to-table guarantees for bits and bytes. In *28th USENIX Security Symposium (USENIX Security 19)* (pp. 1393–1410). USENIX Association. https://www.usenix.org/conference/usenixsecurity19/presentation/torres-arias

## Release effect

No artifact-analysis receipt is release-authoritative while the canonical v1 bundled foundation evidence can be omitted or duplicated without invalidating the bundle. A GREEN for #64 does not establish trusted producer origin and does not waive #49/#50/#52/#54/#56/#58/#60/#62, real positive isolation, exact-head coverage/security/review, protected integration, SBOM/provenance/reproducibility, rollback, or immutable release requirements.
