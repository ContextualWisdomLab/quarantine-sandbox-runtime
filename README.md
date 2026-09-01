# Quarantine Sandbox Runtime

[![Ask DeepWiki](https://deepwiki.com/badge.svg)](https://deepwiki.com/ContextualWisdomLab/quarantine-sandbox-runtime)

**Credential-free hostile-workload isolation and evidence runtime for ContextualWisdomLab products.**

Quarantine Sandbox Runtime gives security and agent products a reusable place to run untrusted work **without moving product credentials, verdict authority, or user-facing policy into the sandbox**. It owns isolation lifecycle and evidence; the calling product keeps the decisions that belong to its domain.

> This README describes the current candidate stack. Protected integration history remains shipped authority until the stacked PRs, exact-head security/runtime gates, and release evidence complete normally.

## Why it exists

Untrusted artifacts and short-lived application workloads need stronger boundaries than an ordinary subprocess, but every consumer should not build its own container lifecycle, cleanup, attestation, and resource-control code.

Quarantine Sandbox Runtime provides a shared isolation boundary for two explicit use cases:

| Consumer job | Runtime responsibility | Consumer retains |
| --- | --- | --- |
| Artifact analysis | Admit hostile bytes, classify/analyze them, return attributable evidence | Maliciousness verdicts, incidents, quarantine/block/notification/retention policy |
| Isolated application service | Launch an approved immutable image with bounded resources, readiness, lease, cleanup, and attestation | Conversation/agent/task/tool policy, authorization, image selection, secrets, user-visible actions |

## Product boundary

The Core bounded context is `sandbox_execution`. `artifact_analysis` and `application_service` are separate Supporting contexts; Podman and future container backends stay behind infrastructure adapters.

```text
            consumer-owned intent / bytes
                       │
                       ▼
          ┌─────────────────────────┐
          │ Quarantine Sandbox      │
          │ Runtime                 │
          │                         │
          │ isolation · lifecycle   │
          │ resources · cleanup     │
          │ evidence · attestation  │
          └───────────┬─────────────┘
                      │
                 bounded evidence
          ┌───────────┴─────────────┐
          ▼                         ▼
       Wardnet             contextual-orchestrator
   verdict / response      task / auth / secrets
```

Consumers integrate through versioned contracts or Anti-Corruption Layers. They do not copy the sandbox implementation or depend on Podman/containerd details as domain APIs.

## Current capabilities

### Artifact analysis

The current candidate supports immutable SHA-256 artifact identity, bounded in-memory ingestion, deterministic common-format classification, versioned request/evidence contracts, pluggable static analyzers, attributable analyzer failures, and fail-closed `inconclusive` results when requested dynamic evidence is unavailable.

Static foundation evidence explicitly distinguishes “content was inspected” from “content was executed.” It does not claim a maliciousness verdict on behalf of Wardnet or another consumer.

### Isolated application service

The current candidate accepts immutable OCI image digests and uses rootless Podman as the first backend. The P0 policy includes no implicit image pull, private/loopback-only networking, read-only root filesystem, bounded tmpfs, dropped Linux capabilities, `no-new-privileges`, isolated namespaces, numeric non-root identity, explicit CPU/RAM/PID/time/readiness/shutdown limits, and no ambient consumer credentials.

The stacked application-service work also scopes idempotency and cleanup by caller-derived lease ownership rather than trusting an owner string in application payloads.

### Effective isolation evidence

The active stack adds verification that leases are derived from observed effective isolation rather than configuration intent alone. Backend inspection and cleanup evidence remain part of the sandbox authority boundary; consumer verdicts and authorization do not.

## Security defaults

The default application-service profile is deliberately restrictive:

- immutable image digest; no implicit registry pull;
- no arbitrary Internet egress;
- no credential injection;
- no host network/PID/IPC access;
- no privileged mode, host devices, or runtime socket;
- no arbitrary host-path mount;
- no wildcard public bind;
- no shell interpretation of application argv.

A later controlled-egress or secret-broker profile must be a separately reviewed contract. It may not weaken these defaults implicitly.

See [`docs/SECURITY.md`](docs/SECURITY.md) and [`docs/THREAT_MODEL.md`](docs/THREAT_MODEL.md) for the detailed trust model.

## Quickstart for contributors

The current crate is `quarantine-sandbox-runtime` `0.1.0`, uses Rust 2024 edition, and requires Rust 1.97+.

```bash
cargo fmt --check
cargo test --locked --workspace --all-targets
cargo clippy --locked --workspace --all-targets -- -D warnings
RUSTDOCFLAGS="-D warnings" cargo doc --locked --workspace --no-deps
python3 scripts/validate_repository.py
```

There is not yet a published stable package, image, network API, or consumer transport to install from a release channel. Consumer handoff should wait for immutable release evidence rather than depending on an open PR checkout.

## Runtime and deployment status

Real rootless-Podman acceptance exists in the product stack, but fake-process tests and source-level container policy are not substitutes for exact-head host isolation evidence. The dedicated hostile-workload runner lane is the authority for real isolation behavior.

The current tail PR is building a **fail-closed commercial release contract**: locked packaging, full owned statement/branch coverage, SPDX SBOM, checksums, artifact attestation, release publication, and real isolation acceptance are required before publication. That gate is candidate work; no tag, immutable package/image, or production release is claimed by this README.

## Quality contract

The repository enforces missing public documentation and forbids unsafe Rust in the package metadata/lint boundary. Its tests cover domain invariants, process/backend behavior, cleanup, ownership/idempotency, and hostile isolation cases, with separate real-container acceptance where environment capability matters.

Do not translate those engineering gates into unsupported customer, certification, performance, or deployment claims. Current gaps and evidence state live in [`docs/product-technical-gap-baseline.md`](docs/product-technical-gap-baseline.md).

## Documentation map

- [`docs/index.md`](docs/index.md) — public documentation home.
- [`docs/PRD.md`](docs/PRD.md) — product requirements and buyer boundary.
- [`docs/TRD.md`](docs/TRD.md) — technical requirements.
- [`docs/ARCHITECTURE.md`](docs/ARCHITECTURE.md) — runtime architecture and adapter boundary.
- [`docs/SECURITY.md`](docs/SECURITY.md) / [`docs/THREAT_MODEL.md`](docs/THREAT_MODEL.md) — security posture and threats.
- [`docs/TEST_STRATEGY.md`](docs/TEST_STRATEGY.md) — executable verification strategy.
- [`docs/OPERABILITY.md`](docs/OPERABILITY.md) — runtime and operational guidance.
- [`docs/product-technical-gap-baseline.md`](docs/product-technical-gap-baseline.md) — current gaps and acceptance evidence.
- [`docs/adr/README.md`](docs/adr/README.md) — architecture decisions.
- [`docs/doctoring/STANDARD_TRACEABILITY.md`](docs/doctoring/STANDARD_TRACEABILITY.md) — standards traceability.

## Contributing

Keep hostile execution, lifecycle, resource enforcement, cleanup, and attestation here. Keep consumer authentication, secrets, user actions, maliciousness verdicts, and product-specific workflow authority in their owning repositories.

New software must permit commercial use under the intended distribution model and retain required provenance and attribution. Update tests, security/architecture documentation, and customer-facing claims together when an isolation contract changes.

## License

Quarantine Sandbox Runtime is licensed under the [MIT License](LICENSE).
