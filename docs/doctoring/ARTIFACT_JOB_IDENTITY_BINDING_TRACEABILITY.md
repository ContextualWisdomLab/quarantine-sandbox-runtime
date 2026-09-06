# Artifact Analysis Job Identity Binding Traceability

## Status

Issue #66 is a causally executed evidence-integrity defect on artifact-analysis parent Draft #18 exact `994ba5fa5ceda20c622861efbff241294867dc39`. Initial test-bearing commit `2e60083b36c7e007844dac0cddc76883f87f9e67` added the hostile receipt regression; `b19f467d7c3758e3373463b1ddbafe6767f0abd9` added runtime policy identity because production already includes that policy in deterministic job correlation. CodeRabbit's `MD018` documentation finding was repaired by `fbd9204c57d3031f43fd81b82ec60cd11aca3aab` and the review thread is resolved.

Exact RED head `4b0a192c6ed6251e434819e2066807b377801356` executed on native CI run `34038409956`. Verify job `101500613351` checked out the exact head, passed formatting, repository policy and preceding tests, then failed in `tests/artifact_analysis_job_identity_binding_red.rs` at `analysis_job_identity_must_bind_identity_bearing_receipt_inputs` with `changing request_id without changing analysis_job_id must invalidate the receipt`. This is the intended stale-job-identity cause. Hosted negative rootless/AppArmor passed on the same RED run; positive-LSM remains an independent capability gate.

Production lineage `69f2a6f65eb9f5141ade1dbaf837ec67076c6597` through `a03b1b0aa72b6ecbdcc4ce672d5c2d9cefd5ddb2` established a semantic receipt boundary and initially rejected every duplicate `PolicyBoundary`. CodeRabbit review correctly narrowed that rule: #66 needs exactly one policy-identifying source, not a blanket singleton for the entire evidence kind. A `PolicyBoundary` without `policy_id` is not a second identity source; two records that both carry `policy_id` are ambiguous even if the values match. This keeps complete foundation-cardinality ownership with #64/#65. A separate compatibility review found that the first repair changed the public meaning of `analysis_job_id` while continuing to publish the evidence contract as `1.0.0`, contrary to the live TRD and consumer-contract version rules. Commit `c53249b4ff884903800c08cc6d5aa5e999cc3b80` repaired that compatibility finding by preserving `analysis_job_id` as the existing opaque deterministic correlation identifier and introducing required `analysis_job_identity_sha256` evidence in artifact-analysis evidence schema `1.1.0`.

Current-head review then found a compile-time DDD boundary regression in `924124cd3e095a9df7941aa952c824d56c8dc16b`: the private runtime assembler still imported `crate::EvidenceBundle`, but the crate root now re-exported the public v1.1 receipt type. The private assembler constructs the v1.0 wire shape and therefore cannot construct the public v1.1 type because `analysis_job_identity_sha256` is intentionally added only after generated-wire validation. Commit `72d2c400a14e6276904ec8403da8391d32c8b937` repairs only that ownership boundary by importing `contracts::EvidenceBundle` inside `runtime.rs`; the public `analysis_engine` wrapper remains solely responsible for converting validated v1.0 assembly output through `EvidenceBundle::from_generated`. This is a type/architecture repair, not a schema-semantic change.

A fresh security review of integrated head `e5fbf2c959716587eec8ac45412514a63caeacdb` found that the v1.1 companion digest still does not satisfy the issue title's self-verifiability requirement. Every digest input is already public receipt data. A reconstructor can therefore change `request_id` (or another bound field), recompute the documented unsigned `analysis_job_identity_sha256`, retain the stale opaque `analysis_job_id`, and satisfy the current validator. Commit `4e7a0ee5de2efb3d729190531a24bc165ab1e02d` adds that hostile reconstruction as a RED: the test independently recomputes the exact published digest algorithm after changing `request_id` and requires rejection. This is intentionally test-only. The v1.1 companion digest remains useful checksum-style contradiction detection, but it is not proof that the opaque job ID was actually derived from the claimed inputs.

## Problem and contract authority

The runtime's internal `deterministic_job_id()` hashes `request_id`, requested profile, artifact SHA-256, policy ID, runtime source revision, and configured analyzer IDs. The TRD requires deterministic evidence identity for the same request/configuration/bytes. Before #66, the public validator checked `analysis_job_id` only as bounded text, so a deserialized receipt could keep a stale identifier while changing receipt-visible identity fields.

A same-version rewrite of `analysis_job_id` into a new composite format is not an acceptable repair. The consumer contract says a field-meaning change requires a new major contract version, while a minor revision may add required security evidence for that revision. #66 therefore needs an explicit versioned route that preserves v1.0 semantics while making the derivation independently checkable in the new receipt revision. A checksum over already-public fields is insufficient because an untrusted reconstructor can recompute it.

`#60/#61` is narrower: it binds each `EvidenceRecord.evidence_id` to its enclosing job ID and sequence. Canonical record IDs do not make the enclosing job identity truthful. #58/#59 binds duplicated artifact-subject representations; #62/#63 binds duplicated runtime-boundary facts; #64/#65 owns foundation-record cardinality; #54/#55 owns stable analyzer provenance; #52/#53 owns profile/execution/completeness semantics. True #66 verification also depends on #54/#55 because configured analyzer identity participates in `deterministic_job_id()` but is not yet available as stable versioned receipt input.

## DDD ownership

`artifact_analysis` owns deterministic analysis-job identity, normalized evidence identity, artifact subject identity, runtime policy attribution inside the receipt, and receipt-integrity validation. `sandbox_execution` owns worker/runtime isolation and lifecycle evidence. Stable analyzer provenance remains #54/#55 and must not be replaced with mutable `producer_id` display strings.

