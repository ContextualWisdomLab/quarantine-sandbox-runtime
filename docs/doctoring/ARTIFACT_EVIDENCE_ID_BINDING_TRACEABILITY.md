# Artifact Evidence Identifier Binding Traceability

Status: issue #60 has an executed causal RED on exact `36cea922b5f9170ad9e2813d52d0c0f64f23bacf`. Minimal production repair `c024ecc84a83f5af7a24adf7b92b191792337762` binds every `EvidenceRecord.evidence_id` to the enclosing `analysis_job_id` and one-based sequence and upgrades both hostile regressions to the dedicated typed mismatch error. The repair is not GREEN or release evidence until the exact document-bearing successor completes current-head CI and the remaining repository gates.

## Problem

`artifact_analysis` exposes one top-level `EvidenceBundle.analysis_job_id` and one `EvidenceRecord.evidence_id` for every normalized record. Production assembly already emits record identifiers as `<analysis_job_id>:evidence:<zero-padded one-based sequence>`, but the pre-repair public validation boundary did not enforce that relation.

On predecessor `36cea922...`, `EvidenceRecord::validate(expected_sequence)` validated bounded `evidence_id` text and the numeric sequence independently. `EvidenceBundle::validate()` supplied only the expected sequence; it did not bind the record identifier to the enclosing `analysis_job_id`. A deserialized or reconstructed receipt could therefore carry an unrelated but syntactically valid `evidence_id`, or reuse the first record identifier at another valid sequence, while retaining all other fields.

This is a referential-integrity defect at the evidence-contract boundary. It is not an analyzer verdict, malware classification, sandbox-isolation claim, or consumer authorization decision.

## DDD ownership

`artifact_analysis` owns analysis-job identity, normalized evidence identity, deterministic evidence ordering, and receipt semantics. `sandbox_execution` owns reusable runtime/worker isolation, resource bounds, lifecycle, termination, and cleanup evidence. Infrastructure adapters translate concrete backend observations into those contracts. AppGuardrail remains static-scan/SARIF authority, Noema admission/activation authority, and Wardnet verdict/incident authority.

Issue #58 owns the separate subject-binding relation between the top-level `ArtifactDescriptor` and nested `ArtifactIdentity` evidence. Issue #60 is intentionally separate because its invariant is record/reference identity inside one analysis job.

## Executed causal RED

Native CI `35219614102` executed exact `36cea922b5f9170ad9e2813d52d0c0f64f23bacf` with Rust 1.97.1. Verify job `105196251862` passed exact checkout, dependency lock, repository policy validation, coverage-parser tests, and `cargo fmt --check`, then ran the full locked workspace/all-target test command.

The dedicated witness reached all three identity tests:

- `emitted_evidence_ids_match_the_documented_job_and_sequence_identity` passed, proving the runtime producer already emits the intended deterministic identity form;
- `foreign_job_evidence_identifier_fails_closed` failed because validation accepted `analysis_job_foreign:evidence:0001` at sequence 1;
- `duplicate_sequence_evidence_identifier_fails_closed` failed because validation accepted record 1's identifier at sequence 2.

Both hostile records retained the correct numeric sequence, so `InvalidEvidenceSequence` could not satisfy the witness. Hosted negative rootless/AppArmor also passed on this exact. Broad coverage and branch-coverage jobs failed during repository-wide Rust evidence generation and remain separate gates; positive SELinux did not provide qualifying current-head evidence.

## Minimal causal repair

Production commit `c024ecc84a83f5af7a24adf7b92b191792337762` changes only the contract relation required by the executed RED and the two hostile regressions:

1. `EvidenceBundle::validate()` passes the enclosing `analysis_job_id` into `EvidenceRecord::validate()`;
2. record validation first preserves the existing contiguous one-based sequence check, then computes the canonical identifier `<analysis_job_id>:evidence:<sequence padded to four decimal digits>`;
3. a contradiction is rejected as `ContractError::InvalidEvidenceIdentity { expected_evidence_id, actual_evidence_id }` rather than rewritten, normalized, or misreported as a sequence failure;
4. the foreign-job and duplicate-ID tests now assert the exact typed error and exact expected/actual identities.

Runtime emission, public wire fields, schema version, analyzer execution, disposition semantics, and consumer verdict ownership are unchanged. The public JSON Schema is also unchanged: portable Draft 2020-12 does not provide a straightforward cross-instance formatted-string equality relation between top-level `analysis_job_id`, array position, and each `evidence_id`. Do not claim schema parity for this relation without a deliberate versioned dialect contract and independent tests.

## Evidence and provenance rationale

The repair keeps the identifier relation deterministic and independently checkable at receipt validation. That is consistent with provenance systems that require subjects and metadata to remain attributable to the producer context rather than accepting internally contradictory identifiers. SLSA version 1.2 remains the current published specification as checked on 2026-09-18; SLSA provenance is defined using the in-toto attestation framework. in-toto likewise treats signed link metadata and the authorized step/functionary relation as verifiable supply-chain evidence rather than best-effort labels.

This evidence is supporting architecture rationale, not a claim that issue #60 by itself makes the runtime SLSA-compliant or provides cryptographic attestation. Immutable release provenance remains a separate repository gate.

## Risk and effect

Without the binding, a receipt can be structurally well formed yet expose conflicting reference identities. That weakens audit reconstruction and can make a stored or later signed record identifier appear attributable to evidence outside the job whose top-level provenance a consumer is evaluating. The repair narrows admissible receipts to the identity semantics the runtime producer already emits; it does not authorize artifact verdicts, analyzer execution, or consumer admission.

## Related release gates

Issue #60 remains independent of #49 analyzer capability isolation, #50 bounded worker-result ingestion, #52 truthful dynamic execution/completeness, #54 stable analyzer provenance, #56 ToolFailure/completeness consistency, #58 artifact-subject binding, #64 foundation cardinality, and #129 public-schema cardinality parity. Passing this contract slice cannot promote ADR-0009 or artifact-analysis release readiness by itself.

The exact repaired successor must still pass repository validation, rustfmt, full locked workspace/all-target tests, Clippy and public/private rustdoc with warnings denied, complete owned-production statement/function/region/branch/edge coverage, qualifying review/security/thread gates, applicable positive isolation, protected integration, and immutable version/package/SBOM/provenance/reproducibility/rollback publication.

## References

SLSA Community. (2025). *SLSA specification version 1.2: Provenance*. https://slsa.dev/spec/v1.2/provenance

SLSA Community. (2025). *SLSA specification version 1.2*. https://slsa.dev/spec/v1.2/

Torres-Arias, S., Afzali, H., Kuppusamy, T. K., Curtmola, R., & Cappos, J. (2019). in-toto: Providing farm-to-table guarantees for bits and bytes. In *28th USENIX Security Symposium (USENIX Security 19)* (pp. 1393–1410). USENIX Association. https://www.usenix.org/conference/usenixsecurity19/presentation/torres-arias
