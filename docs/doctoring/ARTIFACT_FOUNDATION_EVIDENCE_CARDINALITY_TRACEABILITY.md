# Artifact Foundation Evidence Cardinality Traceability

## Status

Issue #64 is a RED-only evidence-integrity finding on artifact-analysis parent Draft #18 exact `994ba5fa5ceda20c622861efbff241294867dc39`. Initial test-bearing commit `02f07f44e686debe044d3b91d536c818e1173d02` establishes missing/duplicate foundation-cardinality REDs. Review hardening `dd5364f557bfd0e49870910b2f574ec51027e8d4` adds an ownership-spoof RED so a later GREEN cannot satisfy "runtime-owned" merely by counting evidence kinds while trusting mutable `producer_id` display text. Production Rust and JSON Schema remain unchanged until the hardened RED executes for the intended bundle-level integrity gaps.

## Problem and contract authority

`AnalysisEngine::analyze_bytes()` emits the foundation evidence set before analyzer evidence: one `ArtifactIdentity`, one bundled `FileFormat`, and one `PolicyBoundary` record. The public `EvidenceBundle` is deserializable, but `EvidenceBundle::validate()` currently requires only a non-empty evidence vector and then validates records independently.

Consequently, a reconstructed receipt can remove one required foundation record, duplicate one, or replace the producer identity of a foundation record with another syntactically valid identifier while keeping all other content unchanged. It can also append a same-kind duplicate that claims a foreign producer. Content-binding controls such as #58/#59 and #62/#63 cannot protect a foundation fact that is missing, and they do not remove ambiguity when foundation authority appears more than once or when its origin is represented only by caller-mutable display text.

The regression deliberately rewrites both `sequence_number` and the already-emitted canonical `<analysis_job_id>:evidence:<sequence>` identity after removal/duplication. This prevents #60/#61's separate record/job identity invariant from becoming the accidental failure reason. The ownership-spoof cases leave record identity, sequence, kind, summary, and attributes otherwise intact, so the intended failure cause is the absence of a trustworthy runtime-origin discriminator rather than unrelated syntax validation.

## DDD ownership

`artifact_analysis` owns normalized evidence composition, evidence identity, subject identity, runtime-boundary evidence semantics, and receipt integrity. `sandbox_execution` owns the enforcement and lifecycle facts that may later support isolated worker evidence. AppGuardrail remains SAST/SARIF authority, Noema remains admission/activation authority, and Wardnet remains verdict/incident authority.

The cardinality rule applies only to the foundation evidence set unless another versioned profile says otherwise. Analyzer/worker evidence such as `StaticCapability` can legitimately be repeatable and must not be constrained by a repository-wide magic count. "Runtime-owned" is an authority property, not a naming convention: a consumer-controlled or analyzer-controlled `producer_id` string cannot by itself establish that property.

## RED acceptance

`tests/artifact_analysis_foundation_evidence_cardinality_red.rs` requires:

- an untouched `StaticOnly` control bundle to remain valid;
- the control to contain exactly one `ArtifactIdentity`, one `FileFormat`, and one `PolicyBoundary` record;
- removing any one of those records, while restoring contiguous sequence/record identities, to fail closed;
- duplicating any one of those records, while restoring contiguous sequence/record identities, to fail closed;
- replacing the producer identity of any foundation record with a different valid foreign identifier to fail closed;
- appending a same-kind duplicate whose producer claims a different foreign identifier to fail closed rather than treating the foreign record as harmless non-foundation evidence.

The current validator has no evidence-kind cardinality or foundation-origin binding, so these are checked-in RED candidates. They are not causal execution evidence until an exact-head runner executes the hardened test and fails for these reasons.

## Smallest causal GREEN after executed RED

The repair should make exactly-one foundation evidence a versioned `artifact_analysis` invariant and reject absence, duplication, or origin substitution rather than rewriting the receipt. It must compose with #58/#59 subject binding, #60/#61 record/job identity binding, #62/#63 runtime-manifest/PolicyBoundary value binding, and #54/#55 analyzer provenance.

A false GREEN would merely require `evidence.len() >= 3`, trust fixed record positions, count only a preferred `producer_id` while ignoring a foreign same-kind duplicate, or treat human-readable `producer_id` as proof of runtime ownership. If runtime-owned versus analyzer-owned evidence cannot be established without trusting a spoofable display identifier, the contract needs an explicit versioned origin/provenance discriminator that is generated and validated by the trusted composition boundary.

## Evidence basis

SLSA v1.2 Provenance is Approved and defines provenance as verifiable information about where, when, and how an artifact was produced. NIST SP 800-53 Release 5.2.0 was finalized on 2025-08-27 and includes software-integrity and validation updates. The in-toto supply-chain model provides peer-reviewed support for binding evidence to the artifact and steps it describes. These sources support the integrity rationale; the repository's versioned receipt semantics remain the normative authority for this cardinality and origin rule.

## References

National Institute of Standards and Technology. (2025). *Security and privacy controls for information systems and organizations (NIST SP 800-53 Rev. 5, Release 5.2.0).* https://csrc.nist.gov/pubs/sp/800/53/r5/upd1/final

SLSA Community. (2025). *SLSA specification v1.2: Provenance.* https://slsa.dev/spec/v1.2/provenance

Torres-Arias, S., Afzali, H., Kuppusamy, T. K., Curtmola, R., & Cappos, J. (2019). in-toto: Providing farm-to-table guarantees for bits and bytes. In *28th USENIX Security Symposium (USENIX Security 19)* (pp. 1393–1410). USENIX Association. https://www.usenix.org/conference/usenixsecurity19/presentation/torres-arias

## Release effect

No artifact-analysis receipt is release-authoritative while required foundation evidence may be absent, duplicated, or reassigned to an untrusted origin without invalidating the bundle. A GREEN for #64 would not waive #49/#50/#52/#54/#56/#58/#60/#62, real positive isolation, exact-head coverage/security/review, protected integration, SBOM/provenance/reproducibility, rollback, or immutable release requirements.
