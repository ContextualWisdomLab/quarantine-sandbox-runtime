# ADR 0004: Deterministic attributable evidence

- Status: Accepted
- Date: 2026-08-17

## Context

SOC automation, re-analysis, dispute resolution, and model evaluation require evidence to be attributable and reproducible.

## Decision

Job IDs derive from request ID, requested profile, and artifact SHA-256. Evidence records receive one-based deterministic sequence numbers and IDs. Each record identifies its producer. Analyzer failures become evidence.

## Consequences

- Same inputs are idempotency-friendly.
- Evidence order remains stable across serialization.
- A future attestation signer can sign the canonical bundle.
- Wall-clock timestamps belong in the host event envelope, not deterministic core output.
