# Artifact Evidence Identifier Binding Traceability

Status: issue #60 current-owner RED is split into independent hostile identity witnesses and hardened against sequence-mismatch false positives. Assertion-level typed identity-mismatch binding remains intentionally incomplete until the current exact head executes the semantic RED and the dedicated contract error exists; production GREEN remains unchanged.

## Problem

`artifact_analysis` exposes one top-level `EvidenceBundle.analysis_job_id` and one `EvidenceRecord.evidence_id` for every normalized record. Production assembly already emits record identifiers as `<analysis_job_id>:evidence:<zero-padded one-based sequence>`, but the public validation boundary does not enforce that relation.

On current owner ancestry, `EvidenceRecord::validate(expected_sequence)` validates bounded `evidence_id` text and the numeric sequence independently. `EvidenceBundle::validate()` supplies only the expected sequence; it does not bind the record identifier to the enclosing `analysis_job_id`. A deserialized or reconstructed receipt can therefore carry an unrelated but syntactically valid `evidence_id`, or reuse the first record identifier at another valid sequence, while retaining all other fields.

This is a referential-integrity defect at the evidence-contract boundary. It is not an analyzer verdict, malware classification, sandbox-isolation claim, or consumer authorization decision.

## DDD ownership

`artifact_analysis` owns analysis-job identity, normalized evidence identity, deterministic evidence ordering, and receipt semantics. `sandbox_execution` owns reusable runtime/worker isolation, resource bounds, lifecycle, termination, and cleanup evidence. Infrastructure adapters translate concrete backend observations into those contracts. AppGuardrail remains static-scan/SARIF authority, Noema admission/activation authority, and Wardnet verdict/incident authority.

Issue #58 owns the separate subject-binding relation between the top-level `ArtifactDescriptor` and nested `ArtifactIdentity` evidence. Issue #60 is intentionally separate because its invariant is record/reference identity inside one analysis job.

## Current implementation evidence

`AnalysisEngine::deterministic_job_id()` creates one deterministic job identity. `push_record()` then assigns every emitted record `format!("{analysis_job_id}:evidence:{sequence_number:04}")`. The runtime producer is therefore stricter than the public validator.

That asymmetry matters after serialization. Audit stores, provenance/signature systems, incident references, deduplication, and downstream evidence links can reasonably treat `evidence_id` as the stable identifier for a record produced by the enclosing job. Accepting a different identity at validation time lets the stored reference disagree with the job whose provenance a consumer is evaluating.

## RED lineage and review hardening

Initial test-bearing authority `b81495c9d4352ed55b3dc662a19cecb535d7a186` placed both hostile mutations in one test. Current predecessor exact `95438943226b719dda296ada384036e258d81bf4`, CI `34057675450`, completed with hosted negative rootless/AppArmor GREEN while verify, coverage, and branch-coverage failed during broader Rust evidence generation. That run proves execution reached the repository's Rust evidence path, but the available GitHub connector evidence does not retain assertion-level log text sufficient to promote the predecessor failure as the precise issue #60 causal RED.

Test-only ordinary child `e9c6217306da87dc075a2e102d38eb41ed83cb28` therefore improves causality rather than guessing. It changes no production Rust, public schema, wire shape, or version. The witness has three separately named controls:

1. `emitted_evidence_ids_match_the_documented_job_and_sequence_identity` proves the existing producer emits the deterministic identity form this repair intends to preserve;
2. `foreign_job_evidence_identifier_fails_closed` changes only the first record identifier to `analysis_job_foreign:evidence:0001` and requires rejection;
3. `duplicate_sequence_evidence_identifier_fails_closed` reuses record 1's valid identifier at sequence 2 and independently requires rejection.

