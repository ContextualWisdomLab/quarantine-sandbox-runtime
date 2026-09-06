# Artifact Evidence Identifier Binding Traceability

Status: issue #60 checked-in RED pending causal execution. No production GREEN is authorized yet.

## Problem

`artifact_analysis` exposes one top-level `EvidenceBundle.analysis_job_id` and one `EvidenceRecord.evidence_id` for every normalized record. Production assembly currently creates record identifiers in the canonical form `<analysis_job_id>:evidence:<zero-padded one-based sequence>`, but the public validation boundary does not enforce that relation.

On parent Draft #18 exact `7a48b7f904ce41ce2d2f184028c2eec7a9201d55`, `EvidenceRecord::validate(expected_sequence)` checks only bounded text, sequence position, producer, summary, and attributes. `EvidenceBundle::validate()` supplies the expected sequence but never supplies or compares the enclosing `analysis_job_id`, and it does not independently reject duplicate evidence identifiers.

The engine therefore emits internally consistent identifiers while a deserialized or reconstructed receipt can carry an unrelated but syntactically valid `evidence_id`, or reuse one identifier at a different sequence, and still satisfy the current `1.0.0` Rust validator. That is a referential-integrity defect at the evidence contract boundary, not an analyzer verdict or sandbox-isolation finding.

## DDD ownership

`artifact_analysis` owns analysis-job identity, normalized evidence identity, deterministic evidence ordering, and receipt semantics. `sandbox_execution` owns runtime/worker isolation, resource bounds, lifecycle, termination, cleanup, and their runtime evidence. Infrastructure adapters translate those backend-neutral isolation contracts. AppGuardrail remains static-scan/SARIF authority, Noema admission/activation authority, and Wardnet verdict/incident authority.

Issue #58 owns a different subject-binding invariant between the top-level `ArtifactDescriptor` and nested `ArtifactIdentity` evidence. Issue #60 is split because its causal defect and repair concern record/reference identity inside one analysis job.

## Current implementation evidence

`AnalysisEngine::deterministic_job_id()` produces one job identity. `push_record()` then assigns each emitted record an identifier derived from that job identity and the next one-based sequence. This is stronger than what the public validator currently requires.

The gap matters after serialization. Audit systems, signature/provenance stores, incident references, deduplication, and downstream evidence links can reasonably treat `evidence_id` as the stable identifier of the record produced by the enclosing job. The current validator allows that apparent identity to diverge from the job that contains it.

## RED

Test-bearing commit: `b81495c9d4352ed55b3dc662a19cecb535d7a186`.

`tests/artifact_analysis_evidence_id_binding_red.rs` first obtains an untouched `StaticOnly` control bundle through `AnalysisEngine::default()` and requires it to validate. It then checks two independent hostile mutations:

1. replace the first record identifier with a different, bounded identifier unrelated to the enclosing `analysis_job_id` while leaving all other fields intact;
2. copy the first valid record identifier into the second record while leaving the second record's sequence number unchanged.

Both mutated receipts must fail closed. Current production is expected to RED because neither the job/record relation nor duplicate identifier condition is checked by `EvidenceBundle::validate()`.

This RED changes no production Rust or JSON Schema. A checked-in test is not causal execution evidence until an exact-head runner executes it and fails for the intended validation gap.

## Smallest causal GREEN after executed RED

After the RED executes for the intended cause, make record identity a versioned `artifact_analysis` invariant. The smallest compatible repair may require the canonical syntax production already emits: `<analysis_job_id>:evidence:<zero-padded one-based sequence>`. That simultaneously binds every record to one job and makes duplicate identifiers impossible for a valid contiguous sequence.

If that string form is not intended to be stable public `1.0.0` wire semantics, introduce an explicit versioned job/record binding or a contract-version change instead of silently redefining arbitrary `evidence_id` text. In either design:

- validation rejects contradictions rather than rewriting forged identifiers;
- identical semantic inputs retain deterministic identity;
- every record remains attributable to exactly one enclosing analysis job;
- ordering and sequence remain deterministic;
- signing/provenance binds the exact validated serialized identities;
- JSON Schema is not overstated as proof of arbitrary cross-instance equality if the portable schema dialect cannot express the relation directly.

Random record IDs, process-local identity, downstream ignore rules, or sequence-only validation while retaining an apparently authoritative `evidence_id` are rejected alternatives.

## Risk and effect

Without the binding, a receipt can be internally well-formed yet expose conflicting reference identities. That weakens audit reconstruction and can make a signed or stored record identifier refer to evidence outside the job whose top-level provenance a consumer is inspecting. The repair narrows admissible receipts to the identity semantics production already intends; it does not authorize artifact verdicts, analyzer execution, or consumer admission.

## Related release gates

Issue #60 remains independent of #49 analyzer capability isolation, #50 bounded worker-result ingestion, #52 truthful dynamic execution/completeness, #54 stable analyzer provenance, #56 ToolFailure/completeness consistency, and #58 artifact-subject binding. Passing this contract test cannot promote ADR-0009 or artifact-analysis release readiness by itself.

## References

SLSA Community. (2025). *SLSA specification v1.2: Provenance*. https://slsa.dev/spec/v1.2/provenance

SLSA Community. (2025). *SLSA specification v1.2: Build requirements*. https://slsa.dev/spec/v1.2/build-requirements

Torres-Arias, S., Afzali, H., Kuppusamy, T. K., Curtmola, R., & Cappos, J. (2019). in-toto: Providing farm-to-table guarantees for bits and bytes. In *28th USENIX Security Symposium (USENIX Security 19)* (pp. 1393–1410). USENIX Association. https://www.usenix.org/conference/usenixsecurity19/presentation/torres-arias
