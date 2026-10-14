# ADR 0003: Published-contract consumption

- **Status:** Accepted
- **Date:** 2026-09-01

## Context

CWL products must remain independently buildable and deployable. Sibling checkouts, copied source, unmerged branch file imports, or consumer code that directly knows Podman/containerd flags create a distributed monolith and weaken supply-chain provenance.

## Decision

Consumers integrate through versioned public contracts and immutable artifacts.

- During development, an exact full Git commit SHA may be used for review/evidence only.
- Production consumers use a released package/container artifact pinned by immutable content digest and verified provenance.
- Consumer repositories define an Anti-Corruption Layer that translates their domain request into the runtime request and translates runtime lease/evidence back into their domain language.
- Consumer domain code never imports backend-specific Podman/gVisor/containerd DTOs or shells out to those runtimes as a shortcut.
- No cross-service direct application-table SQL is allowed.

## Consequences

Integration may lag runtime implementation until an artifact is published. That is preferable to creating hidden source coupling. Schema evolution requires explicit version compatibility rather than silently rewriting consumer data.
