# Quarantine Sandbox Runtime

Quarantine Sandbox Runtime is ContextualWisdomLab's reusable hostile-workload isolation and evidence runtime for security and agent products.

## Product responsibility

The runtime owns sandbox admission, lifecycle, resource enforcement, cleanup, effective-isolation verification, and attributable runtime evidence. Calling products retain authentication, authorization, secrets, user-facing actions, maliciousness verdicts, incidents, and product-specific workflow authority.

The active stack is still a release candidate. Branch-only code, fake-process tests, or source-level policy are not substitutes for protected integration, real hostile-runner evidence, and immutable release artifacts.

## Start here

- [Repository README](../README.md) — product boundary, supported use cases, current capabilities, and security defaults.
- [Product requirements](PRD.md) — buyer and product requirements.
- [Technical requirements](TRD.md) — runtime requirements and constraints.
- [Architecture](ARCHITECTURE.md) — bounded contexts and adapter architecture.
- [Security](SECURITY.md) and [threat model](THREAT_MODEL.md) — isolation and trust boundaries.
- [Operability](OPERABILITY.md) — runtime and operator guidance.
- [Product and technical gap baseline](product-technical-gap-baseline.md) — current gaps and acceptance evidence.
- [Architecture decisions](adr/) — accepted decisions.
- [Standards traceability](doctoring/STANDARD_TRACEABILITY.md) — standards and evidence mapping.
- [GitHub Releases](https://github.com/ContextualWisdomLab/quarantine-sandbox-runtime/releases) — immutable release artifacts when available.
- [Ask DeepWiki](https://deepwiki.com/ContextualWisdomLab/quarantine-sandbox-runtime) — repository-aware navigation and questions.

## Integration boundary

Consumers integrate through reviewed versioned contracts or Anti-Corruption Layers. They should not copy container-runtime implementation details, depend directly on Podman/containerd as a domain API, pass ambient product credentials into hostile workloads, or read another product's application tables to infer sandbox state.

## Release and verification

A production or commercial release requires exact protected-source identity, locked packaging, required statement and branch coverage, software-bill-of-materials and provenance evidence, real rootless-container isolation acceptance on the designated hostile-workload runner, cleanup verification, and then-current review/security governance. Historical or predecessor evidence is not transferred to a changed head.

## Publication status

This file is a GitHub Pages source prerequisite, not proof that Pages is live. Publication is complete only after the reviewed source reaches the protected default branch, the organization-owned repository metadata reconciler applies the intended Pages configuration, deployment succeeds, and the public HTTPS content is re-read successfully.
