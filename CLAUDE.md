# Quarantine Sandbox Runtime Context

Read `AGENTS.md` first.

This repository implements a source-agnostic artifact-analysis runtime. The initial release is deliberately static and credential-free. It receives bytes, validates bounded metadata, computes immutable identity, classifies container/executable format, invokes pluggable static analyzers, and returns deterministic evidence.

Wardnet or another authorized consumer owns maliciousness verdicts and response policy. Never add a `malicious: bool` shortcut to this runtime.

Canonical references:

- `docs/PRD.md`
- `docs/TRD.md`
- `docs/ARCHITECTURE.md`
- `docs/SECURITY.md`
- `docs/THREAT_MODEL.md`
- `docs/TEST_STRATEGY.md`
- `docs/OPERABILITY.md`
- `docs/doctoring/STANDARD_TRACEABILITY.md`
