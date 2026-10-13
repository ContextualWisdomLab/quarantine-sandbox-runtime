# CWL Artifact Analysis Contract Vocabulary 1.0.0

Vocabulary URI: `https://contextualwisdomlab.org/vocab/quarantine-artifact-analysis-contract-1.0.0`

This document is the normative human-readable specification for the required ContextualWisdomLab artifact-analysis JSON Schema vocabulary. A meta-schema declaration alone is not sufficient interoperability evidence: an implementation that does not understand a vocabulary declared as required must refuse schemas using it, so these keyword semantics and the companion conformance vectors are part of the versioned contract.

## `x-cwl-maxUtf8Bytes`

The keyword value is a non-negative integer byte ceiling and applies to a JSON string instance.

An implementation **MUST count UTF-8 octets of the JSON string value**. It counts the UTF-8 encoding of the string value itself, not Unicode scalar values, grapheme clusters, UTF-16 code units, JSON quotes, or JSON escape syntax.

An implementation **MUST accept the instance if and only if the UTF-8 octet count is less than or equal to the keyword value**.

No Unicode normalization is performed by this vocabulary. The code-point sequence supplied by the parsed JSON instance is preserved when its UTF-8 representation is counted.

## `x-cwl-maxSerializedUtf8Bytes`

The keyword value is a non-negative integer byte ceiling and applies to a `BoundedSourceContext` JSON object.

Before canonical serialization, an implementation **MUST materialize every missing nullable BoundedSourceContext property as JSON null before serialization**. For vocabulary version 1.0.0 the complete nullable field set is:

- `source_channel_code`
- `original_file_name`
- `declared_media_type`
- `host_artifact_reference`
- `submitted_at`

An implementation **MUST apply RFC 8785 JSON Canonicalization Scheme (JCS) after CWL nullable-field materialization**. This includes RFC 8785 property ordering and JSON string escaping; Unicode string data is preserved rather than normalized.

An implementation **MUST encode the RFC 8785 canonical representation as UTF-8 before counting octets** and **MUST count the UTF-8 octets of that canonical serialization**.

An implementation **MUST accept the instance if and only if the serialized UTF-8 octet count is less than or equal to the keyword value**.

The byte ceiling therefore applies to the CWL-normalized, JCS-canonical JSON representation, not to the source text supplied by a caller and not to an implementation-specific map serialization. A missing nullable field and the same field explicitly set to `null` have the same counted representation after materialization.

## `x-cwl-rfc3339Profile`

The keyword applies to a JSON string instance. Vocabulary version 1.0.0 defines one assertion value: `utc_z_only_with_gregorian_day_validation_no_leap_second_notation`. A validator that recognizes this required vocabulary but does not implement that profile value MUST refuse the schema rather than silently treating the keyword as annotation-only data.

For the defined profile, an implementation MUST require a four-digit year, Gregorian month `01` through `12`, a valid Gregorian calendar day for that year and month, time-of-day hour `00` through `23`, minute `00` through `59`, and second `00` through `59`. Fractional seconds may be present. The timestamp MUST end with the uppercase UTC designator `Z`; numeric UTC offsets and lower-case `z` are not accepted by this profile.

MUST reject a calendar date whose day exceeds the Gregorian month length. February therefore has 28 days in an ordinary year and 29 days in a Gregorian leap year; April, June, September, and November have 30 days.

MUST treat a year divisible by 4 as a leap year except a century year not divisible by 400. A century year divisible by 400 remains a leap year.

MUST reject leap-second notation and require the uppercase UTC designator Z. Seconds equal to `60`, including otherwise valid RFC 3339 leap-second forms, are outside this narrower CWL profile.

MUST require the uppercase date/time separator T. RFC 3339 §5.6 permits a lower-case `t` in the base syntax, but this profile narrows it exactly as it narrows `z`; a lower-case `t` is invalid.

MUST accept RFC 3339 fractional seconds of one or more decimal digits (`time-secfrac = "." 1*DIGIT`) of any length. The profile itself has no length bound; a length limit applies only through a sibling `x-cwl-maxUtf8Bytes` keyword, as on the request's `submitted_at` field. A decimal point without at least one following digit is invalid.

An implementation MUST accept the string if and only if all of the profile constraints above hold. These semantics intentionally match the current Rust artifact-analysis request validator rather than weakening it to the stock Draft 2020-12 `format` annotation or to the structural day-of-month regular expression in the request schema.