Current-head review correctly found that plain `is_err()` assertions could still false-GREEN on an unrelated validation failure. Test-only ordinary child `0181766da395167ab3ca28bcbc2d211efe3a5731` therefore keeps both hostile mutations on their intended one-based sequence, computes the canonical expected identity for that sequence, proves the supplied hostile identity differs from it, and explicitly fails if validation returns `InvalidEvidenceSequence`. This does not invent the future production error or turn the semantic RED into a compile-time missing-variant RED.

The dedicated identity-mismatch error and its expected/actual identity fields remain part of the minimum GREEN contract below. Once the current exact witness executes and proves that the existing validator still accepts both hostile identities, the production repair must add that typed failure and the regression must then assert it exactly. Until then, describing this lane as fully assertion-level typed hardening would overstate the checked-in evidence.

Splitting the mutations prevents the first failing assertion from hiding the second; the sequence guard prevents a later unrelated sequence failure from satisfying either hostile case. No production behavior is authorized to change until the current exact witness executes for the intended validator gap.

## Smallest causal GREEN after executed RED

After causal execution, the minimum compatible repair is to validate the identity relation the producer already emits: for each one-based sequence `n`, the accepted record identifier is exactly `<analysis_job_id>:evidence:<n padded to four decimal digits>`. Validation should reject a contradiction rather than rewrite or normalize the supplied identifier.

A typed contract error should distinguish evidence-identity mismatch from `InvalidEvidenceSequence`. That preserves the diagnostic boundary between an incorrect numeric position and an identifier that does not belong to the enclosing job/position. The validator should receive the enclosing `analysis_job_id` explicitly rather than recover it by parsing untrusted `evidence_id` text. The post-RED regression must bind both hostile cases to this dedicated error and verify the expected canonical identity for sequence 1 or 2 respectively.

If this deterministic string form is not intended to remain stable public `1.0.0` semantics, the alternative is an explicit versioned contract change. Random record IDs, process-local identity, downstream ignore rules, sequence-only validation while retaining authoritative-looking `evidence_id`, or another unsigned recomputable companion checksum are rejected alternatives.

JSON Schema remains an independent limitation: portable Draft 2020-12 keywords do not provide a straightforward cross-instance string-equality function between `analysis_job_id`, array position, and a formatted `evidence_id`. The Rust validator must not claim the JSON Schema proves this relation unless the published dialect is deliberately extended and independently tested.

## Risk and effect

Without the binding, a receipt can be structurally well formed yet expose conflicting reference identities. That weakens audit reconstruction and can make a signed or stored record identifier appear attributable to evidence outside the job whose top-level provenance is being inspected. The repair narrows admissible receipts to the identity semantics the runtime producer already intends; it does not authorize artifact verdicts, analyzer execution, or consumer admission.

## Related release gates

Issue #60 remains independent of #49 analyzer capability isolation, #50 bounded worker-result ingestion, #52 truthful dynamic execution/completeness, #54 stable analyzer provenance, #56 ToolFailure/completeness consistency, #58 artifact-subject binding, #64 foundation cardinality, and #129 public-schema cardinality parity. Passing this contract slice cannot promote ADR-0009 or artifact-analysis release readiness by itself.

The exact repaired successor must still pass repository validation, rustfmt, full locked workspace/all-target tests, Clippy and public/private rustdoc with warnings denied, complete owned-production statement/function/region/branch/edge coverage, qualifying review/security/thread gates, applicable positive isolation, protected integration, and immutable version/package/SBOM/provenance/reproducibility/rollback publication.

## References

SLSA Community. (2025). *SLSA specification version 1.2: Provenance*. https://slsa.dev/spec/v1.2/provenance

SLSA Community. (2025). *SLSA specification version 1.2*. https://slsa.dev/spec/v1.2/

Torres-Arias, S., Afzali, H., Kuppusamy, T. K., Curtmola, R., & Cappos, J. (2019). in-toto: Providing farm-to-table guarantees for bits and bytes. In *28th USENIX Security Symposium (USENIX Security 19)* (pp. 1393–1410). USENIX Association. https://www.usenix.org/conference/usenixsecurity19/presentation/torres-arias
