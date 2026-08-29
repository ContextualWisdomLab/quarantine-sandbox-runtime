# Changelog

All notable changes to this project are documented in this file.

The format follows Keep a Changelog, and this project uses Semantic Versioning.

## [Unreleased]

### Added

- Source-agnostic analysis request and evidence contracts.
- Bounded immutable artifact ingestion and SHA-256 identity.
- Deterministic executable, archive, document, script, text, and unknown-format classification.
- Pluggable static analyzer interface and attributable analyzer-failure evidence.
- Explicit runtime boundary declaration for no credentials, no network access, and no artifact execution in the foundation profile.
- Draft 2020-12 JSON Schemas for requests and evidence bundles.
- Security, threat-model, operability, testing, architecture, ADR, and research-traceability baselines.
- Exact-head Rust quality and coverage workflows.
- Property tests for deterministic immutable ingestion.
- Optional typed `BoundedSourceContext` for source channel, original leaf file name, declared media type, opaque host artifact reference, and UTC submission time.

### Changed

- Evidence identity now includes policy, source revision, and ordered analyzer identifiers.
- Static analyzer findings are restricted to file-format and static-capability evidence.
- JSON Schemas now expose UTF-8 byte limits and reject control text consistently with Rust validation.
- Runtime analysis no longer requires a caller-supplied artifact name; optional source-context file names remain untrusted classification metadata.
- Required free-form source metadata was replaced by a closed typed context capped at 1,024 serialized UTF-8 bytes.
- Source timestamp and media-type validation now trace to RFC 3339 and BCP 13 (RFC 6838 as updated by RFC 9694).

### Security

- Duplicate or malformed analyzer identifiers fail engine construction.
- Universal Mach-O headers are structurally bounded so Java class magic is not treated as Mach-O by signature alone.
- Unsupported dynamic profiles and analyzer failures remain explicitly inconclusive.
- Path-like file names, URL/credential-shaped host references, unknown source-context fields, malformed media types, invalid UTC timestamps, and oversized context payloads fail closed before artifact analysis.