The Gregorian profile semantics, the uppercase-`T` and fractional-second clauses above, and the applicability, schema-refusal and I-JSON rules below were all integrated into vocabulary 1.0.0 before its first immutable publication. They therefore share the same vocabulary identity and conformance publication as the byte-bound keywords; consumers must not obtain this version from mutable branch source.

## Applicability and schema refusal

Each keyword is an assertion, never an annotation-only fallback in the way stock `format` may be. Like Draft 2020-12 type-specific validation keywords, a keyword evaluates as valid for an instance type it does not apply to: `x-cwl-maxUtf8Bytes` and `x-cwl-rfc3339Profile` apply only to strings, and `x-cwl-maxSerializedUtf8Bytes` applies only to objects. The sibling `type` keyword owns type rejection; a nullable `submitted_at` set to `null` therefore satisfies `x-cwl-rfc3339Profile`.

For an object instance, `x-cwl-maxSerializedUtf8Bytes` MUST evaluate as invalid when the object has a member outside the closed five-member `BoundedSourceContext` set above or a member whose value is neither a string nor `null`. Such an instance has no defined CWL materialization; the keyword must not count an implementation-specific serialization of it.

A consumer MUST refuse the schema, rather than evaluate an instance, when an `x-cwl-` keyword is not defined by this version, when `x-cwl-rfc3339Profile` names a profile value other than the exact case-sensitive token defined above, or when a byte-ceiling keyword value is not a non-negative integer. Following Draft 2020-12, an integer-valued number such as `30.0` is an integer. A byte ceiling greater than 9007199254740991 (2^53 − 1) is refused because not every JSON consumer can represent it exactly.

An instance that is not I-JSON (RFC 7493) — for example one with duplicate member names or a string containing an unpaired surrogate escape such as `\ud800` — has no single CWL interpretation. A consumer MUST treat such an instance as invalid rather than choose one parse of it; the reference request validator rejects both forms before any keyword is evaluated. The conformance vectors cannot express these forms because their parsed JSON representation is already I-JSON.

## Conformance publication

The companion machine-readable vectors are published at `tests/fixtures/cwl_artifact_analysis_contract_vocabulary_1_0_0_vectors.json`. They are normative examples for vocabulary version 1.0.0 and include:

- a multibyte UTF-8 inclusive boundary and overflow for `x-cwl-maxUtf8Bytes`;
- an inclusive serialized boundary after missing-null materialization;
- an overflow caused by that same materialization;
- an escaping/non-ASCII JCS boundary that pins the exact canonical JSON representation instead of trusting fixture metadata;
- a valid Gregorian leap-day control for `x-cwl-rfc3339Profile`;
- invalid non-leap February 29 and impossible month/day controls;
- invalid leap-second notation and numeric-offset substitution controls for the uppercase-`Z` profile;
- century controls distinguishing the 4/100/400 rule (`1900-02-29` invalid, `2000-02-29` valid) and a 30-day-month control (`2026-04-31` invalid);
- case-sensitivity controls rejecting lower-case `z` and lower-case `t`;
- an accepting integer-valued number ceiling (`30.0`) that a consumer must treat as the integer `30`;
- an accepting fractional-second control, an accepting 41-byte fractional-second control showing the profile has no length bound of its own, a rejected empty fraction, and a rejected `2100-02-29`;
- not-applicable controls (`null`/non-object instances evaluate as valid) and invalid unknown-member and non-string-member controls for `x-cwl-maxSerializedUtf8Bytes`;
- a separate `schema_refusal_cases` array of keyword occurrences that a consumer must refuse instead of evaluating.

Implementations claiming this vocabulary version must produce the same validity result for those vectors. The vectors are conformance evidence for the exact versioned vocabulary identity; later keywords or semantic changes require a new released contract version rather than mutation of this publication.

## References

Andrews, H., Hutton, B., Dennis, G., & Wright, A. (2022). *JSON Schema: A media type for describing JSON documents (Draft 2020-12).* JSON Schema. https://json-schema.org/draft/2020-12/json-schema-core

Klyne, G., & Newman, C. (2002). *Date and Time on the Internet: Timestamps* (RFC 3339). RFC Editor. https://doi.org/10.17487/RFC3339

Rundgren, A., Jordan, B., & Erdtman, S. (2020). *JSON Canonicalization Scheme (JCS) (RFC 8785).* RFC Editor. https://doi.org/10.17487/RFC8785
