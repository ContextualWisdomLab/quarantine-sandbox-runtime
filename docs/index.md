# Quarantine Sandbox Runtime

Quarantine Sandbox Runtime is the shared isolation and evidence boundary for hostile artifacts and short-lived application workloads used by ContextualWisdomLab products. It owns sandbox lifecycle, resource enforcement, cleanup, and attributable runtime evidence while consuming products retain authentication, credentials, verdicts, user-facing actions, and domain policy.

[Ask DeepWiki](https://deepwiki.com/ContextualWisdomLab/quarantine-sandbox-runtime)

## Start here

- [Repository overview](../README.md) — product value, current capabilities, quickstart, maturity, and release truth.
- [Product requirements](PRD.md) — supported jobs, buyer outcomes, and non-goals.
- [Technical requirements](TRD.md) — executable technical constraints.
- [Architecture](ARCHITECTURE.md) — bounded contexts and backend adapter boundary.
- [Security](SECURITY.md) and [threat model](THREAT_MODEL.md) — isolation assumptions and failure posture.
- [Test strategy](TEST_STRATEGY.md) — deterministic and real-runtime verification lanes.
- [Operability](OPERABILITY.md) — runtime and operational guidance.
- [Architecture decisions](adr/README.md) — accepted design decisions.
- [Current product and technical gaps](product-technical-gap-baseline.md) — evidence-bound remaining work.

## Product boundary

The runtime has one Core bounded context, `sandbox_execution`, and Supporting contexts for artifact analysis and isolated application services. Infrastructure details such as rootless Podman remain behind adapters. Consumers integrate through versioned contracts and keep business authority outside the sandbox.

```text
consumer intent / hostile bytes
            │
            ▼
  Quarantine Sandbox Runtime
  isolation · resources · lifecycle
  cleanup · evidence · attestation
            │
            ▼
  bounded evidence back to caller
```

The runtime does not decide whether an artifact is malicious, authorize user actions, own LLM/provider credentials, or turn sandbox evidence into enterprise truth.

## Contributor quickstart

The current candidate crate is `quarantine-sandbox-runtime` `0.1.0`, Rust 2024 edition, with Rust 1.97+.

```bash
cargo fmt --check
cargo test --locked --workspace --all-targets
cargo clippy --locked --workspace --all-targets -- -D warnings
RUSTDOCFLAGS="-D warnings" cargo doc --locked --workspace --no-deps
python3 scripts/validate_repository.py
```

Real rootless-container isolation evidence is collected separately from fake-process and source-level tests; neither substitutes for the other.

## Release status

There is no published stable package, immutable image, or production consumer transport yet. The active release work is establishing a fail-closed contract requiring exact protected source identity, locked packaging, full owned coverage, SPDX SBOM, checksums, artifact attestation, release publication, and real hostile-runtime acceptance. Until those gates complete and an immutable release is published, source and open pull requests remain candidate evidence rather than a shipped release.

## Security posture

Default application-service isolation is intentionally restrictive: immutable image digests, no implicit pulls, no arbitrary Internet egress, no credential injection, no host namespace access, no privileged mode or runtime socket, no arbitrary host-path mounts, loopback/private networking, read-only roots, dropped capabilities, `no-new-privileges`, and bounded CPU/RAM/PID/time/readiness/shutdown behavior.

Changes that weaken those defaults require an explicit reviewed contract rather than an implicit exception.
