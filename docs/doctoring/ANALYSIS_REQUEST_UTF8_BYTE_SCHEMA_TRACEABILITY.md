# Analysis-request UTF-8 byte-schema traceability

## Scope and authority

Issue #101 owns the artifact-analysis public-contract mismatch between `schemas/analysis-request.schema.json` and `AnalysisRequest::validate()`. The original causal parent was artifact-analysis PR #18 exact `c0647152ec052d82969b2ae078891e25e6d4d69a`. Draft #102 later adopted current #18 exact `703b4b1a047321bb08d1d6cb1de78d01cb350696` by ordinary/non-force ancestry without copying or dropping the #101 contract delta.

This is an `artifact_analysis` contract-publishing concern. It does not change `sandbox_execution`, application-service request validation, consumer verdict authority, or analyzer execution isolation.

## Problem

The original published request schema identified the stock JSON Schema Draft 2020-12 dialect. `request_id` declared both `maxLength: 128` and `x-cwl-maxUtf8Bytes: 128`. Draft 2020-12 defines `maxLength` in JSON-string characters, while the Rust domain validator uses `str::len()` and therefore enforces UTF-8 bytes.

A request ID containing 65 copies of `é` is the concrete witness: it has 65 characters and 130 UTF-8 bytes. The stock assertions accept that value, while `AnalysisRequest::validate()` returns `ContractError::FieldTooLong { field_name: "request_id", maximum_bytes: 128 }`.

The extension keyword cannot be assumed to repair this mismatch. JSON Schema Core states that unrecognized individual keywords are annotations. A dialect can instead declare a required vocabulary; an implementation that does not recognize a vocabulary whose `$vocabulary` value is `true` must refuse to process schemas that declare that meta-schema. Unsupported byte validation must therefore fail closed rather than silently treating the byte limit as documentation.

## Executed causal RED

Test-bearing commit `ce3a489149742dbb51eebd63122e8b6b00359563` introduced `tests/artifact_analysis_schema_utf8_byte_contract_red.rs` without changing production Rust or JSON Schema. Formatter-only `36953fe0e8e2fdc2457584f83efab82c446e6a61` removed the formatting prerequisite.

Native CI `34244836156` on exact `36953fe0...` passed exact checkout, dependency lock, repository policy, coverage-parser tests, and formatting, then failed at the focused witness assertion. The test proved that the same 65-character/130-byte request ID was admitted by the stock schema assertions and rejected by the Rust domain validator. That is the causal contract RED.

## Selected repair candidate

The selected design is a versioned Draft 2020-12 dialect with a required CWL artifact-analysis contract vocabulary.

`schemas/cwl-artifact-analysis-contract-dialect-1.0.0.schema.json` declares the normal Draft 2020-12 vocabularies plus `https://contextualwisdomlab.org/vocab/quarantine-artifact-analysis-contract-1.0.0` with value `true`. The vocabulary owns `x-cwl-maxUtf8Bytes`, `x-cwl-maxSerializedUtf8Bytes`, and the bounded RFC 3339 profile extension used by the request contract. `schemas/analysis-request.schema.json` declares this versioned CWL dialect rather than pretending that stock Draft 2020-12 alone enforces UTF-8 byte limits.

The runtime's existing Rust contract validator remains the canonical executable implementation of the CWL byte semantics in this repository. A schema processor that does not recognize the required CWL vocabulary must refuse to process the schema rather than silently accepting instances solely because their character counts satisfy `maxLength`.

The original regression verifies both sides of that boundary: the Unicode witness still demonstrates the difference between character and byte counts, and the published dialect must require the CWL vocabulary. Relaxing the runtime to character counts, truncating/coercing input, or treating an unknown extension keyword as a stock assertion remain rejected alternatives.

## Review finding: required vocabulary semantics are not yet publication-complete

Formal review `5239036676` on #102 exact `79eea85c925fe1f3e05c409381a20417e20e22b0` found a second contract-publication gap. Draft 2020-12 defines a vocabulary as a set of keywords, their syntax, and their semantics. A meta-schema describes valid keyword syntax; it does not by itself define how a recognizing implementation evaluates those keywords. An implementation that claims support for a required vocabulary must process it consistently with the vocabulary's semantic definitions.

The current dialect constrains the syntactic forms of `x-cwl-maxUtf8Bytes`, `x-cwl-maxSerializedUtf8Bytes`, and `x-cwl-rfc3339Profile`, but there is no versioned normative vocabulary specification plus conformance vectors from which an independent processor can reproduce the byte assertions. That omission is especially material for `x-cwl-maxSerializedUtf8Bytes`: the Rust boundary serializes `BoundedSourceContext` after deserialization, and `Option::None` fields are serialized as JSON `null`. A schema processor operating on the parsed request instance needs the same normalization rule or it can compute a different byte count.

