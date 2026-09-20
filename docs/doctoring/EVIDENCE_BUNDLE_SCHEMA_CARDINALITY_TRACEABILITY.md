# EvidenceBundle schema-cardinality traceability

## Problem and bounded-context authority

`artifact_analysis` owns the public `EvidenceBundle` contract. Runtime validator work on prerequisite #65 requires v1 bundles to contain exactly one `ArtifactIdentity`, one `FileFormat`, and one `PolicyBoundary` evidence record. The published `schemas/evidence-bundle.schema.json` still constrains only the evidence array shape and `minItems`, so a schema-only consumer can admit serialized states that the Rust aggregate rejects.

This is public contract parity, not analyzer provenance. Stable analyzer/runtime origin remains owned by #54/#55; mutable `producer_id` is not trusted provenance. Subject binding, record/job identity, runtime-boundary consistency, and job-identity derivation remain separate contracts.

## RED design, review findings, and prerequisite adoption

The initial #130 witness at `47a60a8f9f0ebcb5c6c49b3f0b61ddd37df9de94` recursively searched every descendant below `properties.evidence` for `contains` with adjacent `minContains` and `maxContains`. Review `5224839312` found a false-positive path: matching keywords could live in an unused `$defs` entry or an optional `anyOf`/`oneOf` branch without constraining every evidence array instance.

Test-only repair `19e6b171a6e6b57d9f5ab048f903297fdc005a1a` therefore recognizes cardinality only when exact-count subschemas are directly composed under `properties.evidence.allOf`. This binds the structural witness to an unconditional conjunction rather than to keyword presence somewhere in the schema document. Production Rust and the public schema remain unchanged until the witness executes a causal RED.

Review `5260036723` found a narrower false-GREEN in that direct-`allOf` witness. A branch could expose `/contains/properties/evidence_kind/const` plus `minContains: 1` and `maxContains: 1` while also hiding an unrelated predicate such as `producer_id: { const: "__never__" }`. The structural map would report `(1, 1)` even though ordinary foundation records would not satisfy that `contains`, allowing an unusable schema repair to pass the owner test. Test-only `2b9e80e9d07096e07adf0a367e9c257e55804aa6` hardens the witness before any schema GREEN: each foundation selector must explicitly require only `evidence_kind`, constrain only that property, and bind it only with the expected `const`. A synthetic hostile control proves an extra `producer_id` predicate is rejected by the harness. Production Rust and `schemas/evidence-bundle.schema.json` remain unchanged.

Review `5260141554` found one remaining outer-subschema false-GREEN. A counted direct `allOf` member could retain the narrow `contains` selector and exact `minContains`/`maxContains`, but add a sibling assertion or applicator such as `maxItems: 0`. The structural map would still record `(1, 1)` although ordinary non-empty EvidenceBundle arrays could never satisfy that branch. Test-only `0a8636a33aee3e5a3f043aa049558cc6a1883c83` therefore requires every counted foundation occurrence object itself to contain exactly `contains`, `minContains`, and `maxContains`, then reuses the narrow inner selector check. A second synthetic hostile repair proves an outer `maxItems: 0` predicate is rejected. Production Rust and the public schema remain unchanged.

The prerequisite #65 branch first advanced from historical `1813b49fd908f111ba6f6a43066bacd5f84e836b` to `598a65562968a48ce1fa61814fee1cbc96f7fff8` after review found stale contract fixtures under the new Rust cardinality invariant. #130 adopted that prerequisite through ordinary non-force two-parent commit `7240a80f392e5660d728f8ba0c2df829adba4d87`, preserving this lane's test and TRACEABILITY blobs while inheriting #65's fixture repairs.

A later owner-boundary review, `5228331398`, found that #65 still carried an older #18-era delta to the global `docs/product-technical-gap-baseline.md`, even though current gap-ledger authority is maintained by #121. #65 repaired that single-writer violation at `8252dbe0d5e55efbee9894dcdd8aa415af962124` by restoring the exact #18 base blob `ea0310394a3d842246bae380977a30c72c18cbf9`. #130 adopted that owner-clean tree through non-force two-parent `169d3fa6ab97d71b97736464116de29f414679da` and recorded it in docs child `28787f2ac534e10d3455903b7550ca6017cefc8b`.

