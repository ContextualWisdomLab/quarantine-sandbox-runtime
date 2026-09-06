# Artifact Analysis Job Identity Binding Traceability

## Status

Issue #66 is a causally executed evidence-integrity defect on artifact-analysis parent Draft #18 exact `994ba5fa5ceda20c622861efbff241294867dc39`. Initial test-bearing commit `2e60083b36c7e007844dac0cddc76883f87f9e67` added the hostile receipt regression; `b19f467d7c3758e3373463b1ddbafe6767f0abd9` added runtime policy identity because production already includes that policy in deterministic job correlation. CodeRabbit's `MD018` documentation finding was repaired by `fbd9204c57d3031f43fd81b82ec60cd11aca3aab` and the review thread is resolved.

Exact RED head `4b0a192c6ed6251e434819e2066807b377801356` executed on native CI run `34038409956`. Verify job `101500613351` checked out the exact head, passed formatting, repository policy and preceding tests, then failed in `tests/artifact_analysis_job_identity_binding_red.rs` at `analysis_job_id_must_bind_identity_bearing_receipt_inputs` with `changing request_id without changing analysis_job_id must invalidate the receipt`. This is the intended stale-job-identity cause. Hosted negative rootless/AppArmor passed on the same RED run; positive-LSM remains an independent capability gate.

Production lineage `69f2a6f65eb9f5141ade1dbaf837ec67076c6597` through `a03b1b0aa72b6ecbdcc4ce672d5c2d9cefd5ddb2` established a semantic receipt boundary and then closed ambiguous duplicate `PolicyBoundary` selection. Review of that candidate found a separate compatibility defect: it changed the public meaning of `analysis_job_id` while continuing to publish the evidence contract as `1.0.0`, contrary to the live TRD and consumer-contract version rules. Commit `c53249b4ff884903800c08cc6d5aa5e999cc3b80` repairs that compatibility finding by preserving `analysis_job_id` as the existing opaque deterministic correlation identifier and introducing required `analysis_job_identity_sha256` evidence in artifact-analysis evidence schema `1.1.0`.

## Problem and contract authority

The runtime's internal `deterministic_job_id()` hashes `request_id`, requested profile, artifact SHA-256, policy ID, runtime source revision, and configured analyzer IDs. The TRD requires deterministic evidence identity for the same request/configuration/bytes. Before #66, the public validator checked `analysis_job_id` only as bounded text, so a deserialized receipt could keep a stale identifier while changing receipt-visible identity fields.

A same-version rewrite of `analysis_job_id` into a new composite format is not an acceptable repair. The consumer contract says a field-meaning change requires a new major contract version, while a minor revision may add required security evidence for that revision. #66 therefore uses the additive minor-version route: v1.0 keeps its opaque job identifier semantics; v1.1 adds an explicit digest field that binds that identifier to receipt-visible identity inputs.

`#60/#61` is narrower: it binds each `EvidenceRecord.evidence_id` to its enclosing job ID and sequence. Canonical record IDs do not make the enclosing job identity truthful. #58/#59 binds duplicated artifact-subject representations; #62/#63 binds duplicated runtime-boundary facts; #64/#65 owns foundation-record cardinality; #54/#55 owns stable analyzer provenance; #52/#53 owns profile/execution/completeness semantics. #66 consumes only the minimum policy-boundary uniqueness needed to make its policy hash input unambiguous; #64/#65 still owns the complete canonical foundation-set invariant for `ArtifactIdentity`, bundled `FileFormat`, and `PolicyBoundary`.

## DDD ownership

`artifact_analysis` owns deterministic analysis-job identity, normalized evidence identity, artifact subject identity, runtime policy attribution inside the receipt, and receipt-integrity validation. `sandbox_execution` owns worker/runtime isolation and lifecycle evidence. Stable analyzer provenance remains #54/#55 and must not be replaced with mutable `producer_id` display strings.

The public artifact-analysis boundary therefore separates the private v1.0-compatible assembly model from the published evidence revision. Request schema `1.0.0` remains unchanged. Evidence schema `1.1.0` is a deliberate consumer upgrade and carries the additional identity-binding evidence.

## Causal GREEN design

EvidenceBundle `1.1.0` preserves the existing `analysis_job_id` value and adds required field:

`analysis_job_identity_sha256`

The field is the full lower-case SHA-256 of these UTF-8 components in order, with one NUL byte after every component:

1. `analysis_job_id`;
2. `request_id`;
3. `runtime.requested_profile.as_str()`;
4. `artifact.artifact_sha256`;
5. the single unambiguous `PolicyBoundary.attributes["policy_id"]`;
6. `runtime.source_revision`.