The public artifact-analysis boundary separates the private v1.0-compatible assembly model from the published evidence revision. Request schema `1.0.0` remains unchanged. `runtime.rs` must use the private `contracts::EvidenceBundle` assembly type; only `analysis_engine.rs` may cross into the public receipt type after structural validation. Importing the public type back into the private assembler collapses the ACL between generated wire data and published receipt validation and is rejected as an architectural regression.

## Causal GREEN design

The current EvidenceBundle `1.1.0` candidate preserves the existing `analysis_job_id` value and adds `analysis_job_identity_sha256`, the full lower-case SHA-256 of these UTF-8 components in order, with one NUL byte after every component:

1. `analysis_job_id`;
2. `request_id`;
3. `runtime.requested_profile.as_str()`;
4. `artifact.artifact_sha256`;
5. the single unambiguous policy-identifying `PolicyBoundary.attributes["policy_id"]`;
6. `runtime.source_revision`.

That representation detects stale-field contradictions only while the companion digest itself is held fixed. It does not prove that `analysis_job_id` was derived from the remaining fields because the digest can be recomputed from the receipt. The new RED therefore reopens the GREEN design. A valid completion must expose enough versioned canonical derivation input to recompute the expected job identity, or otherwise use a cryptographically authenticated derivation/attestation that an untrusted receipt editor cannot regenerate. Because configured analyzer IDs are part of the current deterministic job-ID derivation, #54/#55 stable analyzer identity is now a prerequisite rather than an optional follow-up. Random UUIDs, mutable `producer_id`, output-derived identity, or an unsigned checksum over public fields are not substitutes.

Generated `EvidenceRecord.evidence_id` values remain unchanged and continue to reference the legacy-format `analysis_job_id`; #60/#61 separately owns their referential validation. #64/#65 remains responsible for broader canonical foundation-set cardinality. #66 may consume only the minimum unambiguous policy and analyzer identity needed to reproduce its own derivation contract.

JSON Schema Draft 2020-12 can require a digest field and canonical SHA-256 syntax but cannot recompute cryptographic derivation across fields. Any successor wire shape therefore needs Rust semantic validation and immutable schema snapshots in addition to structural schema validation. The prior v1.0 schema remains immutable; no fix may silently rewrite old semantics.

## RED and compatibility acceptance

`tests/artifact_analysis_job_identity_binding_red.rs` requires:

- an untouched `StaticOnly` control receipt to remain valid for the currently published candidate revision;
- changing `analysis_job_id` while retaining the old companion digest to fail closed;
- changing only `request_id` while retaining the old identity evidence to fail closed;
- changing top-level artifact SHA-256 and nested `ArtifactIdentity.artifact_sha256` together to one alternate valid digest, while retaining the old identity evidence, to fail closed without relying on #58/#59;
- changing runtime-owned `PolicyBoundary.attributes["policy_id"]` while retaining the old identity evidence to fail closed;
- removing the only `policy_id` source to fail closed;
- adding a non-identifying `PolicyBoundary` without `policy_id` to remain valid within #66;
- adding a second policy-identifying `PolicyBoundary` to fail closed whether its `policy_id` matches or contradicts the first;
- changing `RuntimeManifest.source_revision` while retaining the old identity evidence to fail closed;
- malformed `analysis_job_identity_sha256` values to fail closed;
- **changing `request_id`, recomputing the documented companion digest, and retaining the stale opaque `analysis_job_id` to still fail closed**;
- current/versioned JSON Schemas to agree and archived prior schemas to remain immutable;
- the private runtime assembler to remain typed as the private assembly contract while the public wrapper performs publication.

The new recomputation RED is the decisive guard against treating checksum consistency as self-verifiable derivation. Production must not be changed for this new cause until it executes. Requested-profile completeness remains #52/#53. Stable analyzer derivation input remains #54/#55 and is a prerequisite for a complete #66 GREEN.

## Evidence basis

SLSA v1.2 was released on 2025-11-24 and is the current Approved specification. Its Provenance section defines provenance as verifiable information used to trace an artifact back to where and how it was produced. The peer-reviewed in-toto model likewise relies on verifiable links between artifacts and the steps/evidence describing them. These sources support verifiable identity and integrity; they do not make an unsigned, attacker-recomputable checksum authoritative.

## References

SLSA Community. (2025, November 24). *Announcing SLSA v1.2*. https://slsa.dev/blog/2025/11/announce-slsa-v1.2

SLSA Community. (2025). *SLSA specification v1.2: Provenance*. https://slsa.dev/spec/v1.2/provenance

Torres-Arias, S., Afzali, H., Kuppusamy, T. K., Curtmola, R., & Cappos, J. (2019). in-toto: Providing farm-to-table guarantees for bits and bytes. In *28th USENIX Security Symposium (USENIX Security 19)* (pp. 1393–1410). USENIX Association. https://www.usenix.org/conference/usenixsecurity19/presentation/torres-arias

## Release effect

No artifact-analysis receipt is release-authoritative while identity-bearing request/subject/policy/runtime/analyzer inputs cannot be independently tied to the published job identifier, while an untrusted reconstructor can retain a stale opaque `analysis_job_id` and merely recompute public checksum evidence, while policy or analyzer derivation identity is absent/ambiguous, while the private assembly/public receipt ACL is collapsed, or while consumers can confuse schema revisions. Exact-head causal RED then GREEN is required; a GREEN here does not waive #49/#50/#52/#54/#56/#58/#60/#62/#64, real positive isolation, exact-head review/security/coverage, protected integration, SBOM/provenance/reproducibility, rollback, or immutable release requirements.
