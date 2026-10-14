# Quarantine Sandbox Runtime

[![Ask DeepWiki](https://deepwiki.com/badge.svg)](https://deepwiki.com/ContextualWisdomLab/quarantine-sandbox-runtime)

Quarantine Sandbox Runtime is the ContextualWisdomLab runtime for credential-free hostile-workload isolation and deterministic artifact-analysis evidence. It owns sandbox execution, lifecycle, resource limits, cleanup, and isolation evidence; consuming products retain authorization, verdict, incident, task, secret, and user-action authority.

## Start here

- [Product requirements](PRD.md)
- [Technical requirements](TRD.md)
- [Architecture](ARCHITECTURE.md)
- [Security](SECURITY.md)
- [Threat model](THREAT_MODEL.md)
- [Test strategy](TEST_STRATEGY.md)
- [Operability](OPERABILITY.md)
- [Product and technical gap baseline](product-technical-gap-baseline.md)
- [Architecture decisions](adr/README.md)
- [Standards traceability](doctoring/STANDARD_TRACEABILITY.md)

## Product boundary

The runtime provides reusable isolation and evidence contracts for two bounded consumer scenarios: hostile artifact analysis and short-lived isolated application services. Wardnet and other security products own maliciousness verdicts and response decisions. `contextual-orchestrator` and other Agent control planes own conversation, task/tool policy, authorization, image selection, secrets, and user-visible actions. Consumers integrate through released versioned contracts rather than sibling source or container-engine implementation details.

## Current status

The repository is pre-release. Active development is validating effective rootless isolation, bounded command execution, release provenance, and consumer-safe contracts. Draft pull requests and branch-local documentation are proposals until integrated into the protected default branch. GitHub Pages publication must not be inferred from this source file alone.

## Development

The implementation is Rust-first. From the repository root, the standard local verification entry points are:

```bash
cargo fmt --check
cargo test --locked --workspace --all-targets
cargo clippy --locked --workspace --all-targets -- -D warnings
RUSTDOCFLAGS="-D warnings" cargo doc --locked --workspace --no-deps
python3 scripts/validate_repository.py
```

Release, deployment, and supported-consumer claims are authoritative only when backed by the exact integrated source, repository governance, and immutable release evidence documented in the linked product and operability material.
