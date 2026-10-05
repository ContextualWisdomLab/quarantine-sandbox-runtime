# Application-service UTF-8 byte schema traceability

## Authority

Issue #138 owns the `application_service` request-schema mismatch between runtime UTF-8-byte bounds and stock JSON Schema string-length assertions. The causal parent is application-service owner #21 exact `717edef989d8b6bb7e1f672d91d9258f7942a6b2`.

Issue #101 / Draft #102 remains the canonical owner of the repository's versioned `x-cwl-maxUtf8Bytes` / `x-cwl-maxSerializedUtf8Bytes` vocabulary semantics and publication contract. This lane must consume a released/versioned vocabulary contract; it must not copy mutable #102 source or publish a competing vocabulary.

## Problem and current evidence

`ApplicationServiceRequest::validate()` uses Rust `str::len()` for `request_id` and direct command arguments. Rust defines `str::len()` as byte length. The runtime limits are therefore 128 UTF-8 bytes for `request_id` and 1,024 UTF-8 bytes per command argument.

The published `schemas/application-service-request.schema.json` still declares stock JSON Schema Draft 2020-12. Draft 2020-12 defines `maxLength` in JSON-string characters, not UTF-8 octets, and an ordinary validator has no assertion semantics for the extension keyword `x-cwl-maxUtf8Bytes`.

The test-only causal candidate `be040a3e308fc09e6286bf0a5d0801b0a5c047b5` fixes no production or schema behavior. Its witness uses:

- `request_id = "é" × 65`: 65 characters, 130 UTF-8 bytes. Stock `maxLength: 128` admits the character count while runtime validation rejects `InvalidRequestId`.
- one command argument `"é" × 513`: 513 characters, 1,026 UTF-8 bytes. Stock `maxLength: 1024` admits the character count while runtime validation rejects `InvalidCommandArgument { argument_index: 0 }`.

Review `5253486572` found a false-GREEN in the original final assertion: requiring only `$schema != https://json-schema.org/draft/2020-12/schema` would allow an unrelated custom dialect to satisfy the regression while still giving `x-cwl-maxUtf8Bytes` no executable semantics. Test-only successor `cc6a6d901101d4cd35be78db525f312a23a35f39` therefore binds the RED to the versioned CWL artifact-analysis contract dialect identity owned by #101/#102.

Review `5253616141` then found a narrower false-GREEN in that hardened witness: changing only the application-service `$schema` string to the canonical URI could pass while the referenced dialect artifact was absent or did not require the CWL vocabulary/declare the byte keyword. Test-only `5d0434019e152832577e62a079f4ec5c17cbcad1` keeps the same first failure on the live stock `$schema`, but once that identity is adopted it additionally requires the referenced dialect artifact to publish the exact `$id`, remain based on Draft 2020-12, require `https://contextualwisdomlab.org/vocab/quarantine-artifact-analysis-contract-1.0.0`, and declare `x-cwl-maxUtf8Bytes` as a non-negative integer keyword. This still changes no production Rust, public schema, or vocabulary source.

The live application-service schema remains on stock Draft 2020-12, so the intended causal failure is preserved. The extra assertions prevent a URI-only substitution from being mistaken for consumer integration.

## Decision boundary

Do not change the runtime to character counting. The byte ceiling is a resource/admission invariant and is already code-current in the supporting bounded context.

After the causal RED executes, the minimum GREEN is to adopt the released/versioned CWL byte-assertion contract into the application-service public schema validation path, with unsupported implementations failing closed. Exact-bound positive vectors and one-byte-overflow negative vectors are required for both `request_id` and command arguments.

The application-service schema remains domain truth for application-service fields. The vocabulary definition remains a separately versioned repository contract owned by #101/#102. Referring to its versioned dialect identity in this RED does not make mutable #102 source consumer authority; publication and semantics remain prerequisites. The local consumer RED may verify the adopted dialect surface, but the normative vocabulary specification and conformance vectors remain #101/#102 authority. Mutable-branch dependency and source copying are forbidden.

## Executed consumer evidence — 2026-10-05

Exact predecessor `ab42260cda53dc4ce02bae4fb56b492a81c99456` executed the intended dialect-identity RED in native CI `35461316380`: verify, coverage and branch-coverage reached the schema identity assertion after the multibyte runtime controls. A source-identical local Rust 1.97.1 focused run independently reached the same assertion. This is executed RED evidence, not a released vocabulary or consumer GREEN.

The test-only follow-up adds two independent boundary controls without changing production Rust, public schemas, workflows, vocabulary files or the global Gap ledger:

- For request identifiers, ASCII plus two-, three- and four-byte Unicode scalar fixtures cover 127, 128 and 129 UTF-8 bytes.
- For command arguments, the same scalar widths cover 1,023, 1,024 and 1,025 bytes. Overflow rejection is also bound to argument indices 0, 1 and 63 within the existing 64-argument limit.
- Exact and below-bound values must remain accepted. One-byte overflow must return the field-specific error; multibyte overflow still satisfies the current stock character-count controls.

These controls pass against the existing byte-bounded runtime. They are regression coverage for implemented behavior, not a new production GREEN or execution of a standards-compliant JSON Schema validator. The stock string helper is a local character-length/control-character witness only. Actual validator interoperability and unsupported-vocabulary fail-closed execution remain part of the released consumer integration gate.

Two independent scratch-only production mutations prove the added assertions discriminate: replacing byte counts with character counts fails both overflow controls, while changing the inclusive `>` ceiling to `>=` fails both exact-bound controls. Restoring the original runtime passes both. Mutants use a separate build target; no mutation is committed to the canonical owner branch.

Historical shared-vocabulary predecessor `a7fc09fc... / 35991876348` must also be classified per job: verify observed an invocation error in the Linux ownership fixture, but stable coverage and nightly branch-coverage passed all seven ownership tests before failing the separate vocabulary publication fixture. A deterministic stale-error expectation is not established. The normal nonzero-exit expectation stays unchanged pending lower-level evidence. Current mutable #102 `c914f05fa1c76d23f086aef84548defc0cf62675` is not immutable released authority and is not copied into this consumer.

## Risks and follow-up

A schema-only consumer can currently accept requests that the runtime later rejects, creating interoperability drift at an external contract boundary. A URI-only repair would create a second interoperability defect: the application-service schema could claim a dialect contract that is absent or not actually required. Conversely, weakening runtime byte bounds to match `maxLength` would change resource semantics and is not an acceptable compatibility repair.

No release claim is valid until the hardened causal RED executes, the released vocabulary is adopted, exact-head repository/fmt/test/Clippy/rustdoc and complete owned-production coverage gates pass, applicable security/review/isolation gates are terminal, and the contract is included in immutable publication with SBOM/provenance/reproducibility/rollback evidence.

## References

Bray, T. (2017). *The JavaScript Object Notation (JSON) data interchange format* (RFC 8259). Internet Engineering Task Force. https://www.rfc-editor.org/rfc/rfc8259

JSON Schema. (2022). *JSON Schema: A media type for describing JSON documents (Draft 2020-12).* https://json-schema.org/draft/2020-12/json-schema-core

JSON Schema. (2022). *JSON Schema validation: A vocabulary for structural validation of JSON (Draft 2020-12).* https://json-schema.org/draft/2020-12/json-schema-validation

Rust Project Developers. (2026). *Primitive type `str`: `len`.* https://doc.rust-lang.org/std/primitive.str.html
