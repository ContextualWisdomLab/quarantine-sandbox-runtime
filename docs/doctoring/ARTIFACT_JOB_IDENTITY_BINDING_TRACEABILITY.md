# Artifact Analysis Job Identity Binding Traceability

## Status

Issue #66 is a RED-only evidence-integrity finding on artifact-analysis parent Draft #18 exact `994ba5fa5ceda20c622861efbff241294867dc39`. Initial test-bearing commit `2e60083b36c7e007844dac0cddc76883f87f9e67` added the hostile receipt regression; latest hardening `b19f467d7c3758e3373463b1ddbafe6767f0abd9` also binds the runtime policy identity that production already hashes into `analysis_job_id`. Production Rust and JSON Schema remain inherited unchanged until the exact RED executes for the intended cause.

## Problem and contract authority

`AnalysisEngine::deterministic_job_id()` hashes `request_id`, requested profile, artifact SHA-256, policy ID, runtime source revision, and configured analyzer IDs. The TRD requires evidence identifiers and ordering to be deterministic for the same request/configuration/bytes. The public `EvidenceBundle` in turn labels `analysis_job_id` as the deterministic analysis-job identifier.

`EvidenceBundle::validate()` does not verify any relationship between that identifier and the identity-bearing fields serialized beside it. It only validates `analysis_job_id` as bounded non-empty text. A reconstructed receipt can therefore keep the old job ID while changing request identity, artifact subject, runtime policy identity, or runtime source revision and still remain structurally acceptable.

`#60/#61` is narrower: it binds each `EvidenceRecord.evidence_id` to the enclosing job ID and sequence. Canonical record IDs do not make the enclosing job ID itself truthful. #58/#59 binds duplicated artifact-subject representations; #62/#63 binds duplicated runtime-boundary booleans; #54/#55 owns stable analyzer provenance; #52/#53 owns profile/execution/completeness semantics.

## DDD ownership

`artifact_analysis` owns deterministic analysis-job identity, normalized evidence identity, subject identity, runtime policy attribution within the receipt, and receipt integrity. `sandbox_execution` owns worker/runtime isolation and lifecycle evidence. Analyzer implementation provenance remains #54/#55; this issue must compose with that versioned provenance rather than treating mutable `producer_id` strings as identity authority.

## RED acceptance

`tests/artifact_analysis_job_identity_binding_red.rs` requires:

- an untouched `StaticOnly` control receipt to remain valid;
- changing only `request_id` while preserving the old `analysis_job_id` and evidence IDs to fail closed;
- changing top-level artifact SHA-256 and the nested `ArtifactIdentity.artifact_sha256` together to the same alternate valid digest, while preserving the old job ID, to fail closed so #58/#59 cannot be the accidental failure cause;
- changing `PolicyBoundary.attributes["policy_id"]` while preserving the old job ID to fail closed because production hashes that policy ID into deterministic job identity;
- changing `RuntimeManifest.source_revision` while preserving the old job ID to fail closed.

The RED intentionally does not mutate requested profile because #52/#53 already owns profile/completeness semantics. It does not mutate analyzer display IDs because #54/#55 owns stable analyzer provenance.

## Smallest causal GREEN after executed RED

A regex or prefix check on `analysis_job_id` is not sufficient. The job identity must become verifiable from a versioned canonical identity descriptor or another cryptographically bound representation. Validation must reject contradiction rather than silently recalculate and overwrite a forged/stale ID.

If schema `1.0.0` does not expose enough trustworthy identity inputs to recompute the current hash without relying on mutable producer text, the repair should introduce an explicit versioned job-identity input/provenance structure and compatibility/version transition. Hash input order and separators must be documented as contract data if consumers are expected to verify the digest; otherwise a signed/attested identity structure should be authoritative and the opaque job ID should be treated only as correlation metadata.

The repair must preserve #58/#59 subject consistency, #60/#61 record/job referential integrity, #62/#63 runtime-boundary consistency, #64/#65 structural foundation cardinality, and #54/#55 analyzer provenance.

## Evidence basis

SLSA v1.2 was released on 2025-11-24 and is the current Approved specification. Its Provenance section defines provenance as verifiable information used to trace an artifact back to where and how it was produced. The peer-reviewed in-toto model likewise relies on verifiable links between artifacts and the steps/evidence describing them. These sources support verifiable identity and integrity; this repository's versioned job-identity contract remains the normative authority.

## References

SLSA Community. (2025, November 24). *Announcing SLSA v1.2*. https://slsa.dev/blog/2025/11/announce-slsa-v1.2

SLSA Community. (2025). *SLSA specification v1.2: Provenance*. https://slsa.dev/spec/v1.2/provenance

Torres-Arias, S., Afzali, H., Kuppusamy, T. K., Curtmola, R., & Cappos, J. (2019). in-toto: Providing farm-to-table guarantees for bits and bytes. In *28th USENIX Security Symposium (USENIX Security 19)* (pp. 1393–1410). USENIX Association. https://www.usenix.org/conference/usenixsecurity19/presentation/torres-arias

## Release effect

No artifact-analysis receipt is release-authoritative while identity-bearing request/subject/policy/runtime fields can change without invalidating the deterministic `analysis_job_id`. A GREEN for #66 does not waive #49/#50/#52/#54/#56/#58/#60/#62/#64, real positive isolation, exact-head review/security/coverage, protected integration, SBOM/provenance/reproducibility, rollback, or immutable release requirements.
