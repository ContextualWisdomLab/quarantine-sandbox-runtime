# Artifact Analysis Job Identity Binding Traceability

## Status

Issue #66 is a causally executed evidence-integrity defect on artifact-analysis parent Draft #18 exact `994ba5fa5ceda20c622861efbff241294867dc39`. Initial test-bearing commit `2e60083b36c7e007844dac0cddc76883f87f9e67` added the hostile receipt regression; `b19f467d7c3758e3373463b1ddbafe6767f0abd9` added runtime policy identity because production already includes that policy in deterministic job correlation. CodeRabbit's `MD018` documentation finding was repaired by `fbd9204c57d3031f43fd81b82ec60cd11aca3aab` and the review thread is resolved.

Exact RED head `4b0a192c6ed6251e434819e2066807b377801356` executed on native CI run `34038409956`. Verify job `101500613351` checked out the exact head, passed formatting, repository policy and preceding tests, then failed in `tests/artifact_analysis_job_identity_binding_red.rs` at `analysis_job_id_must_bind_identity_bearing_receipt_inputs` with `changing request_id without changing analysis_job_id must invalidate the receipt`. This is the intended stale-job-identity cause. Hosted negative rootless/AppArmor passed on the same RED run; positive-LSM remains an independent capability gate.

Production candidate `69f2a6f65eb9f5141ade1dbaf837ec67076c6597` introduced the semantic receipt boundary. `36aead6d6700208a0c6a3dc8962d3739a7071cf3` kept the public receipt shape explicit instead of exposing the private raw wire type. Contract-fixture repairs `5244f21523794db7f080af5c8e49f1e91fa214ea` and `51617260bb3e7ea5e15da7c1d46fddd0d4527187` moved legacy hand-built fixtures onto production-generated valid receipts.

Current-head review then found that the receipt-visible digest selected the first `PolicyBoundary` record while the inherited v1 wire contract still permits duplicate foundation evidence. A reconstructed receipt could therefore retain the original policy record first, append a contradictory second `PolicyBoundary`, and keep the old job identifier. Test commit `df54494d51520b860b32b1fae78b9226e90be859` makes that ambiguity executable; minimal repair `38e6169ec6d01251ec04a4596c76dc0110dcf3b1` requires exactly one `PolicyBoundary` as the policy-identity source. This is semantic unambiguity, not trusted origin, and does not promote mutable `producer_id` to provenance. The candidate is not GREEN authority until exact-head verify, coverage, and branch coverage execute successfully.

## Problem and contract authority

The runtime's internal `deterministic_job_id()` hashes `request_id`, requested profile, artifact SHA-256, policy ID, runtime source revision, and configured analyzer IDs. The TRD requires deterministic evidence identity for the same request/configuration/bytes. Before #66, the public validator checked `analysis_job_id` only as bounded text, so a deserialized receipt could keep a stale identifier while changing receipt-visible identity fields.

`#60/#61` is narrower: it binds each `EvidenceRecord.evidence_id` to its enclosing job ID and sequence. Canonical record IDs do not make the enclosing job ID truthful. #58/#59 binds duplicated artifact-subject representations; #62/#63 binds duplicated runtime-boundary facts; #64/#65 owns foundation-record cardinality; #54/#55 owns stable analyzer provenance; #52/#53 owns profile/execution/completeness semantics. #66 only consumes the minimum cardinality needed to make its policy hash input unambiguous; #64/#65 still owns the complete canonical foundation-set invariant for `ArtifactIdentity`, bundled `FileFormat`, and `PolicyBoundary`.

## DDD ownership

`artifact_analysis` owns deterministic analysis-job identity, normalized evidence identity, artifact subject identity, runtime policy attribution inside the receipt, and receipt-integrity validation. `sandbox_execution` owns worker/runtime isolation and lifecycle evidence. Stable analyzer provenance remains #54/#55 and must not be replaced with mutable `producer_id` display strings.

The repair therefore uses a public artifact-analysis anti-corruption boundary: private runtime assembly may retain its current analyzer-sensitive correlation identifier, while the public receipt publishes a semantically verifiable identity derived from receipt-visible fields.

## Causal GREEN design

