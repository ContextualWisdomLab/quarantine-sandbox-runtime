# Analysis request `submitted_at` schema traceability

## Decision state

**Proposed / RED-only.** Issue #134 owns the public-contract parity gap for `bounded_source_context.submitted_at`. Draft #102 / Issue #101 remains the prerequisite owner of the versioned CWL artifact-analysis contract dialect. This lane must not create a second dialect or copy mutable #102 source.

Current parent authority is artifact-analysis #18 exact `703b4b1a047321bb08d1d6cb1de78d01cb350696`.

## Problem and executable witness

The Rust request validator and the published stock Draft 2020-12 schema do not currently have the same acceptance set.

`src/artifact_analysis/contracts.rs::is_valid_submitted_at` parses year, month, and day and rejects a day greater than `days_in_month(year, month)`. Its leap-year rule is Gregorian: divisible by 4, except centuries not divisible by 400.

The public `schemas/analysis-request.schema.json` instead declares the stock Draft 2020-12 meta-schema, `format: "date-time"`, and a structural pattern whose day component is `01..31` independently of month/year. It also carries `x-cwl-rfc3339Profile`, but an unknown custom keyword under the stock dialect is not an assertion. A schema-only consumer can therefore accept `2026-02-31T00:00:00Z` while `AnalysisRequest::validate()` rejects it.

Test `tests/artifact_analysis_submitted_at_schema_contract_red.rs` keeps two concerns separate:

- the existing Rust validator must reject 2026-02-31 and non-leap 2023-02-29 while accepting 2024-02-29;
- the published schema must bind its named CWL RFC 3339 profile to the canonical #101/#102 dialect, whose CWL contract vocabulary is required and whose meta-schema recognizes `x-cwl-rfc3339Profile`.

Review `5237628204` found a false-GREEN in the initial RED: merely requiring `$schema` to differ from the stock Draft 2020-12 URI would allow an arbitrary non-stock URI to pass without proving canonical dialect/vocabulary adoption. Test-only commit `cd39c38a864f66bc4d51584dcf60f13cb9c574e4` hardens the witness to require the exact canonical CWL dialect URI, the required CWL vocabulary, and recognition of the RFC3339 profile keyword. Production Rust and public schema remain unchanged. On the current parent, the intended first failure remains the stock `$schema` authority itself; the dialect artifact is read only after that exact owner assertion passes.

## Standards basis

JSON Schema Draft 2020-12 split `format` into Format-Annotation and Format-Assertion vocabularies. The default meta-schema requires Format-Annotation; implementations may additionally assert formats, but that behavior is disabled by default. A custom meta-schema may require Format-Assertion, in which case an implementation that cannot provide full support must refuse to process the schema. JSON Schema Core likewise requires an implementation to refuse a meta-schema whose `$vocabulary` marks an unrecognized vocabulary as required.

RFC 3339 §5.7 defines `date-mday` as dependent on month and year: February has 28 days in a normal year and 29 in a leap year; April, June, September, and November have 30. The repository intentionally applies a narrower CWL profile than generic RFC 3339 by requiring UTC `Z`, disallowing leap-second notation, and bounding serialized bytes.

The selected architecture is therefore not “make the regex larger.” The canonical public validation authority is a versioned dialect/vocabulary with fail-closed unsupported-consumer behavior, backed by the released Rust validator. #101/#102 already introduces that dialect for UTF-8 byte semantics and includes `x-cwl-rfc3339Profile` in the same required vocabulary; #134 extends its normative semantics to Gregorian validity.

## Rejected alternatives

- **Rely on validator configuration for `format`.** Rejected because stock Draft 2020-12 does not require assertion behavior and consumer acceptance would depend on out-of-band configuration.
- **Encode calendar validity only with a regex.** Rejected because leap-year arithmetic is not a stable or reviewable regex contract and would duplicate the existing runtime validator.
- **Weaken the Rust validator to the structural regex.** Rejected because that admits impossible dates and violates the existing bounded source-metadata contract.
- **Create a second CWL dialect in this lane.** Rejected because #101/#102 already owns the dialect and duplicate vocabularies would create competing public authorities.
- **Accept any non-stock `$schema` URI as GREEN.** Rejected because it would not prove adoption of the canonical required vocabulary and would let a documentation-only URI substitution satisfy the regression.
- **Treat `x-cwl-rfc3339Profile` as documentation only.** Rejected because the schema advertises it as a contract property while unsupported schema consumers would silently ignore it.

## GREEN and release gate

After the hardened exact RED executes for the intended missing-authority cause, the smallest owner-safe GREEN is to adopt the #101/#102 dialect into current #18 ancestry and define `utc_z_only_with_gregorian_day_validation_no_leap_second_notation` normatively in that required vocabulary. Unsupported validators must fail closed or consume the released runtime validator. Positive controls must include a Gregorian leap day; hostile controls must include impossible month/day combinations and non-leap February 29.

GREEN on this focused contract does not transfer predecessor CI and does not waive repository validation, rustfmt, full locked workspace/all-target tests, Clippy, public/private rustdoc, 100% owned production statement/function/region/branch/edge coverage, qualifying review/thread/security gates, protected integration, positive runtime-isolation evidence where applicable, or immutable dialect/schema/runtime publication with SBOM, provenance, reproducibility, and rollback evidence.

## References

JSON Schema. (2022). *JSON Schema Draft 2020-12: Release notes*. https://json-schema.org/draft/2020-12/release-notes

JSON Schema. (2022). *JSON Schema validation: A vocabulary for structural validation of JSON (Draft 2020-12)*. https://json-schema.org/draft/2020-12/json-schema-validation

JSON Schema. (2022). *JSON Schema: A media type for describing JSON documents (Draft 2020-12)*. https://json-schema.org/draft/2020-12/json-schema-core

Klyne, G., & Newman, C. (2002). *Date and Time on the Internet: Timestamps* (RFC 3339). RFC Editor. https://www.rfc-editor.org/rfc/rfc3339
