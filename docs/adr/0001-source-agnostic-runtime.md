# ADR 0001: Source-agnostic runtime boundary

- Status: Accepted
- Date: 2026-08-17

## Context

Suspicious artifacts arrive through email, uploads, connectors, APIs, and repository collaboration. Binding analysis to one trigger would duplicate the highest-risk code and make evidence inconsistent.

## Decision

The core API accepts artifact bytes, an artifact name, and bounded source context. Trigger-specific behavior remains in adapters outside this repository.

## Consequences

- Wardnet, Naruon, upload services, and future connectors can consume one contract.
- The core does not authenticate GitHub webhooks, parse mailboxes, or call source-system APIs.
- Consumer-specific response policy remains outside the runtime.