Including `analysis_job_id` means the companion digest detects a stale or substituted job identifier without redefining that identifier's v1.0 format. The job identifier continues to carry the runtime's analyzer-sensitive deterministic correlation, but the digest does not make the analyzer list independently verifiable. #54/#55 remains responsible for stable versioned analyzer identity/attestation.

Generated `EvidenceRecord.evidence_id` values remain unchanged and continue to reference the legacy-format `analysis_job_id`; #60/#61 separately owns their referential validation. Public `EvidenceBundle::validate()` requires schema `1.1.0`, validates the inherited structural fields through the private v1.0 assembly contract, requires exactly one `PolicyBoundary` as the policy-identity source, validates the new digest as 64 lower-case hex, recomputes it, and fails closed on contradiction.

The repair does not claim authentication. A party able to rewrite the entire unsigned receipt and recompute the digest can still construct a self-consistent receipt. Signed/attested provenance, immutable worker identity, and analyzer provenance remain separate release gates. This control closes stale-field contradiction; it does not substitute for provenance authenticity.

JSON Schema Draft 2020-12 can require the new digest field and its canonical SHA-256 syntax but cannot recompute it across fields. `schemas/evidence-bundle.schema.json` and immutable snapshot `schemas/evidence-bundle-1.1.0.schema.json` therefore describe the v1.1 structural contract, while the Rust validator owns cross-field equivalence. The prior v1.0 schema is preserved byte-for-byte at `schemas/evidence-bundle-1.0.0.schema.json`; it is not silently rewritten into the new semantics.

## RED and compatibility acceptance

`tests/artifact_analysis_job_identity_binding_red.rs` requires:

- an untouched `StaticOnly` control receipt to remain valid as evidence schema `1.1.0`;
- changing `analysis_job_id` while retaining the old companion digest to fail closed;
- changing only `request_id` while retaining the old identity evidence to fail closed;
- changing top-level artifact SHA-256 and nested `ArtifactIdentity.artifact_sha256` together to one alternate valid digest, while retaining the old identity evidence, to fail closed without relying on #58/#59;
- changing runtime-owned `PolicyBoundary.attributes["policy_id"]` while retaining the old identity evidence to fail closed;
- removing the policy identity or introducing a second contradictory `PolicyBoundary` to fail closed rather than selecting one record by order;
- changing `RuntimeManifest.source_revision` while retaining the old identity evidence to fail closed;
- malformed `analysis_job_identity_sha256` values to fail closed;
- the current and immutable v1.1 JSON Schemas to agree, while the archived v1.0 schema remains explicitly `1.0.0` and lacks the new required field;
- feeding a v1.0 schema version into the v1.1 Rust evidence validator to fail explicitly as an unsupported revision rather than silently applying v1.1 semantics.

Requested-profile mutation remains outside this focused RED because #52/#53 owns profile/completeness semantics. Analyzer display-ID mutation remains outside because #54/#55 owns stable analyzer provenance.

## Evidence basis

SLSA v1.2 was released on 2025-11-24 and is the current Approved specification. Its Provenance section defines provenance as verifiable information used to trace an artifact back to where and how it was produced. The peer-reviewed in-toto model likewise relies on verifiable links between artifacts and the steps/evidence describing them. These sources support verifiable identity and integrity; this repository's versioned receipt semantics remain normative.

## References

SLSA Community. (2025, November 24). *Announcing SLSA v1.2*. https://slsa.dev/blog/2025/11/announce-slsa-v1.2

SLSA Community. (2025). *SLSA specification v1.2: Provenance*. https://slsa.dev/spec/v1.2/provenance

Torres-Arias, S., Afzali, H., Kuppusamy, T. K., Curtmola, R., & Cappos, J. (2019). in-toto: Providing farm-to-table guarantees for bits and bytes. In *28th USENIX Security Symposium (USENIX Security 19)* (pp. 1393–1410). USENIX Association. https://www.usenix.org/conference/usenixsecurity19/presentation/torres-arias

## Release effect

No artifact-analysis receipt is release-authoritative while identity-bearing request/subject/policy/runtime fields can contradict its published job identity evidence, while the policy identity used for that digest is ambiguous, or while a consumer can mistake the v1.1 semantics for v1.0. Exact-head GREEN for #66 is still required. A GREEN here does not waive #49/#50/#52/#54/#56/#58/#60/#62/#64, real positive isolation, exact-head review/security/coverage, protected integration, SBOM/provenance/reproducibility, rollback, or immutable release requirements.
