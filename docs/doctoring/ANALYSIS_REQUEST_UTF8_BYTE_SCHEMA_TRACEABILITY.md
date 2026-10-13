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

Formal review `5241478053` found that the strengthened wording still did not define one interoperable byte representation for `x-cwl-maxSerializedUtf8Bytes`. RFC 8259 permits multiple JSON spellings for the same parsed string; for example, a reverse solidus may be encoded as either the two-character escape `\\` or the six-character Unicode escape `\u005C`. Therefore “compact JSON encoded as UTF-8” can produce different octet counts for semantically identical instances, and two otherwise conforming vocabulary implementations can disagree at a byte boundary.

RFC 8785 JSON Canonicalization Scheme (JCS) supplies a deterministic JSON representation by defining strict primitive/string serialization, deterministic property sorting, and UTF-8 output. Test-only `97c9eaed45824fab495db969235450dbd8e30198` hardens the RED accordingly. The future vocabulary specification must apply RFC 8785 JCS after the existing CWL missing-nullable-field materialization and count the UTF-8 octets of that canonical representation. The conformance publication must also include `max_serialized_utf8_bytes_jcs_escaping_boundary`: a `BoundedSourceContext` whose `original_file_name` contains non-ASCII `é`, a reverse solidus, and a quotation mark while the other nullable fields are omitted. The witness pins the exact JCS representation, verifies that it parses back to the CWL-materialized object, and derives its 136-byte UTF-8 boundary from the canonical representation rather than trusting a fixture-reported byte total. This prevents generic JSON escaping choices from false-GREENing the serialized-byte contract.

These changes are RED hardening only. Production Rust, `analysis-request.schema.json`, the dialect meta-schema, and repository-policy behavior are unchanged. The missing specification/vector publication remains the intended first failure, and semantic publication artifacts must not be added before the current hardened exact head executes that cause.

Issue #134 / Draft #135 separately owns the Gregorian semantics of `x-cwl-rfc3339Profile`. #102 may publish the extension point and byte-keyword semantics, but it must not preempt #135's hardened Gregorian RED or copy mutable #135 source.

## Compatibility and publication

The request wire shape and `schema_version: 1.0.0` are unchanged. No immutable public release currently exists, so the Draft dialect remains review evidence rather than consumer authority. Before first immutable publication, however, the vocabulary URI, keyword semantics, conformance vectors, dialect `$id`, request-schema `$id`, and runtime/package identity must agree as one versioned contract.

Consumers using only a stock validator must fail closed until they add support for the released CWL vocabulary or use the released runtime validator. They must not continue as though validation succeeded. Consumers that do implement the vocabulary must have enough normative information and conformance evidence to reproduce the same acceptance set as the runtime.

## Current serialized-boundary RCA — 2026-10-05

The current canonical #102 predecessor `a7fc09fc85018a58be581cc5a564f4b163870b35` does not contain the byte shapes claimed in its historical repair prose. Direct Rust 1.97.1 focused execution measured the positive source instance at 1,046 raw bytes, not 1,004. Independently decoded JSON shows 247 quotation marks plus a trailing `a` (248 filename bytes). Each quotation mark contributes two bytes after JSON escaping, so removing 21 excess quotation marks restores 226 quotation marks plus `a`: 1,004 raw / 1,024 after materializing `submitted_at: null`.

The negative sibling is also malformed on that exact: 281 quotation marks, 1,113 raw / 1,133 normalized bytes, and a filename exceeding the 255-byte per-field ceiling. Repairing only the positive fixture caused the existing Rust witness to advance and fail at the negative filename-length assertion (actual 281, expected 227). Removing the 54 excess quotation marks restores the intended independent aggregate-overflow witness: 227 filename bytes, 1,005 raw / 1,025 normalized. The complete vocabulary-publication test then passed.

This repair changes only the two decoded `original_file_name` strings. Keyword identities, ceilings, expected validity flags, other fields, schema, dialect, normative vocabulary prose, Rust production code, and all Gregorian vectors remain unchanged. These are keyword-level serialized-byte conformance instances, not complete valid requests: the inherited media-type string lacks the required type/subtype separator. They must not be represented as end-to-end runtime admission evidence.

An independent read-only reviewer reproduced both byte calculations, checked the two-path decoded delta and individual field byte bounds, and reviewed the corrected fixture SHA-256 `63efc912d752133a88a7314fc328f9d9e2eadb4723fdf82492d6c0275fa033df`. Local focused publication, repository policy, rustfmt, and three coverage-parser unit tests passed. The original direct script invocation could not import the package; `python3 -m unittest scripts.test_check_coverage` is the verified entrypoint.

The inherited Gregorian witness passes locally on #102, but its six-edge publication witness still fails for the missing 1900 non-400-century vector. That narrow command bypasses Linux/full-suite prerequisites. The `application_service_ownership` test is Linux-only and executes zero tests on macOS, which is NOT_RUN, not GREEN. Subsequent original-log inspection of CI `35991876348` found that verify observed one invocation-class failure, while both coverage jobs passed all seven ownership tests on the same exact predecessor before reaching the vocabulary fixture failure. This is not evidence of a deterministic stale expectation: an `exit 20` fixture must still produce `BackendCommandFailed`, and transient Spawn/Wait/Capture failure requires its own diagnostic evidence. Do not change the expected error class to mask the intermittent failure.

