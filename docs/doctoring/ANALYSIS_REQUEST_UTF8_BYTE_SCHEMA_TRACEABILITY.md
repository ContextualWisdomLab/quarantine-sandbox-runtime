# Analysis-request UTF-8 byte-schema traceability

## Scope and authority

Issue #101 owns the artifact-analysis public-contract mismatch between `schemas/analysis-request.schema.json` and `AnalysisRequest::validate()`. The causal parent is artifact-analysis PR #18 exact `c0647152ec052d82969b2ae078891e25e6d4d69a`.

This is an `artifact_analysis` contract-publishing concern. It does not change `sandbox_execution`, application-service request validation, consumer verdict authority, or analyzer execution isolation.

## Problem

The published request schema identifies the stock JSON Schema Draft 2020-12 dialect. `request_id` declares both `maxLength: 128` and `x-cwl-maxUtf8Bytes: 128`. Draft 2020-12 defines `maxLength` in JSON-string characters, while the Rust domain validator uses `str::len()` and therefore enforces UTF-8 bytes.

A request ID containing 65 copies of `é` is the minimum clear witness used by the RED: it has 65 characters and 130 UTF-8 bytes. The stock assertions currently present in the schema accept that value, while `AnalysisRequest::validate()` returns `ContractError::FieldTooLong { field_name: "request_id", maximum_bytes: 128 }`.

The extension keyword cannot be assumed to repair this mismatch. JSON Schema Core states that unrecognized individual keywords are collected as annotations. A dialect can instead declare required vocabularies; an implementation must understand a required vocabulary in order to successfully process the schema. Unsupported validation must therefore fail closed rather than silently treating the byte limit as documentation.

## RED

Test-bearing commit `ce3a489149742dbb51eebd63122e8b6b00359563` adds `tests/artifact_analysis_schema_utf8_byte_contract_red.rs` without changing production Rust or JSON Schema.

The test proves all of the following for one concrete wire value:

- character count is within the published standard `maxLength`;
- UTF-8 byte count exceeds the published `x-cwl-maxUtf8Bytes` budget;
- the Rust domain validator rejects the request with the existing typed byte-limit error;
- the contract must not simultaneously advertise only the stock Draft 2020-12 dialect and leave the witness accepted by the stock assertions.

The same class of mismatch also exists for fields such as `original_file_name`, but the first causal RED is intentionally limited to `request_id`. Application-service byte-bound findings remain a separate bounded-context owner path.

## Minimum repair decision gate

No production repair is selected before the RED executes for the intended cause. A compatible GREEN must preserve the existing byte budget and make validation semantics executable and fail closed.

Acceptable design families are:

1. publish a versioned JSON Schema dialect/vocabulary in which the CWL UTF-8 byte keywords are required, with an implementation path that either understands the vocabulary or refuses to process the schema; or
2. publish an explicit versioned reference validator as part of the released runtime contract and make the consumer contract state that stock-schema validation alone is insufficient for the `x-cwl-*` constraints.

The selected design must not silently truncate/coerce input, relax the runtime from bytes to characters, or present an unknown annotation keyword as a standard assertion. If validator/dialect semantics become a stable reusable responsibility, record that architectural decision before implementation and keep the Shared Kernel minimal.

## Evidence and release conditions

The RED must execute on its exact head. A later GREEN requires exact-head formatting, tests, Clippy, rustdoc, repository policy, 100% owned-production coverage, review/security gates, and dependency-safe non-force adoption by #18. A mutable PR head remains review evidence only, not a consumer dependency or release.

## References (APA 7th)

Wright, A., Andrews, H., Hutton, B., & Dennis, G. (2022). *JSON Schema: A media type for describing JSON documents (Draft 2020-12).* https://json-schema.org/draft/2020-12/draft-bhutton-json-schema-01

Wright, A., Andrews, H., Hutton, B., & Dennis, G. (2022). *JSON Schema validation: A vocabulary for structural validation of JSON (Draft 2020-12).* https://json-schema.org/draft/2020-12/json-schema-validation
