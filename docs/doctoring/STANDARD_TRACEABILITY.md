# Standard and Research Traceability

| External source | Product decision | Evidence in this repository |
|---|---|---|
| JSON Schema Draft 2020-12 | strict versioned request and evidence contracts; non-standard byte-budget annotations make runtime-only UTF-8 limits explicit | `schemas/` and repository validator |
| RFC 3339 | `submitted_at` accepts a deliberately narrower UTC-`Z` Internet timestamp profile and validates Gregorian calendar boundaries before analysis | `BoundedSourceContext`, contract tests, request schema, TRD |
| RFC 6838 / BCP 13 | `declared_media_type` is bounded to media type/subtype syntax and remains an untrusted hint rather than detected content type | `BoundedSourceContext`, contract tests, request schema, TRD |
| RFC 9694 / BCP 13 | RFC 6838 is updated for guidance on new top-level media types; this runtime does not invent or register top-level types and only validates submitted syntax | references and TRD |
| W3C PROV-O | evidence must retain producer and derivation-ready identifiers | `EvidenceRecord`, ADR 0004 |
| SLSA 1.2 | releases require build provenance | security and release roadmap |
| SPDX 3.0.1 | releases require machine-readable SBOM | security and release roadmap |
| Firecracker design | guest egress requires host enforcement and workers require defense in depth | dynamic-worker requirements |
| MITRE ATT&CK T1497 | sandbox evasion must be tested rather than assuming one execution is complete | threat model and dynamic validation plan |
| YARA-X stable release | prefer maintained Rust-based rule engine in a future static adapter | product roadmap, not foundation dependency |
| capa | future capability evidence should be typed and ATT&CK-mappable | analyzer port and roadmap |
| SARIF 2.1.0 | future analyzer findings can project to a standard interchange format | roadmap |
