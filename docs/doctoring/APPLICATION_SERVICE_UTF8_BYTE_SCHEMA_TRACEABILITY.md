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

Review `5253486572` found a false-GREEN in the original final assertion: requiring only `$schema != https://json-schema.org/draft/2020-12/schema` would allow an unrelated custom dialect to satisfy the regression while still giving `x-cwl-maxUtf8Bytes` no executable semantics. Test-only successor `cc6a6d901101d4cd35be78db525f312a23a35f39` therefore binds the RED to the versioned CWL artifact-analysis contract dialect identity owned by #101/#102. It still changes no production Rust, public schema, or vocabulary source. The live schema remains on stock Draft 2020-12, so the intended causal failure is preserved.

## Decision boundary

Do not change the runtime to character counting. The byte ceiling is a resource/admission invariant and is already code-current in the supporting bounded context.

After the causal RED executes, the minimum GREEN is to adopt the released/versioned CWL byte-assertion contract into the application-service public schema validation path, with unsupported implementations failing closed. Exact-bound positive vectors and one-byte-overflow negative vectors are required for both `request_id` and command arguments.

The application-service schema remains domain truth for application-service fields. The vocabulary definition remains a separately versioned repository contract owned by #101/#102. Referring to its versioned dialect identity in this RED does not make mutable #102 source consumer authority; publication and semantics remain prerequisites. Mutable-branch dependency and source copying are forbidden.

## Risks and follow-up

A schema-only consumer can currently accept requests that the runtime later rejects, creating interoperability drift at an external contract boundary. Conversely, weakening runtime byte bounds to match `maxLength` would change resource semantics and is not an acceptable compatibility repair.

No release claim is valid until the hardened causal RED executes, the released vocabulary is adopted, exact-head repository/fmt/test/Clippy/rustdoc and complete owned-production coverage gates pass, applicable security/review/isolation gates are terminal, and the contract is included in immutable publication with SBOM/provenance/reproducibility/rollback evidence.

## References

Bray, T. (2017). *The JavaScript Object Notation (JSON) data interchange format* (RFC 8259). Internet Engineering Task Force. https://www.rfc-editor.org/rfc/rfc8259

JSON Schema. (2022). *JSON Schema: A media type for describing JSON documents (Draft 2020-12).* https://json-schema.org/draft/2020-12/json-schema-core

JSON Schema. (2022). *JSON Schema validation: A vocabulary for structural validation of JSON (Draft 2020-12).* https://json-schema.org/draft/2020-12/json-schema-validation

Rust Project Developers. (2026). *Primitive type `str`: `len`.* https://doc.rust-lang.org/std/primitive.str.html