#65 then made its own doctoring code-current at `cb1e467c1cacbb750fee9de1f2790f10e7e79770`, explicitly recording the single-writer gap-ledger repair without changing cardinality production/tests/fixtures. #130 adopted that exact prerequisite through non-force two-parent `ab23ad405603cccdede126ff281e70873a4ffb5f`, replacing only the inherited #65 TRACEABILITY blob while preserving this lane's test, this file, and the restored global baseline.

The prerequisite moved again to exact `3bb72c6cdbbc07823ba1c5e59f6a7b3528b29591`. Its only new semantic-neutral delta after `cb1e467c...` is the formatter repair in `tests/artifact_analysis_foundation_evidence_cardinality_red.rs`, plus the current #18 owner-repair/global-baseline blobs inherited on that prerequisite. #130 non-force adopted that live parent through two-parent merge `41bfc6d9086108b4e7c086e6dc44e0db8ef428e3`. The merge tree takes the three parent-moved paths from exact #65 and preserves the two #130-owned paths unchanged. Relative to current #65, the PR delta is therefore again only `tests/artifact_analysis_foundation_schema_cardinality_red.rs` and this TRACEABILITY file; production Rust and the public schema remain unchanged.

These adoptions matter to causality: #130's schema RED must execute on the repaired prerequisite contract and owner-clean tree rather than on a stale #65 snapshot whose unrelated fixture, documentation, or global-ledger deltas could mask or conflict with this lane. Historical queued #130 runs do not transfer to a moved head.

## Selected minimum repair after causal RED

After the current witness executes and fails because `properties.evidence.allOf` is absent, the minimum candidate GREEN is three direct subschemas. Each counted `allOf` member is itself narrow and unconditional: it contains only a narrow `contains` selector plus adjacent `minContains: 1` and `maxContains: 1`. The `contains` selector explicitly requires only `evidence_kind`, constrains only that property to the one expected foundation kind, and adds no unrelated predicate. The existing `items` schema remains the authority for evidence-record shape.

The repair must not singleton-constrain `static_capability`, `runtime_behavior`, `network_attempt`, or `tool_failure`. It must not use `minItems: 3`, fixed positions, `uniqueItems`, `producer_id`, schema-version rewriting, undocumented keywords, unrelated predicates inside a foundation `contains`, outer sibling assertions/applicators on a counted occurrence branch, or constraints hidden in dead/optional branches.

A different Draft 2020-12 construction is acceptable only if its test proves that the occurrence constraint is unconditionally effective for the evidence array rather than merely present in the schema document.

## Standards basis

Draft 2020-12 remains the current JSON Schema specification version. The Validation vocabulary defines `minContains` and `maxContains` as occurrence bounds on matches produced by an adjacent `contains`; if `contains` is absent in the same schema object, those bounds have no effect. The Core/Applicator semantics define `allOf` as successful only when the instance validates against every listed subschema. That makes direct `allOf` composition suitable for three independent exactly-one foundation constraints.

APA 7th:

- JSON Schema. (2022). *JSON Schema Draft 2020-12*. https://json-schema.org/draft/2020-12
- JSON Schema. (2022). *JSON Schema Validation: A vocabulary for structural validation of JSON (Draft 2020-12).* https://json-schema.org/draft/2020-12/json-schema-validation
- JSON Schema. (2022). *JSON Schema: A media type for describing JSON documents (Draft 2020-12).* https://json-schema.org/draft/2020-12/json-schema-core

## Evidence and release gate

The current #130 head must first execute the hardened schema-cardinality witness for the intended missing-effective-constraint cause on top of exact prerequisite #65 `3bb72c6cdbbc07823ba1c5e59f6a7b3528b29591`. Only then may the public schema change. The resulting unchanged exact head must reacquire repository validation, formatting, full locked workspace/all-target tests, Clippy and rustdoc with warnings denied, complete applicable owned-production coverage, review/security gates, prerequisite integration, protected-head verification, and immutable publication evidence.

Neither #65 nor #129 is complete while Rust validation and the published v1 schema disagree or while either exact owner head lacks its own verification. No predecessor GREEN transfers across head movement.
