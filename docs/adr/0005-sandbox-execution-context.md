# ADR 0005: Sandbox execution is the Core bounded context

- **Status:** Accepted
- **Date:** 2026-09-01

## Context

The earlier repository layout flattened artifact contracts, ingestion, and runtime orchestration at the crate root. Adding Agent application serving without a domain split would turn a malware-oriented module into an unrelated generic runtime bucket.

## Decision

The bounded contexts are:

### Core — `sandbox_execution`

Owns isolation policy, resource budget, sandbox lifecycle/lease, endpoint receipt, termination/cleanup, runtime attestation, and backend ports.

### Supporting — `artifact_analysis`

Owns artifact identity, bounded source context, ingestion, analysis profile, analyzer evidence, and analysis completeness. Dynamic detonation consumes Core isolation rather than owning container lifecycle.

### Supporting — `application_service`

Owns the consumer-neutral intent to start one approved immutable application image as a short-lived service and return an attested loopback lease. It does not authorize which application an Agent may use.

### Infrastructure

Podman, gVisor, containerd, Kubernetes RuntimeClass, VM pools, packet capture, and similar provider/runtime mechanisms are adapters. Their DTOs and command syntax are not domain entities.

## Dependency direction

```text
artifact_analysis -----> sandbox_execution <----- application_service
                              ^
                              |
                       infrastructure adapters
```

External consumers depend on public contracts/ACLs rather than internal modules.

## Consequences

The active implementation moves existing artifact files under `src/artifact_analysis/` and introduces `src/sandbox_execution/` plus `src/application_service/`. Architectural fitness checks must prevent generic root modules or cross-context backend leakage from returning.