A concrete boundary vector uses a 64-byte `source_channel_code`, a 227-quote `original_file_name`, a 255-byte media type (`127-byte type`, `/`, `127-byte subtype`), a 128-byte host artifact reference, and an omitted `submitted_at`. The compact parsed context representation is 1,005 UTF-8 bytes. The runtime's current domain serialization materializes the omitted nullable `submitted_at` as `"submitted_at":null`, producing 1,025 bytes and therefore rejecting the context against the 1,024-byte aggregate limit. The public vocabulary must state which normalization is authoritative and provide a conformance vector that prevents independent implementations from silently choosing the other acceptance set.

Test-only `40efbce506090d515ec9415b036e0962f8b228c5` introduced `tests/artifact_analysis_contract_vocabulary_publication_red.rs`; `57df5081788f9f719f07ca35ebc4d58fd8bea5a6` first pinned the 1,005/1,025-byte normalization expectation; formatter-only `5e26f01196e5c1d9a32d5d90fb0516718450de1c` applied the expected multiline layout. Review of that checked-in witness then found a false-GREEN risk: a fixture could merely self-report the expected byte totals without containing an instance that actually produces them. Test-only `7ad42e7b9db44af02dc50519374965966fc49260` removes that weakness by requiring the concrete context fields and deriving both byte counts with `serde_json::to_vec`, after materializing every nullable `BoundedSourceContext` property as `null` for the canonical CWL form. The witness also pins the 65-character/130-byte UTF-8 overflow case. No production Rust, schema, or vocabulary implementation changed in these commits.

Formal review `5240880301` found a further publication-witness false-GREEN on exact `509ac4de99db9c421920c220cc3a38686ab93e61`. The test accepted a Markdown file that merely mentioned the vocabulary URI and keyword names, while its vectors covered only rejecting overflow cases. A placeholder specification plus an implementation that always rejects could therefore satisfy the witness without defining the intended acceptance set.

Test-only `54fa1813758f4ba75cc46812e9a252177bdbd233` hardens the publication RED in two ways. First, the future normative specification must explicitly define UTF-8-octet counting, inclusive `count <= keyword_value` admission, compact UTF-8 JSON serialization, and materialization of every missing nullable `BoundedSourceContext` property as JSON `null` before serialized-byte measurement. Second, conformance publication must contain both positive boundary and negative overflow vectors for each byte keyword, with byte counts derived from the fixture instance rather than self-reported totals. Immediate self-review found an arithmetic typo in the positive serialized boundary's raw size; test-only `eaaa96d9c5252557b316c5bada71928946ecfa73` corrects the expected raw compact size to 1,004 bytes so materializing `,"submitted_at":null` yields the exact 1,024-byte accepted boundary. The existing overflow remains 1,005 raw / 1,025 canonical and invalid.

These changes are RED hardening only. Production Rust, `analysis-request.schema.json`, the dialect meta-schema, and repository-policy behavior are unchanged. The missing specification/vector publication remains the intended first failure, and semantic publication artifacts must not be added before the current hardened exact head executes that cause.

Issue #134 / Draft #135 separately owns the Gregorian semantics of `x-cwl-rfc3339Profile`. #102 may publish the extension point and byte-keyword semantics, but it must not preempt #135's hardened Gregorian RED or copy mutable #135 source.

## Compatibility and publication

The request wire shape and `schema_version: 1.0.0` are unchanged. No immutable public release currently exists, so the Draft dialect remains review evidence rather than consumer authority. Before first immutable publication, however, the vocabulary URI, keyword semantics, conformance vectors, dialect `$id`, request-schema `$id`, and runtime/package identity must agree as one versioned contract.

Consumers using only a stock validator must fail closed until they add support for the released CWL vocabulary or use the released runtime validator. They must not continue as though validation succeeded. Consumers that do implement the vocabulary must have enough normative information and conformance evidence to reproduce the same acceptance set as the runtime.

## Evidence and release conditions

GREEN requires the unchanged exact candidate head to pass repository validation, formatting, tests, Clippy, rustdoc, complete owned-production coverage, qualifying review/security gates, and dependency-safe non-force ancestry over current #18. Positive effective-LSM remains an independent runtime gate and cannot be substituted by hosted negative confinement evidence.

Immutable publication additionally requires the dialect, request schema, normative vocabulary specification, conformance vectors, package/tag, SBOM/provenance, reproducibility evidence, and rollback target. Mutable branch artifacts are never consumer dependencies.

## References (APA 7th)

Wright, A., Andrews, H., Hutton, B., & Dennis, G. (2022). *JSON Schema: A media type for describing JSON documents (Draft 2020-12).* https://json-schema.org/draft/2020-12/json-schema-core

Wright, A., Andrews, H., Hutton, B., & Dennis, G. (2022). *JSON Schema validation: A vocabulary for structural validation of JSON (Draft 2020-12).* https://json-schema.org/draft/2020-12/json-schema-validation
