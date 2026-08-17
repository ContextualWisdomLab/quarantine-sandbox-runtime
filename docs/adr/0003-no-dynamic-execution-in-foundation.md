# ADR 0003: No dynamic execution in the foundation

- Status: Accepted
- Date: 2026-08-17

## Context

Dynamic detonation requires hypervisor isolation, host firewalling, disposable storage, telemetry, platform-specific workers, capacity control, and escape testing. Pretending those controls exist in an initial library would create a dangerous false boundary.

## Decision

The foundation performs no artifact execution and no network access. Linux or Windows dynamic requests return static foundation evidence with `inconclusive` and `dynamic_analysis_not_configured`.

## Consequences

- The first PR is safely reviewable and useful as a contract base.
- Dynamic workers require separate ADRs and infrastructure PRs.
- Runtime attestation makes the missing capability machine-readable.
