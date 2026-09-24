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

An implementation MUST accept the string if and only if all of the profile constraints above hold. These semantics intentionally match the current Rust artifact-analysis request validator rather than weakening it to the stock Draft 2020-12 `format` annotation or to the structural day-of-month regular expression in the request schema.

The Gregorian profile semantics were integrated into vocabulary 1.0.0 before its first immutable publication. They therefore share the same vocabulary identity and conformance publication as the byte-bound keywords; consumers must not obtain this version from mutable branch source.

## Conformance publication

The companion machine-readable vectors are published at `tests/fixtures/cwl_artifact_analysis_contract_vocabulary_1_0_0_vectors.json`. They are normative examples for vocabulary version 1.0.0 and include:

- a multibyte UTF-8 inclusive boundary and overflow for `x-cwl-maxUtf8Bytes`;
- an inclusive serialized boundary after missing-null materialization;
- an overflow caused by that same materialization;
- an escaping/non-ASCII JCS boundary that pins the exact canonical JSON representation instead of trusting fixture metadata;
- a valid Gregorian leap-day control for `x-cwl-rfc3339Profile`;
- invalid non-leap February 29 and impossible month/day controls;
- invalid leap-second notation and numeric-offset substitution controls for the uppercase-`Z` profile.

Implementations claiming this vocabulary version must produce the same validity result for those vectors. The vectors are conformance evidence for the exact versioned vocabulary identity; later keywords or semantic changes require a new released contract version rather than mutation of this publication.

## References

Andrews, H., Hutton, B., Dennis, G., & Wright, A. (2022). *JSON Schema: A media type for describing JSON documents (Draft 2020-12).* JSON Schema. https://json-schema.org/draft/2020-12/json-schema-core

Klyne, G., & Newman, C. (2002). *Date and Time on the Internet: Timestamps* (RFC 3339). RFC Editor. https://doi.org/10.17487/RFC3339

Rundgren, A., Jordan, B., & Erdtman, S. (2020). *JSON Canonicalization Scheme (JCS) (RFC 8785).* RFC Editor. https://doi.org/10.17487/RFC8785
