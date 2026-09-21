# Analysis request `submitted_at` schema traceability

## Decision state

**Proposed / executable RED hardening on the canonical shared-vocabulary owner.** Issue #134 remains the public-contract parity defect. Focused #135 exact `107ce5356c477f2a5a67392b2ac4e8bb7fe6169f` executed its Gregorian-profile RED, and canonical vocabulary owner #102 ordinarily adopted that complete focused test/TRACEABILITY delta through two-parent merge `e840c7274b6725c3a35f187dbe74bc4f0c7259b8`. Vocabulary 1.0.0 has not been immutably published, so the current owner may still complete that same version before first publication.

Repository-wide product/technical Gap truth remains #121 single-writer authority. This document owns only the `submitted_at` profile decision and executable evidence.

## Problem and executable witness

The Rust request validator and portable public validation contract must expose the same acceptance set for `bounded_source_context.submitted_at`.

`src/artifact_analysis/contracts.rs::is_valid_submitted_at` parses year, month, and day, rejects a day greater than `days_in_month(year, month)`, applies the Gregorian 4/100/400 leap-year rule, rejects seconds greater than 59, and requires a final upper-case `Z`. The runtime therefore already distinguishes semantic branches that a structural day-of-month regular expression cannot express.

The public request schema uses the canonical CWL Draft 2020-12 dialect and required vocabulary keyword `x-cwl-rfc3339Profile`. The corresponding human-readable 1.0.0 vocabulary now normatively defines the exact profile token `utc_z_only_with_gregorian_day_validation_no_leap_second_notation`, Gregorian month lengths, the 4/100/400 leap-year rule, seconds `00..59`, and upper-case `Z` only.

The original executable witness in `tests/artifact_analysis_submitted_at_schema_contract_red.rs` proves the principal runtime/publication cases: valid `2024-02-29`, invalid `2023-02-29`, invalid `2026-02-31`, leap-second rejection, and numeric-offset rejection. Review `5263902467` found that this was still an interoperability false-GREEN. A conformer using only `year % 4 == 0`, accepting day 31 in every month, or accepting lower-case `z` case-insensitively could pass every existing machine-readable vector while violating the normative 1.0.0 text.

Test-only commit `a026639258eae37729607794a4e46515061847a4` adds `tests/artifact_analysis_rfc3339_profile_edge_vectors_red.rs`. It separates runtime correctness from publication completeness and requires four independent edge branches:

- `1900-02-29T00:00:00Z` is invalid because a century year not divisible by 400 is not a leap year;
- `2000-02-29T00:00:00Z` is valid because a century year divisible by 400 remains a leap year;
- `2026-04-31T00:00:00Z` is invalid because April has 30 days;
- `2024-02-29T23:59:59z` is invalid because this CWL profile requires upper-case `Z`.

The runtime controls are expected to pass on current source. The publication half intentionally requires four vector IDs that do not yet exist in `tests/fixtures/cwl_artifact_analysis_contract_vocabulary_1_0_0_vectors.json`. That missing-vector failure is the intended causal RED. Do not add the vectors or change production runtime logic until this exact RED executes for that cause.

## Earlier false-GREEN repairs retained

Review `5237628204` found that merely requiring `$schema` to differ from the stock Draft 2020-12 URI would allow an arbitrary non-stock URI to pass without proving canonical dialect/vocabulary adoption. Test-only `cd39c38a864f66bc4d51584dcf60f13cb9c574e4` therefore requires the exact canonical CWL dialect URI, the required CWL vocabulary, and recognition of the RFC3339 profile keyword.

Review `5253891162` found that a dialect may recognize `x-cwl-rfc3339Profile` only as `type: string` while publishing no semantics for `utc_z_only_with_gregorian_day_validation_no_leap_second_notation`. Test-only `b8666577349bc7a77d10889fda810608dd62394c` binds the witness to the canonical vocabulary specification and conformance publication.

#102 predecessor `71e63d9e24ae0f6fbe7c098c8a0a66e4eac500f7` and #135 exact `107ce535...` subsequently executed the generic and focused same-version publication REDs. #102 then added the normative profile semantics and the first five Gregorian vectors as a GREEN candidate. The new edge-vector RED does not invalidate that owner-safe succession; it tightens the machine-readable conformance surface before immutable publication.

## Standards basis

