# Artifact Evidence Unknown-Field Deserialization Traceability

## Problem

The published artifact-evidence JSON contract and the Rust deserializer currently accept different document sets. `schemas/evidence-bundle.schema.json` uses Draft 2020-12 `additionalProperties: false` at the top-level bundle and nested artifact, runtime, and evidence-record objects. The corresponding Rust structures derived `Deserialize` without Serde `deny_unknown_fields` at the causal RED head.

Serde documents that self-describing formats such as JSON ignore unknown fields by default. An unreviewed producer field could therefore be rejected by the published schema but silently discarded by the Rust boundary. The resulting in-memory value was not the same contract document that arrived on the wire, which weakened version-drift detection and could conceal unsupported evidence semantics.

## Constraints and ownership

- `artifact_analysis` is the Supporting bounded context that owns the versioned evidence wire ACL.
- Wardnet or another consumer retains verdict/incident authority; strict parsing does not move that authority here.
- Existing `1.0.0` JSON Schema already rejects additional object members, so the repair aligns Rust with the published contract rather than inventing a new wire version.
- Unknown members must not be stripped, coerced, or retained in an untyped extension map without a separately reviewed/versioned contract.
- Isolation/runtime behavior is unchanged by this repair.
- Repository-wide `docs/product-technical-gap-baseline.md` is a single-writer ledger owned by the canonical gap-baseline lane (#121), not by this focused contract leaf.

## Alternatives

1. **Keep Serde's permissive default.** Rejected because schema validation and Rust deserialization would continue to disagree and producer drift could be silently erased.
2. **Accept arbitrary extension members.** Rejected for `1.0.0` because the published schema already sets `additionalProperties: false`; an extension mechanism would be a separate contract decision.
3. **Reject unknown fields in the published Rust wire structures.** Selected because it makes the implementation honor the existing strict schema with the smallest causal change.

## Executed RED

Issue #75 / Draft #76 owns the focused contract repair. Test-bearing commit `cc326024962fcfbd4a988f254e7efbe0ae997d99` added `tests/artifact_analysis_unknown_field_deserialization_red.rs` above artifact-analysis parent #18 exact `c0647152ec052d82969b2ae078891e25e6d4d69a`; ledger movement produced exact RED head `fec3de0e7f6b37e87875143ed0031418b25d2459`.

Native CI `34086035165`, verify job `101630147469`, executed that exact head on Rust 1.97.1. Exact checkout, dependency-lock validation, repository policy, coverage-parser tests, and `cargo fmt --check` all passed before the focused regression ran. The schema-valid positive fixture deserialized and validated successfully. Four independent hostile cases then failed for the intended acceptance-set cause: unknown top-level `EvidenceBundle`, nested `ArtifactDescriptor`, `RuntimeManifest`, and `EvidenceRecord` members were accepted and silently discarded instead of causing deserialization failure. Hosted negative rootless/AppArmor job `101630147476` independently passed; positive-LSM evidence remains a separate release gate.

## Minimum causal repair

Commit `62f45df35212119c9cd1cc5fe957623f9a605a4c` adds only `#[serde(deny_unknown_fields)]` to `ArtifactDescriptor`, `EvidenceRecord`, `RuntimeManifest`, and `EvidenceBundle`. No field, schema, serialization shape, runtime behavior, evidence taxonomy, or verdict authority changed. The valid published fixture remains the positive control for compatibility.

Exact `006849441402332bf05ebe8da2eac0018af623ef` later executed the focused strict-deserialization suite GREEN: the valid published bundle and all four unknown-member rejection cases passed. The broader workspace still failed later in inherited command-runtime ancestry, so this remains focused exact-head evidence rather than whole-head merge authority.

## Single-writer gap-ledger repair

Review `5229219444` found that #76 still carried a historical `docs/product-technical-gap-baseline.md` delta even though repository-wide Gap authority is maintained by #121. Leaving that leaf-owned ledger in place could reintroduce stale owner pointers during later integration.

Ordinary fast-forward `b3600681cbdcb5f5b456f8f7a167ca8c1e9e818f` restores the global baseline byte-for-byte to the exact #18 base blob `ea0310394a3d842246bae380977a30c72c18cbf9`. This owner-boundary repair changes no production Rust, public schema, serialization contract, or focused regression. The current documentation head records the repair locally; no predecessor CI status transfers after head movement.

## Current gate

The moved head must independently reacquire repository validation, rustfmt, full locked workspace/all-target tests, Clippy, public/private rustdoc with warnings denied, complete applicable owned-production statement/function/region/branch/edge coverage, qualifying review/security/thread gates, positive effective-isolation evidence where applicable, protected integration, and immutable version/package/SBOM/provenance/reproducibility/rollback publication. The focused GREEN on `006849441...` remains historical evidence only.

## Evidence and references

The published `schemas/evidence-bundle.schema.json` sets `additionalProperties: false` for the top-level bundle and the nested artifact, runtime, and evidence-record objects. Exact-head native CI demonstrated that the previous Rust `Deserialize` implementations accepted those four unknown-field cases despite the strict published schema.

Serde Project. (n.d.). *Container attributes*. Serde. https://serde.rs/container-attrs.html

Wright, A., Andrews, H., Hutton, B., & Dennis, G. (2022). *JSON Schema: A media type for describing JSON documents (Draft 2020-12)*. JSON Schema. https://json-schema.org/draft/2020-12/json-schema-core

JSON Schema. (2022, June 16). *Draft 2020-12*. https://json-schema.org/draft/2020-12
