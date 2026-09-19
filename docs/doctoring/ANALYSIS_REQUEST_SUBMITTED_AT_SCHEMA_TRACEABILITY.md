# Analysis request `submitted_at` schema traceability

## Decision state

**Proposed / RED-only.** Issue #134 owns the public-contract parity gap for `bounded_source_context.submitted_at`. Draft #102 / Issue #101 remains the prerequisite owner of the versioned CWL artifact-analysis contract dialect and its canonical publication surface. This lane must not create a second dialect or copy mutable #102 source.

Current parent authority is artifact-analysis #18 exact `703b4b1a047321bb08d1d6cb1de78d01cb350696`.

## Problem and executable witness

The Rust request validator and the published stock Draft 2020-12 schema do not currently have the same acceptance set.

`src/artifact_analysis/contracts.rs::is_valid_submitted_at` parses year, month, and day and rejects a day greater than `days_in_month(year, month)`. Its leap-year rule is Gregorian: divisible by 4, except centuries not divisible by 400.

The public `schemas/analysis-request.schema.json` instead declares the stock Draft 2020-12 meta-schema, `format: "date-time"`, and a structural pattern whose day component is `01..31` independently of month/year. It also carries `x-cwl-rfc3339Profile`, but an unknown custom keyword under the stock dialect is not an assertion. A schema-only consumer can therefore accept `2026-02-31T00:00:00Z` while `AnalysisRequest::validate()` rejects it.

Test `tests/artifact_analysis_submitted_at_schema_contract_red.rs` keeps the runtime and publication concerns separate:

- the existing Rust validator must reject `2026-02-31` and non-leap `2023-02-29` while accepting `2024-02-29`;
- the published schema must bind its named CWL RFC 3339 profile to the canonical #101/#102 dialect, whose CWL contract vocabulary is required and whose meta-schema recognizes `x-cwl-rfc3339Profile`;
- the required vocabulary must publish normative semantics for the exact profile token and conformance vectors for a Gregorian leap day, a non-leap February 29, an impossible month/day, leap-second notation, and a numeric UTC offset.

Review `5237628204` found the first false-GREEN: merely requiring `$schema` to differ from the stock Draft 2020-12 URI would allow an arbitrary non-stock URI to pass without proving canonical dialect/vocabulary adoption. Test-only `cd39c38a864f66bc4d51584dcf60f13cb9c574e4` therefore requires the exact canonical CWL dialect URI, the required CWL vocabulary, and recognition of the RFC3339 profile keyword.

Review `5253891162` found a second false-GREEN: a dialect may recognize `x-cwl-rfc3339Profile` only as `type: string` while publishing no semantics for `utc_z_only_with_gregorian_day_validation_no_leap_second_notation`. That would satisfy keyword recognition while leaving the profile documentation-only. Test-only `b8666577349bc7a77d10889fda810608dd62394c` reuses #102's canonical publication paths—`docs/contracts/cwl_artifact_analysis_contract_vocabulary_1_0_0.md` and `tests/fixtures/cwl_artifact_analysis_contract_vocabulary_1_0_0_vectors.json`—and requires the profile's normative clauses plus positive/negative conformance vectors. Production Rust, public schema, dialect, and vocabulary publication remain unchanged in this RED hardening.

On current #18 ancestry the intended first failure remains the stock `$schema` authority. The dialect and publication artifacts are read only after that exact owner assertion passes, so later assertions prevent a URI-only or keyword-declaration-only false GREEN without changing the current causal entry point.

## Standards basis

JSON Schema Draft 2020-12 defines a vocabulary as a set of keywords together with their syntax **and semantics**. When a meta-schema marks a vocabulary `true` in `$vocabulary`, an implementation that does not recognize that vocabulary must refuse to process schemas using the meta-schema. Unknown keywords otherwise remain annotations. Draft 2020-12 also separates Format-Annotation from Format-Assertion; the default meta-schema requires annotation behavior rather than universal assertion behavior.

RFC 3339 §5.7 makes `date-mday` depend on month and year: February has 28 days in a normal year and 29 in a leap year; April, June, September, and November have 30. RFC 3339 itself permits leap-second notation in defined circumstances and permits numeric offsets, so the repository's named CWL profile is intentionally narrower: uppercase UTC `Z` only, no leap-second notation, and Gregorian day validity.

The selected architecture is therefore not “make the regex larger.” The public validation authority is a versioned required dialect/vocabulary with independently implementable semantics and conformance vectors, backed by the released Rust validator. #101/#102 owns the shared vocabulary publication mechanism; #134/#135 owns the Gregorian profile semantics that must be added through that canonical contract after the prerequisite is released.

## Rejected alternatives

- **Rely on validator configuration for `format`.** Stock Draft 2020-12 does not make that a portable fail-closed contract.
- **Encode calendar validity only with a regex.** Leap-year arithmetic would duplicate runtime logic and remain difficult to review as a public contract.
- **Weaken the Rust validator to the structural regex.** That admits impossible calendar dates.
- **Create a second CWL dialect in this lane.** #101/#102 already owns the shared dialect and vocabulary publication surface.
- **Accept any non-stock `$schema` URI as GREEN.** A documentation-only URI substitution would satisfy it.
- **Treat keyword recognition as executable semantics.** A meta-schema declaration such as `type: string` proves syntax only; it does not define what the named profile means.
- **Treat `x-cwl-rfc3339Profile` as documentation only.** The public schema advertises it as contract data while unsupported consumers could silently ignore it.

## GREEN and release gate

After this exact RED executes for the intended missing-authority cause, the smallest owner-safe GREEN is to adopt the released #101/#102 dialect into current #18 ancestry and extend the canonical required vocabulary publication with the named profile semantics and conformance vectors. Unsupported validators must fail closed or consume the released runtime validator. The repair must not copy a mutable #102 tree, introduce a second dialect, or weaken the Rust acceptance set.

Focused GREEN does not transfer predecessor CI and does not waive repository validation, rustfmt, full locked workspace/all-target tests, Clippy, public/private rustdoc, 100% owned production statement/function/region/branch/edge coverage, qualifying review/thread/security gates, protected integration, positive runtime-isolation evidence where applicable, or immutable dialect/schema/runtime publication with SBOM, provenance, reproducibility, and rollback evidence.

## References

JSON Schema. (2022). *JSON Schema Draft 2020-12: Release notes*. https://json-schema.org/draft/2020-12/release-notes

JSON Schema. (2022). *JSON Schema validation: A vocabulary for structural validation of JSON (Draft 2020-12)*. https://json-schema.org/draft/2020-12/json-schema-validation

JSON Schema. (2022). *JSON Schema: A media type for describing JSON documents (Draft 2020-12)*. https://json-schema.org/draft/2020-12/json-schema-core

Klyne, G., & Newman, C. (2002). *Date and Time on the Internet: Timestamps* (RFC 3339). RFC Editor. https://www.rfc-editor.org/rfc/rfc3339