JSON Schema Draft 2020-12 defines a vocabulary as a set of keywords together with their syntax and semantics. When a meta-schema marks a vocabulary `true` in `$vocabulary`, an implementation that does not recognize that vocabulary must refuse to process schemas using the meta-schema. Unknown keywords outside recognized required vocabularies otherwise behave as annotations. Draft 2020-12 also separates Format-Annotation from Format-Assertion, so stock `format: date-time` alone is not this repository's portable fail-closed Gregorian authority.

RFC 3339 §5.7 makes `date-mday` depend on month and year: February has 28 days in an ordinary year and 29 in a leap year; April, June, September, and November have 30. RFC 3339 Appendix C gives the Gregorian leap-year computation including the century/400 exception. RFC 3339 permits lower-case `t`/`z` in its base syntax and leap-second notation in defined circumstances, while explicitly allowing profiles to require upper-case letters. The CWL profile is therefore intentionally narrower: upper-case UTC `Z` only, no leap-second notation, and full Gregorian day validity.

Concrete 1900/2000 century controls are necessary because the prose rule has two materially different century branches. A single 2023/2024 pair does not distinguish correct Gregorian arithmetic from the common but incorrect `year % 4 == 0` shortcut. Likewise, a February 31 control does not prove 30-day-month behavior, and a numeric-offset control does not prove case-sensitive `Z` behavior.

## DDD and publication boundary

`artifact_analysis` owns the request and vocabulary semantics. The CWL dialect/vocabulary is a versioned public contract, not runtime implementation detail. The Rust validator is executable producer authority; the machine-readable vectors are independent interoperability evidence for consumers. A mutable PR head is never consumer authority.

Because 1.0.0 has no immutable publication, the smallest allowed GREEN after causal execution is to add only the four missing same-version vectors when the runtime controls remain GREEN. If any runtime edge control fails instead, repair only the corresponding validator branch and preserve the exact public profile. Do not normalize lower-case `z`, weaken Gregorian validity, create a second dialect, or change the vocabulary URI after first immutable publication.

## Rejected alternatives

- **Rely on validator configuration for `format`.** Stock Draft 2020-12 does not make that a portable fail-closed contract.
- **Encode all calendar validity only with a regex.** Leap-year arithmetic would duplicate runtime logic and remain difficult to audit.
- **Use only 2023/2024 leap-day vectors.** That permits a `% 4` false implementation to claim conformance.
- **Use only February for month-length evidence.** That does not prove April/June/September/November limits.
- **Treat numeric-offset rejection as proof of upper-case `Z`.** A case-insensitive `z` implementation could still pass.
- **Weaken the Rust validator to the structural schema regex.** That admits impossible calendar dates.
- **Create a second CWL dialect in this lane.** #101/#102 already owns the shared dialect and vocabulary publication surface.
- **Treat keyword recognition as executable semantics.** A meta-schema declaration such as `type: string` proves syntax only.

## GREEN and release gate

Let the current #102 exact containing `a026639...` and this TRACEABILITY update execute unchanged. The expected RED is missing edge vectors after runtime controls remain GREEN. Only after that causal evidence may #102 add the four minimum conformance cases and reacquire repository validation, rustfmt, full locked workspace/all-target tests, Clippy/rustdoc, generic vocabulary-completeness and focused Gregorian tests, complete applicable coverage, qualifying review/thread/security gates, dependency-safe parent integration, protected verification, and applicable positive isolation evidence.

#135 remains open until one exact #102 successor proves the complete inherited focused delta plus these hardened publication semantics. Consumer #139 may bind only an immutable released dialect/vocabulary version. Publication still requires version/package identity, immutable tag/package/GitHub Release or canonical equivalent, SBOM, provenance, reproducibility, and rollback evidence.

## References

Andrews, H., Hutton, B., Dennis, G., & Wright, A. (2022). *JSON Schema: A media type for describing JSON documents (Draft 2020-12).* JSON Schema. https://json-schema.org/draft/2020-12/json-schema-core

Klyne, G., & Newman, C. (2002). *Date and Time on the Internet: Timestamps* (RFC 3339). RFC Editor. https://www.rfc-editor.org/rfc/rfc3339

Wright, A., Andrews, H., Hutton, B., & Dennis, G. (2022). *JSON Schema validation: A vocabulary for structural validation of JSON (Draft 2020-12).* JSON Schema. https://json-schema.org/draft/2020-12/json-schema-validation
