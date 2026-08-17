# Standard and Research Traceability

| External source | Product decision | Evidence in this repository |
|---|---|---|
| JSON Schema Draft 2020-12 | strict versioned request and evidence contracts | `schemas/` and repository validator |
| W3C PROV-O | evidence must retain producer and derivation-ready identifiers | `EvidenceRecord`, ADR 0004 |
| SLSA 1.2 | releases require build provenance | security and release roadmap |
| SPDX 3.0.1 | releases require machine-readable SBOM | security and release roadmap |
| Firecracker design | guest egress requires host enforcement and workers require defense in depth | dynamic-worker requirements |
| MITRE ATT&CK T1497 | sandbox evasion must be tested rather than assuming one execution is complete | threat model and dynamic validation plan |
| YARA-X stable release | prefer maintained Rust-based rule engine in a future static adapter | product roadmap, not foundation dependency |
| capa | future capability evidence should be typed and ATT&CK-mappable | analyzer port and roadmap |
| SARIF 2.1.0 | future analyzer findings can project to a standard interchange format | roadmap |
