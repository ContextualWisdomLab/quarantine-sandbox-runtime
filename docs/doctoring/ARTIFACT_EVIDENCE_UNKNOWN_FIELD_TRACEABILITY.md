# Artifact Evidence Unknown-Field Deserialization Traceability

## Problem

The published artifact-evidence JSON contract and the Rust deserializer currently accept different document sets. `schemas/evidence-bundle.schema.json` uses Draft 2020-12 `additionalProperties: false` at the top-level bundle and nested artifact, runtime, and evidence-record objects. The corresponding Rust structures derive `Deserialize` without Serde `deny_unknown_fields`.

Serde documents that self-describing formats such as JSON ignore unknown fields by default. An unreviewed producer field can therefore be rejected by the published schema but silently discarded by the Rust boundary. The resulting in-memory value is not the same contract document that arrived on the wire, which weakens version-drift detection and can conceal unsupported evidence semantics.

## Constraints and ownership

- `artifact_analysis` is the Supporting bounded context that owns the versioned evidence wire ACL.
- Wardnet or another consumer retains verdict/incident authority; strict parsing does not move that authority here.
- Existing `1.0.0` JSON Schema already rejects additional object members, so the repair must align Rust with the published contract rather than invent a new wire version.
- Unknown members must not be stripped, coerced, or retained in an untyped extension map without a separately reviewed/versioned contract.
- Isolation/runtime behavior is unchanged by this repair.

## Alternatives

1. **Keep Serde's permissive default.** Rejected because schema validation and Rust deserialization would continue to disagree and producer drift could be silently erased.
2. **Accept arbitrary extension members.** Rejected for `1.0.0` because the published schema already sets `additionalProperties: false`; an extension mechanism would be a separate contract decision.
3. **Reject unknown fields in the published Rust wire structures.** Selected because it makes the implementation honor the existing strict schema with the smallest causal change.

## RED

Issue #75 owns the focused contract repair. Test-bearing commit `cc326024962fcfbd4a988f254e7efbe0ae997d99` adds `tests/artifact_analysis_unknown_field_deserialization_red.rs` above artifact-analysis parent #18 exact `c0647152ec052d82969b2ae078891e25e6d4d69a`.

The fixture first proves an unchanged valid `EvidenceBundle` still deserializes and validates. It then injects one otherwise-valid unknown member independently at the `EvidenceBundle`, `ArtifactDescriptor`, `RuntimeManifest`, and `EvidenceRecord` levels and requires `serde_json` deserialization to fail. At the time this document was written the RED is checked in but has not yet been claimed as causally executed; transient queued/running job IDs belong in PR/Issue metadata, not versioned traceability.

## Minimum GREEN

After exact-head CI executes the RED for the intended unknown-field cause, add `#[serde(deny_unknown_fields)]` to exactly the published evidence structures required by the strict schema. Preserve field names, serialization shape, schema version, domain validation, and public ownership boundaries. Re-run fmt, workspace tests, clippy, rustdoc, repository validation, exact 100% owned production statement/function/region/branch coverage, security checks, and review gates on one unchanged head.

## Evidence and references

The repository source at parent #18 shows `additionalProperties: false` in the published evidence schema while the four Rust structures derive `Serialize, Deserialize` without a strict unknown-field attribute.

Serde Project. (n.d.). *Container attributes*. Serde. https://serde.rs/container-attrs.html

Wright, A., Andrews, H., Hutton, B., & Dennis, G. (2022). *JSON Schema: A media type for describing JSON documents (Draft 2020-12)*. JSON Schema. https://json-schema.org/draft/2020-12/json-schema-core

JSON Schema. (2022, June 16). *Draft 2020-12*. https://json-schema.org/draft/2020-12
