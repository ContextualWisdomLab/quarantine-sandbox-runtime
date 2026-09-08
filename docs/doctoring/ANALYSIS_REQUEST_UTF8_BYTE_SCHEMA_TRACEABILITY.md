# Analysis-request UTF-8 byte-schema traceability

## Scope and authority

Issue #101 owns the artifact-analysis public-contract mismatch between `schemas/analysis-request.schema.json` and `AnalysisRequest::validate()`. The causal parent is artifact-analysis PR #18 exact `c0647152ec052d82969b2ae078891e25e6d4d69a`.

This is an `artifact_analysis` contract-publishing concern. It does not change `sandbox_execution`, application-service request validation, consumer verdict authority, or analyzer execution isolation.

## Problem

The original published request schema identified the stock JSON Schema Draft 2020-12 dialect. `request_id` declared both `maxLength: 128` and `x-cwl-maxUtf8Bytes: 128`. Draft 2020-12 defines `maxLength` in JSON-string characters, while the Rust domain validator uses `str::len()` and therefore enforces UTF-8 bytes.

A request ID containing 65 copies of `é` is the concrete witness: it has 65 characters and 130 UTF-8 bytes. The stock assertions accept that value, while `AnalysisRequest::validate()` returns `ContractError::FieldTooLong { field_name: "request_id", maximum_bytes: 128 }`.

The extension keyword cannot be assumed to repair this mismatch. JSON Schema Core states that unrecognized individual keywords are annotations. A dialect can instead declare a required vocabulary; an implementation that does not recognize a vocabulary whose `$vocabulary` value is `true` must refuse to process schemas that declare that meta-schema. Unsupported byte validation must therefore fail closed rather than silently treating the byte limit as documentation.

## Executed RED

Test-bearing commit `ce3a489149742dbb51eebd63122e8b6b00359563` introduced `tests/artifact_analysis_schema_utf8_byte_contract_red.rs` without changing production Rust or JSON Schema. Formatter-only `36953fe0e8e2fdc2457584f83efab82c446e6a61` removed the formatting prerequisite.

Native CI `34244836156` on exact `36953fe0...` passed exact checkout, dependency lock, repository policy, coverage-parser tests, and formatting, then failed at the focused witness assertion. The test proved that the same 65-character/130-byte request ID was admitted by the stock schema assertions and rejected by the Rust domain validator. That is the causal contract RED.

## Selected repair

The selected design is a versioned Draft 2020-12 dialect with a required CWL artifact-analysis contract vocabulary.

`schemas/cwl-artifact-analysis-contract-dialect-1.0.0.schema.json` declares the normal Draft 2020-12 vocabularies plus `https://contextualwisdomlab.org/vocab/quarantine-artifact-analysis-contract-1.0.0` with value `true`. The vocabulary owns `x-cwl-maxUtf8Bytes`, `x-cwl-maxSerializedUtf8Bytes`, and the bounded RFC 3339 profile annotation used by the request contract. `schemas/analysis-request.schema.json` now declares this versioned CWL dialect rather than pretending that stock Draft 2020-12 alone enforces UTF-8 byte limits.

The runtime's existing Rust contract validator remains the canonical executable implementation of the CWL byte semantics in this repository. A schema processor that does not recognize the required CWL vocabulary must refuse to process the schema rather than silently accepting instances solely because their character counts satisfy `maxLength`.

The regression now verifies both sides of that boundary: the Unicode witness still demonstrates the difference between character and byte counts, and the published dialect must require the CWL vocabulary. Relaxing the runtime to character counts, truncating/coercing input, or treating an unknown extension keyword as a stock assertion remain rejected alternatives.

## Compatibility and publication

The request wire shape and `schema_version: 1.0.0` are unchanged. The change tightens schema-processing semantics by making a previously advisory extension vocabulary required. Consumers using only a stock validator must fail closed until they add support for the CWL vocabulary or use the released runtime validator. They must not continue as though validation succeeded.

The dialect and request schema are repository artifacts until an immutable release publishes them. A mutable branch is review evidence only and must not become a consumer dependency. The release must preserve the dialect `$id`, request-schema `$id`, vocabulary URI, package/version identity, SBOM/provenance, reproducibility evidence, and rollback target.

## Evidence and release conditions

GREEN requires the unchanged exact candidate head to pass repository validation, formatting, tests, Clippy, rustdoc, complete owned-production coverage, qualifying review/security gates, and dependency-safe non-force adoption by #18. Positive effective-LSM remains an independent runtime gate and cannot be substituted by hosted negative confinement evidence.

## References (APA 7th)

Wright, A., Andrews, H., Hutton, B., & Dennis, G. (2022). *JSON Schema: A media type for describing JSON documents (Draft 2020-12).* https://json-schema.org/draft/2020-12/draft-bhutton-json-schema-01

Wright, A., Andrews, H., Hutton, B., & Dennis, G. (2022). *JSON Schema validation: A vocabulary for structural validation of JSON (Draft 2020-12).* https://json-schema.org/draft/2020-12/json-schema-validation
