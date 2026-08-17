# Agent Development Rules

## Product boundary

- This repository owns isolated artifact ingestion, analysis execution, evidence normalization, and runtime attestation.
- It does not own malicious/benign verdict policy, incident response, or consumer-specific actions.
- Do not couple the core runtime to GitHub, email, Naruon, Wardnet, or a specific upload service.
- Consumer adapters submit bytes and context through versioned contracts.

## Security

- Treat every artifact, filename, analyzer output, and metadata field as hostile.
- Never execute artifact content in the control process.
- Never make external network requests from the foundation runtime.
- Never provide production credentials to an analysis worker.
- Fail closed when a requested analysis profile is unavailable.
- Preserve original bytes and SHA-256 identity without mutation.
- Keep `#![forbid(unsafe_code)]` unless a reviewed ADR explicitly changes the boundary.
- Parser and analyzer dependencies must be pinned through `Cargo.lock` and reviewed for advisories.

## Rust quality

- Production arithmetic and policy logic are Rust.
- Public modules, types, fields, traits, functions, and methods require explanatory documentation.
- Production statement, function, region, and branch coverage target 100%.
- Tests must exercise hostile inputs and realistic malware-delivery formats.
- Do not silently ignore analyzer failures; emit attributable failure evidence.
- Do not use `unwrap`, `expect`, or `panic` in production code.

## Contracts

- Database and future persistence object names use two or more words in `snake_case`.
- JSON contracts use explicit schema versions and JSON Schema Draft 2020-12.
- Evidence identifiers and ordering must be deterministic for the same request and bytes.
- A runtime disposition describes analysis completeness, not maliciousness.
- Any LLM integration must consume evidence through `contextual-orchestrator`; it cannot be the sole verdict authority.

## Pull requests

- Work test-first.
- Keep changes within the owning repository.
- Update `CHANGELOG.md`, ADRs, security documentation, and test evidence with behavioral changes.
- Never bypass required reviews, exact-head checks, or repository protection.
