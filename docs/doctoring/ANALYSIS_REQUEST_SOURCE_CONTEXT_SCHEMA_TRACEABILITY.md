# Analysis Request Source-Context Schema Traceability

## Decision status

Proposed on 2026-09-08. Issue #95 owns the focused published-schema admission mismatch above artifact-analysis parent Draft #18 exact `c0647152ec052d82969b2ae078891e25e6d4d69a`. The first test-bearing commit is `d383b9b1d69864341b97734d0d1b23c1db984b82`. No production Rust or JSON Schema behavior changes before the exact RED executes for the intended cause.

## Problem

`BoundedSourceContext::validate()` rejects a present context when every supported field is absent (`None`). The published `analysis-request-1.0.0` schema instead uses `minProperties: 1` for `bounded_source_context`, while each known property may itself be `null`. Consequently, an object such as `{"source_channel_code": null}` satisfies the current property-count condition even though it represents the same empty semantic state that the Rust contract rejects.

Property count and semantic presence are different invariants. The wire contract must not accept an all-null object that the domain contract rejects, and the runtime must not be weakened merely to match an under-specified schema.

## DDD and contract boundary

- `artifact_analysis` owns `AnalysisRequest` and `BoundedSourceContext` admission semantics.
- The JSON Schema is a published interoperability contract for the same bounded context, not a looser transport DTO with a separate acceptance set.
- `bounded_source_context: null` remains valid because source context is optional.
- When the context is present as an object, at least one supported property must carry a valid string value.
- Individual fields remain nullable because callers may supply only the context dimensions they possess.
- Unknown members remain forbidden and schema version `1.0.0` remains unchanged in this focused repair.

## RED

`tests/analysis_request_all_null_context_schema_red.rs` first preserves the positive contract: the context itself remains `[object, null]` and each of the five existing fields remains `[string, null]`.

The focused RED then requires a Draft 2020-12 `anyOf` admission rule with one canonical branch per supported field. Each branch must require exactly that field and require its value to be a string. This makes a required-but-null field insufficient while allowing any one valid context dimension to establish semantic presence. The current schema has no such `anyOf`, so the test must fail before a production/schema repair is added.

## Minimum causal GREEN

After the exact RED executes for the missing semantic-presence rule, add only the five-branch applicator to `bounded_source_context`. Preserve:

- top-level context type `["object", "null"]`;
- `additionalProperties: false`;
- existing field names and field-level patterns/limits;
- each field's existing `["string", "null"]` wire type;
- schema version and `$id`.

Do not make all fields required, remove top-level `null`, add a new sentinel field, silently coerce null to absence, or relax the Rust `EmptyBoundedSourceContext` invariant.

## Alternatives

1. Keep `minProperties: 1` only. Rejected because a null-valued property satisfies property count without carrying source context.
2. Require every context property. Rejected because partial source context is intentionally supported and already represented by optional Rust fields.
3. Make every property non-null whenever the object is present. Rejected because that also turns partial context into an all-fields-required contract.
4. Add `anyOf` branches requiring one known property with `type: string`. Selected because it represents the existing domain invariant without broadening or over-constraining the wire contract.

## Acceptance

The slice remains Draft until the exact test-only head executes the intended RED, the minimum schema repair executes GREEN on an unchanged exact head, repository validation and full Rust fmt/test/clippy/rustdoc gates pass, owned production coverage remains 100%, review/security/thread gates are satisfied, and the delta is dependency-safely integrated through the artifact-analysis stack. Predecessor checks do not transfer after head movement.

## References

Wright, A., Andrews, H., Hutton, B., & Dennis, G. (2022). *JSON Schema: A media type for describing JSON documents (Draft 2020-12)*. JSON Schema. https://json-schema.org/draft/2020-12/json-schema-core

Wright, A., Andrews, H., & Hutton, B. (2022). *JSON Schema validation: A vocabulary for structural validation of JSON (Draft 2020-12)*. JSON Schema. https://json-schema.org/draft/2020-12/json-schema-validation
