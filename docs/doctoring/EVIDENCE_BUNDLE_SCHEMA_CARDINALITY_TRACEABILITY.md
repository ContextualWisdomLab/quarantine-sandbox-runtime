# EvidenceBundle schema-cardinality traceability

## Problem and bounded-context authority

`artifact_analysis` owns the public `EvidenceBundle` contract. Runtime validator work on prerequisite #65 requires v1 bundles to contain exactly one `ArtifactIdentity`, one `FileFormat`, and one `PolicyBoundary` evidence record. The published `schemas/evidence-bundle.schema.json` still constrains only the evidence array shape and `minItems`, so a schema-only consumer can admit serialized states that the Rust aggregate rejects.

This is public contract parity, not analyzer provenance. Stable analyzer/runtime origin remains owned by #54/#55; mutable `producer_id` is not trusted provenance. Subject binding, record/job identity, runtime-boundary consistency, and job-identity derivation remain separate contracts.

## RED design and review finding

Draft #130 is based exactly on #65 exact `1813b49fd908f111ba6f6a43066bacd5f84e836b` so schema-parity work does not move the prerequisite owner head.

The initial witness at `47a60a8f9f0ebcb5c6c49b3f0b61ddd37df9de94` recursively searched every descendant below `properties.evidence` for `contains` with adjacent `minContains` and `maxContains`. Review `5224839312` found a false-positive path: matching keywords could live in an unused `$defs` entry or an optional `anyOf`/`oneOf` branch without constraining every evidence array instance.

Test-only repair `19e6b171a6e6b57d9f5ab048f903297fdc005a1a` therefore recognizes cardinality only when exact-count subschemas are directly composed under `properties.evidence.allOf`. This binds the structural witness to an unconditional conjunction rather than to keyword presence somewhere in the schema document. Production Rust and the public schema remain unchanged until that exact lineage executes a causal RED.

## Selected minimum repair after causal RED

After the current witness executes and fails because `properties.evidence.allOf` is absent, the minimum candidate GREEN is three direct subschemas. Each subschema matches one stable foundation `evidence_kind` with `contains` and sets adjacent `minContains: 1` and `maxContains: 1`. The existing `items` schema remains the authority for evidence-record shape.

The repair must not singleton-constrain `static_capability`, `runtime_behavior`, `network_attempt`, or `tool_failure`. It must not use `minItems: 3`, fixed positions, `uniqueItems`, `producer_id`, schema-version rewriting, undocumented keywords, or constraints hidden in dead/optional branches.

A different Draft 2020-12 construction is acceptable only if its test proves that the occurrence constraint is unconditionally effective for the evidence array rather than merely present in the schema document.

## Standards basis

Draft 2020-12 remains the current JSON Schema specification version. The Validation vocabulary defines `minContains` and `maxContains` as occurrence bounds on matches produced by an adjacent `contains`; if `contains` is absent in the same schema object, those bounds have no effect. The Core/Applicator semantics define `allOf` as successful only when the instance validates against every listed subschema. That makes direct `allOf` composition suitable for three independent exactly-one foundation constraints.

APA 7th:

- JSON Schema. (2022). *JSON Schema Draft 2020-12*. https://json-schema.org/draft/2020-12
- JSON Schema. (2022). *JSON Schema Validation: A vocabulary for structural validation of JSON (Draft 2020-12).* https://json-schema.org/draft/2020-12/json-schema-validation
- JSON Schema. (2022). *JSON Schema: A media type for describing JSON documents (Draft 2020-12).* https://json-schema.org/draft/2020-12/json-schema-core

## Evidence and release gate

The current #130 head must first execute the schema-cardinality witness for the intended missing-effective-constraint cause. Only then may the public schema change. The resulting unchanged exact head must reacquire repository validation, formatting, full locked workspace/all-target tests, Clippy and rustdoc with warnings denied, complete applicable owned-production coverage, review/security gates, prerequisite integration, protected-head verification, and immutable publication evidence.

Neither #65 nor #129 is complete while Rust validation and the published v1 schema disagree or while either exact owner head lacks its own verification. No predecessor GREEN transfers across head movement.