For generated receipts, the public job identifier is:

`analysis_job_<receipt_identity_sha256>_<legacy_analyzer_sensitive_suffix>`

where `receipt_identity_sha256` is the full lower-case SHA-256 of these UTF-8 components in order, with one NUL byte after every component:

1. `request_id`;
2. `runtime.requested_profile.as_str()`;
3. `artifact.artifact_sha256`;
4. the single unambiguous `PolicyBoundary.attributes["policy_id"]`;
5. `runtime.source_revision`.

The trailing 32-hex suffix preserves the runtime's existing analyzer-sensitive deterministic correlation. It is syntax-checked but is not promoted to trusted provenance. #54/#55 remains responsible for a stable versioned analyzer identity/attestation contract.

Generated `EvidenceRecord.evidence_id` values are rewritten against the published composite job ID so the public receipt remains internally referentially consistent. `EvidenceBundle::validate()` first applies the inherited `1.0.0` structural wire checks and then recomputes the receipt-visible digest. Missing or duplicated policy boundaries, missing policy identity, malformed composite job identity, or a digest contradiction fail closed.

The repair does not claim authentication. A party able to rewrite the entire receipt and recompute an unsigned digest can still construct a self-consistent receipt. Signed/attested provenance, immutable worker identity, and analyzer provenance remain separate release gates. This control closes stale-field contradiction; it does not substitute for provenance authenticity.

The JSON Schema remains the structural `1.0.0` wire schema. Draft 2020-12 cannot recompute SHA-256 over multiple fields, so the semantic cross-field binding is enforced by the Rust public validator rather than represented as a misleading regex. The composite identifier remains within the existing 128-byte bounded string contract, preserving the schema's opaque identifier compatibility.

## RED acceptance retained

`tests/artifact_analysis_job_identity_binding_red.rs` requires:

- an untouched `StaticOnly` control receipt to remain valid;
- changing only `request_id` while preserving the old job/evidence IDs to fail closed;
- changing top-level artifact SHA-256 and nested `ArtifactIdentity.artifact_sha256` together to one alternate valid digest, while preserving the old job ID, to fail closed without relying on #58/#59;
- changing runtime-owned `PolicyBoundary.attributes["policy_id"]` while preserving the old job ID to fail closed;
- removing the policy identity or introducing a second contradictory `PolicyBoundary` to fail closed rather than selecting one record by order;
- changing `RuntimeManifest.source_revision` while preserving the old job ID to fail closed.

Requested profile mutation remains outside this focused RED because #52/#53 owns profile/completeness semantics. Analyzer display-ID mutation remains outside because #54/#55 owns stable analyzer provenance.

## Evidence basis

SLSA v1.2 was released on 2025-11-24 and is the current Approved specification. Its Provenance section defines provenance as verifiable information used to trace an artifact back to where and how it was produced. The peer-reviewed in-toto model likewise relies on verifiable links between artifacts and the steps/evidence describing them. These sources support verifiable identity and integrity; this repository's versioned receipt semantics remain normative.

## References

SLSA Community. (2025, November 24). *Announcing SLSA v1.2*. https://slsa.dev/blog/2025/11/announce-slsa-v1.2

SLSA Community. (2025). *SLSA specification v1.2: Provenance*. https://slsa.dev/spec/v1.2/provenance

Torres-Arias, S., Afzali, H., Kuppusamy, T. K., Curtmola, R., & Cappos, J. (2019). in-toto: Providing farm-to-table guarantees for bits and bytes. In *28th USENIX Security Symposium (USENIX Security 19)* (pp. 1393–1410). USENIX Association. https://www.usenix.org/conference/usenixsecurity19/presentation/torres-arias

## Release effect

No artifact-analysis receipt is release-authoritative while identity-bearing request/subject/policy/runtime fields can contradict its published deterministic job identity, or while the policy identity used for that digest is ambiguous. Exact-head GREEN for #66 is still required. A GREEN here does not waive #49/#50/#52/#54/#56/#58/#60/#62/#64, real positive isolation, exact-head review/security/coverage, protected integration, SBOM/provenance/reproducibility, rollback, or immutable release requirements.
