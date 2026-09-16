# Analysis Request Source-Context Schema Traceability

## Decision status

Implemented as a focused schema-parity repair; current branch authority is Draft #96. Issue #95 owns the published-schema admission mismatch above artifact-analysis parent Draft #18 exact `c0647152ec052d82969b2ae078891e25e6d4d69a`.

## Problem

`BoundedSourceContext::validate()` rejects a present context when every supported field is absent (`None`). The published `analysis-request-1.0.0` schema originally used `minProperties: 1` for `bounded_source_context`, while each known property could itself be `null`. Consequently, an object such as `{"source_channel_code": null}` satisfied the property-count condition even though it represented the same empty semantic state that the Rust contract rejects.

Property count and semantic presence are different invariants. The wire contract must not accept an all-null object that the domain contract rejects, and the runtime must not be weakened merely to match an under-specified schema.

## DDD and contract boundary

- `artifact_analysis` owns `AnalysisRequest` and `BoundedSourceContext` admission semantics.
- The JSON Schema is a published interoperability contract for the same bounded context, not a looser transport DTO with a separate acceptance set.
- `bounded_source_context: null` remains valid because source context is optional.
- When the context is present as an object, at least one supported property must carry a valid string value.
- Individual fields remain nullable because callers may supply only the context dimensions they possess.
- Unknown members remain forbidden and schema version `1.0.0` remains unchanged in this focused repair.
- Repository-wide `docs/product-technical-gap-baseline.md` is a single-writer ledger owned by canonical gap-baseline lane #121, not by this focused schema leaf.

## Causal RED

Test-bearing `d383b9b1d69864341b97734d0d1b23c1db984b82` first preserved the positive wire contract: the context itself remains `[object, null]` and each of the five existing fields remains `[string, null]`. The hostile witness then required a Draft 2020-12 `anyOf` admission rule with one canonical branch per supported field; each branch requires that field and constrains its value to `type: string`.

The RED reached executable CI on the earlier test-only lineage before the schema repair. The missing semantic-presence rule was the intended defect: the published schema admitted all-null source-context objects that the Rust invariant rejected.

## Minimum causal repair

Production/schema commit `eba6e09c6f68f09eff6341f2311d0dd7d2fdd2f2` adds only the five-branch `anyOf` applicator under `bounded_source_context`. It preserves:

- top-level context type `["object", "null"]`;
- `additionalProperties: false`;
- existing field names and field-level patterns/limits;
- each field's existing `["string", "null"]` wire type;
- schema version and `$id`.

It does not make all fields required, remove top-level `null`, add a sentinel field, coerce null to absence, or relax the Rust `EmptyBoundedSourceContext` invariant.

Formatter-only `db0720d033b59c22f0c14017a88e281341d807ea` left the schema semantics unchanged.

## Focused exact-head GREEN

Native CI `34245286346` executed exact `db0720d033b59c22f0c14017a88e281341d807ea`. Verify passed exact checkout, dependency lock, repository policy, coverage-parser tests, and formatting. Both focused regressions passed: `bounded_source_context_wire_fields_remain_individually_nullable` and `published_schema_requires_one_supported_context_field_to_be_a_string`.

The broader workspace was not whole-head GREEN: verify later reached inherited `podman_command_execution_entrypoint_red::requested_command_is_encoded_as_exact_entrypoint_argv`, an issue #38 / canonical command-runtime defect. That failure does not invalidate the focused schema-parity GREEN and must not be repaired by weakening this artifact-analysis contract.

## Single-writer gap-ledger repair

Review `5229235266` found that #96 still carried a historical `docs/product-technical-gap-baseline.md` delta even though repository-wide Gap authority is maintained by #121. Leaving that leaf-owned ledger in place could regress or conflict with the current owner graph during later integration.

Ordinary fast-forward `154fa4830956cab599298a31776cd2ac72a907ed` restores the global baseline byte-for-byte to the exact #18 base blob `ea0310394a3d842246bae380977a30c72c18cbf9`. This repair changes no schema, Rust production behavior, or focused test semantics. The current documentation head records the ownership repair locally; predecessor execution does not transfer after branch movement.

## Alternatives

1. Keep `minProperties: 1` only. Rejected because a null-valued property satisfies property count without carrying source context.
2. Require every context property. Rejected because partial source context is intentionally supported and already represented by optional Rust fields.
3. Make every property non-null whenever the object is present. Rejected because that also turns partial context into an all-fields-required contract.
4. Add `anyOf` branches requiring one known property with `type: string`. Selected because it represents the existing domain invariant without broadening or over-constraining the wire contract.

## Current gate

The focused repair is historical exact-head GREEN on `db0720d033...`, but the moved owner-safe head must independently reacquire repository validation, rustfmt, full locked workspace/all-target tests, Clippy, public/private rustdoc with warnings denied, complete applicable owned-production coverage, qualifying review/security/thread gates, dependency-safe parent integration, positive effective-LSM where applicable, protected integration, and immutable version/package/SBOM/provenance/reproducibility/rollback publication.

## References

Wright, A., Andrews, H., Hutton, B., & Dennis, G. (2022). *JSON Schema: A media type for describing JSON documents (Draft 2020-12)*. JSON Schema. https://json-schema.org/draft/2020-12/json-schema-core

Wright, A., Andrews, H., & Hutton, B. (2022). *JSON Schema validation: A vocabulary for structural validation of JSON (Draft 2020-12)*. JSON Schema. https://json-schema.org/draft/2020-12/json-schema-validation