On repaired exact `7399aac4b4b13cd994ae90810910689676eb0504`, the macOS full suite passed the byte-publication witness and reached the intended missing-1900-vector RED; Linux-only suites remained NOT_RUN. Rustdoc with warnings denied and five focused tests across four contract targets passed. Clippy exposed an inherited `needless_as_bytes` assertion at publication-test line 148. The first invocation resolved Homebrew Clippy 0.1.98 despite `rustup run 1.97.1`; explicitly prepending the 1.97.1 toolchain `bin` directory reproduced the same lint with Clippy 0.1.97. Replacing `canonical_json.as_bytes().len()` with `canonical_json.len()` preserves the UTF-8 byte count of the `&str`; pinned full-workspace/all-target Clippy, the publication test, and rustfmt then passed. Independent peer review verified that one-line delta at test-file SHA-256 `e8120e747c0aa6ae0bf8d6e88545f3a86a1cd544067845b89053ca8eabe7b154`.

New exact CI `37217000329` did not execute hosted source tests: verify, coverage, branch coverage, and negative isolation annotations state that the account is locked due to a billing issue. Positive LSM remains queued. This is an account-admission failure, not fixture or Gregorian semantic evidence. Billing settings require payer/operator authority; do not blindly rerun, bypass protection, weaken tests, or infer GREEN. Local success is not complete coverage, positive effective-LSM evidence, protected integration, or immutable publication. A fresh unchanged prerequisite-safe native exact must reach the artifact publication frontier before adding Gregorian semantic GREEN.

## Offline candidate package binding — 2026-10-05

A successful Cargo packaging operation is not an immutable publication or proof that its claimed Git revision contains the shipped contract bytes. The standalone `scripts/audit_artifact_contract_package.py` command checks six fixed contract artifacts against actual blobs at a caller-selected full Git revision: original Cargo manifest, request schema, dialect, normative vocabulary document, conformance vectors, and Rust domain contract. It reads archive members without extracting or executing them. It rejects non-regular/link/sparse entries, duplicate or noncanonical paths, missing or altered contract bytes, duplicate/invalid/dirty revision metadata, and configured compressed/expanded/member-count limits. The CLI uses exit 0 for matching candidate bytes and exit 2 with a fixed non-secret JSON error for audit failure. Every result retains `release_allowed: false`.

Example, from any working directory:

```sh
python3 /path/to/repository/scripts/audit_artifact_contract_package.py \
  --archive /path/to/candidate.crate \
  --repository /path/to/repository \
  --revision FULL_40_CHARACTER_COMMIT_SHA \
  --prefix quarantine-sandbox-runtime-0.1.0
```

Run synthetic regression tests from the repository root with `PYTHONDONTWRITEBYTECODE=1 python3 -m unittest scripts.test_audit_artifact_contract_package`. The actual candidate experiment on `c914f05fa1c76d23f086aef84548defc0cf62675` produced two identical source archives from one clean checkout: 155 regular members, 234,127 compressed bytes, SHA-256 `2388393355de9bbc2193baa6612e0496c4e6059ee6ffa6a3f1ea08d07d496225`. Both archives contain the expected normative artifacts; six audited contract files match the selected Git source bytes. Cargo regenerates `Cargo.lock` and normalized the root dependency reference `getrandom 0.3.4` to `getrandom`; separate inspection confirmed all 56 package identities/checksums and resolved dependency edges were unchanged. The CLI deliberately does not claim exact source equality for Cargo-generated manifest/lock transformations.

This audit binds only the fixed contract bytes to caller-selected local source. It does not authenticate the publisher, check all archive content, validate contract semantics, verify archive signatures/SBOM/provenance, execute packaged tests, or grant consumer/release authority. Two invocations from one checkout do not satisfy the release runbook's two-clean-checkout reproducibility gate. Missing Gregorian vectors and native/security/protected/positive-isolation acceptance remain separate unresolved gates. A forged local Git source and matching package are not authentic distribution evidence.

## Evidence and release conditions

GREEN requires the unchanged exact candidate head to pass repository validation, formatting, tests, Clippy, rustdoc, complete owned-production coverage, qualifying review/security gates, and dependency-safe non-force ancestry over current #18. Positive effective-LSM remains an independent runtime gate and cannot be substituted by hosted negative confinement evidence.

Immutable publication additionally requires the dialect, request schema, normative vocabulary specification, conformance vectors, package/tag, SBOM/provenance, reproducibility evidence, and rollback target. Mutable branch artifacts are never consumer dependencies.

## References (APA 7th)

Bray, T. (2017). *The JavaScript Object Notation (JSON) Data Interchange Format (RFC 8259).* RFC Editor. https://www.rfc-editor.org/rfc/rfc8259.html

Rundgren, A., Jordan, B., & Erdtman, S. (2020). *JSON Canonicalization Scheme (JCS) (RFC 8785).* RFC Editor. https://www.rfc-editor.org/rfc/rfc8785.html

Wright, A., Andrews, H., Hutton, B., & Dennis, G. (2022). *JSON Schema: A media type for describing JSON documents (Draft 2020-12).* https://json-schema.org/draft/2020-12/json-schema-core

Wright, A., Andrews, H., Hutton, B., & Dennis, G. (2022). *JSON Schema validation: A vocabulary for structural validation of JSON (Draft 2020-12).* https://json-schema.org/draft/2020-12/json-schema-validation
