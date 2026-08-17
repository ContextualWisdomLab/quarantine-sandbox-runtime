# Quarantine Sandbox Runtime

`quarantine-sandbox-runtime` is the source-agnostic, credential-free artifact-analysis runtime for the ContextualWisdomLab security ecosystem.

The runtime accepts untrusted artifact bytes plus submission context, produces deterministic evidence with provenance, and returns an execution-completeness disposition. It **does not decide whether an artifact is malicious**. Wardnet or another authorized consumer owns verdict policy, incident handling, quarantine, blocking, and user-facing action.

## Foundation scope

This initial foundation provides:

- immutable SHA-256 artifact identity;
- bounded in-memory ingestion;
- deterministic PE, ELF, Mach-O, ZIP, PDF, OLE, script, text, and unknown-format classification;
- versioned request and evidence contracts;
- pluggable static analyzers;
- fail-closed handling for unavailable analyzers;
- explicit proof that no artifact was executed, no network access occurred, and no credentials were available;
- JSON Schema Draft 2020-12 contracts;
- Rust 1.97.1, edition 2024, `unsafe` forbidden;
- exact-head CI for formatting, tests, linting, documentation, repository policy, and coverage.

Dynamic detonation, unpacking engines, YARA-X, capa, Ghidra, Windows VM workers, Linux microVM workers, network sinkholes, and Wardnet verdict integration are intentionally separate follow-on slices.

## Trust boundary

```text
Untrusted bytes
      |
      v
Quarantine Sandbox Runtime
- bounded ingestion
- identity and format evidence
- static analyzer adapters
- no credentials
- no network
- no execution
      |
      v
Signed / attributable evidence bundle
      |
      v
Wardnet or another consumer
- verdict
- incident
- quarantine
- block / allow / review policy
```

## Development

```bash
cargo fmt --check
cargo test --locked --workspace --all-targets
cargo clippy --locked --workspace --all-targets -- -D warnings
RUSTDOCFLAGS="-D warnings" cargo doc --locked --workspace --no-deps
python3 scripts/validate_repository.py
```

The canonical design and implementation plan are:

- `docs/superpowers/specs/2026-08-17-quarantine-runtime-foundation-design.md`
- `docs/superpowers/plans/2026-08-17-quarantine-runtime-foundation.md`
